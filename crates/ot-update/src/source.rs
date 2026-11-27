//! Where this copy looks for newer releases (FEAT-0077, ADR-024).
//!
//! OmniToaster is published from two forges — the self-hosted Forgejo at
//! `git.zacx.net` and a GitHub mirror of the same history — and each builds its
//! own release. A copy updates from the forge it was built for, so a GitHub
//! download looks at GitHub and a git.zacx.net one at git.zacx.net, with nothing
//! to detect and nothing for a user to set.
//!
//! The repository is fixed **when the program is built**, by the release script
//! that builds it, through `OMNITOASTER_RELEASE_REPO`; a build with nothing set,
//! which is every local build, reads the Forgejo. It is deliberately not a
//! setting: a setting could be pointed anywhere by anyone who can edit the file.
//! (A release is also signed, so a hostile source could not install anything —
//! FEAT-0078 — but the build is still the only input trusted about *where* to
//! look.)

/// The repository this copy's releases live in, as the web URL a person would
/// open: `https://git.zacx.net/owner/omni-toaster` or
/// `https://github.com/mzaki-raup/omni-toaster`.
pub const RELEASE_REPO: &str = match option_env!("OMNITOASTER_RELEASE_REPO") {
    Some(url) => url,
    None => DEFAULT_REPO,
};

/// The repository again, as a marker only this build's own repository appears in
/// (`[omnitoaster-update-source=<repository>]`), kept in the finished exe so the
/// release check can prove it was built for the forge it is published on
/// (`scripts/verify-release.ps1`, FEAT-0081). `used` keeps the compiler from
/// dropping a string nothing else reads.
#[used]
pub static SOURCE_MARKER: &str = env!("OMNITOASTER_UPDATE_SOURCE_MARKER");

/// The Forgejo, where development happens and where a build with nothing set looks.
pub const DEFAULT_REPO: &str = "https://git.zacx.net/owner/omni-toaster";

/// Which kind of forge answers, which decides only the shape of the API's URL:
/// both answer the release in the same JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Forge {
    Forgejo,
    GitHub,
}

/// Why a repository URL cannot be used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SourceError {
    #[error("{0:?} is not an https URL of the form https://host/owner/repo")]
    NotARepo(String),
}

/// A repository, taken apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repo {
    pub host: String,
    pub owner: String,
    pub name: String,
    pub forge: Forge,
}

impl Repo {
    /// Read `https://host/owner/repo`, with or without a trailing slash or `.git`.
    /// Anything else — plain HTTP, a missing owner, extra path — is refused, so a
    /// malformed build setting fails its own test rather than shipping a copy
    /// that can never update.
    pub fn parse(url: &str) -> Result<Self, SourceError> {
        let bad = || SourceError::NotARepo(url.to_owned());
        let rest = url.strip_prefix("https://").ok_or_else(bad)?;
        let rest = rest.trim_end_matches('/');
        let rest = rest.strip_suffix(".git").unwrap_or(rest);
        let mut parts = rest.split('/');
        let host = parts.next().filter(|h| !h.is_empty()).ok_or_else(bad)?;
        let owner = parts.next().filter(|o| !o.is_empty()).ok_or_else(bad)?;
        let name = parts.next().filter(|n| !n.is_empty()).ok_or_else(bad)?;
        if parts.next().is_some() {
            return Err(bad());
        }
        let clean = |part: &str| {
            part.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        };
        let host_ok = host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | ':'));
        if !(host_ok && clean(owner) && clean(name)) {
            return Err(bad());
        }
        Ok(Self {
            host: host.to_owned(),
            owner: owner.to_owned(),
            name: name.to_owned(),
            forge: if host.eq_ignore_ascii_case("github.com") {
                Forge::GitHub
            } else {
                Forge::Forgejo
            },
        })
    }

    /// The web address of the repository, for showing the user where updates come from.
    pub fn web_url(&self) -> String {
        format!("https://{}/{}/{}", self.host, self.owner, self.name)
    }

    /// The API address of the newest published, non-draft, non-prerelease release.
    /// GitHub's API lives on `api.github.com`; Forgejo's beside the web UI.
    pub fn latest_release_api(&self) -> String {
        match self.forge {
            Forge::GitHub => format!(
                "https://api.github.com/repos/{}/{}/releases/latest",
                self.owner, self.name
            ),
            Forge::Forgejo => format!(
                "https://{}/api/v1/repos/{}/{}/releases/latest",
                self.host, self.owner, self.name
            ),
        }
    }
}

/// The repository this copy was built for.
pub fn this_repo() -> Repo {
    Repo::parse(RELEASE_REPO).unwrap_or_else(|error| {
        // A build whose release script set a malformed value: say so loudly and
        // fall back to the Forgejo rather than a copy that can never update.
        tracing::error!(%error, "the baked release repository is unusable; using the default");
        Repo::parse(DEFAULT_REPO).expect("the default repository is well-formed")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_forgejo_repository_reads_its_api_beside_the_web_ui() {
        let repo = Repo::parse("https://git.zacx.net/owner/omni-toaster").expect("parses");
        assert_eq!(repo.forge, Forge::Forgejo);
        assert_eq!(
            repo.latest_release_api(),
            "https://git.zacx.net/api/v1/repos/owner/omni-toaster/releases/latest"
        );
        assert_eq!(repo.web_url(), "https://git.zacx.net/owner/omni-toaster");
    }

    #[test]
    fn a_github_repository_reads_its_api_on_the_api_host() {
        let repo = Repo::parse("https://github.com/mzaki-raup/omni-toaster").expect("parses");
        assert_eq!(repo.forge, Forge::GitHub);
        assert_eq!(
            repo.latest_release_api(),
            "https://api.github.com/repos/mzaki-raup/omni-toaster/releases/latest"
        );
    }

    #[test]
    fn a_trailing_slash_or_dot_git_is_tolerated() {
        for url in [
            "https://github.com/mzaki-raup/omni-toaster/",
            "https://github.com/mzaki-raup/omni-toaster.git",
            "https://github.com/mzaki-raup/omni-toaster.git/",
        ] {
            let repo = Repo::parse(url).unwrap_or_else(|e| panic!("{url}: {e}"));
            assert_eq!(
                (repo.owner.as_str(), repo.name.as_str()),
                ("mzaki-raup", "omni-toaster")
            );
        }
    }

    #[test]
    fn anything_but_an_https_owner_and_repo_is_refused() {
        for url in [
            "http://git.zacx.net/owner/omni-toaster",
            "git.zacx.net/owner/omni-toaster",
            "https://git.zacx.net/owner",
            "https://git.zacx.net/owner/omni-toaster/extra",
            "https://git.zacx.net//omni-toaster",
            "https://git.zacx.net/owner/omni toaster",
            "https://git.zacx.net/owner/..%2f",
            "https://evil.example/owner/x?y=1",
            "",
        ] {
            assert!(Repo::parse(url).is_err(), "{url:?} should be refused");
        }
    }

    #[test]
    fn this_build_is_for_a_forge_and_names_this_repository() {
        let repo = this_repo();
        assert_eq!(repo.name, "omni-toaster");
        assert!(repo.latest_release_api().starts_with("https://"));
    }

    /// The marker the release check looks for names this build's repository, and
    /// build.rs's own default has not drifted from `DEFAULT_REPO`.
    #[test]
    fn the_source_marker_names_the_repository_this_build_reads() {
        assert_eq!(
            SOURCE_MARKER,
            format!("[omnitoaster-update-source={RELEASE_REPO}]")
        );
    }

    /// A local build must read the Forgejo. If this fails, something in the
    /// environment set `OMNITOASTER_RELEASE_REPO`, and every binary built from this
    /// machine would update from wherever that points.
    #[test]
    fn a_build_with_nothing_set_reads_the_forgejo() {
        if option_env!("OMNITOASTER_RELEASE_REPO").is_none() {
            assert_eq!(RELEASE_REPO, DEFAULT_REPO);
            assert_eq!(this_repo().host, "git.zacx.net");
        }
    }
}
