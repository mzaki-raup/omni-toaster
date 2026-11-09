//! The three themes, and the semantic tokens every surface draws with.
//!
//! Dark is the default; Light is its counterpart; System follows the Windows
//! app theme and rechecks every five seconds, so a change there is picked up
//! without a restart (`docs/04-ui-design.md`,
//! `docs/specs/tray-and-settings.md` §8).
//!
//! **Nothing hard-codes a colour.** A surface asks for `tokens.surface` or
//! `tokens.danger`, never for a hex value, so Light and Dark cannot drift
//! apart and a toast's own accent can override exactly one thing without
//! taking the rest of the palette with it.

use std::time::Instant;

use egui::Color32;
use ot_platform::{SystemTheme, system_theme};

/// What the user chose in Settings ▸ General ▸ Theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    /// Follow Windows.
    System,
    /// The default, and what the product is designed in.
    #[default]
    Dark,
    Light,
}

impl ThemeMode {
    /// The settings value, and what round-trips through the store.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    /// Parse a stored value, falling back to the default rather than failing:
    /// a hand-edited `settings.toml` should not stop the app starting.
    pub fn parse(text: &str) -> Self {
        match text.trim().to_ascii_lowercase().as_str() {
            "system" => Self::System,
            "light" => Self::Light,
            _ => Self::Dark,
        }
    }

    /// The Fluent key naming this mode in the picker.
    pub fn name_key(self) -> &'static str {
        match self {
            Self::System => "settings-theme-system",
            Self::Dark => "settings-theme-dark",
            Self::Light => "settings-theme-light",
        }
    }

    /// Every mode, in the order the picker shows them.
    pub fn all() -> [Self; 3] {
        [Self::System, Self::Dark, Self::Light]
    }
}

/// Whether a surface draws dark or light. `System` has already been resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appearance {
    Dark,
    Light,
}

/// The semantic palette. The names say what a colour is *for*, not what it
/// looks like, which is what lets Light and Dark stay in step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tokens {
    /// The window and pane background.
    pub surface: Color32,
    /// A card, a header band, anything lifted off the background.
    pub surface_raised: Color32,
    /// Body text.
    pub text: Color32,
    /// Hints, placeholders, disabled labels.
    pub text_dim: Color32,
    /// The one colour that draws the eye: selection, focus, the active item.
    pub accent: Color32,
    /// Rules, field outlines, the line under a header band.
    pub border: Color32,
    /// Destructive verbs, and only those.
    pub danger: Color32,
}

impl Tokens {
    /// The dark palette, which is the product's default and the one the
    /// toasts were designed against.
    pub const DARK: Self = Self {
        surface: Color32::from_rgb(0x1b, 0x1b, 0x1f),
        surface_raised: Color32::from_rgb(0x26, 0x26, 0x2c),
        text: Color32::from_rgb(0xec, 0xec, 0xf0),
        text_dim: Color32::from_rgb(0x9a, 0x9a, 0xa6),
        accent: Color32::from_rgb(0x3b, 0x9e, 0xff),
        border: Color32::from_rgb(0x3a, 0x3a, 0x42),
        danger: Color32::from_rgb(0xff, 0x6b, 0x6b),
    };

    /// The light palette.
    pub const LIGHT: Self = Self {
        surface: Color32::from_rgb(0xf7, 0xf7, 0xfa),
        surface_raised: Color32::from_rgb(0xff, 0xff, 0xff),
        text: Color32::from_rgb(0x1b, 0x1b, 0x1f),
        text_dim: Color32::from_rgb(0x5f, 0x5f, 0x6b),
        accent: Color32::from_rgb(0x12, 0x6b, 0xd6),
        border: Color32::from_rgb(0xd4, 0xd4, 0xdc),
        danger: Color32::from_rgb(0xc4, 0x28, 0x28),
    };

    /// The palette for an appearance.
    pub fn for_appearance(appearance: Appearance) -> Self {
        match appearance {
            Appearance::Dark => Self::DARK,
            Appearance::Light => Self::LIGHT,
        }
    }
}

/// What a mode draws as, given what Windows said. Pure, so the System path
/// can be asserted without touching the machine's settings.
fn resolve(mode: ThemeMode, reading: Option<SystemTheme>) -> Appearance {
    match mode {
        ThemeMode::Dark => Appearance::Dark,
        ThemeMode::Light => Appearance::Light,
        ThemeMode::System => match reading {
            Some(SystemTheme::Light) => Appearance::Light,
            Some(SystemTheme::Dark) => Appearance::Dark,
            // Windows did not say. Fall back to the app's default rather than
            // guessing light (`docs/04-ui-design.md`).
            None => Appearance::Dark,
        },
    }
}

/// Resolves the chosen mode to an appearance, polling Windows for System.
///
/// Holds the last reading and when it was taken, so System costs one registry
/// read every five seconds rather than one per frame — an immediate-mode UI
/// would otherwise do it sixty times a second
/// (`.claude/rules/ui.md`).
#[derive(Debug)]
pub struct Theme {
    mode: ThemeMode,
    appearance: Appearance,
    last_polled: Option<Instant>,
}

impl Theme {
    /// Start in `mode`, resolving it once.
    pub fn new(mode: ThemeMode) -> Self {
        let mut theme = Self {
            mode,
            appearance: Appearance::Dark,
            last_polled: None,
        };
        theme.appearance = theme.resolve_now();
        theme
    }

    /// The chosen mode.
    pub fn mode(&self) -> ThemeMode {
        self.mode
    }

    /// Change the mode, resolving immediately so the UI does not wait out the
    /// poll interval before it changes.
    pub fn set_mode(&mut self, mode: ThemeMode) {
        self.mode = mode;
        self.last_polled = None;
        self.appearance = self.resolve_now();
    }

    /// What to draw now.
    pub fn appearance(&self) -> Appearance {
        self.appearance
    }

    /// The palette to draw with.
    pub fn tokens(&self) -> Tokens {
        Tokens::for_appearance(self.appearance)
    }

    /// Re-read Windows if System is selected and the interval has elapsed.
    ///
    /// Returns whether the appearance changed, so the caller can repaint only
    /// when it must. Call once per frame; it is cheap when it does nothing.
    pub fn poll(&mut self) -> bool {
        if self.mode != ThemeMode::System {
            return false;
        }
        let due = self
            .last_polled
            .is_none_or(|at| at.elapsed() >= ot_platform::system_theme::POLL_INTERVAL);
        if !due {
            return false;
        }

        self.adopt(system_theme())
    }

    fn resolve_now(&mut self) -> Appearance {
        if self.mode == ThemeMode::System {
            self.last_polled = Some(Instant::now());
        }
        resolve(self.mode, system_theme())
    }

    /// Take a reading and adopt it, reporting whether the appearance changed.
    ///
    /// Split out from `poll` so the System path can be driven from a test
    /// without changing the machine's own Windows setting.
    fn adopt(&mut self, reading: Option<SystemTheme>) -> bool {
        self.last_polled = Some(Instant::now());
        let resolved = resolve(self.mode, reading);
        let changed = resolved != self.appearance;
        self.appearance = resolved;
        changed
    }

    /// Apply the palette to an egui context.
    pub fn apply(&self, ctx: &egui::Context) {
        ctx.set_visuals(self.visuals());
    }

    /// The egui visuals for the current appearance.
    pub fn visuals(&self) -> egui::Visuals {
        let t = self.tokens();
        let mut visuals = match self.appearance {
            Appearance::Dark => egui::Visuals::dark(),
            Appearance::Light => egui::Visuals::light(),
        };

        visuals.panel_fill = t.surface;
        visuals.window_fill = t.surface;
        visuals.extreme_bg_color = t.surface_raised;
        visuals.hyperlink_color = t.accent;
        visuals.error_fg_color = t.danger;
        visuals.selection.bg_fill = t.accent.linear_multiply(0.35);
        visuals.selection.stroke = egui::Stroke::new(1.0_f32, t.accent);

        for widget in [
            &mut visuals.widgets.noninteractive,
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
            &mut visuals.widgets.open,
        ] {
            // Hover and press change the fill or the border, never the size.
            // egui grows widgets on hover by default, and a control that
            // moves under the pointer is the thing `.claude/rules/ui.md`
            // forbids outright.
            widget.expansion = 0.0;
            widget.bg_stroke = egui::Stroke::new(1.0_f32, t.border);
            widget.fg_stroke = egui::Stroke::new(1.0_f32, t.text);
        }
        visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0_f32, t.text_dim);
        visuals.widgets.noninteractive.bg_fill = t.surface;
        visuals.widgets.inactive.bg_fill = t.surface_raised;
        visuals.widgets.hovered.bg_fill = t.surface_raised;
        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, t.accent);
        visuals.widgets.active.bg_fill = t.accent.linear_multiply(0.25);
        visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0_f32, t.accent);

        visuals
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_is_the_default() {
        // The product is designed dark-first (docs/04-ui-design.md).
        assert_eq!(ThemeMode::default(), ThemeMode::Dark);
        assert_eq!(Theme::new(ThemeMode::Dark).appearance(), Appearance::Dark);
    }

    #[test]
    fn the_explicit_modes_ignore_windows() {
        // Only System should ever consult the OS; Dark and Light are choices.
        assert_eq!(Theme::new(ThemeMode::Dark).appearance(), Appearance::Dark);
        assert_eq!(Theme::new(ThemeMode::Light).appearance(), Appearance::Light);
    }

    #[test]
    fn system_resolves_to_whatever_windows_says_or_falls_back_to_dark() {
        let theme = Theme::new(ThemeMode::System);
        let expected = match system_theme() {
            Some(SystemTheme::Light) => Appearance::Light,
            Some(SystemTheme::Dark) | None => Appearance::Dark,
        };
        assert_eq!(theme.appearance(), expected);
    }

    #[test]
    fn only_system_mode_polls() {
        // Polling in Dark or Light would be a registry read per frame for a
        // value that cannot change the answer.
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            let mut theme = Theme::new(mode);
            assert!(!theme.poll(), "{mode:?} should never poll");
        }
    }

    #[test]
    fn a_fresh_system_poll_is_not_due_immediately() {
        // new() resolves once; polling again straight away must not re-read,
        // or an immediate-mode UI would hit the registry every frame.
        let mut theme = Theme::new(ThemeMode::System);
        assert!(
            !theme.poll(),
            "the interval had not elapsed, so this should have done nothing"
        );
    }

    #[test]
    fn system_follows_a_change_in_the_detected_value() {
        // The behaviour that matters and that a live read cannot assert: the
        // Windows setting changes under the app and System follows it,
        // reporting the change so the caller repaints.
        let mut theme = Theme::new(ThemeMode::System);

        // Starting appearance depends on this machine, so step to a known one
        // first rather than asserting the change on that first reading.
        theme.adopt(Some(SystemTheme::Light));
        assert_eq!(theme.appearance(), Appearance::Light);
        assert!(
            !theme.adopt(Some(SystemTheme::Light)),
            "the same reading twice is not a change"
        );

        assert!(theme.adopt(Some(SystemTheme::Dark)), "dark is a change");
        assert_eq!(theme.appearance(), Appearance::Dark);

        // The value disappearing falls back to the default, not to light.
        theme.adopt(Some(SystemTheme::Light));
        theme.adopt(None);
        assert_eq!(theme.appearance(), Appearance::Dark);
    }

    #[test]
    fn an_explicit_mode_ignores_the_reading_entirely() {
        for (mode, expected) in [
            (ThemeMode::Dark, Appearance::Dark),
            (ThemeMode::Light, Appearance::Light),
        ] {
            for reading in [Some(SystemTheme::Light), Some(SystemTheme::Dark), None] {
                assert_eq!(resolve(mode, reading), expected, "{mode:?} / {reading:?}");
            }
        }
    }

    #[test]
    fn a_due_poll_re_reads_while_one_inside_the_interval_does_not() {
        let mut theme = Theme::new(ThemeMode::System);
        assert!(!theme.poll(), "inside the interval, nothing should happen");

        // Age the last reading past the interval and prove the gate opens:
        // the appearance afterwards is whatever Windows says now.
        theme.last_polled = Instant::now().checked_sub(ot_platform::system_theme::POLL_INTERVAL);
        theme.poll();
        assert_eq!(
            theme.appearance(),
            resolve(ThemeMode::System, system_theme())
        );
        assert!(
            !theme.poll(),
            "the poll should have reset the clock, so the next one is early"
        );
    }

    #[test]
    fn changing_mode_takes_effect_at_once() {
        // Waiting out the five-second interval before the UI changed would
        // look broken.
        let mut theme = Theme::new(ThemeMode::Dark);
        theme.set_mode(ThemeMode::Light);
        assert_eq!(theme.mode(), ThemeMode::Light);
        assert_eq!(theme.appearance(), Appearance::Light);
    }

    #[test]
    fn the_stored_value_round_trips_and_a_broken_one_falls_back() {
        for mode in ThemeMode::all() {
            assert_eq!(ThemeMode::parse(mode.as_str()), mode);
        }
        assert_eq!(ThemeMode::parse(" SYSTEM "), ThemeMode::System);
        // A hand-edited settings.toml must not stop the app starting.
        assert_eq!(ThemeMode::parse("purple"), ThemeMode::Dark);
        assert_eq!(ThemeMode::parse(""), ThemeMode::Dark);
    }

    #[test]
    fn every_token_differs_between_the_palettes() {
        // A token identical in both is a sign one was forgotten when the
        // other was written.
        let d = Tokens::DARK;
        let l = Tokens::LIGHT;
        let pairs = [
            ("surface", d.surface, l.surface),
            ("surface_raised", d.surface_raised, l.surface_raised),
            ("text", d.text, l.text),
            ("text_dim", d.text_dim, l.text_dim),
            ("accent", d.accent, l.accent),
            ("border", d.border, l.border),
            ("danger", d.danger, l.danger),
        ];
        for (name, dark, light) in pairs {
            assert_ne!(dark, light, "{name} is the same in both palettes");
        }
    }

    #[test]
    fn text_is_readable_against_its_surface_in_both_palettes() {
        // Not a full contrast-ratio check, but enough to catch the mistake
        // that matters: light text on a light surface, or the reverse.
        fn luma(c: Color32) -> f32 {
            0.2126 * f32::from(c.r()) + 0.7152 * f32::from(c.g()) + 0.0722 * f32::from(c.b())
        }
        for (name, t) in [("dark", Tokens::DARK), ("light", Tokens::LIGHT)] {
            let gap = (luma(t.text) - luma(t.surface)).abs();
            assert!(
                gap > 100.0,
                "{name}: text and surface are too close ({gap:.0})"
            );
            let dim_gap = (luma(t.text_dim) - luma(t.surface)).abs();
            assert!(
                dim_gap > 40.0,
                "{name}: dim text is nearly invisible ({dim_gap:.0})"
            );
        }
    }

    #[test]
    fn hover_never_grows_a_widget() {
        // egui expands widgets on hover by default. A control that moves
        // under the pointer is forbidden outright (.claude/rules/ui.md), and
        // it is the kind of thing only a human notices, so assert it.
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            let visuals = Theme::new(mode).visuals();
            for (name, w) in [
                ("noninteractive", &visuals.widgets.noninteractive),
                ("inactive", &visuals.widgets.inactive),
                ("hovered", &visuals.widgets.hovered),
                ("active", &visuals.widgets.active),
                ("open", &visuals.widgets.open),
            ] {
                assert_eq!(w.expansion, 0.0, "{mode:?}/{name} expands on interaction");
            }
        }
    }

    #[test]
    fn the_visuals_use_the_tokens_rather_than_eguis_defaults() {
        let theme = Theme::new(ThemeMode::Dark);
        let visuals = theme.visuals();
        assert_eq!(visuals.panel_fill, Tokens::DARK.surface);
        assert_eq!(visuals.error_fg_color, Tokens::DARK.danger);
        assert_eq!(visuals.hyperlink_color, Tokens::DARK.accent);
    }
}
