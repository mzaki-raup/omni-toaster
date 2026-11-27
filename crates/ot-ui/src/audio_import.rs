//! Importing a sound the user chooses (FEAT-0060, FEAT-0084).
//!
//! One path for every place a sound comes in — a prayer's template and the Audio
//! library — so the rules cannot drift apart: the **size** before a byte is read, the
//! **format from the bytes** (WAV, MP3 or Ogg Vorbis, never the name), a **bounded
//! decode**, then a copy under a content-hashed name inside the data folder
//! (`docs/06-security-and-privacy.md`). The name the user sees is the file's own to
//! begin with and theirs to change.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, sync_channel};

use ot_store::assets::StoredAudio;

/// A clip that passed every check and is copied in. Not yet in a library: that is a
/// database row, written by whoever asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Imported {
    pub clip: StoredAudio,
    /// What the clip is called to begin with: the file's name without its extension.
    pub label: String,
}

/// Check `path` and copy it in. Blocking: the caller runs it on a worker thread.
///
/// The error is the Fluent key that names the rule the file broke.
pub fn import(root: &Path, path: &Path) -> Result<Imported, &'static str> {
    let unreadable = "error-import-unreadable";
    let metadata = std::fs::metadata(path).map_err(|_| unreadable)?;
    ot_media::audio::check_size(metadata.len()).map_err(|error| error.name_key())?;
    let bytes = std::fs::read(path).map_err(|_| unreadable)?;
    let checked = ot_media::audio::check(&bytes).map_err(|error| error.name_key())?;
    let clip = ot_store::assets::store_audio(root, &bytes, checked.format.extension()).map_err(
        |error| {
            tracing::warn!(%error, "an audio clip could not be copied in");
            unreadable
        },
    )?;
    let label = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().trim().to_owned())
        .unwrap_or_default();
    Ok(Imported { clip, label })
}

/// How an attempt ended.
#[derive(Debug)]
pub enum Outcome {
    Cancelled,
    Imported(Imported),
    /// The file broke a rule; `name` is the file's name and `key` the Fluent key of the
    /// rule.
    Refused {
        name: String,
        key: &'static str,
    },
    DialogFailed(String),
}

/// Choosing a file and importing it, one attempt at a time, off the UI thread.
pub struct Importer {
    root: PathBuf,
    sender: SyncSender<Outcome>,
    receiver: Receiver<Outcome>,
    busy: bool,
}

impl Importer {
    pub fn new(root: &Path) -> Self {
        // One slot: there is only ever one dialog open at a time.
        let (sender, receiver) = sync_channel(1);
        Self {
            root: root.to_path_buf(),
            sender,
            receiver,
            busy: false,
        }
    }

    pub fn is_busy(&self) -> bool {
        self.busy
    }

    /// Open the dialog on a worker thread and import whatever is chosen. `title` and
    /// `filter_name` arrive in the user's language.
    pub fn start(&mut self, title: String, filter_name: String) {
        if self.busy {
            return;
        }
        self.busy = true;
        let root = self.root.clone();
        let sender = self.sender.clone();
        let spawned = std::thread::Builder::new()
            .name("audio-import".into())
            .spawn(move || {
                let outcome = match ot_platform::file_dialog::pick_audio(&title, &filter_name) {
                    Ok(None) => Outcome::Cancelled,
                    Ok(Some(path)) => match import(&root, &path) {
                        Ok(imported) => Outcome::Imported(imported),
                        Err(key) => Outcome::Refused {
                            name: path
                                .file_name()
                                .map(|name| name.to_string_lossy().into_owned())
                                .unwrap_or_default(),
                            key,
                        },
                    },
                    Err(error) => Outcome::DialogFailed(error.to_string()),
                };
                let _ = sender.send(outcome);
            });
        if let Err(error) = spawned {
            self.busy = false;
            tracing::error!(%error, "the audio import thread could not be started");
        }
    }

    /// Take the outcome, if the attempt has finished.
    pub fn drain(&mut self) -> Option<Outcome> {
        match self.receiver.try_recv() {
            Ok(outcome) => {
                self.busy = false;
                Some(outcome)
            }
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The smallest thing `ot_media::audio::check` accepts is a real WAV; the media
    /// crate's own tests build one, so this builds the same: 8-bit mono, 8 kHz, 100
    /// samples of silence.
    fn wav() -> Vec<u8> {
        let samples = vec![128u8; 100];
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + samples.len() as u32).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
        bytes.extend_from_slice(&1u16.to_le_bytes()); // mono
        bytes.extend_from_slice(&8000u32.to_le_bytes());
        bytes.extend_from_slice(&8000u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&8u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&(samples.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&samples);
        bytes
    }

    #[test]
    fn a_good_clip_is_copied_in_by_its_hash_and_named_after_its_file() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        let source = dir.path().join("Door Bell.wav");
        std::fs::write(&source, wav()).expect("written");
        let root = dir.path().join("OmniToasterData");
        std::fs::create_dir_all(&root).expect("root");

        let imported = import(&root, &source).expect("imports");
        assert_eq!(imported.label, "Door Bell");
        assert!(imported.clip.relative_path.starts_with("assets/"));
        assert!(imported.clip.relative_path.ends_with(".wav"));
        assert!(
            root.join(&imported.clip.relative_path).is_file(),
            "it landed inside the data folder"
        );
        // The same file again is the same entry.
        assert_eq!(import(&root, &source).expect("again").clip, imported.clip);
    }

    #[test]
    fn a_file_that_is_not_audio_is_refused_by_its_bytes_and_nothing_is_copied() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        let root = dir.path().join("OmniToasterData");
        std::fs::create_dir_all(&root).expect("root");
        // Named like a sound, but not one.
        let fake = dir.path().join("song.mp3");
        std::fs::write(&fake, b"this is text, whatever the name says").expect("written");
        assert_eq!(import(&root, &fake), Err("error-import-not-audio"));
        assert!(
            !root.join("assets").exists()
                || std::fs::read_dir(root.join("assets"))
                    .expect("assets")
                    .next()
                    .is_none(),
            "a refused clip leaves nothing behind"
        );
        assert_eq!(
            import(&root, &dir.path().join("missing.wav")),
            Err("error-import-unreadable")
        );
    }
}
