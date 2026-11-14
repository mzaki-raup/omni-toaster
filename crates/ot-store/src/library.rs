//! The image library: pictures uploaded once and reused by any template
//! (`docs/specs/toast.md` §8.1).
//!
//! The files are already content-addressed by [`crate::assets::import`]; what
//! this adds is the list — a row per picture, the label the user gave it, and
//! which templates use it.

use std::path::Path;

use rusqlite::{OptionalExtension as _, params};

use crate::assets::{self, Asset, ImportError};
use crate::{Store, StoreError};

/// The `asset.kind` of a library picture. One kind for both uses: any picture
/// may be an icon in one template and an image in another.
const KIND: &str = "image";

/// A picture in the library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryPicture {
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
    /// The same picture was already there, under this label, which is kept:
    /// uploading it again is not a reason to lose the name it was given.
    AlreadyThere(String),
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
    /// Record an imported picture in the library under `label`.
    pub fn add_to_library(&self, asset: &Asset, label: &str) -> Result<Added, StoreError> {
        let existing: Option<String> = self
            .conn
            .query_row(
                "SELECT label FROM asset WHERE sha256 = ?1",
                params![asset.sha256],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(existing) = existing {
            return Ok(Added::AlreadyThere(existing));
        }
        self.conn.execute(
            "INSERT INTO asset (sha256, kind, relative_path, bytes, created_at, label) \
             VALUES (?1, ?2, ?3, ?4, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?5)",
            params![
                asset.sha256,
                KIND,
                asset.relative_path,
                i64::try_from(asset.bytes).unwrap_or(i64::MAX),
                label
            ],
        )?;
        Ok(Added::New)
    }

    /// Every picture in the library, oldest first, so a new upload lands at
    /// the end of the list rather than reshuffling it.
    pub fn library(&self) -> Result<Vec<LibraryPicture>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT sha256, relative_path, label, bytes FROM asset \
             WHERE kind = ?1 ORDER BY created_at, rowid",
        )?;
        let rows = stmt.query_map(params![KIND], |row| {
            Ok(LibraryPicture {
                sha256: row.get(0)?,
                relative_path: row.get(1)?,
                label: row.get(2)?,
                bytes: u64::try_from(row.get::<_, i64>(3)?).unwrap_or(0),
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Rename a picture.
    pub fn set_label(&self, sha256: &str, label: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE asset SET label = ?2 WHERE sha256 = ?1",
            params![sha256, label],
        )?;
        Ok(())
    }

    /// The templates that use a picture, by `category` id.
    pub fn templates_using(&self, sha256: &str) -> Result<Vec<i64>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM category WHERE image_id = ?1 ORDER BY id")?;
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
            Added::AlreadyThere("Mosque".to_string())
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
                "INSERT INTO category (id, kind, image_id) VALUES (7, 'prayer', ?1)",
                params![asset.sha256],
            )
            .expect("a template using it");
        let file = dir.path().join(&asset.relative_path);

        assert_eq!(
            store
                .remove_from_library(dir.path(), &asset.sha256, false)
                .expect("asked"),
            Removal::InUse(vec![7])
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
            .query_row("SELECT image_id FROM category WHERE id = 7", [], |row| {
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
}
