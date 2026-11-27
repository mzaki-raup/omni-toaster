//! Staging the verified exe beside the running one (FEAT-0078,
//! `docs/specs/updater.md` §4 step 6 and §5).
//!
//! A running exe cannot replace itself on Windows, so the worker only *stages* the
//! new program as `OmniToaster.exe.new` next to the running one; the supervisor
//! (FEAT-0079) swaps it in. Staging copies to a `.part` name first and renames, so a
//! `.new` that exists is complete, and it checks the copy's digest again — a disk
//! error between the cache and the folder must not become an update. If the folder
//! is not writable (the exe in `Program Files`, say) it says so, so the pane can offer
//! the manual zip; it never asks for elevation.

use std::path::{Path, PathBuf};

use crate::download::sha256_of_file;

/// Why a staged file could not be made.
#[derive(Debug, thiserror::Error)]
pub enum StageError {
    /// The folder the exe is in cannot be written to: update by hand.
    #[error("the folder {dir} is not writable: {source}")]
    NotWritable {
        dir: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("the staged copy does not match the verified download, and was removed")]
    CopyMismatch,
    #[error("the running program's path is unusable: {0}")]
    BadPath(String),
}

/// Where the staged file goes for the program at `running_exe`: its name with
/// `.new` added, in the same folder.
pub fn staged_path(running_exe: &Path) -> Result<PathBuf, StageError> {
    let name = running_exe
        .file_name()
        .ok_or_else(|| StageError::BadPath(running_exe.display().to_string()))?;
    let mut staged = name.to_os_string();
    staged.push(".new");
    Ok(running_exe.with_file_name(staged))
}

/// Copy `downloaded` (already verified against `sha256`) to `<running exe>.new`.
/// Returns the staged path. An earlier staged file is replaced.
pub fn stage(downloaded: &Path, running_exe: &Path, sha256: &str) -> Result<PathBuf, StageError> {
    let target = staged_path(running_exe)?;
    let dir = running_exe
        .parent()
        .ok_or_else(|| StageError::BadPath(running_exe.display().to_string()))?;
    let part = target.with_extension("new.part");
    let not_writable = |source| StageError::NotWritable {
        dir: dir.to_path_buf(),
        source,
    };
    let _ = std::fs::remove_file(&part);
    if let Err(source) = std::fs::copy(downloaded, &part) {
        let _ = std::fs::remove_file(&part);
        return Err(not_writable(source));
    }
    match sha256_of_file(&part) {
        Ok(digest) if digest == sha256 => {}
        _ => {
            let _ = std::fs::remove_file(&part);
            return Err(StageError::CopyMismatch);
        }
    }
    let _ = std::fs::remove_file(&target);
    if let Err(source) = std::fs::rename(&part, &target) {
        let _ = std::fs::remove_file(&part);
        return Err(not_writable(source));
    }
    Ok(target)
}
