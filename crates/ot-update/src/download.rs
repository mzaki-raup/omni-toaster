//! Downloading the exe and holding it to what the signed manifest says
//! (FEAT-0078, `docs/specs/updater.md` §4 steps 4 and 5).
//!
//! The size and SHA-256 come from a manifest whose signature has been verified, so
//! they are the project's own word about the file. The download is read in chunks
//! into `<cache>/OmniToaster-<version>.exe.part`, **checking the cancel flag between
//! chunks** (a cancel really stops the transfer), abandoned the moment it exceeds
//! the manifest's size, and only renamed to its final name when its length and
//! digest both match. Anything else deletes it: a corrupted or substituted file never
//! stays on disk where it could be mistaken for good.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use semver::Version;
use sha2::{Digest, Sha256};

use ot_net::Cancel;

use crate::manifest::AssetEntry;

/// A program is a few megabytes over a home connection; this bounds the wait
/// without bounding a slow one too tightly.
pub const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);

/// Why a download did not become a verified file.
#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("the request failed: {0}")]
    Network(String),
    #[error("the server answered HTTP {0}")]
    Http(u16),
    #[error("the download is larger than the manifest says ({0} bytes)")]
    TooLarge(u64),
    #[error("the download is {got} bytes, not the {want} the manifest says")]
    WrongSize { got: u64, want: u64 },
    /// The bytes are not the ones the manifest signs for. Deleted.
    #[error("the download does not match the manifest's checksum")]
    WrongDigest,
    #[error("the cache folder {path} could not be written: {source}")]
    Cache {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("the download was cancelled")]
    Cancelled,
}

/// The client that downloads: one agent with the long timeout.
#[derive(Clone)]
pub struct Downloader {
    agent: ureq::Agent,
}

impl Downloader {
    pub fn new() -> Self {
        Self {
            agent: ot_net::agent_with_timeout(true, DOWNLOAD_TIMEOUT),
        }
    }

    #[cfg(test)]
    pub(crate) fn local() -> Self {
        Self {
            agent: ot_net::agent_with_timeout(false, Duration::from_secs(30)),
        }
    }

    /// Fetch `entry` into `cache_dir`, verified. `progress` is told `(done, total)`
    /// after each chunk. Returns the verified file's path.
    pub fn download(
        &self,
        entry: &AssetEntry,
        version: &Version,
        cache_dir: &Path,
        cancel: &Cancel,
        progress: &mut dyn FnMut(u64, u64),
    ) -> Result<PathBuf, DownloadError> {
        let cache = |source| DownloadError::Cache {
            path: cache_dir.to_path_buf(),
            source,
        };
        if cancel.is_cancelled() {
            return Err(DownloadError::Cancelled);
        }
        std::fs::create_dir_all(cache_dir).map_err(cache)?;
        let part = cache_dir.join(format!("OmniToaster-{version}.exe.part"));
        let done = cache_dir.join(format!("OmniToaster-{version}.exe"));
        let _ = std::fs::remove_file(&part);
        let _ = std::fs::remove_file(&done);

        let result = self.transfer(entry, &part, cancel, progress);
        match result {
            Ok(()) => {
                std::fs::rename(&part, &done).map_err(|source| {
                    let _ = std::fs::remove_file(&part);
                    cache(source)
                })?;
                Ok(done)
            }
            Err(error) => {
                // Nothing half-written or wrong is left behind.
                let _ = std::fs::remove_file(&part);
                Err(error)
            }
        }
    }

    fn transfer(
        &self,
        entry: &AssetEntry,
        part: &Path,
        cancel: &Cancel,
        progress: &mut dyn FnMut(u64, u64),
    ) -> Result<(), DownloadError> {
        let mut response = self
            .agent
            .get(&entry.url)
            .call()
            .map_err(|error| DownloadError::Network(error.to_string()))?;
        let status = response.status().as_u16();
        if status != 200 {
            return Err(DownloadError::Http(status));
        }
        let mut reader = response.body_mut().as_reader();
        let mut file = std::fs::File::create(part).map_err(|source| DownloadError::Cache {
            path: part.to_path_buf(),
            source,
        })?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 64 * 1024];
        let mut total = 0u64;
        loop {
            if cancel.is_cancelled() {
                return Err(DownloadError::Cancelled);
            }
            let n = reader
                .read(&mut buffer)
                .map_err(|error| DownloadError::Network(error.to_string()))?;
            if n == 0 {
                break;
            }
            total += n as u64;
            // Past what the manifest says, abandon: a server cannot make this fill the disk.
            if total > entry.size {
                return Err(DownloadError::TooLarge(total));
            }
            hasher.update(&buffer[..n]);
            file.write_all(&buffer[..n])
                .map_err(|source| DownloadError::Cache {
                    path: part.to_path_buf(),
                    source,
                })?;
            progress(total, entry.size);
        }
        file.flush().map_err(|source| DownloadError::Cache {
            path: part.to_path_buf(),
            source,
        })?;
        drop(file);
        if total != entry.size {
            return Err(DownloadError::WrongSize {
                got: total,
                want: entry.size,
            });
        }
        if hex(&hasher.finalize()) != entry.sha256 {
            return Err(DownloadError::WrongDigest);
        }
        Ok(())
    }
}

impl Default for Downloader {
    fn default() -> Self {
        Self::new()
    }
}

/// Lower-case hex of a digest.
pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
}

/// The SHA-256 of a file, as lower-case hex.
pub fn sha256_of_file(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hex(&hasher.finalize()))
}
