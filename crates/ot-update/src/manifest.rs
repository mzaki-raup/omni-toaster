//! The manifest a release publishes, read strictly (FEAT-0078).
//!
//! `manifest.json` is what the installer trusts — once its signature has been
//! checked (`verify`) — so it is read with every field validated and an unknown or
//! malformed one refused. The asset the updater installs is `windows-x64`: the bare
//! `OmniToaster.exe`, with the size and SHA-256 to hold the download to.

use semver::Version;
use serde_json::Value;

/// A manifest is a few hundred bytes; this is generous and stops an endless answer.
pub const MAX_MANIFEST: usize = 64 * 1024;

/// The most the program may be: it is about sixteen megabytes today.
pub const MAX_EXE: u64 = 64 * 1024 * 1024;

/// The key of the bare executable in the manifest's `assets`.
pub const EXE_KEY: &str = "windows-x64";

/// One file the manifest describes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetEntry {
    pub name: String,
    pub url: String,
    pub size: u64,
    /// Lower-case hex, 64 characters.
    pub sha256: String,
}

/// What a release says about itself, once trusted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub version: Version,
    pub channel: String,
    /// The oldest version that may update in place to this one; older copies are
    /// sent to the zip (`docs/specs/updater.md` §3).
    pub min_supported: Version,
    pub notes_url: String,
    pub exe: AssetEntry,
}

/// Why a manifest cannot be used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ManifestError {
    #[error("the manifest is over the {MAX_MANIFEST}-byte limit")]
    TooLarge,
    #[error("the manifest is not JSON: {0}")]
    NotJson(String),
    #[error("the manifest is not valid: {0}")]
    Invalid(String),
}

fn invalid<T>(what: impl Into<String>) -> Result<T, ManifestError> {
    Err(ManifestError::Invalid(what.into()))
}

fn text<'a>(object: &'a Value, key: &str) -> Result<&'a str, ManifestError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ManifestError::Invalid(format!("{key} is missing or not text")))
}

fn version(object: &Value, key: &str) -> Result<Version, ManifestError> {
    let raw = text(object, key)?;
    Version::parse(raw).map_err(|error| ManifestError::Invalid(format!("{key} {raw:?}: {error}")))
}

fn entry(assets: &Value, key: &str, require_https: bool) -> Result<AssetEntry, ManifestError> {
    let item = assets
        .get(key)
        .ok_or_else(|| ManifestError::Invalid(format!("there is no {key} asset")))?;
    let name = text(item, "name")?.to_owned();
    let url = text(item, "url")?.to_owned();
    if require_https && !url.starts_with("https://") {
        return invalid(format!("the {key} url is not https"));
    }
    let size = item
        .get("size")
        .and_then(Value::as_u64)
        .filter(|size| (1..=MAX_EXE).contains(size))
        .ok_or_else(|| {
            ManifestError::Invalid(format!("the {key} size is missing or out of range"))
        })?;
    let sha256 = text(item, "sha256")?.to_owned();
    if sha256.len() != 64
        || !sha256
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    {
        return invalid(format!("the {key} sha256 is not 64 lower-case hex digits"));
    }
    Ok(AssetEntry {
        name,
        url,
        size,
        sha256,
    })
}

/// Read a manifest's bytes. Call it only on bytes whose signature has been checked.
pub fn parse(bytes: &[u8]) -> Result<Manifest, ManifestError> {
    parse_with(bytes, true)
}

/// [`parse`], with HTTPS optionally not required: only a test's local server is
/// plain HTTP, and nothing in the app passes `false`.
pub(crate) fn parse_with(bytes: &[u8], require_https: bool) -> Result<Manifest, ManifestError> {
    if bytes.len() > MAX_MANIFEST {
        return Err(ManifestError::TooLarge);
    }
    let root: Value =
        serde_json::from_slice(bytes).map_err(|error| ManifestError::NotJson(error.to_string()))?;
    if root.get("schema").and_then(Value::as_u64) != Some(1) {
        return invalid("the schema is not 1");
    }
    if text(&root, "product")? != "OmniToaster" {
        return invalid("the product is not OmniToaster");
    }
    let assets = root
        .get("assets")
        .filter(|assets| assets.is_object())
        .ok_or_else(|| ManifestError::Invalid("there are no assets".into()))?;
    let exe = entry(assets, EXE_KEY, require_https)?;
    // The bare executable has one name, and it never carries a path: it is what
    // is written beside the running program.
    if exe.name != crate::release::EXE {
        return invalid(format!("the {EXE_KEY} asset is named {:?}", exe.name));
    }
    Ok(Manifest {
        version: version(&root, "version")?,
        channel: text(&root, "channel")?.to_owned(),
        min_supported: version(&root, "min_supported_version")?,
        notes_url: text(&root, "notes_url")?.to_owned(),
        exe,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real() -> Vec<u8> {
        std::fs::read(format!(
            "{}/tests/fixtures/real-0.18.0/manifest.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .expect("the fixture")
    }

    #[test]
    fn the_real_manifest_of_a_published_release_reads() {
        let manifest = parse(&real()).expect("reads");
        assert_eq!(manifest.version, Version::new(0, 18, 0));
        assert_eq!(manifest.min_supported, Version::new(0, 1, 0));
        assert_eq!(manifest.channel, "stable");
        assert_eq!(manifest.exe.name, "OmniToaster.exe");
        assert_eq!(manifest.exe.size, 16_173_568);
        assert_eq!(
            manifest.exe.sha256,
            "9c55fa26c44709f58c010aa1bf7dbd1c88e76169f3c5e34d32962ee9c75b6b02"
        );
        assert!(manifest.exe.url.starts_with("https://git.zacx.net/"));
    }

    /// The real manifest with one thing changed, as text.
    fn with(from: &str, to: &str) -> Vec<u8> {
        let text = String::from_utf8(real()).expect("utf8");
        assert!(text.contains(from), "{from} is in the fixture");
        text.replacen(from, to, 1).into_bytes()
    }

    #[test]
    fn a_manifest_that_is_not_exactly_what_a_release_publishes_is_refused() {
        let cases = [
            ("schema", with("\"schema\": 1", "\"schema\": 2")),
            ("product", with("\"OmniToaster\",", "\"Other\",")),
            (
                "version",
                with("\"version\": \"0.18.0\"", "\"version\": \"eighteen\""),
            ),
            (
                "no https",
                with(
                    "\"url\": \"https://git.zacx.net/owner/omni-toaster/releases/download/v0.18.0/OmniToaster.exe\"",
                    "\"url\": \"http://git.zacx.net/x/OmniToaster.exe\"",
                ),
            ),
            ("size zero", with("\"size\": 16173568", "\"size\": 0")),
            (
                "size huge",
                with("\"size\": 16173568", "\"size\": 99999999999"),
            ),
            (
                "short hash",
                with(
                    "9c55fa26c44709f58c010aa1bf7dbd1c88e76169f3c5e34d32962ee9c75b6b02",
                    "9c55",
                ),
            ),
            (
                "upper hash",
                with(
                    "9c55fa26c44709f58c010aa1bf7dbd1c88e76169f3c5e34d32962ee9c75b6b02",
                    "9C55FA26C44709F58C010AA1BF7DBD1C88E76169F3C5E34D32962EE9C75B6B02",
                ),
            ),
            (
                "exe renamed",
                with(
                    "\"name\": \"OmniToaster.exe\"",
                    "\"name\": \"..\\\\evil.exe\"",
                ),
            ),
            ("no assets", with("\"windows-x64\": {", "\"other\": {")),
            ("not json", b"{".to_vec()),
            ("empty", Vec::new()),
        ];
        for (what, bytes) in cases {
            assert!(parse(&bytes).is_err(), "{what} should be refused");
        }
    }

    #[test]
    fn an_endless_manifest_is_refused_by_size() {
        assert_eq!(
            parse(&vec![b' '; MAX_MANIFEST + 1]),
            Err(ManifestError::TooLarge)
        );
    }
}
