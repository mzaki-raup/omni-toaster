//! The settings window: a left rail of sections, a content pane, a status
//! line (`docs/specs/tray-and-settings.md` §6).
//!
//! **General is the only section.** The other panes belong to later phases and
//! are deliberately not stubbed in as empty tabs — an empty tab promises
//! something that is not there.
//!
//! Every visible string comes from Fluent. A literal passed to a widget is a
//! defect, and `the_general_pane_names_only_keys_that_exist` is what catches
//! one.

use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};

use ot_core::{DataRoot, Pause, PauseFor, Settings};
use ot_i18n::Language;
use ot_platform::{Autostart, PauseWindow, Tray, TrayCommand};
use ot_store::reminders::NotificationAction;
use raw_window_handle::HasWindowHandle as _;

use crate::layout;
use crate::theme::{Theme, ThemeMode, Tokens};

/// How long the list of monitors is trusted before it is read again. Asking
/// Windows every frame is wasted work; a monitor unplugged mid-toast is
/// noticed within this long, and the toast moves to the primary one.
const MONITOR_REFRESH: Duration = Duration::from_secs(1);

/// How often the window wakes to look at the tray's channels.
///
/// The tray posts into a channel; nothing wakes egui when it does, so the
/// window asks to be repainted on this interval. Short enough that a menu
/// click feels immediate, long enough that an idle app is not spinning —
/// at 120 ms an idle OmniToaster does eight almost-empty frames a second,
/// and its window is usually hidden.
const TRAY_POLL: Duration = Duration::from_millis(120);

/// The interval between frames while a toast moves: 60 a second. Asked for
/// at the start of each frame, so the frame's own work falls inside it.
/// Without vsync nothing else paces the loop, and an unpaced loop spins a
/// core for the length of every animation.
const ANIMATION_FRAME: Duration = Duration::from_micros(16_667);

/// The window's sections. Each arrives with the phase that gives it
/// something to hold; none is stubbed in as an empty tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    General,
    /// The toast's look and placement, with a live preview (Phase 2).
    Appearance,
    /// The image library: pictures uploaded once and used by any
    /// notification (Phase 2, `docs/specs/toast.md` §8.1).
    Pictures,
    /// How toasts arrive and leave (Phase 3, `docs/specs/toast.md` §5).
    Animation,
    /// What Snooze does now; the reminder list and editor with FEAT-0046
    /// (Phase 4, `docs/specs/reminders.md` §10).
    Reminders,
    /// The prayer source, zone and location, and today's times (Phase 5,
    /// `docs/11-prayer-and-azan.md`).
    Prayer,
    Weather,
    /// Which version this is, and looking for and installing a newer one
    /// (`docs/specs/updater.md` §7).
    Updates,
}

impl Section {
    /// The Fluent key naming the section in the rail.
    pub fn name_key(self) -> &'static str {
        match self {
            Self::General => "settings-section-general",
            Self::Appearance => "settings-section-appearance",
            Self::Pictures => "settings-section-pictures",
            Self::Animation => "settings-section-animation",
            Self::Reminders => "settings-section-reminders",
            Self::Prayer => "settings-section-prayer",
            Self::Weather => "settings-section-weather",
            Self::Updates => "settings-section-updates",
        }
    }

    pub fn all() -> [Self; 8] {
        [
            Self::General,
            Self::Appearance,
            Self::Animation,
            Self::Pictures,
            Self::Reminders,
            Self::Prayer,
            Self::Weather,
            Self::Updates,
        ]
    }
}

/// What the language picker offers. `System` is the absence of a choice, which
/// is what a first run has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageChoice {
    System,
    Fixed(Language),
}

impl LanguageChoice {
    /// The Fluent key naming the choice in the picker. Each language names
    /// itself in its own language, which is what a language picker is for.
    pub fn name_key(self) -> &'static str {
        match self {
            Self::System => "language-system",
            Self::Fixed(language) => language.name_key(),
        }
    }

    /// What goes in `settings.toml`: a tag, or nothing for "follow Windows".
    pub fn as_setting(self) -> Option<String> {
        match self {
            Self::System => None,
            Self::Fixed(language) => Some(language.tag().to_string()),
        }
    }

    /// The language this choice actually means now.
    ///
    /// `System` is resolved against Windows each time it is asked for rather
    /// than once at startup, so a user who changes their Windows display
    /// language and comes back gets what they would expect.
    pub fn resolve(self) -> Language {
        match self {
            Self::Fixed(language) => language,
            Self::System => match ot_platform::user_locale() {
                Some(tag) => Language::from_os_locale(&tag),
                None => Language::English,
            },
        }
    }

    /// Read a stored value. An unrecognised tag means follow Windows rather
    /// than refusing to start.
    pub fn parse(stored: Option<&str>) -> Self {
        match stored {
            None => Self::System,
            Some(tag) => match tag.to_ascii_lowercase().as_str() {
                "en-us" | "en" => Self::Fixed(Language::English),
                "ms-my" | "ms" => Self::Fixed(Language::Malay),
                // A tag nobody recognises is treated as no choice at all.
                _ => Self::System,
            },
        }
    }

    pub fn all() -> [Self; 3] {
        [
            Self::System,
            Self::Fixed(Language::English),
            Self::Fixed(Language::Malay),
        ]
    }
}

/// What the status line is currently saying.
///
/// A key plus its argument rather than a formatted string, so the line is
/// re-rendered in the new language when the language changes rather than
/// holding text in the language it was built in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Ready,
    Saved,
    AutostartOn,
    AutostartOff,
    AutostartElsewhere,
    /// Notifications are paused. The text is built from the pause itself, so
    /// it says *until when* rather than only that something happened.
    Paused,
    Resumed,
    /// A picture went into the library under this label.
    LibraryAdded {
        label: String,
    },
    /// The same picture was already there, under this label.
    LibraryAlreadyThere {
        label: String,
    },
    LibraryRemoved,
    /// A file broke an import rule; the message names the file and the rule.
    ImportRefused {
        key: &'static str,
        name: String,
    },
    Failed {
        key: &'static str,
        reason: String,
    },
}

impl Status {
    /// The line to show. `pause` is passed in because the paused line says
    /// until when, which is state rather than a message.
    fn text_with_pause(&self, pause: Pause) -> String {
        match self {
            Self::Paused => match pause {
                Pause::Until(moment) => ot_i18n::text_with(
                    "settings-status-paused-until",
                    &[("time", formatted_time(moment).into())],
                ),
                // Running should not reach here, but saying "paused" of a
                // running app would be worse than saying the open-ended line.
                Pause::UntilResumed | Pause::Running => {
                    ot_i18n::text("settings-status-paused-until-resumed")
                }
            },
            other => other.text(),
        }
    }

    fn text(&self) -> String {
        match self {
            Self::Paused => ot_i18n::text("settings-status-paused-until-resumed"),
            Self::Resumed => ot_i18n::text("settings-status-resumed"),
            Self::LibraryAdded { label } => ot_i18n::text_with(
                "settings-status-library-added",
                &[("label", label.clone().into())],
            ),
            Self::LibraryAlreadyThere { label } => ot_i18n::text_with(
                "settings-status-library-already-there",
                &[("label", label.clone().into())],
            ),
            Self::LibraryRemoved => ot_i18n::text("settings-status-library-removed"),
            Self::ImportRefused { key, name } => {
                ot_i18n::text_with(key, &[("name", name.clone().into())])
            }
            Self::Ready => ot_i18n::text("settings-status-ready"),
            Self::Saved => ot_i18n::text("settings-status-saved"),
            Self::AutostartOn => ot_i18n::text("settings-status-autostart-on"),
            Self::AutostartOff => ot_i18n::text("settings-status-autostart-off"),
            Self::AutostartElsewhere => ot_i18n::text("settings-startup-elsewhere"),
            Self::Failed { key, reason } => {
                ot_i18n::text_with(key, &[("reason", reason.clone().into())])
            }
        }
    }

    fn is_failure(&self) -> bool {
        matches!(self, Self::Failed { .. } | Self::ImportRefused { .. })
    }
}

/// The window's state.
pub struct SettingsWindow {
    root: DataRoot,
    settings: Settings,
    theme: Theme,
    section: Section,
    language: LanguageChoice,
    /// The registry's answer, not the settings file's: a value removed by
    /// other means must show as off (`docs/specs/tray-and-settings.md` §7).
    autostart: Autostart,
    status: Status,
    /// Whether the palette has reached the egui context yet. Applying it is a
    /// style clone, so it happens when it changes and not every frame.
    applied: bool,
    /// The tray icon, held for as long as the app runs — dropping it takes
    /// the icon out of the notification area. `None` when it could not be
    /// created, which is reported rather than hidden.
    tray: Option<Tray>,
    /// Whether notifications are suppressed, and until when.
    pause: Pause,
    /// Set when the user has asked to quit and the close is ours to let
    /// through, rather than a window close that only hides.
    quitting: bool,
    /// Every toast on screen, and the ones waiting for room.
    toasts: ot_toast::Toasts,
    /// Settings ▸ Appearance, which the preview and every real toast read.
    prefs: ot_toast::Prefs,
    /// Toast pictures, decoded off the UI thread.
    images: crate::images::Images,
    /// The file dialog and the import behind it, also off the UI thread.
    importer: crate::images::Importer,
    /// The image library, on a thread of its own with its own connection.
    library: crate::library::Library,
    /// A removal waiting for the user's word because something uses the
    /// picture: its hash, and whether any template does.
    confirm_remove: Option<(String, bool)>,
    /// The test toast's own icon and image, under Test Toast Content. In
    /// memory only: every notification carries its own pictures, so these
    /// are never a global setting.
    test_icon: Option<String>,
    test_image: Option<String>,
    test_image_border: bool,
    test_accent: Option<ot_toast::Rgba>,
    /// Set when the prefs changed and have not been written yet. A slider
    /// drag changes them every frame; they are saved when the pointer is
    /// released, not sixty times a second.
    prefs_dirty: bool,
    /// Set when Quit was asked for while a pause is running: the window comes
    /// back with a confirmation rather than the app going quiet for good
    /// while the user still thinks it is only muted.
    confirming_quit: bool,
    /// The attached monitors, and when they were last asked for.
    monitors: Vec<ot_platform::monitors::Monitor>,
    monitors_read: Option<Instant>,
    /// Whether Windows shows animations, and when it was last asked: off
    /// means every toast fades (FEAT-0035).
    windows_animations: bool,
    motion_read: Option<Instant>,
    /// The development switches this run was started with.
    dev: DevOptions,
    /// Whether the chosen monitor was missing last time, so falling back to
    /// the primary one is logged once, not every frame.
    monitor_missing: bool,
    /// The title bar's mode as Windows last accepted it: dark, light, or not
    /// set yet (CR-0002).
    title_bar_dark: Option<bool>,
    /// The window's own handle, taken from the frame each update, for the
    /// Win32 calls that put the window away and bring it back.
    window: Option<raw_window_handle::RawWindowHandle>,
    /// Where the window was when it was parked; `None` while it is showing
    /// (BUG-0008).
    parked: Option<ot_platform::window_park::Parked>,
    /// The scheduler's fires, drained at frame start; `None` when no
    /// scheduler runs, as in the layout tests.
    fires: Option<Receiver<ot_sched::scheduler::Fired>>,
    /// Where the history rows go: the scheduler holds the store, and the UI
    /// thread never touches SQLite.
    scheduler: Option<Sender<ot_sched::scheduler::Command>>,
    /// Which toasts are reminders, and which fire each one is, so a button
    /// press can be written to the history.
    reminder_toasts: HashMap<ot_toast::ToastId, ReminderToast>,
    /// Set when Settings ▸ Reminders changed and has not been written yet;
    /// written when the pointer is released, as in Appearance.
    reminders_dirty: bool,
    /// Reminders missed while the PC was off or asleep, listed in Settings ▸
    /// Reminders with Snooze and Dismiss (docs/specs/reminders.md §6).
    missed: Vec<MissedEntry>,
    /// The one toast saying reminders were missed, while it is up; its Open
    /// lands on the list.
    missed_toast: Option<ot_toast::ToastId>,
    /// The reminders' own thread; `None` until `run` starts it with the
    /// scheduler's sender, and in the layout tests.
    book: Option<crate::reminder_book::Book>,
    /// The Prayer pane's own state and its threads' controller (FEAT-0057).
    prayer: crate::prayer_pane::State,
    /// The Weather pane's own state (FEAT-0070).
    weather: crate::weather_pane::State,
    /// The Updates pane's own state (FEAT-0080).
    update: crate::update_pane::State,
    /// The service that looks for, downloads and stages updates; `None` until
    /// `run` starts it, and in the layout tests.
    update_service: Option<crate::update_service::Service>,
    /// The service that keeps the day's times current while Settings is closed;
    /// `None` until `run` starts it, and in the layout tests (FEAT-0058).
    prayer_service: Option<crate::prayer_service::Service>,
    /// The service that polls the weather and says when it changes; `None` until
    /// `run` starts it, and in the layout tests (FEAT-0069).
    weather_service: Option<crate::weather_service::Service>,
    /// Where today's prayer times came from, for the label on a prayer's toast.
    prayer_source: Option<ot_prayer::chain::Source>,
    /// The place the prayer times are for, as a prayer's toast names it
    /// (FEAT-0076); `None` until the service has said.
    prayer_place: Option<String>,
    /// The audio thread: opens no device until something plays (ADR-022).
    audio: ot_media::Player,
    /// The prayer toasts whose sound is playing, so it stops with the toast.
    sounding: std::collections::HashSet<ot_toast::ToastId>,
    /// The list's rows, built when a list arrives, never per frame.
    reminder_rows: Option<Vec<crate::reminders::ListRow>>,
    /// The reminder being edited, if any.
    editor: Option<crate::reminder_editor::Editor>,
    /// The edited reminder's history, formatted when it arrives.
    reminder_history: Vec<String>,
    /// Set while Clear History's question is up.
    confirming_clear: bool,
    /// A reminder to open in the editor once the list holds it: after a save,
    /// or from Open on its toast.
    select_when_listed: Option<i64>,
    /// When to read the list again: just after a save, so the row shows the
    /// next time the scheduler has worked out by then.
    refresh_at: Option<Instant>,
    /// The fatigue kinds as the section shows them, and whether they were
    /// changed and not sent yet (sent when the pointer is released).
    fatigue_rows: Vec<crate::reminders::FatigueRow>,
    fatigue_dirty: bool,
}

/// A missed fire in the list: what its buttons act on, and its row's text,
/// formatted once when it arrived rather than every frame.
#[derive(Debug, Clone)]
struct MissedEntry {
    reminder_id: i64,
    fired_at: jiff::Timestamp,
    snoozes: u32,
    /// The reminder and its template, so the row can be said again in
    /// another language or with another clock.
    reminder: ot_store::reminders::Reminder,
    template: Option<ot_store::reminders::Template>,
    row: crate::reminders::MissedRow,
}

/// Which fire a reminder toast is, for its buttons.
#[derive(Debug, Clone, Copy)]
struct ReminderToast {
    reminder_id: i64,
    fired_at: jiff::Timestamp,
    /// How many times this fire has been snoozed already.
    snoozes: u32,
    /// Whether it waits to be acted on, up to its cap.
    persistent: bool,
}

impl SettingsWindow {
    /// Build the window's state from the data folder.
    ///
    /// The autostart state is read here and after every change, never cached
    /// from the settings file.
    pub fn new(root: DataRoot, settings: Settings) -> Self {
        // Every time the app writes follows the user's clock and Windows'
        // regional answer, read once here (FEAT-0047).
        let regional = ot_platform::regional();
        ot_i18n::format::set_regional(ot_i18n::format::Regional {
            month_first: regional.month_first,
            twenty_four_hour: regional.twenty_four_hour,
        });
        ot_i18n::format::set_clock(ot_i18n::format::Clock::parse(&settings.general.clock));
        let language = LanguageChoice::parse(settings.general.language.as_deref());
        let settings_toast = settings.toast.clone();
        let root_path = root.path().to_path_buf();
        let theme = Theme::new(ThemeMode::parse(&settings.general.theme));
        let (autostart, status) = match ot_platform::autostart::status() {
            Ok(state @ Autostart::EnabledElsewhere { .. }) => (state, Status::AutostartElsewhere),
            Ok(state) => (state, Status::Ready),
            Err(error) => (
                Autostart::Disabled,
                Status::Failed {
                    key: "settings-status-autostart-failed",
                    reason: error.to_string(),
                },
            ),
        };
        let pause = Pause::default();
        let (tray, status) = match Tray::new(
            &tray_labels(pause),
            pause.is_active(jiff::Timestamp::now()),
            // Which window owns the icon, so a killed run's ghost can be
            // removed on the next start (CR-0003). In the cache: it is
            // derived data, and losing it costs only a ghost.
            &root_path.join("cache").join("tray-window"),
        ) {
            Ok(tray) => (Some(tray), status),
            Err(error) => {
                // The app still works without it, but the user has lost the
                // only way to reach it once the window is closed, so this is
                // said out loud rather than logged and forgotten.
                tracing::error!(%error, "the tray icon could not be created");
                (
                    None,
                    Status::Failed {
                        key: "settings-status-tray-failed",
                        reason: error.to_string(),
                    },
                )
            }
        };

        Self {
            root,
            settings,
            theme,
            section: Section::General,
            language,
            autostart,
            status,
            applied: false,
            tray,
            pause,
            quitting: false,
            toasts: ot_toast::Toasts::new(),
            prefs: ot_toast::Prefs::from_table(&settings_toast),
            images: crate::images::Images::new(&root_path),
            importer: crate::images::Importer::new(&root_path),
            library: crate::library::Library::new(&root_path),
            confirm_remove: None,
            test_icon: None,
            test_image: None,
            test_image_border: false,
            test_accent: None,
            prefs_dirty: false,
            confirming_quit: false,
            monitors: Vec::new(),
            monitors_read: None,
            windows_animations: true,
            motion_read: None,
            dev: DevOptions::default(),
            monitor_missing: false,
            title_bar_dark: None,
            window: None,
            parked: None,
            fires: None,
            scheduler: None,
            reminder_toasts: HashMap::new(),
            reminders_dirty: false,
            missed: Vec::new(),
            missed_toast: None,
            book: None,
            prayer: crate::prayer_pane::State::new(&root_path),
            weather: crate::weather_pane::State::new(&root_path),
            update: crate::update_pane::State::new(&root_path),
            update_service: None,
            prayer_service: None,
            weather_service: None,
            prayer_source: None,
            prayer_place: None,
            audio: ot_media::Player::start(),
            sounding: std::collections::HashSet::new(),
            reminder_rows: None,
            editor: None,
            reminder_history: Vec::new(),
            confirming_clear: false,
            select_when_listed: None,
            refresh_at: None,
            fatigue_rows: fatigue_rows(None),
            fatigue_dirty: false,
        }
    }

    /// The theme, so the caller can apply it to the context before the first
    /// frame rather than letting one frame draw in egui's own colours.
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    fn save(&mut self) {
        match self.settings.save(&self.root) {
            Ok(()) => self.status = Status::Saved,
            Err(error) => {
                tracing::warn!(%error, "the settings could not be saved");
                self.status = Status::Failed {
                    key: "settings-status-save-failed",
                    reason: error.to_string(),
                };
            }
        }
    }

    fn set_theme(&mut self, mode: ThemeMode) {
        self.theme.set_mode(mode);
        self.applied = false;
        self.settings.general.theme = mode.as_str().to_string();
        self.save();
    }

    /// Change the language of the running app (FEAT-0016).
    ///
    /// Both surfaces have to follow, and they follow differently: the window
    /// reads every string afresh on its next frame, while the tray menu was
    /// built once and belongs to Windows until it is rebuilt. The window's
    /// title is the third: it is set on the viewport, not drawn, so it keeps
    /// the old language until it is told.
    fn set_language(&mut self, choice: LanguageChoice, ctx: &egui::Context) {
        self.language = choice;
        self.settings.general.language = choice.as_setting();

        let language = choice.resolve();
        // Phase 1's exit criterion is a *timing* one — both surfaces inside a
        // second — so the switch says how long it took. Logging the
        // measurement makes it checkable on a user's machine, not only on
        // this one.
        let started = std::time::Instant::now();
        if let Err(error) = ot_i18n::set_language(language) {
            tracing::error!(%error, ?language, "the language could not be switched");
            self.status = Status::Failed {
                key: "settings-status-language-failed",
                reason: error.to_string(),
            };
            return;
        }

        ctx.send_viewport_cmd(egui::ViewportCommand::Title(ot_i18n::text(
            "settings-window-title",
        )));
        self.rebuild_tray(jiff::Timestamp::now());
        self.reformat();
        self.save();
        tracing::info!(
            ?language,
            micros = started.elapsed().as_micros(),
            "the language was switched, catalogues and tray menu included"
        );
    }

    /// Turn autostart on or off, then **re-read the registry**, so the toggle
    /// shows what is true rather than what was asked for.
    fn set_autostart(&mut self, on: bool) {
        let outcome = if on {
            ot_platform::autostart::enable()
        } else {
            ot_platform::autostart::disable()
        };
        if let Err(error) = outcome {
            tracing::warn!(%error, on, "the autostart value could not be changed");
            self.status = Status::Failed {
                key: "settings-status-autostart-failed",
                reason: error.to_string(),
            };
            return;
        }
        match ot_platform::autostart::status() {
            Ok(state) => {
                self.autostart = state;
                self.status = if self.autostart.is_on() {
                    Status::AutostartOn
                } else {
                    Status::AutostartOff
                };
            }
            Err(error) => {
                self.status = Status::Failed {
                    key: "settings-status-autostart-failed",
                    reason: error.to_string(),
                };
            }
        }
    }

    fn rail(&mut self, ui: &mut egui::Ui) {
        for section in Section::all() {
            let selected = section == self.section;
            let label = ot_i18n::text(section.name_key());
            // Selection changes the fill, never the size (ui.md).
            if ui
                .add_sized(
                    [ui.available_width(), layout::field_height(ui)],
                    egui::Button::selectable(selected, egui::RichText::new(label).strong()),
                )
                .clicked()
            {
                self.section = section;
            }
            ui.add_space(layout::ROW_GAP);
        }
    }

    fn general_pane(&mut self, ui: &mut egui::Ui) {
        let tokens = self.theme.tokens();
        // Worked out once, so every row divides the same way and the fields
        // line up in one column.
        let columns = layout::columns(ui);
        let width = columns.field;

        // Language.
        let mut chosen = self.language;
        layout::labelled_row(ui, columns, &ot_i18n::text("settings-language"), |ui| {
            egui::ComboBox::from_id_salt("settings-language")
                .width(width)
                .height(layout::field_height(ui))
                .selected_text(ot_i18n::text(self.language.name_key()))
                .show_ui(ui, |ui| {
                    for choice in LanguageChoice::all() {
                        ui.selectable_value(&mut chosen, choice, ot_i18n::text(choice.name_key()));
                    }
                });
        });
        if chosen != self.language {
            self.set_language(chosen, ui.ctx());
        }
        // What System means is the part a user cannot guess; the switch
        // itself is immediate and needs no explaining.
        layout::field_hint(
            ui,
            columns,
            &ot_i18n::text("settings-language-hint"),
            tokens.text_dim,
        );
        ui.add_space(layout::ROW_GAP);

        // Theme.
        let mut mode = self.theme.mode();
        layout::labelled_row(ui, columns, &ot_i18n::text("settings-theme"), |ui| {
            egui::ComboBox::from_id_salt("settings-theme")
                .width(width)
                .height(layout::field_height(ui))
                .selected_text(ot_i18n::text(self.theme.mode().name_key()))
                .show_ui(ui, |ui| {
                    for option in ThemeMode::all() {
                        ui.selectable_value(&mut mode, option, ot_i18n::text(option.name_key()));
                    }
                });
        });
        if mode != self.theme.mode() {
            self.set_theme(mode);
        }
        ui.add_space(layout::ROW_GAP);

        // Clock.
        let clock = ot_i18n::format::Clock::parse(&self.settings.general.clock);
        let mut chosen_clock = clock;
        layout::labelled_row(ui, columns, &ot_i18n::text("settings-clock"), |ui| {
            egui::ComboBox::from_id_salt("settings-clock")
                .width(width)
                .height(layout::field_height(ui))
                .selected_text(ot_i18n::text(clock.name_key()))
                .show_ui(ui, |ui| {
                    for option in ot_i18n::format::Clock::all() {
                        ui.selectable_value(
                            &mut chosen_clock,
                            option,
                            ot_i18n::text(option.name_key()),
                        );
                    }
                });
        });
        layout::field_hint(
            ui,
            columns,
            &ot_i18n::text("settings-clock-hint"),
            tokens.text_dim,
        );
        if chosen_clock != clock {
            self.settings.general.clock = chosen_clock.as_str().to_owned();
            ot_i18n::format::set_clock(chosen_clock);
            self.reformat();
            self.save();
        }
        ui.add_space(layout::ROW_GAP);

        // Start with Windows.
        let mut on = self.autostart.is_on();
        let before = on;
        layout::labelled_row(ui, columns, &ot_i18n::text("settings-startup"), |ui| {
            ui.checkbox(&mut on, "");
        });
        if on != before {
            self.set_autostart(on);
        }
        layout::field_hint(
            ui,
            columns,
            &ot_i18n::text("settings-startup-hint"),
            tokens.text_dim,
        );
    }

    /// Act on everything the tray has to say, and keep the menu in step.
    fn handle_tray(&mut self, ctx: &egui::Context) {
        let now = jiff::Timestamp::now();
        // A lapsed pause clears itself, and the menu goes back to offering a
        // pause rather than a resume.
        let mut menu_is_stale = self.pause.lapse(now);

        let commands = match &self.tray {
            Some(tray) => tray.drain(),
            None => Vec::new(),
        };

        for command in commands {
            match command {
                TrayCommand::OpenSettings => self.show_window(ctx),
                TrayCommand::Pause(window) => {
                    self.pause = Pause::start(pause_window(window), now);
                    self.status = Status::Paused;
                    menu_is_stale = true;
                }
                TrayCommand::Resume => {
                    self.pause = Pause::Running;
                    self.status = Status::Resumed;
                    menu_is_stale = true;
                }
                TrayCommand::TestToast => self.show_test_toast(),
                // Still disabled in the menu until Phase 4, so it cannot
                // arrive; if it does, that is a defect worth a line rather
                // than a silent no-op.
                TrayCommand::SnoozeLast => {
                    tracing::warn!(?command, "a disabled tray item was clicked");
                }
                TrayCommand::Quit => {
                    if self.pause.is_active(now) {
                        // Quitting silently while muted leaves someone
                        // waiting for notifications that will never come
                        // (docs/specs/tray-and-settings.md §6).
                        self.confirming_quit = true;
                        self.show_window(ctx);
                    } else {
                        self.quit(ctx);
                    }
                }
            }
        }

        if menu_is_stale {
            self.rebuild_tray(now);
        }
    }

    /// Rebuild the menu and the tooltip for the current language and pause.
    fn rebuild_tray(&mut self, now: jiff::Timestamp) {
        let paused = self.pause.is_active(now);
        let labels = tray_labels(self.pause);
        if let Some(tray) = self.tray.as_mut()
            && let Err(error) = tray.rebuild(&labels, paused)
        {
            tracing::warn!(%error, "the tray menu could not be rebuilt");
            self.status = Status::Failed {
                key: "settings-status-tray-failed",
                reason: error.to_string(),
            };
        }
    }

    /// Fire one toast with the current style, for the tray's Test Toast.
    ///
    /// The content is localized here rather than in `ot-toast`: a `ToastSpec`
    /// carries text, never keys, which is what lets a template hold a user's
    /// own words beside a built-in default.
    pub fn show_test_toast(&mut self) {
        let spec = self.test_spec();
        let id = self.toasts.push(spec, Instant::now());
        tracing::info!(?id, "a test toast was fired");
    }

    /// Fire one toast of each shape, for looking at the renderer.
    ///
    /// A development seam, like `--hold-ms`: it is how a session that cannot
    /// click the tray gets every shape on screen at once, and it exercises
    /// stacking while it is there.
    pub fn show_shape_samples(&mut self) {
        for (index, shape) in ot_toast::Shape::all().into_iter().enumerate() {
            let mut spec = self.test_spec();
            spec.style.shape = shape;
            spec.content.title = ot_i18n::text(shape.name_key());
            // The third sample carries a long body, so a screenshot shows
            // the wrap and the ellipsis as well as the frame.
            if index == 2 {
                spec.content.body = format!("{} {}", spec.content.body, spec.content.body);
            }
            // The first carries a long title, a long app name and a long body
            // together: the case where the header has to share one line and
            // the title is what gives way (FEAT-0020).
            if index == 0 {
                spec.content.title = format!("{0} {0} {0}", spec.content.title);
                spec.content.app_name = format!("{0} {0}", spec.content.app_name);
                spec.content.body = format!("{0} {0} {0}", spec.content.body);
            }
            // One of each shadow, and a gradient and a border on the later
            // ones, so a single screenshot shows the whole of FEAT-0019.
            if index == 1 {
                spec.style.background_to = Some(ot_toast::Rgba::new(0x12, 0x2a, 0x4a, 0xff));
            }
            if index == 2 {
                spec.style.border = 2.0;
            }
            let id = self.toasts.push(spec, Instant::now());
            tracing::info!(?id, ?shape, "a sample toast was fired");
        }
    }

    /// The test toast's spec, built by the same function as every real
    /// toast and as the preview, so the three cannot disagree.
    fn test_spec(&self) -> ot_toast::ToastSpec {
        self.prefs.spec(ot_toast::Content {
            title: ot_i18n::text("toast-test-title"),
            body: ot_i18n::text("toast-test-body"),
            app_name: ot_i18n::text("toast-app-name"),
            action_labels: crate::reminder_toast::action_labels(),
            // Formatted here: ot-i18n owns the one formatter that may turn a
            // time into text, and a spec holds text rather than keys.
            timestamp: Some(formatted_time(jiff::Timestamp::now())),
            // The test toast's own pictures, like any notification's: chosen
            // under Test Toast Content and never part of the global style.
            icon: self.test_icon.clone(),
            image: self.test_image.clone(),
            image_border: self.test_image_border,
            accent: self.test_accent,
            ..ot_toast::Content::default()
        })
    }

    /// Ask for the pictures a toast needs and hand back the ones decoded.
    ///
    /// The header shows the notification's own icon, or the app's when it
    /// brings none; the body shows its image, if it has one.
    fn pictures_for(
        &mut self,
        content: &ot_toast::Content,
    ) -> (
        Option<ot_toast::content::Picture>,
        Option<ot_toast::content::Picture>,
    ) {
        let icon = content
            .icon
            .clone()
            .unwrap_or_else(|| crate::images::APP_ICON.to_string());
        self.images.request(&icon);
        if let Some(image) = content.image.as_deref() {
            self.images.request_image(image);
        }
        (
            self.images.picture(&icon),
            content
                .image
                .as_deref()
                .and_then(|path| self.images.picture_image(path)),
        )
    }

    /// The work area of the monitor toasts go on: the chosen one, or the
    /// primary one when none is chosen or the chosen one is not connected.
    /// A plain 1080p one when Windows will not say, so the map still draws.
    fn work_area(&mut self) -> egui::Rect {
        if self
            .monitors_read
            .is_none_or(|read| read.elapsed() >= MONITOR_REFRESH)
        {
            self.monitors = ot_platform::monitors::monitors();
            self.monitors_read = Some(Instant::now());
        }
        let wanted = self.prefs.monitor.as_ref().map(|choice| choice.id.as_str());
        let Some((monitor, missing)) = ot_platform::monitors::pick(&self.monitors, wanted) else {
            return egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1032.0));
        };
        if missing != self.monitor_missing {
            if missing {
                tracing::warn!(
                    wanted,
                    "the chosen monitor is not connected; toasts go to the primary one"
                );
            } else {
                tracing::info!("toasts are on the chosen monitor again");
            }
            self.monitor_missing = missing;
        }
        monitor.work_area_rect()
    }

    /// Move every toast along its timeline, close what has gone, and draw
    /// what is left.
    fn drive_toasts(&mut self, ctx: &egui::Context) {
        // One clock for every toast in this frame (docs/specs/toast.md §5).
        let now = Instant::now();
        for id in self.toasts.tick(now) {
            // A gone toast's viewport is told to close at once, rather than
            // left until egui notices a frame without it.
            ctx.send_viewport_cmd_to(id.viewport_id(), egui::ViewportCommand::Close);
            ot_platform::toast_window::forget(&id.window_title());
            // A prayer's sound stops with its toast, dismissed or timed out.
            if self.sounding.remove(&id) {
                self.audio.stop();
            }
            if self.missed_toast == Some(id) {
                self.missed_toast = None;
            }
            // A reminder toast that left on its own was shown and nothing
            // more; its row was written when it appeared. A persistent one
            // waited to be acted on and was not: it closed at its cap, and
            // that is a missed reminder (docs/specs/reminders.md §8).
            if let Some(toast) = self.reminder_toasts.remove(&id)
                && toast.persistent
            {
                tracing::info!(reminder = toast.reminder_id, "an alarm closed at its cap");
                self.record(
                    toast.reminder_id,
                    toast.fired_at,
                    NotificationAction::Missed,
                );
            }
        }
        if self.toasts.is_empty() {
            return;
        }
        // A frame every sixtieth of a second while anything moves — paced
        // here, since there is no vsync to do it; otherwise sleep until the
        // next countdown runs out, rather than poll.
        if self.toasts.is_animating(now) {
            ctx.request_repaint_after(ANIMATION_FRAME);
        } else if let Some(at) = self.toasts.next_change(now) {
            ctx.request_repaint_after(at.saturating_duration_since(now));
        }
        for toast in self.toasts.visible_mut() {
            toast.note_frame(now);
        }
        // Reduced motion: Windows' "Show animations" off, or forced for
        // checking.
        self.read_windows_animations(now);
        self.toasts.reduced_motion = !self.windows_animations || self.dev.reduce_motion;
        // The frame-time budget, with the fallback forced by counting every
        // frame as 50 ms when asked to.
        self.toasts.frame_floor = if self.dev.slow_frames { 0.05 } else { 0.0 };
        self.toasts.budget_frame(now);

        // `ViewportBuilder` cannot take a window out of Alt+Tab, so the
        // style is set on the real window once it exists (FEAT-0018). Cheap
        // and idempotent: it changes nothing after the first frame.
        ot_platform::toast_window::make_unobtrusive(ot_toast::viewport::WINDOW_TITLE);

        // A monitor can be unplugged while a toast is on screen: a chosen one
        // that has gone falls back to the primary monitor rather than leaving
        // the toast on a screen that is not there (docs/specs/toast.md §7.4).
        let work_area = self.work_area();
        // The tray icon's centre in points, for a toast to shrink into: the
        // primary monitor's scale, since the notification area is on its
        // taskbar. `None` when Windows will not say.
        let tray = self
            .tray
            .as_ref()
            .and_then(ot_platform::tray::Tray::icon_rect)
            .map(|[x, y, w, h]| {
                let scale = self
                    .monitors
                    .iter()
                    .find(|monitor| monitor.primary)
                    .map_or(1.0, |monitor| monitor.scale);
                egui::pos2(
                    ((x + w / 2.0) as f32) / scale,
                    ((y + h / 2.0) as f32) / scale,
                )
            });

        let contents: Vec<ot_toast::Content> = self
            .toasts
            .visible()
            .iter()
            .map(|toast| toast.spec.content.clone())
            .collect();
        for content in &contents {
            let _ = self.pictures_for(content);
        }
        let pictures = self.images.pictures();
        // The toasts whose windows exist and are shaped this frame: their
        // in-animation starts now, and they are shown.
        let mut shaped = Vec::new();
        let shaped_now = &mut shaped;
        // Whether the pointer is over each toast's window this frame. Each
        // toast is its own viewport with its own input, so the pointer is
        // read inside it; a toast is never focused, but Windows still sends
        // the window under the pointer its moves (BUG-0009).
        let mut hovered = Vec::new();
        let hovered_now = &mut hovered;
        // The toasts whose Dismiss was clicked this frame (FEAT-0038).
        let mut pressed = Vec::new();
        let pressed_now = &mut pressed;

        ot_toast::viewport::show_all(
            ctx,
            &self.toasts,
            work_area,
            tray,
            now,
            move |ui, spec, id, look| {
                // The whole toast comes from the spec: the renderer reads no
                // setting and no category, which is what keeps the preview and
                // the real thing from diverging (docs/specs/toast.md §3).
                // Where it sits in its window, and how its effect moves,
                // scales and fades it this frame, is the look's (§5).
                let rect = look.toast;

                // Where Windows will not blur, glass is drawn solid rather than
                // as a tint over nothing (docs/specs/toast.md §4.2). The reason
                // is logged once, by ot-platform.
                let solid;
                let spec = if spec.style.glass.on(spec.style.shape) != ot_toast::Glass::None
                    && !ot_platform::toast_window::glass_available()
                {
                    solid = ot_toast::ToastSpec {
                        style: ot_toast::Style {
                            glass: ot_toast::Glass::None,
                            ..spec.style
                        },
                        ..spec.clone()
                    };
                    &solid
                } else {
                    spec
                };

                // The window is cut to the toast as it is drawn this frame,
                // corners and all, and to the part its effect lets show, so
                // nothing outside them shows (BUG-0007). In this window's own
                // pixels: a toast on a 150% monitor has corners half as large
                // again. Shaped before painting, so a refusal of the glass is
                // known by the next frame's paint.
                let pixels = ui.ctx().pixels_per_point();
                let px = |r: egui::Rect| {
                    [r.left(), r.top(), r.right(), r.bottom()].map(|v| (v * pixels).round() as i32)
                };
                // Grown by a glow's halo while there is one, so the margin it
                // is painted in is not cut away.
                let reach = look.halo_reach();
                let genie = look.genie_rects();
                let outline = if genie.is_some() {
                    // A warp's bands land all along its path: the whole
                    // window, square, cut to the bands below.
                    ot_platform::toast_window::Outline {
                        rect: px(look.clip),
                        radii: [0; 4],
                        clip: px(look.clip),
                    }
                } else {
                    ot_platform::toast_window::Outline {
                        rect: px(look.drawn().expand(reach)),
                        radii: ot_toast::render::corner_radii(spec, rect.height()).map(|radius| {
                            ((radius * look.scale() + reach) * pixels).round() as i32
                        }),
                        clip: px(look.clip),
                    }
                };
                // Opacity fades the whole window, text included (CR-0005), and
                // the effect's own alpha scales it, never past solid.
                let opacity = (spec.alpha() * look.alpha * 255.0)
                    .round()
                    .clamp(0.0, 255.0) as u8;
                // Windows' shadow waits while an effect needs the window
                // see-through: it makes a Rectangle's window opaque.
                let shadow =
                    spec.style.shadow.shows(spec.style.shape, spec.style.glass) && look.shadow_ok;
                let glass = spec.style.glass.on(spec.style.shape) != ot_toast::Glass::None;
                // A dissolve's grains: the window is cut to the cells that
                // show this frame.
                // A Genie's bands, likewise.
                let mask: Option<Vec<[i32; 4]>> = look
                    .dissolve
                    .map(|progress| ot_toast::effects::dissolve_cells(look.toast, progress))
                    .or(genie)
                    .map(|rects| rects.into_iter().map(px).collect());
                if ot_platform::toast_window::set_shape(
                    &id.window_title(),
                    outline,
                    mask.as_deref(),
                    opacity,
                    shadow,
                    glass,
                ) {
                    // Shaped and at its starting alpha: shown now, by the
                    // app rather than by egui (BUG-0012).
                    ot_platform::toast_window::reveal(&id.window_title());
                    shaped_now.push(id);
                }
                // In the toast's own space: a drawer moves it and a pop scales
                // it, so the pointer is brought back through the same
                // transform before it meets the button rects.
                let pointer = ui.ctx().input(|i| i.pointer.hover_pos());
                hovered_now.push((id, pointer.is_some()));
                // Mid-warp the buttons are bands on a path, and take no click.
                let pointer = pointer
                    .filter(|_| look.genie.is_none())
                    .map(|at| look.transform.inverse() * at);

                // The colours come from the theme through here, so the renderer
                // itself still hard-codes none.
                let icon_key = spec
                    .content
                    .icon
                    .as_deref()
                    .unwrap_or(crate::images::APP_ICON);
                let icon = pictures.get(icon_key).copied();
                let image = spec
                    .content
                    .image
                    .as_deref()
                    .and_then(|path| pictures.get(&crate::images::image_key(path)).copied());
                // The toast's own background picks the text colour, not the
                // window's theme, which a toast does not follow.
                let text = Tokens::for_toast(spec);
                // The whole toast, onto one painter: a glow's halo first,
                // under it in the margin, then the frame and the content.
                let draw_toast = |painter: &egui::Painter| {
                    ot_toast::render::halo(painter, rect, spec, look.halo);
                    let content =
                        ot_toast::render::frame_with(painter, rect, spec, look.background);
                    ot_toast::content::draw(
                        painter,
                        content,
                        spec,
                        text.text,
                        text.text_dim,
                        icon,
                        image,
                        pointer,
                    )
                };
                let buttons = match look.genie {
                    // A warp: the toast once per band, each clipped to its
                    // slice and drawn through its own transform, on a layer
                    // of its own (a layer has one transform).
                    Some(bands) => {
                        for (index, band) in bands.iter().enumerate() {
                            let layer = ot_toast::effects::genie_layer(id.0, index);
                            ui.ctx().set_transform_layer(layer, band.transform);
                            let _ = draw_toast(
                                &ui.ctx().layer_painter(layer).with_clip_rect(band.band),
                            );
                        }
                        Vec::new()
                    }
                    None => draw_toast(ui.painter()),
                };
                // A click on a working button acts on exactly what was drawn:
                // the rects come back from the renderer. Snooze is greyed
                // until FEAT-0043 and takes no click (§4.3).
                if ui.ctx().input(|i| i.pointer.primary_clicked())
                    && let Some(at) = pointer
                    && let Some((action, _)) = buttons
                        .iter()
                        .find(|(action, rect)| spec.enabled(*action) && rect.contains(at))
                {
                    pressed_now.push((id, *action));
                }
            },
        );
        for id in shaped {
            self.toasts.appear(id, now);
        }
        for (id, over) in hovered {
            self.toasts.set_hovered(id, over, now);
        }
        for (id, action) in pressed {
            tracing::info!(?id, ?action, "a toast button was pressed");
            self.toasts.dismiss(id, now);
            if action == ot_toast::Action::Open {
                // Settings comes forward; landing on the reminder itself
                // needs the reminder list (FEAT-0046). The missed toast's
                // Open lands on the missed list.
                self.show_window(ctx);
                if self.missed_toast == Some(id) {
                    self.section = Section::Reminders;
                }
            }
            if self.missed_toast == Some(id) {
                self.missed_toast = None;
            }
            let Some(toast) = self.reminder_toasts.remove(&id) else {
                continue;
            };
            match action {
                // The scheduler records the snooze and its history row, and
                // refuses one past the limit (docs/specs/reminders.md §5).
                ot_toast::Action::Snooze => {
                    let settings = self.settings.reminders.clamped();
                    self.send(ot_sched::scheduler::Command::Snooze {
                        reminder_id: toast.reminder_id,
                        fired_at: toast.fired_at,
                        minutes: settings.snooze_minutes,
                        snoozes: toast.snoozes,
                        limit: settings.snooze_limit,
                    });
                }
                ot_toast::Action::Open => {
                    self.record(
                        toast.reminder_id,
                        toast.fired_at,
                        NotificationAction::Opened,
                    );
                    // Lands on the reminder itself, in the editor.
                    self.section = Section::Reminders;
                    self.select_when_listed = Some(toast.reminder_id);
                    if let Some(book) = &self.book {
                        book.refresh();
                    }
                }
                ot_toast::Action::Dismiss => {
                    self.record(
                        toast.reminder_id,
                        toast.fired_at,
                        NotificationAction::Dismissed,
                    );
                }
            }
        }
    }

    /// Whether Windows shows animations, re-read every few seconds like the
    /// theme, and logged when it changes.
    fn read_windows_animations(&mut self, now: Instant) {
        if self.motion_read.is_none_or(|read| {
            now.saturating_duration_since(read) >= ot_platform::motion::POLL_INTERVAL
        }) {
            let shown = ot_platform::motion::show_animations().unwrap_or(true);
            if shown != self.windows_animations || self.motion_read.is_none() {
                tracing::info!(shown, "Windows' Show animations was read");
            }
            self.windows_animations = shown;
            self.motion_read = Some(now);
        }
    }

    /// Settings ▸ Animation: in and out, duration, easing, Reduce Motion,
    /// and Play (FEAT-0036).
    fn animation_pane(&mut self, ui: &mut egui::Ui) {
        let tokens = self.theme.tokens();
        self.read_windows_animations(Instant::now());
        let windows_animations = self.windows_animations;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let columns = layout::columns(ui);
                if crate::animation::controls(
                    ui,
                    columns,
                    &mut self.prefs,
                    windows_animations,
                    tokens,
                ) {
                    self.prefs_dirty = true;
                }
                ui.add_space(layout::ROW_GAP);
                ui.horizontal_wrapped(|ui| {
                    ui.add_space(columns.label);
                    // The play glyph before the label, like every button.
                    let label = format!("\u{25B6}  {}", ot_i18n::text("settings-animation-play"));
                    let button = egui::Button::new(label)
                        .min_size(egui::vec2(0.0, layout::field_height(ui)));
                    if ui.add(button).clicked() {
                        self.show_test_toast();
                    }
                });
            });

        // Written when the pointer is released, as in Appearance: a slider
        // dragged across its range would otherwise rewrite the file a
        // hundred times.
        if self.prefs_dirty && !ui.input(|i| i.pointer.any_down()) {
            self.prefs_dirty = false;
            self.settings.toast = self.prefs.to_table();
            self.save();
        }
    }

    /// Settings ▸ Reminders: for now, the Snooze section.
    fn prayer_pane(&mut self, ui: &mut egui::Ui) {
        let tokens = self.theme.tokens();
        let now = jiff::Timestamp::now();
        let consent = ot_prayer::location::Consent::from(self.settings.location.ip_consent);
        self.prayer.tick(&self.settings.prayer, consent, now);
        // Replies arrive off the UI thread and egui only repaints on an event,
        // so while work is pending the pane asks to be drawn again soon.
        if self.prayer.busy() {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }
        // The library's pictures, as the template editor's picker lists them.
        let pictures: Vec<crate::prayer_pane::PictureChoice> = self
            .library
            .pictures()
            .unwrap_or_default()
            .iter()
            .map(|entry| crate::prayer_pane::PictureChoice {
                sha256: entry.picture.sha256.clone(),
                relative_path: entry.picture.relative_path.clone(),
                label: entry.picture.label.clone(),
            })
            .collect();
        let mut outcome = crate::prayer_pane::Outcome::default();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                outcome = crate::prayer_pane::show(
                    ui,
                    &mut self.prayer,
                    &mut self.settings.prayer,
                    &mut self.settings.location,
                    &pictures,
                    tokens,
                    now,
                );
            });
        if outcome.settings_changed {
            self.save();
        }
        // The editor's Preview: the toast a fire would show, from the same
        // function, to the front.
        if let Some(play) = outcome.play_sound.take()
            && let Some(clip) = crate::prayer::clip_for(
                &play.prayer,
                play.template.sound_mode.as_deref(),
                play.template.sound.as_deref(),
                self.root.path(),
            )
        {
            self.audio.play(clip);
        }
        if outcome.stop_sound {
            self.audio.stop();
        }
        if let Some(preview) = outcome.preview.take() {
            let spec = crate::reminder_toast::prayer_spec(
                &self.prefs,
                &preview.prayer,
                Some(&preview.template),
                Some(formatted_time(now)),
                self.prayer_place.as_deref(),
            );
            let id = self.toasts.push_first(spec, Instant::now());
            // The preview is a fire as far as the sound goes: it plays what a
            // fire would, and stops when the toast is dismissed.
            if let Some(clip) = crate::prayer::clip_for(
                &preview.prayer,
                preview.template.sound_mode.as_deref(),
                preview.template.sound.as_deref(),
                self.root.path(),
            ) {
                self.audio.play(clip);
                self.sounding.insert(id);
            }
        }
        // A changed setting or place is a new day to work out, now.
        if (outcome.settings_changed | self.prayer.take_changed())
            && let Some(service) = &self.prayer_service
        {
            service.changed();
            // A new place is new weather, and the Weather pane names the place.
            if let Some(weather) = &self.weather_service {
                weather.changed();
            }
            self.weather.place_changed();
        }
    }

    fn weather_pane(&mut self, ui: &mut egui::Ui) {
        let tokens = self.theme.tokens();
        self.weather.tick();
        if self.weather.take_place_saved()
            && let Some(service) = &self.weather_service
        {
            service.changed();
        }
        if self.weather.busy() {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        } else if self.weather.has_snapshot() {
            // "5 minutes ago" ages while nothing else happens.
            ui.ctx().request_repaint_after(Duration::from_secs(30));
        }
        let pictures: Vec<crate::prayer_pane::PictureChoice> = self
            .library
            .pictures()
            .unwrap_or_default()
            .iter()
            .map(|entry| crate::prayer_pane::PictureChoice {
                sha256: entry.picture.sha256.clone(),
                relative_path: entry.picture.relative_path.clone(),
                label: entry.picture.label.clone(),
            })
            .collect();
        let mut outcome = crate::weather_pane::Outcome::default();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                outcome = crate::weather_pane::show(
                    ui,
                    &mut self.weather,
                    &mut self.settings.weather,
                    &pictures,
                    tokens,
                    jiff::Timestamp::now(),
                );
            });
        if outcome.settings_changed {
            self.save();
            if let Some(service) = &self.weather_service {
                service.changed();
            }
        }
        if outcome.play_sound {
            self.audio.play(ot_media::Clip::Chime);
        }
        if outcome.stop_sound {
            self.audio.stop();
        }
        // The editor's Preview: the toast a change to this condition would show.
        if let Some((condition, template)) = outcome.preview.take() {
            let from = crate::weather_pane::sample_before(condition);
            let spec = crate::reminder_toast::weather_spec(
                &self.prefs,
                &ot_weather::detect::Change::Condition {
                    from,
                    to: condition,
                },
                Some(&template),
                Some(formatted_time(jiff::Timestamp::now())),
                self.weather.place_name(),
            );
            let id = self.toasts.push_first(spec, Instant::now());
            if template.sound_mode.as_deref() == Some("chime") {
                self.audio.play(ot_media::Clip::Chime);
                self.sounding.insert(id);
            }
        }
    }

    fn updates_pane(&mut self, ui: &mut egui::Ui) {
        let tokens = self.theme.tokens();
        if self.update.busy() {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }
        let mut asked = crate::update_pane::Asked::default();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                asked = crate::update_pane::show(
                    ui,
                    &mut self.update,
                    &mut self.settings.updates,
                    tokens,
                    jiff::Timestamp::now(),
                );
            });
        if asked.settings_changed {
            self.save();
        }
        if let Some(service) = &self.update_service {
            if asked.check {
                self.update.requested(true);
                service.check();
            }
            if let Some(release) = asked.install.take() {
                service.install(release);
            }
            if asked.cancel {
                service.cancel();
            }
        }
        if let Some(version) = asked.restart.take() {
            self.restart_to_update(ui.ctx(), &version);
        }
    }

    /// Start the update helper — this program, told to swap the staged one in once
    /// this one has quit — and quit. If the helper cannot be started nothing has
    /// changed, and the pane says so.
    fn restart_to_update(&mut self, ctx: &egui::Context, version: &semver::Version) {
        let relaunch: Vec<String> = std::env::args().skip(1).collect();
        let args = ot_update::swap::helper_args(self.root.path(), version, &relaunch);
        let started =
            std::env::current_exe().and_then(|exe| ot_platform::process::spawn_hidden(&exe, &args));
        match started {
            Ok(_) => {
                tracing::info!(%version, "restarting to finish an update");
                self.quit(ctx);
            }
            Err(error) => {
                tracing::warn!(%error, "the update helper could not be started");
                self.update.apply(crate::update_service::Note::Failed(
                    crate::update_service::Problem::Other,
                ));
            }
        }
    }

    /// Take what the update service has to say.
    fn drain_update_notes(&mut self) {
        let Some(service) = &self.update_service else {
            return;
        };
        while let Some(note) = service.poll() {
            self.update.apply(note);
        }
    }

    fn reminders_pane(&mut self, ui: &mut egui::Ui) {
        let tokens = self.theme.tokens();
        let rows: Vec<crate::reminders::MissedRow> =
            self.missed.iter().map(|entry| entry.row.clone()).collect();
        let mut pressed = None;
        let mut listed = None;
        let mut edited = None;
        let mut cleared = false;
        // One form, always: a blank new reminder until one is chosen, so New
        // Reminder and Cancel empty the same form rather than hiding it.
        let mut editor = self.editor.take().unwrap_or_else(|| {
            crate::reminder_editor::Editor::new_at(
                jiff::Timestamp::now(),
                &jiff::tz::TimeZone::system(),
            )
        });
        let problems = editor.validate(jiff::Timestamp::now(), &jiff::tz::TimeZone::system());
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                pressed = crate::reminders::missed_list(ui, &rows, tokens);
                ui.add_space(layout::ROW_GAP);
                ui.separator();
                ui.add_space(layout::ROW_GAP);
                let master = crate::reminders::list_and_details(
                    ui,
                    self.reminder_rows.as_deref(),
                    &mut editor,
                    &problems,
                    &self.reminder_history,
                    &mut self.confirming_clear,
                    tokens,
                );
                (listed, edited, cleared) = (master.listed, master.edited, master.cleared);
                ui.add_space(layout::ROW_GAP);
                ui.separator();
                ui.add_space(layout::ROW_GAP);
                let columns = layout::columns(ui);
                if crate::reminders::fatigue_controls(ui, columns, &mut self.fatigue_rows, tokens) {
                    self.fatigue_dirty = true;
                }
                ui.add_space(layout::ROW_GAP);
                ui.separator();
                ui.add_space(layout::ROW_GAP);
                let window = self.settings.reminders.missed_window_hours;
                if crate::reminders::snooze_controls(
                    ui,
                    columns,
                    &mut self.settings.reminders,
                    tokens,
                ) {
                    self.reminders_dirty = true;
                }
                if self.settings.reminders.missed_window_hours != window {
                    self.send(ot_sched::scheduler::Command::SetMissedWindow {
                        hours: self.settings.reminders.clamped().missed_window_hours,
                    });
                }
            });
        self.editor = Some(editor);
        match listed {
            Some(crate::reminders::ListAction::New) => {
                self.editor = Some(crate::reminder_editor::Editor::new_at(
                    jiff::Timestamp::now(),
                    &jiff::tz::TimeZone::system(),
                ));
                self.reminder_history.clear();
                self.confirming_clear = false;
            }
            Some(crate::reminders::ListAction::Select(id)) => self.select_reminder(id),
            None => {}
        }
        if cleared
            && let (Some(book), Some(id)) = (&self.book, self.editor.as_ref().and_then(|e| e.id))
        {
            book.clear_history(id);
        }
        match edited {
            Some(crate::reminder_editor::EditorAction::Save) if problems.is_empty() => {
                if let (Some(book), Some(editor)) = (&self.book, &self.editor) {
                    let category = book
                        .listing()
                        .and_then(|listing| listing.category_of(editor.type_key));
                    let lang = ot_i18n::current_language()
                        .unwrap_or(Language::English)
                        .tag();
                    book.save(editor.id, editor.draft(category, lang));
                }
            }
            Some(crate::reminder_editor::EditorAction::Cancel) => {
                self.editor = None;
                self.reminder_history.clear();
            }
            Some(crate::reminder_editor::EditorAction::Delete) => {
                if let (Some(book), Some(id)) =
                    (&self.book, self.editor.as_ref().and_then(|e| e.id))
                {
                    book.delete(id);
                }
                self.editor = None;
                self.reminder_history.clear();
            }
            _ => {}
        }
        if let Some((index, action)) = pressed
            && index < self.missed.len()
        {
            let entry = self.missed.remove(index);
            match action {
                crate::reminders::MissedAction::Snooze => {
                    let settings = self.settings.reminders.clamped();
                    self.send(ot_sched::scheduler::Command::Snooze {
                        reminder_id: entry.reminder_id,
                        fired_at: entry.fired_at,
                        minutes: settings.snooze_minutes,
                        snoozes: entry.snoozes,
                        limit: settings.snooze_limit,
                    });
                }
                crate::reminders::MissedAction::Dismiss => {
                    self.record(
                        entry.reminder_id,
                        entry.fired_at,
                        NotificationAction::Dismissed,
                    );
                }
            }
        }
        // Fatigue changes go to the reminders thread when the pointer is
        // released, like the settings below: a drag is one change.
        if self.fatigue_dirty && !ui.input(|i| i.pointer.any_down()) {
            self.fatigue_dirty = false;
            if let Some(book) = &self.book {
                let lang = ot_i18n::current_language()
                    .unwrap_or(Language::English)
                    .tag();
                for row in &self.fatigue_rows {
                    book.set_fatigue(row.type_key, row.on, row.minutes, lang);
                }
            }
        }
        // Written when the pointer is released: a slider dragged across its
        // range would otherwise rewrite the file at every step.
        if self.reminders_dirty && !ui.input(|i| i.pointer.any_down()) {
            self.reminders_dirty = false;
            self.save();
        }
    }

    /// Say every cached piece of text again, after the language or the clock
    /// changed: the list's rows, the open history, the missed list, and the
    /// editor's repeat summary. They are built once, not per frame, so a
    /// change has to rebuild them (FEAT-0047).
    fn reformat(&mut self) {
        self.prayer.reformat();
        if let Some(book) = &self.book
            && let Some(listing) = book.listing()
        {
            self.reminder_rows = Some(reminder_rows(listing));
            if let Some(id) = self.editor.as_ref().and_then(|editor| editor.id) {
                book.history(id);
            }
        }
        let limit = self.settings.reminders.clamped().snooze_limit;
        for entry in &mut self.missed {
            entry.row = missed_row(
                &entry.reminder,
                entry.template.as_ref(),
                entry.fired_at,
                entry.snoozes,
                limit,
            );
        }
        if let Some(editor) = &mut self.editor {
            editor.summary = None;
        }
    }

    /// Open a reminder in the editor and ask for its history.
    fn select_reminder(&mut self, id: i64) {
        let Some(book) = &self.book else { return };
        let Some(listing) = book.listing() else {
            self.select_when_listed = Some(id);
            return;
        };
        let Some(reminder) = listing.reminders.iter().find(|r| r.id == id) else {
            return;
        };
        self.editor = Some(crate::reminder_editor::Editor::from_reminder(
            reminder,
            listing.type_of(reminder.draft.category_id),
        ));
        self.reminder_history.clear();
        self.confirming_clear = false;
        book.history(id);
    }

    /// Take what the reminders thread has finished: a fresh list becomes the
    /// rows once, here, not every frame.
    fn drain_book(&mut self) {
        let Some(book) = &mut self.book else { return };
        if self.refresh_at.is_some_and(|at| Instant::now() >= at) {
            self.refresh_at = None;
            book.refresh();
        }
        let (fresh, replies) = book.drain();
        if fresh && let Some(listing) = book.listing() {
            self.reminder_rows = Some(reminder_rows(listing));
            if !self.fatigue_dirty {
                self.fatigue_rows = fatigue_rows(Some(listing));
            }
            // A fresh list follows a fire, which wrote a history row: the
            // open reminder's history is read again, not left as it was.
            if let Some(id) = self.editor.as_ref().and_then(|editor| editor.id) {
                book.history(id);
            }
        }
        for reply in replies {
            match reply {
                crate::reminder_book::Reply::Saved(id) => {
                    self.select_when_listed = Some(id);
                    self.refresh_at = Some(Instant::now() + Duration::from_millis(500));
                    self.status = Status::Saved;
                }
                crate::reminder_book::Reply::Deleted(_) => self.status = Status::Saved,
                crate::reminder_book::Reply::History { id, rows } => {
                    if self.editor.as_ref().and_then(|e| e.id) == Some(id) {
                        self.reminder_history = rows
                            .iter()
                            .map(|(at, action)| {
                                ot_i18n::text_with(
                                    "settings-reminder-history-row",
                                    &[
                                        ("time", formatted_datetime(*at).into()),
                                        ("action", ot_i18n::text(history_key(*action)).into()),
                                    ],
                                )
                            })
                            .collect();
                    }
                }
                crate::reminder_book::Reply::Failed(reason) => {
                    self.status = Status::Failed {
                        key: "settings-status-reminders-failed",
                        reason,
                    };
                }
            }
        }
        if let Some(id) = self.select_when_listed
            && self
                .book
                .as_ref()
                .and_then(|book| book.listing())
                .is_some_and(|listing| listing.reminders.iter().any(|r| r.id == id))
        {
            self.select_when_listed = None;
            self.select_reminder(id);
        }
    }

    /// Bring the settings window back, in front.
    fn show_window(&mut self, ctx: &egui::Context) {
        match (self.window, self.parked.take()) {
            (Some(window), Some(parked)) => {
                ot_platform::window_park::unpark(window, parked);
            }
            _ => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        }
    }

    /// Put the settings window away, leaving the app running behind its tray
    /// icon.
    ///
    /// **Parked, not hidden** (BUG-0008): Windows paints no hidden window,
    /// and without frames nothing drains the tray's clicks or drives a toast.
    /// Hiding is only the fallback where parking is not possible.
    fn hide_window(&mut self, ctx: &egui::Context) {
        // Closing Settings stops the prayer work in flight, so a fetch does not
        // run on for a window nobody is looking at (FEAT-0052, FEAT-0057).
        self.prayer.cancel();
        if self.parked.is_some() {
            return;
        }
        self.parked = self.window.and_then(ot_platform::window_park::park);
        if self.parked.is_none() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
    }

    fn quit(&mut self, ctx: &egui::Context) {
        self.quitting = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    /// The confirmation shown when Quit is asked for while paused.
    fn quit_confirmation(&mut self, ctx: &egui::Context) {
        let tokens = self.theme.tokens();
        let mut answered = None;
        egui::Modal::new(egui::Id::new("confirm-quit")).show(ctx, |ui| {
            ui.set_max_width(layout::FIELD_MAX_WIDTH);
            ui.heading(ot_i18n::text("tray-quit-confirm-title"));
            ui.add_space(layout::ROW_GAP);
            ui.label(
                egui::RichText::new(ot_i18n::text("tray-quit-confirm-body")).color(tokens.text),
            );
            ui.add_space(layout::PANE_GAP);
            ui.horizontal_wrapped(|ui| {
                if ui.button(ot_i18n::text("tray-quit-confirm-quit")).clicked() {
                    answered = Some(true);
                }
                ui.add_space(layout::OPTION_GAP);
                if ui.button(ot_i18n::text("tray-quit-confirm-stay")).clicked() {
                    answered = Some(false);
                }
            });
        });

        match answered {
            Some(true) => {
                self.confirming_quit = false;
                self.quit(ctx);
            }
            Some(false) => self.confirming_quit = false,
            None => {}
        }
    }

    /// Settings ▸ Appearance: the style, the placement and the preview.
    fn appearance_pane(&mut self, ui: &mut egui::Ui) {
        let tokens = self.theme.tokens();
        let work_area = self.work_area();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let columns = layout::columns(ui);

                // The preview first, so what each control does is visible
                // while it is being used.
                ui.label(egui::RichText::new(ot_i18n::text("settings-toast-preview")).strong());
                ui.add_space(layout::ROW_GAP);
                let spec = self.test_spec();
                let (icon, image) = self.pictures_for(&spec.content);
                crate::appearance::preview(ui, columns, &spec, icon, image);
                ui.add_space(layout::ROW_GAP);
                ui.horizontal_wrapped(|ui| {
                    ui.add_space(columns.label);
                    // Field height like every other control, and a glyph
                    // before the label like every other button. The bell is
                    // the tray's own glyph for this same verb.
                    let label = format!("\u{1F514}  {}", ot_i18n::text("settings-toast-test"));
                    let button = egui::Button::new(label)
                        .min_size(egui::vec2(0.0, layout::field_height(ui)));
                    if ui.add(button).clicked() {
                        self.show_test_toast();
                    }
                });
                ui.add_space(layout::PANE_GAP);
                self.test_content(ui, columns, tokens);
                ui.add_space(layout::PANE_GAP);
                ui.separator();
                ui.add_space(layout::PANE_GAP);

                let mut changed =
                    crate::appearance::style_controls(ui, columns, &mut self.prefs, tokens);
                ui.add_space(layout::PANE_GAP);
                ui.separator();
                ui.add_space(layout::PANE_GAP);
                changed |= crate::appearance::position_controls(
                    ui,
                    columns,
                    &mut self.prefs,
                    &self.monitors,
                    work_area,
                    tokens,
                );
                if changed {
                    self.prefs_dirty = true;
                }
            });

        // Written when the pointer is released, not on every frame of a drag:
        // a slider moved across its range would otherwise rewrite the file a
        // hundred times.
        if self.prefs_dirty && !ui.input(|i| i.pointer.any_down()) {
            self.prefs_dirty = false;
            self.settings.toast = self.prefs.to_table();
            self.save();
        }
    }

    /// The test toast's own content: an icon, an image, and whether the image
    /// is framed.
    ///
    /// **Not global settings.** Every notification carries its own icon and
    /// image (`docs/specs/toast.md` §8); these change only the preview and the
    /// test toast, so the style around them can be judged with real pictures
    /// in place. They are held in memory and never written to the settings
    /// file. A notification's own pictures are chosen where the notification
    /// is defined.
    fn test_content(&mut self, ui: &mut egui::Ui, columns: layout::Columns, tokens: Tokens) {
        ui.label(egui::RichText::new(ot_i18n::text("settings-toast-test-content")).strong());
        layout::field_hint(
            ui,
            columns,
            &ot_i18n::text("settings-toast-test-content-hint"),
            tokens.text_dim,
        );
        ui.add_space(layout::ROW_GAP);
        self.picture_row(ui, columns, crate::images::Slot::Icon);
        self.picture_row(ui, columns, crate::images::Slot::Image);

        if self.test_image.is_some() {
            layout::labelled_row(
                ui,
                columns,
                &ot_i18n::text("settings-toast-image-border"),
                |ui| {
                    ui.checkbox(&mut self.test_image_border, "");
                },
            );
            ui.add_space(layout::ROW_GAP);
        }

        // The notification's own accent, overriding the one under Accent
        // Colour below. Off means "use that one", which is what a
        // notification without an accent of its own gets.
        let mut own = self.test_accent.is_some();
        layout::labelled_row(
            ui,
            columns,
            &ot_i18n::text("settings-toast-own-accent"),
            |ui| {
                if ui.checkbox(&mut own, "").changed() {
                    // A green that is plainly not the default blue, so turning
                    // it on visibly does something.
                    self.test_accent = own.then(|| ot_toast::Rgba::new(0x1f, 0x9d, 0x55, 0xff));
                }
                if let Some(accent) = self.test_accent.as_mut() {
                    ui.add_space(layout::OPTION_GAP);
                    let (r, g, b, a) = accent.parts();
                    let mut colour = egui::Color32::from_rgba_unmultiplied(r, g, b, a);
                    if egui::color_picker::color_edit_button_srgba(
                        ui,
                        &mut colour,
                        egui::color_picker::Alpha::Opaque,
                    )
                    .changed()
                    {
                        let [r, g, b, a] = colour.to_srgba_unmultiplied();
                        *accent = ot_toast::Rgba::new(r, g, b, a);
                    }
                    let (r, g, b, _) = accent.parts();
                    ui.label(format!("#{r:02x}{g:02x}{b:02x}"));
                }
            },
        );
        ui.add_space(layout::ROW_GAP);
    }

    /// One picture row: the library's pictures to choose from, with "Upload
    /// a New Picture" at the end of the list (`docs/specs/toast.md` §8.1).
    ///
    /// Uploading opens the Windows dialog **on a worker thread**, which
    /// imports the file there too; the library records it on its own thread,
    /// and the picture lands in this row when both have answered.
    fn picture_row(
        &mut self,
        ui: &mut egui::Ui,
        columns: layout::Columns,
        slot: crate::images::Slot,
    ) {
        use crate::images::Slot;
        let (label_key, none_key, title_key) = match slot {
            Slot::Icon => (
                "settings-toast-icon",
                "settings-library-none-icon",
                "settings-toast-icon-dialog-title",
            ),
            Slot::Image => (
                "settings-toast-image",
                "settings-library-none-image",
                "settings-toast-image-dialog-title",
            ),
        };
        // Asking for a thumbnail already decoded, or on its way, is a set
        // lookup, so this costs nothing once they are in.
        for entry in self.library.pictures().unwrap_or_default() {
            self.images.request(&entry.picture.relative_path);
        }
        // The built-in pictures are listed too, so they are requested too.
        for builtin in ot_store::builtin::all() {
            self.images.request(builtin.key);
        }
        let current = match slot {
            Slot::Icon => self.test_icon.as_deref(),
            Slot::Image => self.test_image.as_deref(),
        };
        let shown = current
            .and_then(|path| picture_label(&self.library, path))
            .unwrap_or_else(|| ot_i18n::text(none_key));
        let height = layout::field_height(ui);
        let busy = self.importer.is_busy();
        let library = &self.library;
        let images = &self.images;
        let mut choice: Option<Option<String>> = None;
        let mut upload = false;
        layout::labelled_row(ui, columns, &ot_i18n::text(label_key), |ui| {
            egui::ComboBox::from_id_salt(label_key)
                .width(columns.field)
                .height(height * 12.0)
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(current.is_none(), ot_i18n::text(none_key))
                        .clicked()
                    {
                        choice = Some(None);
                    }
                    for entry in library.pictures().unwrap_or_default() {
                        let path = entry.picture.relative_path.as_str();
                        let selected = current == Some(path);
                        if picture_choice(
                            ui,
                            images.picture(path),
                            &entry.picture.label,
                            selected,
                            height,
                        )
                        .clicked()
                        {
                            choice = Some(Some(path.to_owned()));
                        }
                    }
                    // What ships with the app, under a heading per kind
                    // (`docs/specs/toast.md` §8.2).
                    for group in ot_store::builtin::Group::all() {
                        ui.separator();
                        ui.label(
                            egui::RichText::new(ot_i18n::text(group.name_key()))
                                .small()
                                .weak(),
                        );
                        for builtin in ot_store::builtin::in_group(group) {
                            let label = ot_i18n::text(builtin.label_key);
                            let selected = current == Some(builtin.key);
                            if picture_choice(
                                ui,
                                images.picture(builtin.key),
                                &label,
                                selected,
                                height,
                            )
                            .clicked()
                            {
                                choice = Some(Some(builtin.key.to_owned()));
                            }
                        }
                    }
                    ui.separator();
                    // The folder for the verb that opens a file, as everywhere
                    // else.
                    let label = format!(
                        "\u{1F4C2}  {}",
                        ot_i18n::text("settings-library-upload-new")
                    );
                    if ui
                        .add_enabled(!busy, egui::Button::selectable(false, label))
                        .clicked()
                    {
                        upload = true;
                    }
                });
        });
        if let Some(choice) = choice {
            match slot {
                Slot::Icon => self.test_icon = choice,
                Slot::Image => self.test_image = choice,
            }
        }
        if upload {
            self.importer.start(
                Some(slot),
                ot_i18n::text(title_key),
                ot_i18n::text("settings-toast-image-filter"),
            );
        }
        ui.add_space(layout::ROW_GAP);
    }

    /// Settings ▸ Picture Library: upload a picture once, name it, use it in
    /// any notification, remove it (`docs/specs/toast.md` §8.1).
    fn pictures_pane(&mut self, ui: &mut egui::Ui) {
        let tokens = self.theme.tokens();
        for entry in self.library.pictures().unwrap_or_default() {
            self.images.request(&entry.picture.relative_path);
        }
        for builtin in ot_store::builtin::all() {
            self.images.request(builtin.key);
        }
        let height = layout::field_height(ui);
        let busy = self.importer.is_busy();
        let mut upload = false;
        let mut rename: Option<(String, String)> = None;
        let mut remove: Option<String> = None;
        let mut confirmed: Option<bool> = None;
        // Named in the question, so the user knows what they would lose.
        let used_here = self
            .confirm_remove
            .as_ref()
            .is_some_and(|(sha256, _)| self.used_by_test_toast(sha256));

        // The bar takes its own room instead of floating over the rows: this
        // pane always scrolls now that the built-in pictures sit below the
        // user's, and a floating bar lay over every Remove button.
        ui.spacing_mut().scroll.floating = false;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(ot_i18n::text("settings-library-hint"))
                        .color(tokens.text_dim),
                );
                ui.add_space(layout::ROW_GAP);
                let label = format!("\u{1F4C2}  {}", ot_i18n::text("settings-library-upload"));
                let button = egui::Button::new(label).min_size(egui::vec2(0.0, height));
                if ui.add_enabled(!busy, button).clicked() {
                    upload = true;
                }
                ui.add_space(layout::PANE_GAP);
                ui.separator();
                ui.add_space(layout::PANE_GAP);

                let images = &self.images;
                let confirm_remove = self.confirm_remove.as_ref();
                match self.library.pictures_mut() {
                    None => {
                        ui.label(
                            egui::RichText::new(ot_i18n::text("settings-library-loading"))
                                .color(tokens.text_dim),
                        );
                    }
                    Some([]) => {
                        ui.label(
                            egui::RichText::new(ot_i18n::text("settings-library-empty"))
                                .color(tokens.text_dim),
                        );
                    }
                    Some(entries) => {
                        let thumb = height * 1.6;
                        for entry in entries {
                            ui.spacing_mut().interact_size.y = height;
                            ui.horizontal(|ui| {
                                ui.set_min_height(thumb);
                                let (slot, _) = ui.allocate_exact_size(
                                    egui::vec2(thumb, thumb),
                                    egui::Sense::hover(),
                                );
                                if let Some(picture) = images.picture(&entry.picture.relative_path)
                                {
                                    let fitted = egui::Rect::from_center_size(
                                        slot.center(),
                                        picture.size * (thumb / picture.size.max_elem()).min(1.0),
                                    );
                                    egui::Image::new(egui::load::SizedTexture::new(
                                        picture.texture,
                                        picture.size,
                                    ))
                                    .paint_at(ui, fitted);
                                }
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        // The trash can for Remove, the same
                                        // glyph as every other Remove.
                                        let label = format!(
                                            "\u{1F5D1}  {}",
                                            ot_i18n::text("settings-library-remove")
                                        );
                                        let button = egui::Button::new(label)
                                            .min_size(egui::vec2(0.0, height));
                                        if ui.add(button).clicked() {
                                            remove = Some(entry.picture.sha256.clone());
                                        }
                                        let field = egui::TextEdit::singleline(&mut entry.edit)
                                            .desired_width(ui.available_width())
                                            .min_size(egui::vec2(0.0, height))
                                            .vertical_align(egui::Align::Center);
                                        let response = ui.add(field).on_hover_text(ot_i18n::text(
                                            "settings-library-label-hint",
                                        ));
                                        if response.lost_focus()
                                            && entry.edit != entry.picture.label
                                        {
                                            rename = Some((
                                                entry.picture.sha256.clone(),
                                                entry.edit.clone(),
                                            ));
                                        }
                                    },
                                );
                            });
                            if let Some((sha256, templates)) = confirm_remove
                                && *sha256 == entry.picture.sha256
                            {
                                ui.add_space(layout::ROW_GAP);
                                egui::Frame::new()
                                    .fill(tokens.surface_raised)
                                    .inner_margin(layout::ROW_GAP)
                                    .show(ui, |ui| {
                                        // As wide as the row it belongs to.
                                        ui.set_min_width(ui.available_width());
                                        ui.label(ot_i18n::text_with(
                                            "settings-library-in-use",
                                            &[("label", entry.picture.label.clone().into())],
                                        ));
                                        if used_here {
                                            ui.label(ot_i18n::text(
                                                "settings-library-used-by-test-toast",
                                            ));
                                        }
                                        if *templates {
                                            ui.label(ot_i18n::text(
                                                "settings-library-used-by-templates",
                                            ));
                                        }
                                        ui.add_space(layout::ROW_GAP);
                                        ui.horizontal_wrapped(|ui| {
                                            let anyway = egui::Button::new(
                                                egui::RichText::new(ot_i18n::text(
                                                    "settings-library-remove-anyway",
                                                ))
                                                .color(tokens.danger),
                                            )
                                            .min_size(egui::vec2(0.0, height));
                                            if ui.add(anyway).clicked() {
                                                confirmed = Some(true);
                                            }
                                            let keep = egui::Button::new(ot_i18n::text(
                                                "settings-library-keep",
                                            ))
                                            .min_size(egui::vec2(0.0, height));
                                            if ui.add(keep).clicked() {
                                                confirmed = Some(false);
                                            }
                                        });
                                    });
                            }
                            ui.add_space(layout::ROW_GAP);
                        }
                    }
                }

                // What ships with the app: not the user's to rename or
                // remove, so listed apart and read-only.
                ui.add_space(layout::PANE_GAP);
                ui.separator();
                ui.add_space(layout::PANE_GAP);
                ui.label(egui::RichText::new(ot_i18n::text("settings-library-builtin")).strong());
                ui.label(
                    egui::RichText::new(ot_i18n::text("settings-library-builtin-hint"))
                        .color(tokens.text_dim),
                );
                ui.add_space(layout::ROW_GAP);
                for group in ot_store::builtin::Group::all() {
                    ui.label(
                        egui::RichText::new(ot_i18n::text(group.name_key())).color(tokens.text_dim),
                    );
                    ui.add_space(layout::ROW_GAP / 2.0);
                    egui::Grid::new(("builtin-pictures", group as u8))
                        .num_columns(2)
                        .spacing([layout::OPTION_GAP, layout::ROW_GAP])
                        .show(ui, |ui| {
                            for builtin in ot_store::builtin::in_group(group) {
                                paint_thumbnail(ui, images.picture(builtin.key), height * 1.6);
                                ui.label(ot_i18n::text(builtin.label_key));
                                ui.end_row();
                            }
                        });
                    ui.add_space(layout::PANE_GAP);
                }
            });

        if upload {
            self.importer.start(
                None,
                ot_i18n::text("settings-library-dialog-title"),
                ot_i18n::text("settings-toast-image-filter"),
            );
        }
        if let Some((sha256, label)) = rename {
            self.library.rename(sha256, label);
        }
        if let Some(sha256) = remove {
            let used_here = self.used_by_test_toast(&sha256);
            self.library.remove(sha256, false, used_here);
        }
        match confirmed {
            Some(true) => {
                if let Some((sha256, _)) = self.confirm_remove.take() {
                    let used_here = self.used_by_test_toast(&sha256);
                    self.library.remove(sha256, true, used_here);
                }
            }
            Some(false) => self.confirm_remove = None,
            None => {}
        }
    }

    /// Whether the test toast shows this picture, as its icon or its image.
    /// The database cannot know: the test toast is never saved.
    fn used_by_test_toast(&self, sha256: &str) -> bool {
        let Some(path) = self
            .library
            .pictures()
            .unwrap_or_default()
            .iter()
            .find(|entry| entry.picture.sha256 == sha256)
            .map(|entry| entry.picture.relative_path.as_str())
        else {
            return false;
        };
        self.test_icon.as_deref() == Some(path) || self.test_image.as_deref() == Some(path)
    }

    /// Take the scheduler's fires, at most a channel's worth a frame, so a
    /// burst cannot stall one frame, and turn each into a toast — or, while
    /// notifications are paused, into a `missed` row and no toast
    /// (docs/specs/tray-and-settings.md §4). Every fire is written to the
    /// history either way.
    fn drain_fires(&mut self) {
        let Some(fires) = &self.fires else { return };
        let arrived: Vec<_> = fires
            .try_iter()
            .take(ot_sched::scheduler::FIRE_CAPACITY)
            .collect();
        let now = jiff::Timestamp::now();
        let mut any_missed = false;
        // A fire moves a next time or finishes a reminder: the list is read
        // again rather than patched.
        if !arrived.is_empty()
            && let Some(book) = &self.book
        {
            book.refresh();
        }
        for fired in arrived {
            let fired_at = fired.fire.at;
            // Missed while the PC was off or asleep: listed, not shown, so a
            // morning does not open with a dozen toasts at once
            // (docs/specs/reminders.md §6).
            if fired.missed {
                let limit = self.settings.reminders.clamped().snooze_limit;
                let row = missed_row(
                    &fired.reminder,
                    fired.template.as_ref(),
                    fired_at,
                    fired.snoozes,
                    limit,
                );
                self.missed.push(MissedEntry {
                    reminder_id: fired.reminder_id,
                    fired_at,
                    snoozes: fired.snoozes,
                    reminder: fired.reminder.clone(),
                    template: fired.template.clone(),
                    row,
                });
                tracing::info!(reminder = fired.reminder_id, "a missed reminder was listed");
                self.record(fired.reminder_id, fired_at, NotificationAction::Missed);
                any_missed = true;
                continue;
            }
            let prayer = match &fired.reminder.draft.trigger {
                ot_store::reminders::Trigger::Prayer { prayer } => Some(prayer.clone()),
                _ => None,
            };
            if pause_suppresses(self.pause.is_active(now), prayer.is_some()) {
                tracing::info!(
                    reminder = fired.reminder_id,
                    "a reminder fired during a pause"
                );
                self.record(fired.reminder_id, fired_at, NotificationAction::Missed);
                continue;
            }
            // The reminder ahead of a prayer: a plain toast, held back by a pause.
            if let ot_store::reminders::Trigger::PrayerBefore { prayer, minutes } =
                &fired.reminder.draft.trigger
            {
                let spec = crate::reminder_toast::prayer_before_spec(
                    &self.prefs,
                    prayer,
                    *minutes,
                    fired.template.as_ref(),
                    Some(formatted_time(now)),
                    self.prayer_place.as_deref(),
                );
                let persistent = spec.behaviour.persistent;
                let id = self.toasts.push(spec, Instant::now());
                tracing::info!(prayer, minutes, toast = ?id, "a pre-prayer reminder became a toast");
                self.reminder_toasts.insert(
                    id,
                    ReminderToast {
                        reminder_id: fired.reminder_id,
                        fired_at,
                        snoozes: 0,
                        persistent,
                    },
                );
                self.record(fired.reminder_id, fired_at, NotificationAction::Shown);
                continue;
            }
            // A prayer is its own toast, ahead of everything, and a pause does
            // not hold it back (`docs/specs/tray-and-settings.md` §4).
            if let Some(prayer) = prayer {
                let spec = crate::reminder_toast::prayer_spec(
                    &self.prefs,
                    &prayer,
                    fired.template.as_ref(),
                    Some(formatted_time(now)),
                    self.prayer_place.as_deref(),
                );
                let persistent = spec.behaviour.persistent;
                let id = self.toasts.push_first(spec, Instant::now());
                tracing::info!(prayer, toast = ?id, "a prayer became a toast");
                if let Some(clip) = crate::prayer::clip_for(
                    &prayer,
                    fired
                        .template
                        .as_ref()
                        .and_then(|t| t.sound_mode.as_deref()),
                    fired.template.as_ref().and_then(|t| t.sound.as_deref()),
                    self.root.path(),
                ) {
                    self.audio.play(clip);
                    self.sounding.insert(id);
                }
                self.reminder_toasts.insert(
                    id,
                    ReminderToast {
                        reminder_id: fired.reminder_id,
                        fired_at,
                        snoozes: 0,
                        persistent,
                    },
                );
                self.record(fired.reminder_id, fired_at, NotificationAction::Shown);
                continue;
            }
            let template = fired.template.as_ref();
            // An offset fire is about a time still ahead: its header says
            // when that is, and its row says how far ahead (FEAT-0047).
            let (timestamp, before) = match fired.fire.offset {
                Some(offset) => (
                    formatted_datetime(fired.fire.occurrence),
                    Some(crate::phrases::offset(offset)),
                ),
                None => (formatted_time(now), None),
            };
            let mut spec = crate::reminder_toast::spec(
                &self.prefs,
                &fired.reminder,
                template,
                Some(timestamp),
                before,
                fired.snoozes,
                self.settings.reminders.clamped().snooze_limit,
            );
            if spec.behaviour.persistent
                && let Some(cap) = self.dev.alarm_cap_secs
            {
                spec.behaviour.duration_secs = cap;
            }
            let persistent = spec.behaviour.persistent;
            let id = if crate::reminder_toast::is_urgent(template) {
                self.toasts.push_first(spec, Instant::now())
            } else {
                self.toasts.push(spec, Instant::now())
            };
            tracing::info!(reminder = fired.reminder_id, toast = ?id, "a reminder fire became a toast");
            self.reminder_toasts.insert(
                id,
                ReminderToast {
                    reminder_id: fired.reminder_id,
                    fired_at,
                    snoozes: fired.snoozes,
                    persistent,
                },
            );
            self.record(fired.reminder_id, fired_at, NotificationAction::Shown);
        }
        if any_missed {
            self.announce_missed();
        }
    }

    /// Take what the weather service has to say: each change of weather, as a
    /// toast, unless the notifications are paused.
    fn drain_weather_notes(&mut self) {
        let Some(service) = &self.weather_service else {
            return;
        };
        for note in service.drain() {
            let (change, template, place) = match note {
                crate::weather_service::Note::Snapshot(snapshot) => {
                    self.weather.set_snapshot(snapshot);
                    continue;
                }
                crate::weather_service::Note::Change(change, template, place) => {
                    (change, template.map(|boxed| *boxed), place)
                }
            };
            if self.pause.is_active(jiff::Timestamp::now()) {
                tracing::info!("a weather change came during a pause and was not shown");
                continue;
            }
            let spec = crate::reminder_toast::weather_spec(
                &self.prefs,
                &change,
                template.as_ref(),
                Some(formatted_time(jiff::Timestamp::now())),
                place.as_deref(),
            );
            let id = self.toasts.push(spec, Instant::now());
            if template.as_ref().and_then(|t| t.sound_mode.as_deref()) == Some("chime") {
                self.audio.play(ot_media::Clip::Chime);
                self.sounding.insert(id);
            }
            tracing::info!(?change, toast = ?id, "a weather change became a toast");
        }
    }

    /// Take what the prayer service has to say: that no times could be worked
    /// out, once a day, as a toast.
    fn drain_prayer_notes(&mut self) {
        let Some(service) = &self.prayer_service else {
            return;
        };
        for note in service.drain() {
            match note {
                crate::prayer_service::Note::Source(source) => self.prayer_source = Some(source),
                crate::prayer_service::Note::Place(place) => self.prayer_place = Some(place),
                crate::prayer_service::Note::Unavailable => {
                    if self.pause.is_active(jiff::Timestamp::now()) {
                        continue;
                    }
                    let mut spec = self.prefs.spec(ot_toast::Content {
                        title: ot_i18n::text("prayer-toast-unavailable-title"),
                        body: ot_i18n::text("prayer-toast-unavailable-body"),
                        app_name: ot_i18n::text("toast-app-name"),
                        timestamp: Some(formatted_time(jiff::Timestamp::now())),
                        action_labels: crate::reminder_toast::action_labels(),
                        ..ot_toast::Content::default()
                    });
                    spec.behaviour.actions = 1;
                    self.toasts.push(spec, Instant::now());
                    tracing::info!("the no-prayer-times toast was shown");
                }
            }
        }
    }

    /// The one toast that says reminders were missed, unless it is already
    /// up or notifications are paused. Built by the same function as every
    /// toast; it has nothing to snooze.
    fn announce_missed(&mut self) {
        if self.missed_toast.is_some() || self.pause.is_active(jiff::Timestamp::now()) {
            return;
        }
        let mut spec = self.prefs.spec(ot_toast::Content {
            title: ot_i18n::text("toast-missed-title"),
            body: ot_i18n::text("toast-missed-body"),
            app_name: ot_i18n::text("toast-app-name"),
            timestamp: Some(formatted_time(jiff::Timestamp::now())),
            action_labels: crate::reminder_toast::action_labels(),
            ..ot_toast::Content::default()
        });
        // Open is what the toast is for, and only the three-button row has
        // it, so this toast has all three whatever the action setting says.
        spec.behaviour.actions = 3;
        self.missed_toast = Some(self.toasts.push(spec, Instant::now()));
    }

    /// Write a history row, through the scheduler's thread.
    fn record(&self, reminder_id: i64, fired_at: jiff::Timestamp, action: NotificationAction) {
        self.send(ot_sched::scheduler::Command::Record {
            reminder_id,
            fired_at,
            action,
        });
    }

    /// Tell the scheduler something; it holds the store.
    fn send(&self, command: ot_sched::scheduler::Command) {
        if let Some(scheduler) = &self.scheduler
            && let Err(error) = scheduler.send(command)
        {
            tracing::warn!(command = ?error.0, "the scheduler has stopped; a command was lost");
        }
    }

    /// Take whatever the import and decode threads have finished.
    fn drain_images(&mut self, ctx: &egui::Context) {
        if let Some(outcome) = self.importer.drain() {
            match outcome {
                crate::images::ImportOutcome::Cancelled => {}
                crate::images::ImportOutcome::Imported(slot, asset, label) => {
                    tracing::info!(path = asset.relative_path, ?slot, "a picture was imported");
                    // Into the library first; the picker gets it when the
                    // library has answered.
                    self.library.add(asset, label, slot);
                }
                crate::images::ImportOutcome::Refused { name, error } => {
                    tracing::warn!(%error, "a picture was refused");
                    self.status = Status::ImportRefused {
                        key: error.name_key(),
                        name,
                    };
                }
                crate::images::ImportOutcome::DialogFailed(reason) => {
                    self.status = Status::Failed {
                        key: "settings-status-dialog-failed",
                        reason,
                    };
                }
            }
        }
        for reply in self.library.drain() {
            match reply {
                crate::library::Reply::Added {
                    slot,
                    relative_path,
                    label,
                    new,
                } => {
                    match slot {
                        Some(crate::images::Slot::Icon) => self.test_icon = Some(relative_path),
                        Some(crate::images::Slot::Image) => self.test_image = Some(relative_path),
                        None => {}
                    }
                    // Test content is not saved, so nothing follows to
                    // overwrite this message the way a deferred save did.
                    self.status = if new {
                        Status::LibraryAdded { label }
                    } else {
                        Status::LibraryAlreadyThere { label }
                    };
                }
                crate::library::Reply::InUse { sha256, templates } => {
                    self.confirm_remove = Some((sha256, templates > 0));
                }
                crate::library::Reply::Removed { relative_path } => {
                    // Whatever showed it falls back: the app icon in the
                    // header, no image in the body.
                    if self.test_icon.as_deref() == Some(relative_path.as_str()) {
                        self.test_icon = None;
                    }
                    if self.test_image.as_deref() == Some(relative_path.as_str()) {
                        self.test_image = None;
                    }
                    self.confirm_remove = None;
                    self.status = Status::LibraryRemoved;
                }
                crate::library::Reply::Failed(reason) => {
                    self.status = Status::Failed {
                        key: "settings-status-library-failed",
                        reason,
                    };
                }
            }
        }
        for (relative, error) in self.images.drain(ctx) {
            self.status = Status::ImportRefused {
                key: error.name_key(),
                name: relative,
            };
        }
    }

    fn status_line(&self, ui: &mut egui::Ui) {
        let tokens = self.theme.tokens();
        let colour = if self.status.is_failure() {
            tokens.danger
        } else {
            tokens.text_dim
        };
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new(self.status.text_with_pause(self.pause)).color(colour));
        });
    }
}

impl eframe::App for SettingsWindow {
    /// Fully transparent, for the toast windows: wherever a toast paints
    /// nothing, the desktop shows through (ADR-016). eframe's own default is
    /// a 70%-opaque near-black, which looks black there. The settings window
    /// paints its whole client area, and its frame is not extended, so this
    /// changes nothing it shows.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Nothing wakes egui when a tray event lands in its channel, so the
        // window asks to be woken. This is also what keeps the process
        // answering the tray while the window is hidden.
        ctx.request_repaint_after(TRAY_POLL);
        self.window = frame.window_handle().ok().map(|handle| handle.as_raw());
        self.drain_fires();
        self.drain_prayer_notes();
        self.drain_weather_notes();
        self.drain_update_notes();
        self.drain_book();
        self.handle_tray(ctx);
        self.drain_images(ctx);
        self.drive_toasts(ctx);

        // Closing the window hides it; only the tray's Quit ends the app
        // (docs/specs/tray-and-settings.md §6). Without this the one way to
        // stop OmniToaster would also be the way to glance at its settings.
        if ctx.input(|i| i.viewport().close_requested()) && !self.quitting {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.hide_window(ctx);
        }

        // System mode re-reads Windows on its own schedule. Applying the
        // palette clones the style, so it happens when the answer changed and
        // not on every frame (`.claude/rules/ui.md`).
        if self.theme.poll() || !self.applied {
            self.theme.apply(ctx);
            self.applied = true;
        }

        // The title bar is Windows' chrome, not egui's, so it is told
        // separately — and again whenever the appearance changes, which is
        // how System follows Windows while the window is open (CR-0002).
        let dark = self.theme.appearance() == crate::theme::Appearance::Dark;
        if title_bar_needs(self.title_bar_dark, dark)
            && let Ok(handle) = frame.window_handle()
            && ot_platform::title_bar::follow_appearance(handle.as_raw(), dark)
        {
            self.title_bar_dark = Some(dark);
        }

        let tokens = self.theme.tokens();
        // Nothing touches a pane edge: the inset is the frame's, on all four
        // sides, rather than a space added to one of them
        // (`.claude/rules/ui.md`).
        let inset = egui::Frame::new()
            .fill(tokens.surface)
            .inner_margin(layout::PANE_MARGIN);

        egui::TopBottomPanel::bottom("status")
            .frame(egui::Frame::new().fill(tokens.surface_raised).inner_margin(
                egui::Margin::symmetric(layout::PANE_MARGIN as i8, (layout::ROW_GAP / 2.0) as i8),
            ))
            .show(ctx, |ui| self.status_line(ui));

        egui::SidePanel::left("rail")
            .resizable(false)
            .exact_width(180.0)
            .frame(inset)
            .show(ctx, |ui| self.rail(ui));

        egui::CentralPanel::default()
            .frame(inset)
            .show(ctx, |ui| match self.section {
                Section::General => self.general_pane(ui),
                Section::Appearance => self.appearance_pane(ui),
                Section::Pictures => self.pictures_pane(ui),
                Section::Animation => self.animation_pane(ui),
                Section::Reminders => self.reminders_pane(ui),
                Section::Prayer => self.prayer_pane(ui),
                Section::Weather => self.weather_pane(ui),
                Section::Updates => self.updates_pane(ui),
            });

        if self.confirming_quit {
            self.quit_confirmation(ctx);
        }
    }
}

/// `ot-platform` names the pause windows without `ot-core`'s clock types, so
/// the two enums meet here, in the crate that depends on both.
fn pause_window(window: PauseWindow) -> PauseFor {
    match window {
        PauseWindow::ThirtyMinutes => PauseFor::ThirtyMinutes,
        PauseWindow::OneHour => PauseFor::OneHour,
        PauseWindow::UntilResumed => PauseFor::UntilResumed,
    }
}

/// Every tray label, resolved in the active language.
///
/// Built fresh each time the menu is built, which is what makes a language
/// change a rebuild rather than a special case — `ot-platform` is handed
/// strings and never sees a Fluent key.
fn tray_labels(pause: Pause) -> ot_platform::Labels {
    ot_platform::Labels {
        open_settings: ot_i18n::text("tray-open-settings"),
        pause: ot_i18n::text("tray-pause"),
        pause_30: ot_i18n::text(PauseFor::ThirtyMinutes.name_key()),
        pause_60: ot_i18n::text(PauseFor::OneHour.name_key()),
        pause_until_resumed: ot_i18n::text(PauseFor::UntilResumed.name_key()),
        resume: ot_i18n::text("tray-resume"),
        test_toast: ot_i18n::text("tray-test-toast"),
        snooze_last: ot_i18n::text("tray-snooze-last"),
        quit: ot_i18n::text("tray-quit"),
        tooltip: tooltip_for(pause),
    }
}

/// The hover text: the paused notice, or the app's name.
///
/// A paused app must say so where the user will see it, because the worst
/// outcome of a pause is forgetting it is on
/// (`docs/specs/tray-and-settings.md` §4). The next reminder or prayer takes
/// this place once there is one (Phases 4 and 5).
fn tooltip_for(pause: Pause) -> String {
    match pause {
        Pause::Running => ot_i18n::text("tray-tooltip-idle"),
        Pause::UntilResumed => ot_i18n::text("tray-tooltip-paused-until-resumed"),
        Pause::Until(moment) => ot_i18n::text_with(
            "tray-tooltip-paused-until",
            &[("time", formatted_time(moment).into())],
        ),
    }
}

/// A time of day through the one formatter, in the PC's own zone. Nothing
/// else may format one (`docs/10-i18n.md`).
fn formatted_time(moment: jiff::Timestamp) -> String {
    let prefs = ot_i18n::format::TimePrefs::current();
    let local = moment
        .to_zoned_datetime(jiff::tz::TimeZone::system())
        .datetime();
    ot_i18n::format::format_time(&local, prefs)
}

/// The list's rows: the title as typed, or the type's name for an untitled
/// one, and its type with when it next fires.
fn reminder_rows(listing: &crate::reminder_book::Listing) -> Vec<crate::reminders::ListRow> {
    use ot_store::reminders::Status as ReminderStatus;
    listing
        .reminders
        .iter()
        // Fatigue schedules have their own section, and the prayers are managed
        // from the Prayer settings.
        .filter(|reminder| {
            !reminder.draft.trigger.is_fatigue() && !reminder.draft.trigger.is_prayer()
        })
        .map(|reminder| {
            let type_key = listing
                .type_of(reminder.draft.category_id)
                .unwrap_or(crate::reminder_toast::DEFAULT_TYPE);
            let type_name = ot_i18n::text(type_key);
            let title = if reminder.draft.title.trim().is_empty() {
                type_name.clone()
            } else {
                reminder.draft.title.clone()
            };
            let with_time = |key, at| {
                ot_i18n::text_with(
                    key,
                    &[
                        ("type", type_name.clone().into()),
                        ("time", formatted_datetime(at).into()),
                    ],
                )
            };
            let detail = match (reminder.status, reminder.next_fire_at) {
                (ReminderStatus::Done | ReminderStatus::Cancelled, _) => ot_i18n::text_with(
                    "settings-reminder-row-done",
                    &[("type", type_name.clone().into())],
                ),
                (ReminderStatus::Snoozed, Some(at)) => {
                    with_time("settings-reminder-row-snoozed", at)
                }
                (_, Some(at)) => match &reminder.draft.recurrence {
                    Some(recurrence) => ot_i18n::text_with(
                        "settings-reminder-row-next-repeat",
                        &[
                            ("type", type_name.clone().into()),
                            ("repeat", crate::phrases::repeat(recurrence).into()),
                            ("time", formatted_datetime(at).into()),
                        ],
                    ),
                    None => with_time("settings-reminder-row-next", at),
                },
                (_, None) => ot_i18n::text_with(
                    "settings-reminder-row-plain",
                    &[("type", type_name.clone().into())],
                ),
            };
            crate::reminders::ListRow {
                id: reminder.id,
                title,
                detail,
            }
        })
        .collect()
}

/// A missed reminder's row: its title (the type's name, in the active
/// language, if it has none) and "Due …".
fn missed_row(
    reminder: &ot_store::reminders::Reminder,
    template: Option<&ot_store::reminders::Template>,
    fired_at: jiff::Timestamp,
    snoozes: u32,
    limit: u32,
) -> crate::reminders::MissedRow {
    crate::reminders::MissedRow {
        title: match &reminder.draft.trigger {
            ot_store::reminders::Trigger::Prayer { prayer } => {
                crate::prayer_pane::prayer_name(prayer)
            }
            _ => crate::reminder_toast::content(reminder, template, None).title,
        },
        due: ot_i18n::text_with(
            "settings-missed-due",
            &[("time", formatted_datetime(fired_at).into())],
        ),
        can_snooze: snoozes < limit,
    }
}

/// Whether a pause holds a fire back: it holds every reminder, and never a
/// prayer (`docs/specs/tray-and-settings.md` §4).
fn pause_suppresses(paused: bool, is_prayer: bool) -> bool {
    paused && !is_prayer
}

/// The fatigue section's rows: each kind from its managed reminder if it has
/// one (on, with its interval), else off at its default.
fn fatigue_rows(
    listing: Option<&crate::reminder_book::Listing>,
) -> Vec<crate::reminders::FatigueRow> {
    use ot_store::reminders::Trigger;
    ot_store::reminders::FATIGUE
        .iter()
        .map(|(type_key, repeats, default)| {
            let stored = listing.and_then(|listing| {
                let category = listing.category_of(type_key)?;
                listing
                    .reminders
                    .iter()
                    .find(|r| r.draft.trigger.is_fatigue() && r.draft.category_id == Some(category))
            });
            let minutes = match stored.map(|r| &r.draft.trigger) {
                Some(Trigger::Every { minutes } | Trigger::WorkHours { minutes }) => *minutes,
                _ => *default,
            };
            crate::reminders::FatigueRow {
                type_key,
                repeats: *repeats,
                on: stored.is_some(),
                minutes,
                unit: None,
            }
        })
        .collect()
}

/// The history's name for what happened to a fire.
fn history_key(action: NotificationAction) -> &'static str {
    match action {
        NotificationAction::Shown => "settings-history-shown",
        NotificationAction::Snoozed => "settings-history-snoozed",
        NotificationAction::Dismissed => "settings-history-dismissed",
        NotificationAction::Opened => "settings-history-opened",
        NotificationAction::Missed => "settings-history-missed",
    }
}

/// A date and time through the one formatter, in the PC's own zone.
fn formatted_datetime(moment: jiff::Timestamp) -> String {
    let prefs = ot_i18n::format::TimePrefs::current();
    let local = moment
        .to_zoned_datetime(jiff::tz::TimeZone::system())
        .datetime();
    ot_i18n::format::format_datetime(&local, prefs)
}

/// The fonts, installed into a fresh `FontDefinitions`.
///
/// egui's `default_fonts` is off (ADR-014), so this is the only text the app
/// has: a failure here means a window of blank rectangles, which is why it is
/// an error the caller reports rather than something to log and continue past.
///
/// Public so the layout tests install the same fonts the window draws with:
/// with `default_fonts` off, a context without these measures every glyph as
/// zero wide, and a layout test on it proves nothing (`docs/GOTCHAS.md`).
pub fn font_definitions() -> Result<egui::FontDefinitions, ot_platform::FontError> {
    let proportional = ot_platform::ui_font::proportional()?;
    let monospace = ot_platform::ui_font::monospace()?;
    tracing::info!(
        proportional = %proportional.path.display(),
        monospace = %monospace.path.display(),
        "the UI fonts were loaded from the Windows font folder"
    );

    let mut fonts = egui::FontDefinitions::empty();
    fonts.font_data.insert(
        "ui".to_owned(),
        std::sync::Arc::new(egui::FontData::from_owned(proportional.bytes)),
    );
    fonts.font_data.insert(
        "mono".to_owned(),
        std::sync::Arc::new(egui::FontData::from_owned(monospace.bytes)),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .push("ui".to_owned());
    // The monospace family falls back to the UI font, so a glyph missing from
    // Consolas still draws rather than showing a replacement box.
    let mono = fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default();
    mono.push("mono".to_owned());
    mono.push("ui".to_owned());

    // The symbol fallback goes last in both families, so it only ever
    // answers for a glyph the UI font lacks — a bell, a clock — and never
    // changes how ordinary text looks.
    if let Some(symbols) = ot_platform::ui_font::symbols() {
        fonts.font_data.insert(
            "symbols".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(symbols.bytes)),
        );
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            fonts
                .families
                .entry(family)
                .or_default()
                .push("symbols".to_owned());
        }
    } else {
        tracing::warn!("Segoe UI Symbol is missing; glyphs the UI font lacks will not draw");
    }
    Ok(fonts)
}

/// Open the settings window and run it until the user closes it.
///
/// Blocks the calling thread, which is the UI thread: eframe owns the event
/// loop. Everything that can block belongs on another thread
/// (`.claude/rules/threads-and-scheduling.md`).
/// Switches for development builds, set from debug-only command-line flags
/// (`crates/omnitoaster/src/main.rs`); every one is off in a release.
#[derive(Debug, Clone, Copy, Default)]
pub struct DevOptions {
    /// Fire one toast of each shape at startup.
    pub test_toast: bool,
    /// Scale the UI as a larger display scaling would.
    pub ui_scale: Option<f32>,
    /// Every toast fades, as with Windows' Show animations off.
    pub reduce_motion: bool,
    /// Every frame counts as 50 ms, so the frame-time fallback is forced.
    pub slow_frames: bool,
    /// An alarm's toast closes after this many seconds instead of the
    /// ten-minute cap, so the cap can be watched.
    pub alarm_cap_secs: Option<f32>,
    /// A weather code to simulate: the weather service reads a sky that is clear
    /// once and then this code, every few seconds, whatever the settings say.
    pub simulate_weather: Option<u8>,
}

/// The app's icon as the window takes it: the 256-pixel master of the set
/// `scripts/make-icons.ps1` makes, which Windows scales for the title bar and the
/// taskbar. The tray and the exe's own file icon come from the resource the build
/// embeds; a window gets none of that by itself and shows eframe's default "e"
/// unless it is handed one (BUG-0019). `None` only if the embedded file were
/// damaged, which the test would catch.
fn window_icon() -> Option<egui::IconData> {
    const PNG: &[u8] = include_bytes!("../../../assets/icon/omnitoaster-256.png");
    let decoded = image::load_from_memory_with_format(PNG, image::ImageFormat::Png)
        .map_err(|error| tracing::warn!(%error, "the window icon could not be read"))
        .ok()?
        .into_rgba8();
    Some(egui::IconData {
        width: decoded.width(),
        height: decoded.height(),
        rgba: decoded.into_raw(),
    })
}

/// The Settings window's own shape: its title, its size and its icon.
fn window_viewport() -> egui::ViewportBuilder {
    let viewport = egui::ViewportBuilder::default()
        .with_title(ot_i18n::text("settings-window-title"))
        .with_inner_size([880.0, 560.0])
        // Narrow enough to prove the layout survives it, and no narrower:
        // the rules require the window to be checked at this size.
        .with_min_inner_size([560.0, 400.0]);
    match window_icon() {
        Some(icon) => viewport.with_icon(icon),
        None => viewport,
    }
}

pub fn run(
    root: DataRoot,
    settings: Settings,
    dev: DevOptions,
    fires: Receiver<ot_sched::scheduler::Fired>,
    scheduler: Sender<ot_sched::scheduler::Command>,
) -> Result<(), WindowError> {
    let ui_scale = dev.ui_scale;
    let fonts = font_definitions().map_err(WindowError::Fonts)?;
    let book_root = root.path().to_path_buf();
    let mut window = SettingsWindow::new(root, settings);
    window.dev = dev;
    window.fires = Some(fires);
    window.book = Some(crate::reminder_book::Book::new(
        &book_root,
        Some(scheduler.clone()),
    ));
    window.prayer_service = Some(crate::prayer_service::Service::start(
        &book_root,
        scheduler.clone(),
    ));
    window.weather_service = Some(crate::weather_service::Service::start(
        &book_root,
        dev.simulate_weather,
    ));
    window.update_service = Some(crate::update_service::Service::start(&book_root));
    window.scheduler = Some(scheduler);
    if dev.test_toast {
        window.show_shape_samples();
    }
    let visuals = window.theme().visuals();

    let options = eframe::NativeOptions {
        viewport: window_viewport(),
        // No vsync. Every toast is its own window painted in the root's frame
        // (ADR-016), and with vsync each window's present waits for the next
        // refresh in turn: four windows, four refreshes, 12–32 fps. Windows'
        // compositor already keeps a windowed app from tearing, so nothing is
        // lost; the animation paces itself instead (`ANIMATION_FRAME`).
        vsync: false,
        ..Default::default()
    };

    eframe::run_native(
        "OmniToaster",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_fonts(fonts);
            if let Some(scale) = ui_scale {
                // What a larger display scaling does to a layout: every
                // point becomes more pixels, so the same window holds fewer
                // points of content. It is the layout half of the DPI check
                // and not the whole of it - the window's own physical size
                // still comes from the monitor.
                cc.egui_ctx.set_zoom_factor(scale);
            }
            // Before the first frame, so no frame draws in egui's own colours.
            cc.egui_ctx.set_visuals(visuals);
            Ok(Box::new(window))
        }),
    )
    .map_err(|error| WindowError::Eframe(error.to_string()))
}

/// Why the settings window could not open.
#[derive(Debug, thiserror::Error)]
pub enum WindowError {
    #[error("no UI font could be loaded, so nothing would be readable: {0}")]
    Fonts(#[source] ot_platform::FontError),
    #[error("the window could not be opened: {0}")]
    Eframe(String),
}

/// Whether the title bar has to be told the appearance: when Windows has not
/// accepted one yet, or accepted the other one.
fn title_bar_needs(accepted: Option<bool>, dark: bool) -> bool {
    accepted != Some(dark)
}

/// One entry of a picture picker: its thumbnail once decoded, and its name.
fn picture_choice(
    ui: &mut egui::Ui,
    picture: Option<ot_toast::content::Picture>,
    label: &str,
    selected: bool,
    side: f32,
) -> egui::Response {
    let text = egui::RichText::new(label);
    let button = match picture {
        Some(picture) => egui::Button::selectable(
            selected,
            (
                egui::Image::new(egui::load::SizedTexture::new(picture.texture, picture.size))
                    .max_size(egui::vec2(side, side)),
                text,
            ),
        ),
        None => egui::Button::selectable(selected, text),
    };
    ui.add(button)
}

/// A picture fitted into a `side` square, unstretched; an empty square while
/// it is still being decoded, so the rows do not jump when it arrives.
fn paint_thumbnail(ui: &mut egui::Ui, picture: Option<ot_toast::content::Picture>, side: f32) {
    let (slot, _) = ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::hover());
    if let Some(picture) = picture {
        let fitted = egui::Rect::from_center_size(
            slot.center(),
            picture.size * (side / picture.size.max_elem()).min(1.0),
        );
        egui::Image::new(egui::load::SizedTexture::new(picture.texture, picture.size))
            .paint_at(ui, fitted);
    }
}

/// The name a picture goes by: the label it has in the library, or its
/// Fluent name when it is one that ships with the app.
fn picture_label(library: &crate::library::Library, path: &str) -> Option<String> {
    library
        .label_of(path)
        .map(str::to_owned)
        .or_else(|| ot_store::builtin::find(path).map(|builtin| ot_i18n::text(builtin.label_key)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_settings_window_carries_the_apps_own_icon_not_eframes_default() {
        // BUG-0019: without `with_icon` Windows shows eframe's "e" in the title bar and
        // on the taskbar, though the exe itself carries the real icon.
        let icon = window_viewport()
            .icon
            .expect("the window has an icon of its own");
        assert_eq!((icon.width, icon.height), (256, 256));
        assert_eq!(icon.rgba.len(), 256 * 256 * 4);
        assert!(
            icon.rgba
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| pixel[3] == 255),
            "it is not blank"
        );
    }

    #[test]
    fn the_title_bar_is_told_each_appearance_once() {
        // Not set yet: told whichever it is.
        assert!(title_bar_needs(None, true));
        assert!(title_bar_needs(None, false));
        // Already dark: not told dark again every frame...
        assert!(!title_bar_needs(Some(true), true));
        // ...but told when System follows Windows to light, and back.
        assert!(title_bar_needs(Some(true), false));
        assert!(title_bar_needs(Some(false), true));
    }

    /// Every Fluent key the window can ask for.
    fn every_key() -> Vec<&'static str> {
        let mut keys = vec![
            "settings-window-title",
            "settings-language",
            "settings-language-hint",
            "settings-theme",
            "settings-startup",
            "settings-startup-hint",
            "settings-startup-elsewhere",
            "settings-status-ready",
            "settings-status-saved",
            "settings-status-saved",
            "settings-status-save-failed",
            "settings-status-autostart-on",
            "settings-status-autostart-off",
            "settings-status-autostart-failed",
            "settings-status-tray-failed",
            "settings-status-language-failed",
            "settings-status-paused-until",
            "settings-status-paused-until-resumed",
            "settings-status-resumed",
            // The tray menu, its tooltip and the quit confirmation. They go
            // through this crate, so a missing one would draw its key in the
            // one surface a user always has (FEAT-0015).
            "tray-open-settings",
            "tray-pause",
            "tray-resume",
            "tray-test-toast",
            "tray-snooze-last",
            "tray-quit",
            "tray-tooltip-idle",
            "tray-tooltip-paused-until",
            "tray-tooltip-paused-until-resumed",
            "tray-quit-confirm-title",
            "tray-quit-confirm-body",
            "tray-quit-confirm-quit",
            "tray-quit-confirm-stay",
        ];
        keys.extend(Section::all().map(Section::name_key));
        keys.extend(LanguageChoice::all().map(LanguageChoice::name_key));
        keys.extend(ThemeMode::all().map(ThemeMode::name_key));
        keys.extend(PauseFor::all().map(PauseFor::name_key));
        // The pictures that ship with the app, and their groups.
        keys.extend(
            ot_store::builtin::all()
                .iter()
                .map(|picture| picture.label_key),
        );
        keys.extend(ot_store::builtin::Group::all().map(ot_store::builtin::Group::name_key));
        // Settings ▸ Appearance, and every option its pickers offer.
        keys.extend([
            "settings-toast-preview",
            "settings-toast-test",
            "settings-toast-shape",
            "settings-toast-radius",
            "settings-toast-width",
            "settings-toast-opacity",
            "settings-toast-border",
            "settings-toast-shadow",
            "settings-toast-shadow-hint",
            "settings-toast-shadow-glass-hint",
            "settings-toast-glass",
            "settings-toast-glass-hint",
            "settings-toast-glass-shape-hint",
            "settings-toast-background",
            "settings-toast-accent",
            "settings-toast-header",
            "settings-toast-header-icon",
            "settings-toast-header-app-name",
            "settings-toast-header-time",
            "settings-toast-position",
            "settings-toast-position-custom",
            "settings-toast-monitor",
            "settings-toast-monitor-hint",
            "settings-toast-monitor-primary",
            "settings-toast-monitor-option",
            "settings-toast-monitor-option-primary",
            "settings-toast-monitor-unnamed",
            "settings-toast-monitor-missing",
            "settings-toast-duration",
            "settings-toast-duration-hint",
            "settings-toast-max-visible",
            "settings-toast-stack-gap",
            "settings-toast-position-x",
            "settings-toast-position-y",
            "settings-toast-map-hint",
            "settings-toast-image",
            "settings-toast-test-content",
            "settings-toast-test-content-hint",
            "settings-toast-icon",
            "settings-toast-icon-dialog-title",
            "settings-toast-image-border",
            "settings-toast-own-accent",
            "settings-toast-image-dialog-title",
            "settings-toast-image-filter",
            // Settings ▸ Picture Library.
            "settings-library-hint",
            "settings-library-upload",
            "settings-library-upload-new",
            "settings-library-dialog-title",
            "settings-library-loading",
            "settings-library-empty",
            "settings-library-label-hint",
            "settings-library-remove",
            "settings-library-in-use",
            "settings-library-used-by-test-toast",
            "settings-library-used-by-templates",
            "settings-library-remove-anyway",
            "settings-library-keep",
            "settings-library-none-icon",
            "settings-library-none-image",
            "settings-library-builtin",
            "settings-library-builtin-hint",
            "settings-status-library-added",
            "settings-status-library-already-there",
            "settings-status-library-removed",
            "settings-status-library-failed",
            "settings-status-dialog-failed",
            "error-import-too-large",
            "error-import-not-an-image",
            "error-import-too-many-pixels",
            "error-import-undecodable",
            "error-import-escapes",
            "error-import-unreadable",
            "toast-position-custom-hint",
            "toast-test-title",
            "toast-test-body",
            "toast-app-name",
            "toast-action-open",
            "toast-action-snooze",
            "toast-action-dismiss",
            "settings-animation-enter",
            "settings-animation-leave",
            "settings-animation-duration",
            "settings-animation-easing",
            "settings-animation-reduce",
            "settings-animation-windows-on",
            "settings-animation-windows-off",
            "settings-animation-reduced",
            "settings-animation-play",
        ]);
        keys.extend(ot_toast::Effect::all_out().map(ot_toast::Effect::name_key));
        keys.extend(ot_toast::Easing::all().map(ot_toast::Easing::name_key));
        keys.extend(ot_toast::Shape::all().map(ot_toast::Shape::name_key));
        keys.extend(ot_toast::Glass::all().map(ot_toast::Glass::name_key));
        keys.extend(ot_toast::Position::presets().map(ot_toast::Position::name_key));
        keys.push(ot_toast::Position::Custom { x: 0.0, y: 0.0 }.name_key());
        keys
    }

    #[test]
    fn the_general_pane_names_only_keys_that_exist_in_both_catalogues() {
        for tag in ["en-US", "ms-MY"] {
            let present = ot_i18n::message_keys(tag).expect("the catalogue loads");
            for key in every_key() {
                assert!(
                    present.contains(key),
                    "{tag} has no message {key}, so the window would draw its key"
                );
            }
        }
    }

    #[test]
    fn a_stored_language_round_trips_and_a_broken_one_follows_windows() {
        for choice in LanguageChoice::all() {
            assert_eq!(
                LanguageChoice::parse(choice.as_setting().as_deref()),
                choice,
                "{choice:?} did not survive the settings file"
            );
        }
        assert_eq!(LanguageChoice::parse(None), LanguageChoice::System);
        assert_eq!(
            LanguageChoice::parse(Some("klingon")),
            LanguageChoice::System
        );
        // Case and the short form are both things a human types by hand.
        assert_eq!(
            LanguageChoice::parse(Some("MS-MY")),
            LanguageChoice::Fixed(Language::Malay)
        );
        assert_eq!(
            LanguageChoice::parse(Some("en")),
            LanguageChoice::Fixed(Language::English)
        );
    }

    #[test]
    fn the_pause_windows_map_across_the_crate_boundary_without_crossing_over() {
        // `ot-platform` names the windows without ot-core's clock types, so
        // the two enums are translated by hand; swapping two would mute the
        // app for the wrong length of time and nothing would complain.
        assert_eq!(
            pause_window(PauseWindow::ThirtyMinutes),
            PauseFor::ThirtyMinutes
        );
        assert_eq!(pause_window(PauseWindow::OneHour), PauseFor::OneHour);
        assert_eq!(
            pause_window(PauseWindow::UntilResumed),
            PauseFor::UntilResumed
        );
    }

    #[test]
    fn a_pause_holds_back_every_reminder_and_never_a_prayer() {
        assert!(pause_suppresses(true, false));
        assert!(!pause_suppresses(true, true), "an Azan is never paused");
        assert!(!pause_suppresses(false, false));
        assert!(!pause_suppresses(false, true));
    }

    #[test]
    fn the_status_line_distinguishes_a_failure_from_a_notice() {
        assert!(
            Status::Failed {
                key: "settings-status-save-failed",
                reason: "disk full".to_string()
            }
            .is_failure()
        );
        for quiet in [
            Status::Ready,
            Status::Saved,
            Status::AutostartOn,
            Status::AutostartOff,
            Status::AutostartElsewhere,
            Status::Paused,
            Status::Resumed,
        ] {
            assert!(!quiet.is_failure(), "{quiet:?} is not a failure");
        }
    }
}
