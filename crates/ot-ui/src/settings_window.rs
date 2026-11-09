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

use std::time::Duration;

use ot_core::{DataRoot, Pause, PauseFor, Settings};
use ot_i18n::Language;
use ot_platform::{Autostart, PauseWindow, Tray, TrayCommand};

use crate::layout;
use crate::theme::{Theme, ThemeMode};

/// How often the window wakes to look at the tray's channels.
///
/// The tray posts into a channel; nothing wakes egui when it does, so the
/// window asks to be repainted on this interval. Short enough that a menu
/// click feels immediate, long enough that an idle app is not spinning —
/// at 120 ms an idle OmniToaster does eight almost-empty frames a second,
/// and its window is usually hidden.
const TRAY_POLL: Duration = Duration::from_millis(120);

/// The window's sections. One for now; the rail exists so the second costs
/// nothing but a variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    General,
}

impl Section {
    /// The Fluent key naming the section in the rail.
    pub fn name_key(self) -> &'static str {
        match self {
            Self::General => "settings-section-general",
        }
    }

    pub fn all() -> [Self; 1] {
        [Self::General]
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
        matches!(self, Self::Failed { .. })
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
    /// Set when Quit was asked for while a pause is running: the window comes
    /// back with a confirmation rather than the app going quiet for good
    /// while the user still thinks it is only muted.
    confirming_quit: bool,
}

impl SettingsWindow {
    /// Build the window's state from the data folder.
    ///
    /// The autostart state is read here and after every change, never cached
    /// from the settings file.
    pub fn new(root: DataRoot, settings: Settings) -> Self {
        let language = LanguageChoice::parse(settings.general.language.as_deref());
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
            confirming_quit: false,
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
            // One of each shadow, and a gradient and a border on the later
            // ones, so a single screenshot shows the whole of FEAT-0019.
            spec.style.shadow = ot_toast::Shadow::all()[index % 3];
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

    fn test_spec(&self) -> ot_toast::ToastSpec {
        let mut spec = ot_toast::ToastSpec::default();
        spec.content.title = ot_i18n::text("toast-test-title");
        spec.content.body = ot_i18n::text("toast-test-body");
        spec.content.app_name = ot_i18n::text("toast-app-name");
        spec.content.show_icon = true;
        spec.content.show_timestamp = true;
        spec.clamped()
    }

    /// Retire what has had its time, and draw what is left.
    fn drive_toasts(&mut self, ctx: &egui::Context) {
        for id in self.toasts.tick() {
            // A retired toast's viewport has to be told to close: egui keeps
            // a deferred viewport alive until nobody asks for it, and the
            // window would otherwise linger empty.
            ctx.send_viewport_cmd_to(id.viewport_id(), egui::ViewportCommand::Close);
        }
        if self.toasts.is_empty() {
            return;
        }

        // `ViewportBuilder` cannot take a window out of Alt+Tab, so the
        // style is set on the real window once it exists (FEAT-0018). Cheap
        // and idempotent: it changes nothing after the first frame.
        ot_platform::toast_window::make_unobtrusive(ot_toast::viewport::WINDOW_TITLE);

        let tokens = self.theme.tokens();
        ot_toast::viewport::show_all(ctx, &self.toasts, move |ui, spec| {
            // The whole toast comes from the spec: the renderer reads no
            // setting and no category, which is what keeps the preview and
            // the real thing from diverging (docs/specs/toast.md §3).
            let rect = ui.max_rect();
            let content = ot_toast::render::frame(ui.painter(), rect, spec);

            // FEAT-0020 lays the header and the body out properly. Until
            // then the title is drawn in the content rect, which is enough
            // to show that the frame leaves room for it.
            if !spec.content.title.is_empty() {
                ui.painter().text(
                    content.left_center(),
                    egui::Align2::LEFT_CENTER,
                    &spec.content.title,
                    egui::FontId::proportional(16.0),
                    tokens.text,
                );
            }
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
fn font_definitions() -> Result<egui::FontDefinitions, ot_platform::FontError> {
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
