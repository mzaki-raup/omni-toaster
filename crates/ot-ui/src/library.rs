//! The libraries — images, icons and audio — as the settings window sees them
//! (`docs/specs/toast.md` §8.1).
//!
//! SQLite is blocking work, so the library lives on a thread of its own with
//! its own connection: the window sends it requests and drains its replies at
//! frame start, both through **bounded** channels
//! (`.claude/rules/threads-and-scheduling.md`). After every change the thread
//! sends the whole list again, so the window never has to patch its copy.

use std::path::Path;
use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};

use ot_store::Store;
use ot_store::assets::{Asset, StoredAudio};
use ot_store::library::{Added, Kind, LibraryPicture, Removal};

use crate::images::Slot;

/// What the window asks of the library.
#[derive(Debug)]
enum Request {
    /// Record an imported picture; `slot` is where the window wants it once
    /// it is in, if it was uploaded from a picker.
    Add {
        kind: Kind,
        sha256: String,
        relative_path: String,
        bytes: u64,
        label: String,
        slot: Option<Slot>,
    },
    Rename {
        sha256: String,
        label: String,
    },
    /// `used_here` is the window's own knowledge — the test toast uses it —
    /// which the database cannot see.
    Remove {
        sha256: String,
        confirmed: bool,
        used_here: bool,
    },
}

/// What the library tells the window.
#[derive(Debug)]
pub enum Reply {
    Added {
        slot: Option<Slot>,
        relative_path: String,
        label: String,
        /// False when the same file was already there.
        new: bool,
        /// The library the entry is in. For a file that was already there it is the one it
        /// was found in, which need not be the one it was offered to.
        kind: Kind,
    },
    /// The removal needs confirming; `templates` is how many templates use
    /// the picture, as far as the database knows.
    InUse {
        sha256: String,
        templates: usize,
    },
    Removed {
        relative_path: String,
    },
    Failed(String),
}

/// A picture as the window holds it: what the library says, and the label
/// as it stands in its text field, which differs while it is being edited.
#[derive(Debug, Clone)]
pub struct Entry {
    pub picture: LibraryPicture,
    pub edit: String,
}

/// Every list the thread has, sent whole after each change.
struct Lists {
    /// Images and icons, in upload order, each with its `kind`.
    pictures: Vec<LibraryPicture>,
    audio: Vec<LibraryPicture>,
}

/// The library's state on the window's side.
pub struct Library {
    requests: SyncSender<Request>,
    replies: Receiver<Reply>,
    lists: Receiver<Lists>,
    /// `None` until the first list arrives, which is the loading state.
    pictures: Option<Vec<Entry>>,
    audio: Option<Vec<Entry>>,
}

impl Library {
    /// Start the library thread on `root`'s database.
    pub fn new(root: &Path) -> Self {
        // A handful of slots each way: requests come from clicks, so the
        // window can never get far ahead of the thread.
        let (requests, incoming) = sync_channel(16);
        let (reply, replies) = sync_channel(16);
        let (list, lists) = sync_channel(4);
        let root = root.to_path_buf();
        let spawned = std::thread::Builder::new()
            .name("image-library".into())
            .spawn(move || serve(&root, &incoming, &reply, &list));
        if let Err(error) = spawned {
            tracing::error!(%error, "the image library thread could not be started");
        }
        Self {
            requests,
            replies,
            lists,
            pictures: None,
            audio: None,
        }
    }

    /// The pictures, oldest first, or `None` while they are loading.
    pub fn pictures(&self) -> Option<&[Entry]> {
        self.pictures.as_deref()
    }

    /// The same, with their label fields editable.
    pub fn pictures_mut(&mut self) -> Option<&mut [Entry]> {
        self.pictures.as_deref_mut()
    }

    /// The sounds, oldest first, or `None` while they are loading.
    pub fn audio(&self) -> Option<&[Entry]> {
        self.audio.as_deref()
    }

    /// The same, with their name fields editable.
    pub fn audio_mut(&mut self) -> Option<&mut [Entry]> {
        self.audio.as_deref_mut()
    }

    /// The label of the picture at `relative_path`, if it is in the library.
    pub fn label_of(&self, relative_path: &str) -> Option<&str> {
        self.pictures
            .as_deref()?
            .iter()
            .find(|entry| entry.picture.relative_path == relative_path)
            .map(|entry| entry.picture.label.as_str())
    }

    /// Record an imported picture in the library `kind` (Images or Icons).
    pub fn add(&self, kind: Kind, asset: Asset, label: String, slot: Option<Slot>) {
        self.send(Request::Add {
            kind,
            sha256: asset.sha256,
            relative_path: asset.relative_path,
            bytes: asset.bytes,
            label,
            slot,
        });
    }

    /// Record an imported sound in the Audio library.
    pub fn add_clip(&self, clip: StoredAudio, label: String) {
        self.send(Request::Add {
            kind: Kind::Audio,
            sha256: clip.sha256,
            relative_path: clip.relative_path,
            bytes: clip.bytes,
            label,
            slot: None,
        });
    }

    pub fn rename(&self, sha256: String, label: String) {
        self.send(Request::Rename { sha256, label });
    }

    pub fn remove(&self, sha256: String, confirmed: bool, used_here: bool) {
        self.send(Request::Remove {
            sha256,
            confirmed,
            used_here,
        });
    }

    fn send(&self, request: Request) {
        // Never blocks the UI thread. A full queue means sixteen clicks the
        // thread has not reached yet; dropping this one is said in the log
        // rather than stalling a frame.
        match self.requests.try_send(request) {
            Ok(()) => {}
            Err(TrySendError::Full(request)) => {
                tracing::warn!(?request, "the image library is busy; a request was dropped");
            }
            Err(TrySendError::Disconnected(_)) => {
                tracing::warn!("the image library thread has stopped");
            }
        }
    }

    /// Take the newest list and every reply that has arrived.
    pub fn drain(&mut self) -> Vec<Reply> {
        while let Ok(list) = self.lists.try_recv() {
            self.pictures = Some(merge(self.pictures.take(), list.pictures));
            self.audio = Some(merge(self.audio.take(), list.audio));
        }
        let mut replies = Vec::new();
        while let Ok(reply) = self.replies.try_recv() {
            replies.push(reply);
        }
        replies
    }
}

/// A fresh list, keeping any label that was mid-edit when it arrived — an
/// upload finishing while a name is being typed must not wipe the name.
fn merge(old: Option<Vec<Entry>>, list: Vec<LibraryPicture>) -> Vec<Entry> {
    let old = old.unwrap_or_default();
    list.into_iter()
        .map(|picture| {
            let edit = old
                .iter()
                .find(|entry| entry.picture.sha256 == picture.sha256)
                .filter(|entry| entry.edit != entry.picture.label)
                .map_or_else(|| picture.label.clone(), |entry| entry.edit.clone());
            Entry { picture, edit }
        })
        .collect()
}

/// The library thread: one connection, one request at a time.
fn serve(
    root: &Path,
    incoming: &Receiver<Request>,
    reply: &SyncSender<Reply>,
    list: &SyncSender<Lists>,
) {
    let store = match Store::open(root) {
        Ok(store) => store,
        Err(error) => {
            tracing::error!(%error, "the image library could not open the database");
            let _ = reply.send(Reply::Failed(error.to_string()));
            // An empty list rather than an endless loading state: the
            // failure is on the status line.
            let _ = list.send(Lists {
                pictures: Vec::new(),
                audio: Vec::new(),
            });
            return;
        }
    };
    let send_list = || match (store.library(), store.audio_library()) {
        (Ok(pictures), Ok(audio)) => {
            let _ = list.send(Lists { pictures, audio });
        }
        (Err(error), _) | (_, Err(error)) => {
            tracing::warn!(%error, "the libraries could not be listed");
            let _ = reply.send(Reply::Failed(error.to_string()));
        }
    };
    send_list();

    // Ends when the window drops its sender.
    while let Ok(request) = incoming.recv() {
        let answer = match request {
            Request::Add {
                kind,
                sha256,
                relative_path,
                bytes,
                label,
                slot,
            } => match store.add_entry(kind, &sha256, &relative_path, bytes, &label) {
                Ok(added) => {
                    let (label, new, kind) = match added {
                        Added::New => (label, true, kind),
                        Added::AlreadyThere { label, kind } => (label, false, kind),
                    };
                    Some(Reply::Added {
                        slot,
                        relative_path,
                        label,
                        new,
                        kind,
                    })
                }
                Err(error) => Some(Reply::Failed(error.to_string())),
            },
            Request::Rename { sha256, label } => match store.set_label(&sha256, &label) {
                Ok(()) => None,
                Err(error) => Some(Reply::Failed(error.to_string())),
            },
            Request::Remove {
                sha256,
                confirmed,
                used_here,
            } => remove(&store, root, sha256, confirmed, used_here),
        };
        if let Some(answer) = answer {
            let _ = reply.send(answer);
        }
        send_list();
    }
}

fn remove(
    store: &Store,
    root: &Path,
    sha256: String,
    confirmed: bool,
    used_here: bool,
) -> Option<Reply> {
    let relative_path = store
        .library()
        .ok()?
        .into_iter()
        .chain(store.audio_library().ok()?)
        .find(|entry| entry.sha256 == sha256)?
        .relative_path;
    if !confirmed {
        let templates = match store.templates_using(&sha256) {
            Ok(templates) => templates.len(),
            Err(error) => return Some(Reply::Failed(error.to_string())),
        };
        if used_here || templates > 0 {
            return Some(Reply::InUse { sha256, templates });
        }
    }
    match store.remove_from_library(root, &sha256, true) {
        Ok(Removal::Removed) => Some(Reply::Removed { relative_path }),
        // Confirmed removals are never refused.
        Ok(Removal::InUse(_)) => None,
        Err(error) => Some(Reply::Failed(error.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// A data folder with a store.
    fn root() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::create_dir_all(dir.path().join("data")).expect("mkdir");
        drop(Store::open(dir.path()).expect("created"));
        dir
    }

    /// Drain until `done` holds or ten seconds pass; returns every reply seen.
    fn drain_until(library: &mut Library, done: impl Fn(&Library) -> bool) -> Vec<Reply> {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut seen = Vec::new();
        while Instant::now() < deadline {
            seen.extend(library.drain());
            if done(library) {
                return seen;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("the library thread did not answer in time");
    }

    /// Drain until a reply that `wanted` says yes to arrives, and return it.
    fn reply_where(library: &mut Library, wanted: impl Fn(&Reply) -> bool) -> Reply {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            for reply in library.drain() {
                if wanted(&reply) {
                    return reply;
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("the library thread sent no such reply in time");
    }

    fn clip(root: &Path) -> StoredAudio {
        ot_store::assets::store_audio(root, b"RIFFxxxxWAVEdata", "wav").expect("a clip")
    }

    #[test]
    fn a_sound_goes_into_the_audio_list_and_not_the_pictures() {
        let dir = root();
        let mut library = Library::new(dir.path());
        drain_until(&mut library, |library| library.audio().is_some());
        library.add_clip(clip(dir.path()), "Door Bell".to_owned());
        drain_until(&mut library, |library| {
            library.audio().is_some_and(|audio| audio.len() == 1)
        });
        let audio = library.audio().expect("loaded");
        assert_eq!(
            (audio[0].picture.kind, audio[0].picture.label.as_str()),
            (Kind::Audio, "Door Bell")
        );
        assert!(library.pictures().expect("loaded").is_empty());
    }

    #[test]
    fn a_file_already_in_another_library_is_answered_with_that_library() {
        let dir = root();
        let mut library = Library::new(dir.path());
        drain_until(&mut library, |library| library.pictures().is_some());
        let sound = clip(dir.path());
        library.add_clip(sound.clone(), "Bell".to_owned());
        drain_until(&mut library, |library| {
            library.audio().is_some_and(|audio| audio.len() == 1)
        });
        // The same bytes offered again, as another name.
        library.add_clip(sound, "Another".to_owned());
        let Reply::Added {
            new, kind, label, ..
        } = reply_where(&mut library, |reply| matches!(reply, Reply::Added { .. }))
        else {
            unreachable!("matched above")
        };
        let added = (new, kind, label);
        assert_eq!(added, (false, Kind::Audio, "Bell".to_owned()));
    }

    #[test]
    fn removing_a_sound_a_prayer_uses_asks_first_and_then_removes_its_file() {
        let dir = root();
        let mut library = Library::new(dir.path());
        drain_until(&mut library, |library| library.audio().is_some());
        let sound = clip(dir.path());
        let file = dir.path().join(&sound.relative_path);
        library.add_clip(sound.clone(), "Bell".to_owned());
        drain_until(&mut library, |library| {
            library.audio().is_some_and(|audio| audio.len() == 1)
        });
        Store::open(dir.path())
            .expect("opens")
            .connection()
            .execute(
                "INSERT INTO category (id, kind, sound_id, sound_mode) VALUES (301, 'prayer', ?1, 'file')",
                [&sound.sha256],
            )
            .expect("a prayer using it");

        library.remove(sound.sha256.clone(), false, false);
        reply_where(&mut library, |reply| {
            matches!(reply, Reply::InUse { templates: 1, .. })
        });
        assert!(file.exists(), "asked first, nothing was removed");

        library.remove(sound.sha256, true, false);
        drain_until(&mut library, |library| {
            library.audio().is_some_and(<[Entry]>::is_empty)
        });
        assert!(!file.exists(), "and once confirmed the file went with it");
    }
}
