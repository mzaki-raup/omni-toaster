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

use ot_core::{DataRoot, Settings};
use ot_i18n::Language;
use ot_platform::Autostart;

use crate::layout;
use crate::theme::{Theme, ThemeMode};

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
    Failed { key: &'static str, reason: String },
}

impl Status {
    fn text(&self) -> String {
        match self {
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
        Self {
            root,
            settings,
            theme,
            section: Section::General,
            language,
            autostart,
            status,
            applied: false,
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

    fn set_language(&mut self, choice: LanguageChoice) {
        self.language = choice;
        self.settings.general.language = choice.as_setting();
        self.save();
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
            self.set_language(chosen);
        }
        // Honest about what the control does today: the live switch is
        // FEAT-0016, and a picker that appears to do nothing is worse than one
        // that says when it takes effect.
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

    fn status_line(&self, ui: &mut egui::Ui) {
        let tokens = self.theme.tokens();
        let colour = if self.status.is_failure() {
            tokens.danger
        } else {
            tokens.text_dim
        };
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new(self.status.text()).color(colour));
        });
    }
}

impl eframe::App for SettingsWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
    }
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
pub fn run(root: DataRoot, settings: Settings) -> Result<(), WindowError> {
    let fonts = font_definitions().map_err(WindowError::Fonts)?;
    let window = SettingsWindow::new(root, settings);
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
        ];
        keys.extend(Section::all().map(Section::name_key));
        keys.extend(LanguageChoice::all().map(LanguageChoice::name_key));
        keys.extend(ThemeMode::all().map(ThemeMode::name_key));
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
        ] {
            assert!(!quiet.is_failure(), "{quiet:?} is not a failure");
        }
    }
}
