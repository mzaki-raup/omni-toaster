//! The image library, as the settings window sees it
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
use ot_store::assets::Asset;
use ot_store::library::{Added, LibraryPicture, Removal};

use crate::images::Slot;

/// What the window asks of the library.
#[derive(Debug)]
enum Request {
    /// Record an imported picture; `slot` is where the window wants it once
    /// it is in, if it was uploaded from a picker.
    Add {
        asset: Asset,
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
        /// False when the same picture was already there.
        new: bool,
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

/// The library's state on the window's side.
pub struct Library {
    requests: SyncSender<Request>,
    replies: Receiver<Reply>,
    lists: Receiver<Vec<LibraryPicture>>,
    /// `None` until the first list arrives, which is the loading state.
    pictures: Option<Vec<Entry>>,
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

    /// The label of the picture at `relative_path`, if it is in the library.
    pub fn label_of(&self, relative_path: &str) -> Option<&str> {
        self.pictures
            .as_deref()?
            .iter()
            .find(|entry| entry.picture.relative_path == relative_path)
            .map(|entry| entry.picture.label.as_str())
    }

    pub fn add(&self, asset: Asset, label: String, slot: Option<Slot>) {
        self.send(Request::Add { asset, label, slot });
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
            self.pictures = Some(merge(self.pictures.take(), list));
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
    list: &SyncSender<Vec<LibraryPicture>>,
) {
    let store = match Store::open(root) {
        Ok(store) => store,
        Err(error) => {
            tracing::error!(%error, "the image library could not open the database");
            let _ = reply.send(Reply::Failed(error.to_string()));
            // An empty list rather than an endless loading state: the
            // failure is on the status line.
            let _ = list.send(Vec::new());
            return;
        }
    };
    let send_list = || match store.library() {
        Ok(pictures) => {
            let _ = list.send(pictures);
        }
        Err(error) => {
            tracing::warn!(%error, "the image library could not be listed");
            let _ = reply.send(Reply::Failed(error.to_string()));
        }
    };
    send_list();

    // Ends when the window drops its sender.
    while let Ok(request) = incoming.recv() {
        let answer = match request {
            Request::Add { asset, label, slot } => match store.add_to_library(&asset, &label) {
                Ok(added) => {
                    let (label, new) = match added {
                        Added::New => (label, true),
                        Added::AlreadyThere(existing) => (existing, false),
                    };
                    Some(Reply::Added {
                        slot,
                        relative_path: asset.relative_path,
                        label,
                        new,
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
        .find(|picture| picture.sha256 == sha256)?
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
