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
    /// Settings ▸ Reminders.
    pub reminders: Reminders,
    /// Settings ▸ Prayer ▸ Location (`docs/11-prayer-and-azan.md` §6).
    pub location: LocationSettings,
    /// Settings ▸ Prayer: the source, the zone and the calculation.
    pub prayer: PrayerSettings,
    /// Settings ▸ Weather (`docs/specs/weather.md` §9).
    pub weather: WeatherSettings,
}

/// Settings ▸ Weather. Off by default: turning it on sends the saved place,
/// rounded, to Open-Meteo, and turning it off stops every weather request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WeatherSettings {
    pub enabled: bool,
    /// Minutes between polls, within [`WEATHER_POLL_MINUTES`].
    pub poll_minutes: u32,
    /// The condition classes a change **to** notifies about: any of `clear`,
    /// `cloudy`, `rain`, `storm`, `snow` and `fog`.
    pub notify_classes: Vec<String>,
    /// Tell about a fall in temperature of `fall_delta_c` whole degrees (°C)
    /// or more within `fall_window_hours`.
    pub fall_enabled: bool,
    pub fall_delta_c: u32,
    pub fall_window_hours: u32,
    /// The same for a rise.
    pub rise_enabled: bool,
    pub rise_delta_c: u32,
    pub rise_window_hours: u32,
}

/// How often the weather may be polled, in minutes: the service's own values
/// change in 15-minute steps, so faster buys nothing (ADR-023).
pub const WEATHER_POLL_MINUTES: std::ops::RangeInclusive<u32> = 15..=240;

/// How big a sudden temperature move may be asked for, in whole °C.
pub const WEATHER_DELTA_C: std::ops::RangeInclusive<u32> = 1..=30;

/// Over how many hours: the request asks the service for six past hours.
pub const WEATHER_WINDOW_HOURS: std::ops::RangeInclusive<u32> = 1..=6;

/// The six classes, by the name the settings file and the service use.
pub const WEATHER_CLASSES: [&str; 6] = ["clear", "cloudy", "rain", "storm", "snow", "fog"];

impl Default for WeatherSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            poll_minutes: 30,
            notify_classes: WEATHER_CLASSES.map(str::to_owned).to_vec(),
            fall_enabled: true,
            fall_delta_c: 5,
            fall_window_hours: 3,
            rise_enabled: false,
            rise_delta_c: 8,
            rise_window_hours: 3,
        }
    }
}

impl WeatherSettings {
    /// The poll interval brought into range, so a hand-edited file cannot make
    /// the app poll every second.
    pub fn poll_minutes(&self) -> u32 {
        self.poll_minutes
            .clamp(*WEATHER_POLL_MINUTES.start(), *WEATHER_POLL_MINUTES.end())
    }

    /// The thresholds brought into range, so a hand-edited file cannot ask for
    /// a window the service's history does not cover.
    pub fn clamped(&self) -> Self {
        let delta = |value: u32| value.clamp(*WEATHER_DELTA_C.start(), *WEATHER_DELTA_C.end());
        let window =
            |value: u32| value.clamp(*WEATHER_WINDOW_HOURS.start(), *WEATHER_WINDOW_HOURS.end());
        Self {
            poll_minutes: self.poll_minutes(),
            fall_delta_c: delta(self.fall_delta_c),
            fall_window_hours: window(self.fall_window_hours),
            rise_delta_c: delta(self.rise_delta_c),
            rise_window_hours: window(self.rise_window_hours),
            ..self.clone()
        }
    }

    /// Whether a change to the class named `class` is one to tell about.
    pub fn notifies(&self, class: &str) -> bool {
        self.notify_classes.iter().any(|chosen| chosen == class)
    }
}

/// Settings ▸ Prayer (`docs/11-prayer-and-azan.md` §1, §3, §8). Strings, not
/// the prayer crate's enums, because `ot-core` depends on nothing else in the
/// workspace; `ot_prayer` parses them and an unknown value there falls back to
/// the default, so a hand-edited file cannot break the pane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PrayerSettings {
    /// `auto`, `jakim` or `calculated`.
    pub source: String,
    /// A JAKIM zone code the user chose, which overrides the detected zone.
    /// Absent follows the saved location.
    pub zone: Option<String>,
    /// `muslim-world-league`, `isna`, `umm-al-qura` or `malaysia`. Absent picks
    /// by location: `malaysia` inside Malaysia, `muslim-world-league` outside.
    pub preset: Option<String>,
    /// `shafii` or `hanafi`.
    pub asr: String,
    /// Whether Syuruk (sunrise) has a notification of its own. The five daily
    /// prayers always do; Imsak and Dhuha stay hidden until they can be
    /// switched on.
    pub notify_syuruk: bool,
    /// Whether Jumu'ah, the Friday prayer, has a notification. On Fridays it
    /// takes Dhuhr's place: one notification at Dhuhr's time, Jumu'ah's.
    pub notify_jumuah: bool,
    /// How late a prayer missed while the PC was off may be and still be
    /// listed, in minutes, within [`AZAN_WINDOW_MINUTES`].
    pub azan_window_minutes: u32,
    /// The prayers that also have a reminder ahead of them, by scheduler key.
    pub pre_prayer: Vec<String>,
    /// How far ahead that reminder comes, in minutes, within
    /// [`PRE_PRAYER_MINUTES`].
    pub pre_prayer_minutes: u32,
}

/// The catch-up window for a missed prayer, in minutes: one minute to a day.
pub const AZAN_WINDOW_MINUTES: std::ops::RangeInclusive<u32> = 1..=1440;

/// How far ahead of a prayer its reminder may come, in minutes: one to two hours.
pub const PRE_PRAYER_MINUTES: std::ops::RangeInclusive<u32> = 1..=120;

impl Default for PrayerSettings {
    fn default() -> Self {
        Self {
            source: "auto".to_string(),
            zone: None,
            preset: None,
            asr: "shafii".to_string(),
            notify_syuruk: false,
            notify_jumuah: false,
            azan_window_minutes: 60,
            pre_prayer: Vec::new(),
            pre_prayer_minutes: 10,
        }
    }
}

impl PrayerSettings {
    /// The prayers that have a notification, by the key the scheduler knows
    /// them by: the five daily prayers, and Syuruk if it is switched on.
    pub fn enabled_prayers(&self) -> Vec<&'static str> {
        let mut keys = vec!["fajr"];
        if self.notify_syuruk {
            keys.push("syuruk");
        }
        keys.push("dhuhr");
        if self.notify_jumuah {
            keys.push("jumuah");
        }
        keys.extend(["asr", "maghrib", "isha"]);
        keys
    }

    /// The Azan window brought into range, so a hand-edited file cannot make a
    /// missed prayer list for no time or for ever.
    pub fn azan_window(&self) -> u32 {
        self.azan_window_minutes
            .clamp(*AZAN_WINDOW_MINUTES.start(), *AZAN_WINDOW_MINUTES.end())
    }

    /// The lead of the pre-prayer reminder brought into range.
    pub fn pre_prayer_lead(&self) -> u32 {
        self.pre_prayer_minutes
            .clamp(*PRE_PRAYER_MINUTES.start(), *PRE_PRAYER_MINUTES.end())
    }

    /// The prayers that get a reminder ahead of them: those chosen that also
    /// have a notification, so Syuruk's is dormant while Syuruk is off.
    pub fn pre_prayer_keys(&self) -> Vec<&'static str> {
        self.enabled_prayers()
            .into_iter()
            .filter(|key| self.pre_prayer.iter().any(|chosen| chosen == key))
            .collect()
    }
}

/// Whether the app may look up the PC's place from its IP address. Nothing is
/// sent to the geolocation service until this is `Some(true)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LocationSettings {
    /// `None` until the user has been asked; then their answer. Written as
    /// `ip_consent = true` or `false`, and absent while unasked.
    pub ip_consent: Option<bool>,
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
    /// `automatic`, `12` or `24`, parsed by `ot_i18n::format::Clock`.
    /// Automatic is 24-hour in Malay and Windows' own choice in English
    /// (`docs/10-i18n.md`).
    pub clock: String,
}

impl Default for General {
    fn default() -> Self {
        Self {
            language: None,
            theme: "dark".to_string(),
            clock: "automatic".to_string(),
        }
    }
}

/// Settings ▸ Reminders: what the toast's Snooze does
/// (`docs/specs/reminders.md` §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Reminders {
    /// How long one press of Snooze puts a reminder off, in minutes. One of
    /// [`SNOOZE_PRESETS`], or a custom value within [`SNOOZE_MINUTES`].
    pub snooze_minutes: u32,
    /// How many times one fire may be snoozed before its toast offers only
    /// Dismiss, within [`SNOOZE_LIMIT`].
    pub snooze_limit: u32,
    /// How old a reminder missed while the PC was off or asleep may be and
    /// still be listed at the next start, in hours, within
    /// [`MISSED_WINDOW_HOURS`] (`docs/specs/reminders.md` §6).
    pub missed_window_hours: u32,
}

/// The snooze durations offered by name, in minutes: 5 minutes, 15 minutes,
/// 1 hour and 1 day. Anything else is Custom.
pub const SNOOZE_PRESETS: [u32; 4] = [5, 15, 60, 1440];

/// A custom snooze, in minutes: one minute to one week.
pub const SNOOZE_MINUTES: std::ops::RangeInclusive<u32> = 1..=10_080;

/// How many snoozes a fire may have.
pub const SNOOZE_LIMIT: std::ops::RangeInclusive<u32> = 1..=50;

/// The catch-up window for missed reminders, in hours: one hour to a week.
pub const MISSED_WINDOW_HOURS: std::ops::RangeInclusive<u32> = 1..=168;

impl Default for Reminders {
    fn default() -> Self {
        Self {
            snooze_minutes: 15,
            snooze_limit: 10,
            missed_window_hours: 24,
        }
    }
}

impl Reminders {
    /// The values brought into range, so a hand-edited file cannot snooze for
    /// no time at all or for ever.
    pub fn clamped(self) -> Self {
        Self {
            snooze_minutes: self
                .snooze_minutes
                .clamp(*SNOOZE_MINUTES.start(), *SNOOZE_MINUTES.end()),
            snooze_limit: self
                .snooze_limit
                .clamp(*SNOOZE_LIMIT.start(), *SNOOZE_LIMIT.end()),
            missed_window_hours: self
                .missed_window_hours
                .clamp(*MISSED_WINDOW_HOURS.start(), *MISSED_WINDOW_HOURS.end()),
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
    fn the_ip_lookup_consent_is_unasked_until_answered_and_survives_a_save() {
        let (_dir, root) = scratch();
        assert_eq!(Settings::default().location.ip_consent, None, "unasked");
        for answer in [true, false] {
            let mut settings = Settings::default();
            settings.location.ip_consent = Some(answer);
            settings.save(&root).expect("the save succeeds");
            let loaded = Settings::load(&root).expect("loads");
            assert_eq!(loaded.location.ip_consent, Some(answer));
        }
        // Unasked writes nothing, so a first run leaves no stray key.
        Settings::default().save(&root).expect("the save succeeds");
        let text = std::fs::read_to_string(Settings::path_in(&root)).expect("the file");
        assert!(!text.contains("ip_consent"), "{text}");
    }

    #[test]
    fn the_weather_settings_default_to_off_and_survive_a_save_and_are_brought_into_range() {
        let (_dir, root) = scratch();
        let defaults = Settings::default().weather;
        assert!(
            !defaults.enabled,
            "weather sends the place out, so it is off until asked for"
        );
        assert_eq!(
            (
                defaults.poll_minutes(),
                defaults.fall_delta_c,
                defaults.fall_window_hours
            ),
            (30, 5, 3)
        );
        assert!(defaults.fall_enabled && !defaults.rise_enabled);
        for class in WEATHER_CLASSES {
            assert!(defaults.notifies(class), "{class}");
        }
        assert!(!defaults.notifies("hail"));

        let mut settings = Settings::default();
        settings.weather.enabled = true;
        settings.weather.poll_minutes = 5;
        settings.weather.notify_classes = vec!["rain".into(), "storm".into()];
        settings.save(&root).expect("saved");
        let loaded = Settings::load(&root).expect("loads").weather;
        assert_eq!(loaded, settings.weather);
        assert_eq!(
            loaded.poll_minutes(),
            15,
            "five minutes is brought up to the minimum"
        );
        assert!(loaded.notifies("rain") && !loaded.notifies("snow"));
        let mut wild = loaded.clone();
        wild.fall_delta_c = 0;
        wild.fall_window_hours = 99;
        wild.rise_delta_c = 500;
        let tamed = wild.clamped();
        assert_eq!(
            (
                tamed.fall_delta_c,
                tamed.fall_window_hours,
                tamed.rise_delta_c
            ),
            (1, 6, 30)
        );
        let mut slow = loaded.clone();
        slow.poll_minutes = 100_000;
        assert_eq!(slow.poll_minutes(), 240);
    }

    #[test]
    fn the_prayer_settings_default_to_auto_and_shafii_and_survive_a_save() {
        let (_dir, root) = scratch();
        let defaults = Settings::default().prayer;
        assert_eq!(
            (
                defaults.source.as_str(),
                defaults.zone.as_deref(),
                defaults.preset.as_deref(),
                defaults.asr.as_str()
            ),
            ("auto", None, None, "shafii")
        );
        assert!(!defaults.notify_syuruk);
        assert_eq!(defaults.azan_window(), 60);
        assert_eq!(
            defaults.enabled_prayers(),
            ["fajr", "dhuhr", "asr", "maghrib", "isha"]
        );
        let mut on = defaults.clone();
        on.notify_syuruk = true;
        on.notify_jumuah = true;
        on.azan_window_minutes = 0;
        assert_eq!(
            on.enabled_prayers(),
            [
                "fajr", "syuruk", "dhuhr", "jumuah", "asr", "maghrib", "isha"
            ]
        );
        assert_eq!(on.azan_window(), 1, "a zero window is brought into range");
        assert_eq!(defaults.pre_prayer_lead(), 10);
        assert!(defaults.pre_prayer_keys().is_empty());
        on.pre_prayer = vec!["isha".into(), "syuruk".into(), "bogus".into()];
        on.pre_prayer_minutes = 0;
        assert_eq!(on.pre_prayer_keys(), ["syuruk", "isha"]);
        assert_eq!(on.pre_prayer_lead(), 1, "a zero lead is brought into range");
        on.notify_syuruk = false;
        assert_eq!(
            on.pre_prayer_keys(),
            ["isha"],
            "Syuruk's is dormant while it is off"
        );
        on.notify_syuruk = true;
        let mut settings = Settings::default();
        settings.prayer.source = "jakim".to_string();
        settings.prayer.zone = Some("SGR01".to_string());
        settings.prayer.preset = Some("isna".to_string());
        settings.prayer.asr = "hanafi".to_string();
        settings.save(&root).expect("the save succeeds");
        assert_eq!(
            Settings::load(&root).expect("loads").prayer,
            settings.prayer
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
    fn the_snooze_settings_default_round_trip_and_clamp() {
        let (_dir, root) = scratch();
        assert_eq!(
            Settings::default().reminders,
            Reminders {
                snooze_minutes: 15,
                snooze_limit: 10,
                missed_window_hours: 24,
            }
        );
        let mut settings = Settings::default();
        settings.reminders.snooze_minutes = 90;
        settings.reminders.snooze_limit = 3;
        settings.save(&root).expect("the save succeeds");
        assert_eq!(
            Settings::load(&root).expect("parses").reminders,
            settings.reminders
        );

        let wild = Reminders {
            snooze_minutes: 0,
            snooze_limit: 1000,
            missed_window_hours: 0,
        }
        .clamped();
        assert_eq!(wild.snooze_minutes, 1);
        assert_eq!(wild.snooze_limit, 50);
        assert_eq!(wild.missed_window_hours, 1);
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
