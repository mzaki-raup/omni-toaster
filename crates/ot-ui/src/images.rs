//! Pictures for toasts: choosing one, importing it, and decoding it to draw.
//!
//! Every step that touches a file happens on a worker thread, and its result
//! comes back through a **bounded** channel drained at frame start
//! (`.claude/rules/threads-and-scheduling.md`). The UI thread never opens the
//! dialog, never reads the file and never decodes it: the dialog runs its own
//! modal loop on whatever thread calls it, and a 5 MB decode in a frame is a
//! visible stall.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, sync_channel};

use ot_store::assets::{Asset, ImportError, Thumbnail};

/// The side a thumbnail is shrunk to. The icon slot is 20 points, which is
/// 50 pixels at 250%; 64 leaves room without carrying the original around.
const THUMBNAIL_SIDE: u32 = 64;

/// Which picture an import is for. They are kept apart because they mean
/// different things: the icon sits in the header, the image in the body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Icon,
    Image,
}

/// How a choose-and-import attempt ended.
#[derive(Debug)]
pub enum ImportOutcome {
    /// The user closed the dialog without choosing. Not an error.
    Cancelled,
    /// Imported, for `slot` if it came from a picker; the string is the
    /// label it starts with in the library — the file's own name, which the
    /// user may change.
    Imported(Option<Slot>, Asset, String),
    /// The file broke one of the import rules; `name` is its file name, for
    /// the message.
    Refused { name: String, error: ImportError },
    /// The dialog itself could not be shown.
    DialogFailed(String),
}

/// Choosing a file and importing it, one attempt at a time.
pub struct Importer {
    root: PathBuf,
    sender: SyncSender<ImportOutcome>,
    receiver: Receiver<ImportOutcome>,
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

    /// Whether a dialog or an import is under way, so the button can say so
    /// rather than open a second dialog behind the first.
    pub fn is_busy(&self) -> bool {
        self.busy
    }

    /// Open the dialog on a worker thread and import whatever is chosen.
    ///
    /// `title` and `filter_name` arrive in the user's language.
    pub fn start(&mut self, slot: Option<Slot>, title: String, filter_name: String) {
        if self.busy {
            return;
        }
        self.busy = true;
        let root = self.root.clone();
        let sender = self.sender.clone();
        let spawned = std::thread::Builder::new()
            .name("image-import".into())
            .spawn(move || {
                let outcome = match ot_platform::file_dialog::pick_image(&title, &filter_name) {
                    Ok(None) => ImportOutcome::Cancelled,
                    Ok(Some(path)) => match ot_store::assets::import(&root, &path) {
                        Ok(asset) => ImportOutcome::Imported(
                            slot,
                            asset,
                            path.file_stem()
                                .map(|stem| stem.to_string_lossy().into_owned())
                                .unwrap_or_default(),
                        ),
                        Err(error) => ImportOutcome::Refused {
                            name: path
                                .file_name()
                                .map(|name| name.to_string_lossy().into_owned())
                                .unwrap_or_default(),
                            error,
                        },
                    },
                    Err(error) => ImportOutcome::DialogFailed(error.to_string()),
                };
                // The receiver lives as long as the window; if the window has
                // gone there is nobody to tell.
                let _ = sender.send(outcome);
            });
        if let Err(error) = spawned {
            self.busy = false;
            tracing::error!(%error, "the import thread could not be started");
        }
    }

    /// Take the outcome, if the attempt has finished.
    pub fn drain(&mut self) -> Option<ImportOutcome> {
        match self.receiver.try_recv() {
            Ok(outcome) => {
                self.busy = false;
                Some(outcome)
            }
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => None,
        }
    }
}

/// The key under which the app's own icon is cached. Not a path: it can
/// never collide with an asset, whose name is a hash.
pub const APP_ICON: &str = "@app-icon";

/// The app's own icon, embedded at build time from the same generated set the
/// exe and the tray use, so the header cannot show a different picture from
/// either. It is the header icon of any notification that brings none.
const APP_ICON_PNG: &[u8] = include_bytes!("../../../assets/icon/omnitoaster-64.png");

type Decoded = (String, Result<Thumbnail, ImportError>);

/// Decoded pictures, as egui textures, keyed by their relative path.
pub struct Images {
    root: PathBuf,
    textures: HashMap<String, egui::TextureHandle>,
    pending: HashSet<String>,
    /// Pictures that would not decode. Not retried every frame: a broken file
    /// stays broken until something changes it, and the error was reported
    /// once already.
    failed: HashSet<String>,
    sender: SyncSender<Decoded>,
    receiver: Receiver<Decoded>,
}

impl Images {
    pub fn new(root: &Path) -> Self {
        // A handful of slots: there are at most five toasts on screen and one
        // preview, so this never fills in practice — and if it does, the
        // worker waits rather than the UI.
        let (sender, receiver) = sync_channel(8);
        Self {
            root: root.to_path_buf(),
            textures: HashMap::new(),
            pending: HashSet::new(),
            failed: HashSet::new(),
            sender,
            receiver,
        }
    }

    /// Make sure this picture is decoded, or on its way.
    pub fn request(&mut self, relative: &str) {
        if self.textures.contains_key(relative)
            || self.pending.contains(relative)
            || self.failed.contains(relative)
        {
            return;
        }
        self.pending.insert(relative.to_string());
        let root = self.root.clone();
        let relative = relative.to_string();
        let sender = self.sender.clone();
        let spawned = std::thread::Builder::new()
            .name("image-decode".into())
            .spawn(move || {
                let result = if relative == APP_ICON {
                    ot_store::assets::decode(APP_ICON_PNG, Path::new(APP_ICON), THUMBNAIL_SIDE)
                } else {
                    ot_store::assets::thumbnail(&root, &relative, THUMBNAIL_SIDE)
                };
                let _ = sender.send((relative, result));
            });
        if let Err(error) = spawned {
            tracing::error!(%error, "the decode thread could not be started");
        }
    }

    /// Turn whatever has finished decoding into textures. Returns the
    /// pictures that would not decode, so the caller can say so.
    pub fn drain(&mut self, ctx: &egui::Context) -> Vec<(String, ImportError)> {
        let mut failures = Vec::new();
        while let Ok((relative, result)) = self.receiver.try_recv() {
            self.pending.remove(&relative);
            match result {
                Ok(thumb) => {
                    let image = egui::ColorImage::from_rgba_unmultiplied(
                        [thumb.width as usize, thumb.height as usize],
                        &thumb.rgba,
                    );
                    let texture =
                        ctx.load_texture(relative.clone(), image, egui::TextureOptions::LINEAR);
                    self.textures.insert(relative, texture);
                }
                Err(error) => {
                    tracing::warn!(%error, relative, "a toast picture could not be decoded");
                    self.failed.insert(relative.clone());
                    failures.push((relative, error));
                }
            }
        }
        failures
    }

    /// A decoded picture, with its size so it can be fitted unstretched.
    pub fn picture(&self, relative: &str) -> Option<ot_toast::content::Picture> {
        self.textures
            .get(relative)
            .map(|texture| ot_toast::content::Picture {
                texture: texture.id(),
                size: texture.size_vec2(),
            })
    }

    /// Every decoded picture, for the toast windows. Their paint callbacks
    /// cannot borrow the window, so they get a copy.
    pub fn pictures(&self) -> HashMap<String, ot_toast::content::Picture> {
        self.textures
            .keys()
            .filter_map(|path| self.picture(path).map(|picture| (path.clone(), picture)))
            .collect()
    }
}
