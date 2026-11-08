//! Where the app is allowed to write, and nowhere else.
//!
//! Omni Toaster is portable: everything it owns lives under a single data root
//! beside the executable, and nothing is written to the registry, `%APPDATA%`,
//! `%LOCALAPPDATA%` or `%TEMP%` (`docs/05-data-model-and-storage.md`,
//! ADR-004). This module is the only place that decides where that root is and
//! the only place that turns a caller's path into a real one.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

/// The folder created beside the executable when `--data-dir` is not given.
pub const DATA_DIR_NAME: &str = "OmniToasterData";

/// Subdirectories created on first run. Kept in one list so the layout in the
/// spec and the layout on disk cannot drift apart.
const SUBDIRS: &[&str] = &[
    "config",
    "data",
    "locales",
    "assets/images",
    "assets/audio/azan",
    "assets/audio/chimes",
    "assets/icons",
    "logs/crash",
    "cache",
];

/// Why the data root could not be used.
///
/// The variants a caller has to render are separate, because the user-facing
/// message differs: "this folder is read-only, move the app somewhere you can
/// write" is advice, while a failure to find the executable is a bug report.
#[derive(Debug, thiserror::Error)]
pub enum PathError {
    /// The executable's own location could not be determined, so there is no
    /// "beside the exe" to resolve.
    #[error("the executable's location could not be determined: {source}")]
    ExeLocationUnknown { source: io::Error },

    /// The data root exists but is not writable. The app must exit **without
    /// touching existing data** when this happens.
    #[error("the data folder {} is not writable: {source}", path.display())]
    NotWritable { path: PathBuf, source: io::Error },

    /// The data root could not be created.
    #[error("the data folder {} could not be created: {source}", path.display())]
    NotCreatable { path: PathBuf, source: io::Error },

    /// A relative path resolved outside the data root. Returned rather than
    /// silently clamped, because a caller asking for an outside path is a
    /// defect or an attack, not a thing to quietly fix.
    #[error("{} resolves outside the data folder", requested.display())]
    Escapes { requested: PathBuf },
}

/// The resolved, writable data root. Holding one is proof the checks passed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataRoot {
    root: PathBuf,
}

impl DataRoot {
    /// Resolve the data root, create its subtree and prove it is writable.
    ///
    /// `data_dir` is the `--data-dir` argument when the user gave one.
    /// Otherwise the root is `OmniToasterData/` beside the current executable.
    pub fn resolve(data_dir: Option<&Path>) -> Result<Self, PathError> {
        let root = match data_dir {
            Some(dir) => dir.to_path_buf(),
            None => {
                let exe = std::env::current_exe()
                    .map_err(|source| PathError::ExeLocationUnknown { source })?;
                // An executable always has a parent; if it somehow does not,
                // treat the current directory as the base rather than panicking.
                exe.parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(DATA_DIR_NAME)
            }
        };

        fs::create_dir_all(&root).map_err(|source| PathError::NotCreatable {
            path: root.clone(),
            source,
        })?;

        // Canonicalize after creating, so the stored root has no `.` or `..`
        // left in it and later comparisons are against a real path. A failure
        // here is reported as not-writable because that is what it means in
        // practice for the caller.
        let root = fs::canonicalize(&root).map_err(|source| PathError::NotWritable {
            path: root.clone(),
            source,
        })?;
        let root = strip_verbatim_prefix(root);

        probe_writable(&root)?;

        for sub in SUBDIRS {
            let path = root.join(sub);
            fs::create_dir_all(&path).map_err(|source| PathError::NotCreatable {
                path: path.clone(),
                source,
            })?;
        }

        Ok(Self { root })
    }

    /// The root itself.
    pub fn path(&self) -> &Path {
        &self.root
    }

    /// Resolve `relative` inside the root, refusing anything that climbs out.
    ///
    /// This is the only way a caller should build a path to read or write. It
    /// rejects `..` components and absolute paths before touching the
    /// filesystem, so it works for a file that does not exist yet — which
    /// `fs::canonicalize` cannot do.
    pub fn join(&self, relative: impl AsRef<Path>) -> Result<PathBuf, PathError> {
        let relative = relative.as_ref();
        let escapes = || PathError::Escapes {
            requested: relative.to_path_buf(),
        };

        let mut resolved = self.root.clone();
        for component in relative.components() {
            match component {
                Component::Normal(part) => resolved.push(part),
                // `./` is harmless and common in a joined path.
                Component::CurDir => {}
                // Everything else either climbs out or restarts from a root.
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err(escapes());
                }
            }
        }

        // Belt and braces: even with no `..` component, a symlink inside the
        // root could point outside it. Where the path already exists, check
        // where it really lands.
        //
        // The stripping matters: `canonicalize` hands back a `\\?\` path while
        // `self.root` has had that prefix removed, so comparing them directly
        // makes `starts_with` false for every existing path inside the root
        // and this check rejects everything.
        if let Ok(real) = fs::canonicalize(&resolved)
            && !strip_verbatim_prefix(real).starts_with(&self.root)
        {
            return Err(escapes());
        }

        Ok(resolved)
    }
}

/// Drop Windows' `\\?\` extended-length prefix that `fs::canonicalize` adds.
///
/// Without this the root reaches the user as `\\?\D:\…` in log lines and error
/// messages, and reaches other programs as an argument many of them do not
/// accept. Stripping is only safe while the path is short enough not to need
/// the prefix, so a path at or beyond the classic `MAX_PATH` limit keeps it —
/// a long but working path beats a short but broken one.
fn strip_verbatim_prefix(path: PathBuf) -> PathBuf {
    const MAX_PATH: usize = 260;

    if cfg!(not(windows)) {
        return path;
    }

    let text = path.to_string_lossy();
    let stripped = match text.strip_prefix(r"\\?\UNC\") {
        // `\\?\UNC\server\share` is the verbatim form of `\\server\share`.
        Some(rest) => format!(r"\\{rest}"),
        None => match text.strip_prefix(r"\\?\") {
            Some(rest) => rest.to_string(),
            None => return path,
        },
    };

    if stripped.len() >= MAX_PATH {
        return path;
    }
    PathBuf::from(stripped)
}

/// Write and delete a probe file, so "writable" means proven rather than
/// assumed. Checking permissions instead would still be wrong on a full disk
/// or a read-only mount.
fn probe_writable(root: &Path) -> Result<(), PathError> {
    let probe = root.join(".omnitoaster-write-probe");
    let outcome = fs::write(&probe, b"omnitoaster").and_then(|()| fs::remove_file(&probe));
    outcome.map_err(|source| PathError::NotWritable {
        path: root.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root() -> tempfile::TempDir {
        tempfile::tempdir().expect("a temp dir")
    }

    #[test]
    fn resolve_creates_the_whole_subtree() {
        let temp = temp_root();
        let root = DataRoot::resolve(Some(&temp.path().join("data"))).expect("resolves");

        for sub in SUBDIRS {
            let path = root.path().join(sub);
            assert!(path.is_dir(), "{sub} was not created");
        }
    }

    #[test]
    fn resolve_is_idempotent_and_leaves_existing_files_alone() {
        let temp = temp_root();
        let dir = temp.path().join("data");

        let first = DataRoot::resolve(Some(&dir)).expect("first resolve");
        let marker = first.join("config/settings.toml").expect("inside the root");
        fs::write(&marker, b"theme = 'dark'").expect("write the marker");

        let second = DataRoot::resolve(Some(&dir)).expect("second resolve");
        assert_eq!(first, second);
        assert_eq!(
            fs::read_to_string(&marker).expect("marker survives"),
            "theme = 'dark'",
            "a second resolve must not touch existing data"
        );
    }

    #[test]
    fn resolve_leaves_no_probe_file_behind() {
        let temp = temp_root();
        let root = DataRoot::resolve(Some(&temp.path().join("data"))).expect("resolves");

        let leftovers: Vec<_> = fs::read_dir(root.path())
            .expect("read the root")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains("probe"))
            .collect();

        assert!(
            leftovers.is_empty(),
            "probe file left behind: {leftovers:?}"
        );
    }

    #[test]
    fn join_refuses_a_parent_component() {
        let temp = temp_root();
        let root = DataRoot::resolve(Some(&temp.path().join("data"))).expect("resolves");

        for attempt in ["../outside.txt", "config/../../outside.txt", ".."] {
            let error = root.join(attempt).expect_err("must be refused");
            assert!(
                matches!(error, PathError::Escapes { .. }),
                "{attempt} gave {error:?}"
            );
        }
    }

    #[test]
    fn join_refuses_an_absolute_path() {
        let temp = temp_root();
        let root = DataRoot::resolve(Some(&temp.path().join("data"))).expect("resolves");

        // Both shapes matter on Windows: a drive-prefixed path and a
        // root-relative one.
        for attempt in [r"C:\Windows\System32\drivers\etc\hosts", "/etc/passwd"] {
            let error = root.join(attempt).expect_err("must be refused");
            assert!(
                matches!(error, PathError::Escapes { .. }),
                "{attempt} gave {error:?}"
            );
        }
    }

    #[test]
    fn join_allows_a_normal_relative_path_that_does_not_exist_yet() {
        let temp = temp_root();
        let root = DataRoot::resolve(Some(&temp.path().join("data"))).expect("resolves");

        let path = root
            .join("data/omnitoaster.db")
            .expect("a plain relative path is fine");
        assert!(path.starts_with(root.path()));
        assert!(!path.exists(), "join must not create anything");
    }

    #[test]
    fn join_allows_a_path_that_already_exists() {
        // Regression: the existence check canonicalizes, which on Windows
        // returns a `\\?\` path, while the stored root has that prefix
        // stripped. Comparing the two directly made `starts_with` false for
        // every existing path inside the root, so `join` refused all of them -
        // including `logs`, which stopped logging from starting at all.
        let temp = temp_root();
        let root = DataRoot::resolve(Some(&temp.path().join("data"))).expect("resolves");

        for existing in ["logs", "config", "assets/audio/azan", "logs/crash"] {
            assert!(
                root.path().join(existing).is_dir(),
                "{existing} should have been created by resolve"
            );
            let joined = root
                .join(existing)
                .unwrap_or_else(|e| panic!("join({existing}) was refused: {e}"));
            assert!(joined.starts_with(root.path()));
        }
    }

    #[test]
    fn join_tolerates_a_current_dir_component() {
        let temp = temp_root();
        let root = DataRoot::resolve(Some(&temp.path().join("data"))).expect("resolves");

        let path = root.join("./config/./settings.toml").expect("fine");
        assert_eq!(path, root.path().join("config").join("settings.toml"));
    }

    #[test]
    fn the_root_has_no_verbatim_prefix() {
        let temp = temp_root();
        let root = DataRoot::resolve(Some(&temp.path().join("data"))).expect("resolves");

        let shown = root.path().display().to_string();
        assert!(
            !shown.starts_with(r"\\?\"),
            "the root reaches users and other programs as {shown}"
        );
    }

    #[test]
    fn stripping_leaves_a_long_path_alone() {
        // Past the classic limit the prefix is what makes the path work, so it
        // stays even though it is ugly.
        let long = format!(r"\\?\D:\{}", "x".repeat(300));
        let kept = strip_verbatim_prefix(PathBuf::from(&long));
        assert_eq!(kept, PathBuf::from(&long));
    }

    #[test]
    fn stripping_handles_a_unc_path() {
        if cfg!(windows) {
            let unc = strip_verbatim_prefix(PathBuf::from(r"\\?\UNC\server\share\data"));
            assert_eq!(unc, PathBuf::from(r"\\server\share\data"));
        }
    }

    #[test]
    fn not_writable_names_the_folder_and_keeps_the_cause() {
        // A genuinely unwritable folder is awkward to produce portably, so the
        // error path is exercised directly: what matters to the caller is that
        // the message names the folder and the io::Error is preserved for the
        // log.
        let error = PathError::NotWritable {
            path: PathBuf::from(r"C:\Program Files\OmniToaster\OmniToasterData"),
            source: io::Error::from(io::ErrorKind::PermissionDenied),
        };
        let rendered = error.to_string();
        assert!(rendered.contains("OmniToasterData"), "{rendered}");
        assert!(rendered.contains("not writable"), "{rendered}");
    }

    #[test]
    fn a_file_where_the_root_should_be_is_reported_not_panicked() {
        let temp = temp_root();
        let blocker = temp.path().join("data");
        fs::write(&blocker, b"not a directory").expect("create the blocking file");

        let error = DataRoot::resolve(Some(&blocker)).expect_err("must fail");
        assert!(
            matches!(error, PathError::NotCreatable { .. }),
            "got {error:?}"
        );
    }
}
