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

use std::time::{Duration, Instant};

use ot_core::{DataRoot, Pause, PauseFor, Settings};
use ot_i18n::Language;
use ot_platform::{Autostart, PauseWindow, Tray, TrayCommand};

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
}

impl Section {
    /// The Fluent key naming the section in the rail.
    pub fn name_key(self) -> &'static str {
        match self {
            Self::General => "settings-section-general",
            Self::Appearance => "settings-section-appearance",
            Self::Pictures => "settings-section-pictures",
        }
    }

    pub fn all() -> [Self; 3] {
        [Self::General, Self::Appearance, Self::Pictures]
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
    /// Whether the chosen monitor was missing last time, so falling back to
    /// the primary one is logged once, not every frame.
    monitor_missing: bool,
}

impl SettingsWindow {
    /// Build the window's state from the data folder.
    ///
    /// The autostart state is read here and after every change, never cached
    /// from the settings file.
    pub fn new(root: DataRoot, settings: Settings) -> Self {
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
        let (tray, status) =
            match Tray::new(&tray_labels(pause), pause.is_active(chrono::Local::now())) {
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
            monitor_missing: false,
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
        self.rebuild_tray(chrono::Local::now());
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
        let now = chrono::Local::now();
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
    fn rebuild_tray(&mut self, now: chrono::DateTime<chrono::Local>) {
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
        let id = self.toasts.push(spec);
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
            let id = self.toasts.push(spec);
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
            // Formatted here: ot-i18n owns the one formatter that may turn a
            // time into text, and a spec holds text rather than keys.
            timestamp: Some(formatted_time(chrono::Local::now())),
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
            self.images.request(image);
        }
        (
            self.images.picture(&icon),
            content
                .image
                .as_deref()
                .and_then(|path| self.images.picture(path)),
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

    /// Retire what has had its time, and draw what is left.
    fn drive_toasts(&mut self, ctx: &egui::Context) {
        for id in self.toasts.tick() {
            // A retired toast's viewport has to be told to close: egui keeps
            // a deferred viewport alive until nobody asks for it, and the
            // window would otherwise linger empty.
            ctx.send_viewport_cmd_to(id.viewport_id(), egui::ViewportCommand::Close);
            ot_platform::toast_window::forget(&id.window_title());
        }
        if self.toasts.is_empty() {
            return;
        }

        // `ViewportBuilder` cannot take a window out of Alt+Tab, so the
        // style is set on the real window once it exists (FEAT-0018). Cheap
        // and idempotent: it changes nothing after the first frame.
        ot_platform::toast_window::make_unobtrusive(ot_toast::viewport::WINDOW_TITLE);

        // A monitor can be unplugged while a toast is on screen: a chosen one
        // that has gone falls back to the primary monitor rather than leaving
        // the toast on a screen that is not there (docs/specs/toast.md §7.4).
        let work_area = self.work_area();

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

        ot_toast::viewport::show_all(ctx, &self.toasts, work_area, move |ui, spec, id| {
            // The whole toast comes from the spec: the renderer reads no
            // setting and no category, which is what keeps the preview and
            // the real thing from diverging (docs/specs/toast.md §3).
            let rect = ui.max_rect();
            let content = ot_toast::render::frame(ui.painter(), rect, spec);

            // The window is cut to the corners just painted, so nothing
            // outside them shows (BUG-0007). In this window's own pixels:
            // a toast on a 150% monitor has corners half as large again.
            let pixels = ui.ctx().pixels_per_point();
            let radii = ot_toast::render::corner_radii(spec, rect.height())
                .map(|radius| (radius * pixels).round() as i32);
            // Opacity fades the whole window, text included (CR-0005).
            let opacity = (spec.alpha() * 255.0).round().clamp(0.0, 255.0) as u8;
            let shadow = spec.style.shadow.shows(spec.style.shape);
            ot_platform::toast_window::set_shape(&id.window_title(), radii, opacity, shadow);

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
                .and_then(|path| pictures.get(path).copied());
            // The toast's own background picks the text colour, not the
            // window's theme, which a toast does not follow.
            let text = Tokens::for_toast(spec);
            ot_toast::content::draw(
                ui.painter(),
                content,
                spec,
                text.text,
                text.text_dim,
                icon,
                image,
            );
        });
    }

    fn show_window(&self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    fn hide_window(&self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
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
        let current = match slot {
            Slot::Icon => self.test_icon.as_deref(),
            Slot::Image => self.test_image.as_deref(),
        };
        let shown = current
            .and_then(|path| self.library.label_of(path))
            .map_or_else(|| ot_i18n::text(none_key), str::to_owned);
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
                        let text = egui::RichText::new(&entry.picture.label);
                        let button = match images.picture(path) {
                            Some(picture) => egui::Button::selectable(
                                selected,
                                (
                                    egui::Image::new(egui::load::SizedTexture::new(
                                        picture.texture,
                                        picture.size,
                                    ))
                                    .max_size(egui::vec2(height, height)),
                                    text,
                                ),
                            ),
                            None => egui::Button::selectable(selected, text),
                        };
                        if ui.add(button).clicked() {
                            choice = Some(Some(path.to_owned()));
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Nothing wakes egui when a tray event lands in its channel, so the
        // window asks to be woken. This is also what keeps the process
        // answering the tray while the window is hidden.
        ctx.request_repaint_after(TRAY_POLL);
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

/// A time of day through the one formatter. Nothing else may format one
/// (`docs/10-i18n.md`).
fn formatted_time(moment: chrono::DateTime<chrono::Local>) -> String {
    let prefs = ot_i18n::format::TimePrefs::for_language(
        ot_i18n::current_language().unwrap_or(Language::English),
    );
    ot_i18n::format::format_time(&moment.naive_local(), prefs)
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
pub fn run(
    root: DataRoot,
    settings: Settings,
    test_toast: bool,
    ui_scale: Option<f32>,
) -> Result<(), WindowError> {
    let fonts = font_definitions().map_err(WindowError::Fonts)?;
    let mut window = SettingsWindow::new(root, settings);
    if test_toast {
        window.show_shape_samples();
    }
    let visuals = window.theme().visuals();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(ot_i18n::text("settings-window-title"))
            .with_inner_size([880.0, 560.0])
            // Narrow enough to prove the layout survives it, and no narrower:
            // the rules require the window to be checked at this size.
            .with_min_inner_size([560.0, 400.0]),
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

#[cfg(test)]
mod tests {
    use super::*;

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
        ]);
        keys.extend(ot_toast::Shape::all().map(ot_toast::Shape::name_key));
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
