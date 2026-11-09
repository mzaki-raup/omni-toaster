//! `config/settings.toml`, the source of truth for configuration (ADR-007).
//!
//! Human-editable TOML under the data folder, so a portable copy carries its
//! own configuration and someone can fix a bad value in Notepad. The
//! `settings` table in SQLite mirrors only what the scheduler must read inside
//! a transaction; it is not a second source of truth
//! (`docs/05-data-model-and-storage.md`).
//!
//! Values are stored as strings, not as this crate's own enums: `ot-core`
//! depends on nothing else in the workspace, so the language tag is parsed by
//! `ot-i18n` and the theme name by `ot-ui`. Each of those already has to
//! tolerate a value a human typed.
//!
//! **A broken file never stops the app starting.** It is reported and the
//! defaults are used, because the alternative is an app that cannot be
//! launched to fix its own settings.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::paths::DataRoot;

/// The file, relative to the data folder.
pub const SETTINGS_PATH: &[&str] = &["config", "settings.toml"];

/// Everything `config/settings.toml` holds.
///
/// Each pane has its own section rather than widening another's, so a field
/// is never silently shared between two panes.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub general: General,
    /// Settings ▸ Appearance: the toast's style, its placement and its
    /// header. Held as a raw table because `ot-core` depends on nothing else
    /// in the workspace; `ot_toast::Prefs` owns the shape and reads it, and a
    /// table it cannot read falls back to the defaults there rather than
    /// failing the whole file here.
    pub toast: toml::Table,
}

/// Settings ▸ General.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct General {
    /// A BCP-47 tag, or `None` to follow Windows. Absent means follow
    /// Windows, which is what a first run gets.
    pub language: Option<String>,
    /// `system`, `dark` or `light`. Dark is the product's default
    /// (`docs/04-ui-design.md`), and the string is parsed by `ot-ui`.
    pub theme: String,
}

impl Default for General {
    fn default() -> Self {
        Self {
            language: None,
            theme: "dark".to_string(),
        }
    }
}

impl Settings {
    /// The file's path inside a data folder.
    pub fn path_in(root: &DataRoot) -> PathBuf {
        SETTINGS_PATH
            .iter()
            .fold(root.path().to_path_buf(), |path, part| path.join(part))
    }

    /// Read the file, or return the defaults when it is absent.
    ///
    /// A file that cannot be read or parsed is an `Err` carrying the defaults
    /// alongside the reason, so a caller can report the problem *and* carry
    /// on — the file is the user's to edit, and a typo in it must not lock
    /// them out of the app that would let them fix it.
    pub fn load(root: &DataRoot) -> Result<Self, SettingsError> {
        let path = Self::path_in(root);
        match fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).map_err(|error| SettingsError::Parse {
                path,
                reason: error.message().to_string(),
            }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(SettingsError::Read { path, error }),
        }
    }

    /// Write the file, creating `config/` if it is not there yet.
    ///
    /// The write goes to a temporary file in the same folder and is renamed
    /// over the target, so a crash mid-write cannot leave a half-written
    /// settings file — the one file the app cannot start cleanly without.
    pub fn save(&self, root: &DataRoot) -> Result<(), SettingsError> {
        let path = Self::path_in(root);
        let folder = path
            .parent()
            .ok_or_else(|| SettingsError::Parse {
                path: path.clone(),
                reason: "the settings path has no parent folder".to_string(),
            })?
            .to_path_buf();
        fs::create_dir_all(&folder).map_err(|error| SettingsError::Write {
            path: folder.clone(),
            error,
        })?;

        let text = toml::to_string_pretty(self).map_err(|error| SettingsError::Serialize {
            reason: error.to_string(),
        })?;
        let temporary = path.with_extension("toml.tmp");
        fs::write(&temporary, text).map_err(|error| SettingsError::Write {
            path: temporary.clone(),
            error,
        })?;
        fs::rename(&temporary, &path).map_err(|error| SettingsError::Write {
            path: path.clone(),
            error,
        })
    }
}

/// The stored language choice, read before anything has been validated.
///
/// Startup needs the language *before* it can report a problem with the data
/// folder — those messages are bilingual and they are the only text a user
/// sees when the app gives up — and the choice lives inside that folder. So
/// this reads the file directly, creates nothing, validates nothing, and
/// answers `None` for every failure: an unreadable file here means "no
/// choice recorded", and the real load a moment later is what reports it.
pub fn stored_language(data_dir: Option<&Path>) -> Option<String> {
    let root = match data_dir {
        Some(dir) => dir.to_path_buf(),
        None => crate::paths::default_root().ok()?,
    };
    let path = SETTINGS_PATH
        .iter()
        .fold(root, |path, part| path.join(part));
    let text = fs::read_to_string(path).ok()?;
    let settings: Settings = toml::from_str(&text).ok()?;
    settings.general.language
}

/// Why `config/settings.toml` could not be read or written.
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("{path} could not be read: {error}")]
    Read { path: PathBuf, error: io::Error },
    #[error("{path} is not valid TOML: {reason}")]
    Parse { path: PathBuf, reason: String },
    #[error("{path} could not be written: {error}")]
    Write { path: PathBuf, error: io::Error },
    #[error("the settings could not be encoded as TOML: {reason}")]
    Serialize { reason: String },
}

impl SettingsError {
    /// The path the failure is about, for a message that names the file.
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Read { path, .. } | Self::Parse { path, .. } | Self::Write { path, .. } => {
                Some(path)
            }
            Self::Serialize { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> (tempfile::TempDir, DataRoot) {
        let dir = tempfile::tempdir().expect("a scratch folder");
        let root = DataRoot::resolve(Some(dir.path())).expect("the scratch folder resolves");
        (dir, root)
    }

    #[test]
    fn a_first_run_has_no_file_and_gets_the_defaults() {
        let (_dir, root) = scratch();
        let settings = Settings::load(&root).expect("an absent file is not an error");
        assert_eq!(settings, Settings::default());
        // Dark is the product's default, and follow-Windows is the default
        // language (docs/04-ui-design.md, docs/10-i18n.md).
        assert_eq!(settings.general.theme, "dark");
        assert_eq!(settings.general.language, None);
        assert!(
            !Settings::path_in(&root).exists(),
            "loading must not create the file; only a save writes it"
        );
    }

    #[test]
    fn what_is_saved_is_what_loads_back() {
        let (_dir, root) = scratch();
        let mut settings = Settings::default();
        settings.general.theme = "light".to_string();
        settings.general.language = Some("ms-MY".to_string());
        settings.save(&root).expect("the save succeeds");

        let loaded = Settings::load(&root).expect("the file parses");
        assert_eq!(loaded, settings);
    }

    #[test]
    fn the_file_lands_under_config_inside_the_data_folder() {
        // Portability: nothing is written outside the data folder
        // (.claude/rules/security.md).
        let (_dir, root) = scratch();
        Settings::default().save(&root).expect("the save succeeds");

        let path = Settings::path_in(&root);
        assert!(path.is_file(), "{} was not written", path.display());
        assert!(
            path.starts_with(root.path()),
            "{} is outside the data folder",
            path.display()
        );
        assert_eq!(
            path.parent().and_then(Path::file_name).expect("a parent"),
            "config"
        );
    }

    #[test]
    fn a_save_leaves_no_temporary_file_behind() {
        let (_dir, root) = scratch();
        Settings::default().save(&root).expect("the save succeeds");
        let temporary = Settings::path_in(&root).with_extension("toml.tmp");
        assert!(
            !temporary.exists(),
            "{} was left behind",
            temporary.display()
        );
    }

    #[test]
    fn a_broken_file_is_reported_and_names_itself() {
        let (_dir, root) = scratch();
        let path = Settings::path_in(&root);
        fs::create_dir_all(path.parent().expect("a parent")).expect("config/ is created");
        fs::write(&path, "[general]\ntheme = \n").expect("the broken file is written");

        let error = Settings::load(&root).expect_err("broken TOML is an error");
        assert_eq!(error.path(), Some(path.as_path()));
        // The caller carries on with the defaults; see the module docs.
        assert!(matches!(error, SettingsError::Parse { .. }));
    }

    #[test]
    fn an_unknown_key_is_refused_rather_than_silently_ignored() {
        // A typo'd key that loads as a default would look like the app
        // ignoring the user's edit, with nothing said.
        let (_dir, root) = scratch();
        let path = Settings::path_in(&root);
        fs::create_dir_all(path.parent().expect("a parent")).expect("config/ is created");
        fs::write(&path, "[general]\nthme = \"light\"\n").expect("the file is written");

        let error = Settings::load(&root).expect_err("an unknown key is an error");
        assert!(matches!(error, SettingsError::Parse { .. }));
    }

    #[test]
    fn a_windows_path_survives_a_round_trip() {
        // A double-quoted TOML string processes `\U`, so a Windows path has to
        // come back exactly as it went in (docs/GOTCHAS.md).
        let (_dir, root) = scratch();
        let mut settings = Settings::default();
        settings.general.language = Some(r"C:\Users\Someone".to_string());
        settings.save(&root).expect("the save succeeds");
        let loaded = Settings::load(&root).expect("the file parses");
        assert_eq!(
            loaded.general.language.as_deref(),
            Some(r"C:\Users\Someone")
        );
    }
}
