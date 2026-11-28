//! The libraries: images, icons and audio uploaded once and reused by any template
//! (`docs/specs/toast.md` §8.1).
//!
//! The files are already content-addressed by [`crate::assets::import`]; what
//! this adds is the list — a row per picture, the label the user gave it, and
//! which templates use it.

use std::path::Path;

use rusqlite::{OptionalExtension as _, params};

use crate::assets::{self, Asset, ImportError};
use crate::{Store, StoreError};

/// Which library an entry is in: the `asset.kind` column. `asset.sha256` is the primary
/// key, so one file is one entry in one library whatever it is used for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// A picture for a toast's body (FEAT-0025).
    Image,
    /// A small picture for a toast's header, 16 to 256 pixels (FEAT-0084).
    Icon,
    /// A sound (FEAT-0060, FEAT-0084).
    Audio,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Icon => "icon",
            Self::Audio => "audio",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "image" => Some(Self::Image),
            "icon" => Some(Self::Icon),
            "audio" => Some(Self::Audio),
            _ => None,
        }
    }
}

/// An entry in a library: a picture, an icon or a sound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryPicture {
    pub kind: Kind,
    pub sha256: String,
    /// Relative to the data folder.
    pub relative_path: String,
    pub label: String,
    pub bytes: u64,
}

/// What adding a picture did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Added {
    New,
    /// The same file was already there, under this label, which is kept: uploading it
    /// again is not a reason to lose the name it was given. `kind` is the library it is
    /// in, which may not be the one it was added to: the key is the file's content, so
    /// an image cannot also be an icon.
    AlreadyThere {
        label: String,
        kind: Kind,
    },
}

/// What a removal did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Removal {
    Removed,
    /// Templates still use it and the removal was not confirmed: nothing was
    /// touched. The ids are the templates' `category` rows.
    InUse(Vec<i64>),
}

/// Why a library change failed.
#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    File(#[from] ImportError),
}

impl From<rusqlite::Error> for LibraryError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Store(StoreError::Query(error))
    }
}

impl Store {
    /// Record an imported picture in the Images library under `label`.
    pub fn add_to_library(&self, asset: &Asset, label: &str) -> Result<Added, StoreError> {
        self.add_to(Kind::Image, asset, label)
    }

    /// Record an imported file in the library `kind` under `label`.
    pub fn add_to(&self, kind: Kind, asset: &Asset, label: &str) -> Result<Added, StoreError> {
        self.add_entry(
            kind,
            &asset.sha256,
            &asset.relative_path,
            asset.bytes,
            label,
        )
    }

    /// Record a stored sound in the Audio library under `label`.
    pub fn add_audio(
        &self,
        clip: &crate::assets::StoredAudio,
        label: &str,
    ) -> Result<Added, StoreError> {
        self.add_entry(
            Kind::Audio,
            &clip.sha256,
            &clip.relative_path,
            clip.bytes,
            label,
        )
    }

    /// Record a file in the library `kind`: the one place a row is written.
    pub fn add_entry(
        &self,
        kind: Kind,
        sha256: &str,
        relative_path: &str,
        bytes: u64,
        label: &str,
    ) -> Result<Added, StoreError> {
        let existing: Option<(String, String)> = self
            .conn
            .query_row(
                "SELECT label, kind FROM asset WHERE sha256 = ?1",
                params![sha256],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        if let Some((existing, was)) = existing {
            return Ok(Added::AlreadyThere {
                label: existing,
                kind: Kind::parse(&was).unwrap_or(kind),
            });
        }
        self.conn.execute(
            "INSERT INTO asset (sha256, kind, relative_path, bytes, created_at, label) \
             VALUES (?1, ?2, ?3, ?4, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?5)",
            params![
                sha256,
                kind.as_str(),
                relative_path,
                i64::try_from(bytes).unwrap_or(i64::MAX),
                label
            ],
        )?;
        Ok(Added::New)
    }

    /// The pictures — Images, then Icons are mixed in upload order — oldest first, so a
    /// new upload lands at the end of the list rather than reshuffling it.
    pub fn library(&self) -> Result<Vec<LibraryPicture>, StoreError> {
        self.entries(&[Kind::Image, Kind::Icon])
    }

    /// The sounds, oldest first.
    pub fn audio_library(&self) -> Result<Vec<LibraryPicture>, StoreError> {
        self.entries(&[Kind::Audio])
    }

    fn entries(&self, kinds: &[Kind]) -> Result<Vec<LibraryPicture>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT sha256, relative_path, label, bytes, kind FROM asset \
             ORDER BY created_at, rowid",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                LibraryPicture {
                    kind: Kind::parse(&row.get::<_, String>(4)?).unwrap_or(Kind::Image),
                    sha256: row.get(0)?,
                    relative_path: row.get(1)?,
                    label: row.get(2)?,
                    bytes: u64::try_from(row.get::<_, i64>(3)?).unwrap_or(0),
                },
                row.get::<_, String>(4)?,
            ))
        })?;
        let mut found = Vec::new();
        for row in rows {
            let (entry, kind) = row?;
            // A kind this build does not know (from a newer one) is not listed.
            if Kind::parse(&kind).is_some_and(|kind| kinds.contains(&kind)) {
                found.push(entry);
            }
        }
        Ok(found)
    }

    /// Rename a picture.
    pub fn set_label(&self, sha256: &str, label: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE asset SET label = ?2 WHERE sha256 = ?1",
            params![sha256, label],
        )?;
        Ok(())
    }

    /// The templates that use an entry, by `category` id: as their picture, their header
    /// icon or their sound.
    pub fn templates_using(&self, sha256: &str) -> Result<Vec<i64>, StoreError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id FROM category WHERE image_id = ?1 OR sound_id = ?1 OR icon_id = ?1 ORDER BY id",
            )?;
        let rows = stmt.query_map(params![sha256], |row| row.get(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Remove a picture from the library and its file from the data folder.
    ///
    /// Refused, with nothing touched, while a template uses it and
    /// `confirmed` is false. Once confirmed, the templates fall back — the
    /// schema's `ON DELETE SET NULL` clears their reference in the same
    /// statement, so none is left pointing at a file that is gone.
    pub fn remove_from_library(
        &self,
        root: &Path,
        sha256: &str,
        confirmed: bool,
    ) -> Result<Removal, LibraryError> {
        let users = self.templates_using(sha256)?;
        if !users.is_empty() && !confirmed {
            return Ok(Removal::InUse(users));
        }
        let relative: Option<String> = self
            .conn
            .query_row(
                "SELECT relative_path FROM asset WHERE sha256 = ?1",
                params![sha256],
                |row| row.get(0),
            )
            .optional()?;
        self.conn
            .execute("DELETE FROM asset WHERE sha256 = ?1", params![sha256])?;
        // The row first: a file left behind is wasted space, a row left
        // behind is a picture in the list that will not draw.
        if let Some(relative) = relative {
            assets::remove(root, &relative)?;
        }
        Ok(Removal::Removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A data folder with a store and a real PNG to import.
    fn setup() -> (tempfile::TempDir, Store, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::create_dir_all(dir.path().join("data")).expect("mkdir");
        let store = Store::open(dir.path()).expect("opens");
        let source = dir.path().join("mosque.png");
        let mut png = Vec::new();
        image::RgbaImage::from_pixel(4, 4, image::Rgba([10, 20, 30, 255]))
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .expect("encode");
        std::fs::write(&source, png).expect("write");
        (dir, store, source)
    }

    #[test]
    fn two_uploads_of_the_same_file_are_one_entry_and_keep_the_first_label() {
        let (dir, store, source) = setup();
        let first = assets::import(dir.path(), &source).expect("import");
        assert_eq!(
            store.add_to_library(&first, "Mosque").expect("add"),
            Added::New
        );
        let second = assets::import(dir.path(), &source).expect("import again");
        assert_eq!(
            store.add_to_library(&second, "Masjid").expect("add again"),
            Added::AlreadyThere {
                label: "Mosque".to_string(),
                kind: Kind::Image
            }
        );
        let library = store.library().expect("list");
        assert_eq!(library.len(), 1);
        assert_eq!(library[0].label, "Mosque");
        assert_eq!(library[0].relative_path, first.relative_path);
    }

    #[test]
    fn a_label_round_trips_and_the_list_survives_a_restart() {
        let (dir, store, source) = setup();
        let asset = assets::import(dir.path(), &source).expect("import");
        store.add_to_library(&asset, "mosque").expect("add");
        store
            .set_label(&asset.sha256, "Masjid Negara")
            .expect("rename");
        drop(store);

        let reopened = Store::open(dir.path()).expect("reopens");
        let library = reopened.library().expect("list");
        assert_eq!(library.len(), 1);
        assert_eq!(library[0].label, "Masjid Negara");
        assert_eq!(library[0].sha256, asset.sha256);
    }

    #[test]
    fn removing_a_used_picture_is_refused_until_confirmed_and_then_the_template_falls_back() {
        let (dir, store, source) = setup();
        let asset = assets::import(dir.path(), &source).expect("import");
        store.add_to_library(&asset, "Mosque").expect("add");
        store
            .connection()
            .execute(
                "INSERT INTO category (id, kind, image_id) VALUES (107, 'prayer', ?1)",
                params![asset.sha256],
            )
            .expect("a template using it");
        let file = dir.path().join(&asset.relative_path);

        assert_eq!(
            store
                .remove_from_library(dir.path(), &asset.sha256, false)
                .expect("asked"),
            Removal::InUse(vec![107])
        );
        assert_eq!(store.library().expect("list").len(), 1, "nothing removed");
        assert!(file.exists(), "nor its file");

        assert_eq!(
            store
                .remove_from_library(dir.path(), &asset.sha256, true)
                .expect("removed"),
            Removal::Removed
        );
        assert!(store.library().expect("list").is_empty());
        assert!(!file.exists(), "the file went with it");
        let image_id: Option<String> = store
            .connection()
            .query_row("SELECT image_id FROM category WHERE id = 107", [], |row| {
                row.get(0)
            })
            .expect("the template is still there");
        assert_eq!(image_id, None, "and it no longer points at the picture");
    }

    #[test]
    fn an_unused_picture_is_removed_without_asking() {
        let (dir, store, source) = setup();
        let asset = assets::import(dir.path(), &source).expect("import");
        store.add_to_library(&asset, "Alarm").expect("add");
        assert_eq!(
            store
                .remove_from_library(dir.path(), &asset.sha256, false)
                .expect("removed"),
            Removal::Removed
        );
        assert!(!dir.path().join(&asset.relative_path).exists());
    }

    /// A PNG of the given size in the temp folder.
    fn png(dir: &std::path::Path, name: &str, width: u32, height: u32) -> std::path::PathBuf {
        let path = dir.join(name);
        let mut bytes = Vec::new();
        image::RgbaImage::from_pixel(width, height, image::Rgba([200, 30, 30, 255]))
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .expect("encode");
        std::fs::write(&path, bytes).expect("write");
        path
    }

    /// What is in the data folder's assets directory.
    fn files_in_assets(root: &std::path::Path) -> usize {
        std::fs::read_dir(root.join(assets::ASSETS_DIR))
            .map(|entries| entries.count())
            .unwrap_or(0)
    }

    #[test]
    fn an_icon_must_be_from_16_to_256_pixels_and_a_refused_one_leaves_nothing_behind() {
        let (dir, store, _) = setup();
        for (w, h) in [(16, 16), (64, 64), (256, 256), (16, 256)] {
            let file = png(dir.path(), &format!("ok-{w}x{h}.png"), w, h);
            let icon = assets::import_icon(dir.path(), &file).expect("an icon in range");
            store.add_to(Kind::Icon, &icon, "Icon").expect("added");
        }
        let before = files_in_assets(dir.path());
        for (w, h) in [(15, 16), (16, 15), (257, 64), (64, 300), (4, 4)] {
            let file = png(dir.path(), &format!("bad-{w}x{h}.png"), w, h);
            let error = assets::import_icon(dir.path(), &file).expect_err("out of range");
            assert!(
                matches!(error, assets::ImportError::IconSize { width, height, .. } if (width, height) == (w, h)),
                "{w}x{h}: {error:?}"
            );
            assert_eq!(error.name_key(), "error-import-icon-size");
        }
        assert_eq!(
            files_in_assets(dir.path()),
            before,
            "a refused icon was never copied in"
        );
        // The same 300 px file is still a fine image.
        let big = png(dir.path(), "big.png", 300, 300);
        assert!(assets::import(dir.path(), &big).is_ok());
    }

    #[test]
    fn icons_and_images_are_separate_lists_and_audio_is_a_third() {
        let (dir, store, source) = setup();
        let image = assets::import(dir.path(), &source).expect("import");
        store.add_to(Kind::Image, &image, "Mosque").expect("added");
        let icon_file = png(dir.path(), "mark.png", 32, 32);
        let icon = assets::import_icon(dir.path(), &icon_file).expect("icon");
        store.add_to(Kind::Icon, &icon, "Mark").expect("added");
        let clip = assets::store_audio(dir.path(), b"RIFFxxxxWAVEdata", "wav").expect("clip");
        store.add_audio(&clip, "Bell").expect("added");

        let pictures = store.library().expect("pictures");
        let kinds: Vec<(Kind, &str)> = pictures
            .iter()
            .map(|entry| (entry.kind, entry.label.as_str()))
            .collect();
        assert_eq!(kinds, [(Kind::Image, "Mosque"), (Kind::Icon, "Mark")]);
        let audio = store.audio_library().expect("audio");
        assert_eq!(audio.len(), 1);
        assert_eq!(
            (audio[0].kind, audio[0].label.as_str()),
            (Kind::Audio, "Bell")
        );
    }

    #[test]
    fn a_file_already_in_one_library_is_not_added_to_another_and_says_which() {
        let (dir, store, _) = setup();
        let file = png(dir.path(), "logo.png", 64, 64);
        let image = assets::import(dir.path(), &file).expect("import");
        store.add_to(Kind::Image, &image, "Logo").expect("added");
        // The same bytes offered as an icon: one primary key, one entry, in Images.
        let same = assets::import_icon(dir.path(), &file).expect("icon-sized");
        assert_eq!(
            store
                .add_to(Kind::Icon, &same, "Logo icon")
                .expect("answered"),
            Added::AlreadyThere {
                label: "Logo".to_string(),
                kind: Kind::Image
            }
        );
        assert_eq!(
            store.library().expect("list").len(),
            1,
            "still one entry, not two"
        );
    }

    #[test]
    fn removing_a_sound_a_prayer_uses_asks_first_and_the_prayer_falls_back_to_the_chime() {
        let (dir, store, _) = setup();
        let clip = assets::store_audio(dir.path(), b"RIFFxxxxWAVEdata", "wav").expect("clip");
        store.add_audio(&clip, "Bell").expect("added");
        store
            .connection()
            .execute(
                "INSERT INTO category (id, kind, sound_id, sound_mode) VALUES (207, 'prayer', ?1, 'file')",
                params![clip.sha256],
            )
            .expect("a prayer using it");
        assert_eq!(
            store.templates_using(&clip.sha256).expect("uses"),
            [207],
            "a sound counts as used"
        );
        assert_eq!(
            store
                .remove_from_library(dir.path(), &clip.sha256, false)
                .expect("asked"),
            Removal::InUse(vec![207])
        );
        assert_eq!(
            store.audio_library().expect("list").len(),
            1,
            "nothing removed"
        );
        assert_eq!(
            store
                .remove_from_library(dir.path(), &clip.sha256, true)
                .expect("removed"),
            Removal::Removed
        );
        let sound_id: Option<String> = store
            .connection()
            .query_row("SELECT sound_id FROM category WHERE id = 207", [], |row| {
                row.get(0)
            })
            .expect("the prayer is still there");
        assert_eq!(sound_id, None, "it no longer points at the clip");
        // `sound_mode` stays 'file' with no clip, which `clip_for` plays as the chime.
    }

    #[test]
    fn a_templates_icon_image_and_sound_read_back_and_an_icon_in_use_asks_first() {
        // FEAT-0088: a prayer's template names one entry of each library.
        let (dir, store, source) = setup();
        let image = assets::import(dir.path(), &source).expect("import");
        store.add_to(Kind::Image, &image, "Mosque").expect("added");
        let icon_file = png(dir.path(), "mark.png", 48, 48);
        let icon = assets::import_icon(dir.path(), &icon_file).expect("icon");
        store.add_to(Kind::Icon, &icon, "Mark").expect("added");
        let clip = assets::store_audio(dir.path(), b"RIFFxxxxWAVEdata", "wav").expect("clip");
        store.add_audio(&clip, "Bell").expect("added");

        let edit = crate::reminders::PrayerTemplateEdit {
            title: None,
            body: None,
            accent: None,
            image: Some(image.sha256.clone()),
            icon: Some(icon.sha256.clone()),
            authored_lang: "en-US",
            sound_mode: Some("file".to_owned()),
            sound: Some(clip.sha256.clone()),
        };
        assert!(store.set_prayer_template("fajr", &edit).expect("saved"));
        let fajr = store
            .template_for_prayer("fajr")
            .expect("reads")
            .expect("there");
        assert_eq!(fajr.image.as_deref(), Some(image.relative_path.as_str()));
        assert_eq!(fajr.icon.as_deref(), Some(icon.relative_path.as_str()));
        assert_eq!(fajr.sound.as_deref(), Some(clip.relative_path.as_str()));

        // The icon counts as used, so removing it asks first and names the template.
        let users = store.templates_using(&icon.sha256).expect("uses");
        assert_eq!(users, [fajr.id]);
        assert_eq!(
            store
                .remove_from_library(dir.path(), &icon.sha256, false)
                .expect("asked"),
            Removal::InUse(vec![fajr.id])
        );
        // Confirmed, the template falls back to the app's own icon and keeps the rest.
        store
            .remove_from_library(dir.path(), &icon.sha256, true)
            .expect("removed");
        let after = store
            .template_for_prayer("fajr")
            .expect("reads")
            .expect("there");
        assert_eq!(after.icon, None);
        assert_eq!(after.image, fajr.image);
        assert_eq!(after.sound, fajr.sound);
    }
}
