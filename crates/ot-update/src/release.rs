//! Reading a release the forge describes, and comparing versions (FEAT-0077).
//!
//! Forgejo copied GitHub's releases API and kept it, so one reader covers both:
//! `tag_name`, `draft`, `prerelease`, `html_url` and a list of `assets`, each with
//! a `name`, a `size` and a `browser_download_url`. Nothing about a release is
//! trusted yet — the version here is a hint for the user; what is installed is
//! decided by the signed manifest (FEAT-0078).

use semver::Version;
use serde_json::Value;

/// The three assets a release must carry for the app to install it
/// (`docs/specs/updater.md` §3): a release missing any of them reads as
/// "no installable release", and the user stays where they are.
pub const MANIFEST: &str = "manifest.json";
pub const MANIFEST_SIGNATURE: &str = "manifest.json.minisig";
pub const EXE: &str = "OmniToaster.exe";

/// The most a release description may be: a few KB in practice.
pub const MAX_JSON: usize = 1 << 20;

/// One file attached to a release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
}

/// A published release, as the forge describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: Version,
    pub tag: String,
    /// The release's page in the browser, for the notes.
    pub page_url: String,
    pub assets: Vec<Asset>,
}

impl Release {
    pub fn asset(&self, name: &str) -> Option<&Asset> {
        self.assets.iter().find(|asset| asset.name == name)
    }

    /// Whether it carries what the app needs to install it.
    pub fn installable(&self) -> bool {
        [MANIFEST, MANIFEST_SIGNATURE, EXE]
            .iter()
            .all(|name| self.asset(name).is_some())
    }
}

/// Why a release description could not be used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReleaseError {
    #[error("the answer is not JSON: {0}")]
    NotJson(String),
    #[error("the answer is not a release: {0}")]
    Shape(String),
    #[error("the release is a draft")]
    Draft,
    #[error("the release is a prerelease, and only stable ones are offered")]
    Prerelease,
}

/// The version a tag names: `v0.18.0` and `0.18.0` both.
pub fn version_of_tag(tag: &str) -> Result<Version, ReleaseError> {
    let bare = tag.strip_prefix('v').unwrap_or(tag);
    Version::parse(bare)
        .map_err(|error| ReleaseError::Shape(format!("{tag:?} is not a version: {error}")))
}

/// Read the forge's answer for a release.
pub fn parse(body: &[u8]) -> Result<Release, ReleaseError> {
    let root: Value =
        serde_json::from_slice(body).map_err(|error| ReleaseError::NotJson(error.to_string()))?;
    let text = |key: &str| root.get(key).and_then(Value::as_str);
    let flag = |key: &str| root.get(key).and_then(Value::as_bool).unwrap_or(false);
    if flag("draft") {
        return Err(ReleaseError::Draft);
    }
    if flag("prerelease") {
        return Err(ReleaseError::Prerelease);
    }
    let tag = text("tag_name").ok_or_else(|| ReleaseError::Shape("there is no tag_name".into()))?;
    let version = version_of_tag(tag)?;
    // A stable offer carries no prerelease part, whatever the forge's flag said.
    if !version.pre.is_empty() {
        return Err(ReleaseError::Prerelease);
    }
    let page_url = text("html_url").unwrap_or_default().to_owned();
    let mut assets = Vec::new();
    for entry in root
        .get("assets")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice)
    {
        let (Some(name), Some(url)) = (
            entry.get("name").and_then(Value::as_str),
            entry.get("browser_download_url").and_then(Value::as_str),
        ) else {
            continue;
        };
        // Only HTTPS is ever fetched.
        if !url.starts_with("https://") {
            continue;
        }
        assets.push(Asset {
            name: name.to_owned(),
            url: url.to_owned(),
            size: entry.get("size").and_then(Value::as_u64).unwrap_or(0),
        });
    }
    Ok(Release {
        version,
        tag: tag.to_owned(),
        page_url,
        assets,
    })
}

/// The running program's version.
pub fn running() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).expect("the workspace version is semver")
}

/// Whether `offered` is newer than `running`: by semver, never by comparing strings,
/// so `0.10.0` is newer than `0.9.0` and an equal or older release offers nothing.
pub fn is_newer(offered: &Version, running: &Version) -> bool {
    offered > running
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Vec<u8> {
        let path = format!(
            "{}/tests/fixtures/releases-2026-11-27/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
    }

    #[test]
    fn both_forges_answer_in_the_same_shape_and_both_read() {
        for (name, host) in [
            ("forgejo-latest.json", "git.zacx.net"),
            ("github-latest.json", "github.com"),
        ] {
            let release = parse(&fixture(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(release.version, Version::new(0, 18, 0), "{name}");
            assert!(
                release.page_url.contains(host),
                "{name}: {}",
                release.page_url
            );
            assert!(
                release.installable(),
                "{name} carries the three assets: {:?}",
                release.assets
            );
            for asset in &release.assets {
                assert!(asset.url.starts_with("https://"), "{}", asset.url);
                assert!(
                    asset.url.contains(host),
                    "an asset of {name} is on its own forge: {}",
                    asset.url
                );
            }
            assert!(release.asset(EXE).expect("the exe").size > 1_000_000);
        }
    }

    #[test]
    fn a_tag_with_or_without_v_is_a_version_and_nonsense_is_not() {
        assert_eq!(
            version_of_tag("v0.18.0").expect("v"),
            Version::new(0, 18, 0)
        );
        assert_eq!(
            version_of_tag("0.18.0").expect("bare"),
            Version::new(0, 18, 0)
        );
        assert!(version_of_tag("latest").is_err());
        assert!(version_of_tag("").is_err());
    }

    #[test]
    fn versions_compare_by_semver_not_by_string() {
        let v = |s: &str| Version::parse(s).expect("semver");
        assert!(
            is_newer(&v("0.10.0"), &v("0.9.0")),
            "0.10 is newer than 0.9"
        );
        assert!(is_newer(&v("1.0.0"), &v("0.99.99")));
        assert!(
            !is_newer(&v("0.18.0"), &v("0.18.0")),
            "equal offers nothing"
        );
        assert!(
            !is_newer(&v("0.17.9"), &v("0.18.0")),
            "older offers nothing"
        );
        assert!(
            is_newer(&v("0.18.0"), &v("0.18.0-beta.2")),
            "a release follows its beta"
        );
        assert!(!is_newer(&v("0.18.0-beta.1"), &v("0.18.0")));
    }

    #[test]
    fn drafts_prereleases_and_unversioned_tags_are_not_offered() {
        let release =
            |extra: &str| format!(r#"{{"tag_name":"v1.2.3","html_url":"https://x/y"{extra}}}"#);
        assert!(parse(release("").as_bytes()).is_ok());
        assert_eq!(
            parse(release(r#","draft":true"#).as_bytes()),
            Err(ReleaseError::Draft)
        );
        assert_eq!(
            parse(release(r#","prerelease":true"#).as_bytes()),
            Err(ReleaseError::Prerelease)
        );
        assert_eq!(
            parse(br#"{"tag_name":"v1.2.3-beta.1"}"#),
            Err(ReleaseError::Prerelease),
            "a beta tag is a prerelease whatever the flag says"
        );
        assert!(matches!(
            parse(br#"{"tag_name":"nightly"}"#),
            Err(ReleaseError::Shape(_))
        ));
        assert!(matches!(
            parse(br#"{"assets":[]}"#),
            Err(ReleaseError::Shape(_))
        ));
    }

    #[test]
    fn broken_answers_are_errors_not_panics() {
        for body in [
            &b""[..],
            b"<html>502</html>",
            b"[]",
            b"null",
            b"{",
            b"\xff\xfe",
        ] {
            assert!(parse(body).is_err(), "{body:?}");
        }
    }

    #[test]
    fn a_release_missing_an_asset_is_not_installable_and_plain_http_assets_are_dropped() {
        let body = br#"{"tag_name":"v1.0.0","assets":[
            {"name":"OmniToaster.exe","browser_download_url":"https://h/x/OmniToaster.exe","size":5},
            {"name":"manifest.json","browser_download_url":"http://h/x/manifest.json","size":5},
            {"name":"manifest.json.minisig","browser_download_url":"https://h/x/manifest.json.minisig"},
            {"name":"nameless"}]}"#;
        let release = parse(body).expect("reads");
        assert!(
            release.asset("manifest.json").is_none(),
            "the http one is dropped"
        );
        assert!(!release.installable());
        assert_eq!(release.asset("manifest.json.minisig").expect("sig").size, 0);
    }
}
