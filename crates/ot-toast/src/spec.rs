//! `ToastSpec`: the only thing the renderer draws (`docs/specs/toast.md` §3).
//!
//! A spec is built from a reminder and its category template by one function,
//! which both the Settings preview and the scheduler call — so the preview and
//! a real toast cannot drift apart. Anything that renders without going
//! through a `ToastSpec` is a defect.
//!
//! **The limits in §4's table are enforced here, not in the UI.** The model is
//! what the renderer trusts, and a settings file edited by hand to width 20 is
//! as real an input as a slider. Every clamp logs what it changed, naming the
//! field and both values, because a value that silently becomes a different
//! value is worse than one that is refused.

use serde::{Deserialize, Serialize};

/// Where a toast can be drawn, in pixels. Narrower than this and the text has
/// nowhere to go; wider and it stops reading as a notification.
pub const WIDTH: Range = Range::new(280.0, 600.0, 360.0);
/// A fixed height, when the user sets one rather than letting it size to its
/// content.
pub const HEIGHT: Range = Range::new(80.0, 300.0, 120.0);
/// Corner radius. 0 is a rectangle; the top of the range is where a 120 px
/// toast becomes a pill on its own.
pub const RADIUS: Range = Range::new(0.0, 40.0, 12.0);
/// Opacity as a percentage. The floor is not 0: a toast nobody can read is
/// not a toast, and someone who wants silence has Pause.
/// 70% at the lowest (CR-0005): opacity fades the whole toast, text
/// included, so below that the text competes with whatever is behind it.
pub const OPACITY: Range = Range::new(70.0, 100.0, 100.0);
/// Border width.
pub const BORDER: Range = Range::new(0.0, 4.0, 0.0);
/// How long a toast stays, in seconds.
pub const DURATION_SECS: Range = Range::new(3.0, 60.0, 8.0);
/// A persistent toast is capped rather than endless: ten minutes of something
/// on top of everything else is already a lot.
pub const PERSISTENT_CAP_SECS: f32 = 600.0;
/// How many toasts are on screen at once before the rest queue.
pub const MAX_VISIBLE: IntRange = IntRange::new(1, 5, 3);
/// Buttons on a toast.
pub const ACTIONS: IntRange = IntRange::new(1, 3, 2);
/// Between one toast and the next in a stack.
pub const STACK_GAP: Range = Range::new(0.0, 40.0, 8.0);
/// Animation duration, in milliseconds.
pub const ANIMATION_MS: IntRange = IntRange::new(150, 1000, 350);

/// A floating-point limit with a default, so every clamp reads the same way.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Range {
    pub min: f32,
    pub max: f32,
    pub default: f32,
}

impl Range {
    pub const fn new(min: f32, max: f32, default: f32) -> Self {
        Self { min, max, default }
    }

    /// Bring `value` inside the range, saying so when it had to move.
    ///
    /// A NaN is replaced by the default rather than clamped: `f32::clamp`
    /// panics on a NaN bound and silently keeps a NaN value, and a NaN width
    /// reaches the renderer as a toast of no size at all.
    pub fn clamp(self, value: f32, field: &'static str) -> f32 {
        if !value.is_finite() {
            tracing::warn!(field, %value, replacement = self.default, "a non-finite value was replaced by the default");
            return self.default;
        }
        let clamped = value.clamp(self.min, self.max);
        if clamped != value {
            tracing::warn!(
                field,
                was = value,
                now = clamped,
                min = self.min,
                max = self.max,
                "a value outside its range was clamped"
            );
        }
        clamped
    }
}

/// The same, for the counts that are whole numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntRange {
    pub min: u32,
    pub max: u32,
    pub default: u32,
}

impl IntRange {
    pub const fn new(min: u32, max: u32, default: u32) -> Self {
        Self { min, max, default }
    }

    pub fn clamp(self, value: u32, field: &'static str) -> u32 {
        let clamped = value.clamp(self.min, self.max);
        if clamped != value {
            tracing::warn!(
                field,
                was = value,
                now = clamped,
                min = self.min,
                max = self.max,
                "a value outside its range was clamped"
            );
        }
        clamped
    }
}

/// The outline a toast is drawn with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Shape {
    Rectangle,
    #[default]
    RoundedRectangle,
    /// Fully round ends, whatever the radius says. Kept as its own shape
    /// rather than "radius 40" because it must stay round as the height
    /// changes, which a fixed radius does not (`docs/specs/toast.md` §12).
    Pill,
}

impl Shape {
    /// The corner radius this shape actually draws with.
    ///
    /// A pill's radius is half its height, so it stays a pill at any size; a
    /// rectangle's is zero whatever the setting says. Only the rounded
    /// rectangle uses the number the user chose.
    pub fn radius_for(self, requested: f32, height: f32) -> f32 {
        match self {
            Self::Rectangle => 0.0,
            Self::RoundedRectangle => RADIUS.clamp(requested, "style.radius"),
            Self::Pill => (height / 2.0).max(0.0),
        }
    }

    pub fn all() -> [Self; 3] {
        [Self::Rectangle, Self::RoundedRectangle, Self::Pill]
    }

    /// The Fluent key naming this shape in the picker.
    pub fn name_key(self) -> &'static str {
        match self {
            Self::Rectangle => "toast-shape-rectangle",
            Self::RoundedRectangle => "toast-shape-rounded",
            Self::Pill => "toast-shape-pill",
        }
    }
}

/// Whether a toast casts a shadow (CR-0006).
///
/// Windows' own shadow, which it draws only for a window without a region —
/// the Rectangle shape. A rounded rectangle or a pill is cut to its shape
/// (BUG-0007), and Windows draws no shadow for a cut window, so for those
/// this setting does nothing and the settings say so.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Shadow {
    Off,
    /// `soft` and `strong` are what a settings file held before CR-0006,
    /// and both still mean a shadow.
    #[default]
    #[serde(alias = "soft", alias = "strong")]
    On,
}

impl Shadow {
    /// Whether a toast of `shape` actually shows a shadow.
    pub fn shows(self, shape: Shape) -> bool {
        self == Self::On && shape == Shape::Rectangle
    }
}

/// An RGBA colour, stored as eight-digit hex so a hand-edited settings file
/// is readable and so no colour type leaks out of the renderer's crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Rgba(pub u32);

impl Rgba {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self(u32::from_be_bytes([r, g, b, a]))
    }

    pub const fn parts(self) -> (u8, u8, u8, u8) {
        let [r, g, b, a] = self.0.to_be_bytes();
        (r, g, b, a)
    }
}

/// What the toast says.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Content {
    pub title: String,
    pub body: String,
    /// The app's own name in the header. Localized by the caller, like
    /// everything else here: a spec holds text, never keys.
    pub app_name: String,
    /// The small picture in the **header**, beside the title. Each
    /// notification may carry its own; `None` draws the app's own icon.
    /// A relative path under `OmniToasterData/assets/`, never an absolute
    /// one, so a portable folder can move between machines
    /// (`docs/05-data-model-and-storage.md`).
    pub icon: Option<String>,
    /// The larger picture **inside the content**, beside the body text.
    /// Separate from the icon: a prayer toast can carry a mosque in its body
    /// and the app's mark in its header. Relative, like the icon.
    pub image: Option<String>,
    /// Whether the image is framed. It travels with the image rather than
    /// being a global style, because the right answer differs per picture: a
    /// photo usually wants a border to stand off the background, and a logo
    /// on a transparent background usually does not. Off by default.
    pub image_border: bool,
    /// This notification's own accent colour, overriding the one in
    /// Settings ▸ Appearance. `None` uses that one. Per notification, like
    /// the icon and the image: a prayer and a reminder are told apart at a
    /// glance by their colour (`docs/specs/toast.md` §8).
    pub accent: Option<Rgba>,
    /// Whether the header shows an icon at all.
    pub show_icon: bool,
    /// Already formatted by the caller, like every other string here: a spec
    /// holds text, never keys, and `ot-i18n` owns the one formatter that may
    /// turn a time into text (`docs/10-i18n.md`).
    pub timestamp: Option<String>,
    /// Emphasis for the body, as a whole rather than per segment. A markup
    /// language inside a notification body is a thing to add when something
    /// asks for it, not before.
    pub body_bold: bool,
    pub body_italic: bool,
}

/// What it looks like.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Style {
    pub shape: Shape,
    pub radius: f32,
    pub width: f32,
    /// `None` means size to the content, which is the default.
    pub height: Option<f32>,
    pub background: Rgba,
    /// A second colour makes the background a gradient; `None` is solid.
    pub background_to: Option<Rgba>,
    pub opacity: f32,
    pub border: f32,
    pub border_colour: Rgba,
    pub shadow: Shadow,
    /// The one colour that marks this toast's category.
    pub accent: Rgba,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            shape: Shape::default(),
            radius: RADIUS.default,
            width: WIDTH.default,
            height: None,
            // Dark is the product's default, so the default toast is dark
            // (`docs/04-ui-design.md`).
            background: Rgba::new(0x26, 0x26, 0x2c, 0xff),
            background_to: None,
            opacity: OPACITY.default,
            border: BORDER.default,
            border_colour: Rgba::new(0x3a, 0x3a, 0x42, 0xff),
            shadow: Shadow::default(),
            accent: Rgba::new(0x3b, 0x9e, 0xff, 0xff),
        }
    }
}

/// How long it stays and where.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Behaviour {
    pub duration_secs: f32,
    /// Where toasts appear: one of the nine presets, or a custom point
    /// (`docs/specs/toast.md` §7). A global setting, carried on every spec
    /// so the renderer needs nothing but the spec.
    pub position: crate::placement::Position,
    /// Between stacked toasts.
    pub stack_gap: f32,
    /// A persistent toast waits to be acknowledged, up to
    /// [`PERSISTENT_CAP_SECS`].
    pub persistent: bool,
    pub max_visible: u32,
    pub actions: u32,
}

impl Default for Behaviour {
    fn default() -> Self {
        Self {
            duration_secs: DURATION_SECS.default,
            position: crate::placement::Position::default(),
            stack_gap: STACK_GAP.default,
            persistent: false,
            max_visible: MAX_VISIBLE.default,
            actions: ACTIONS.default,
        }
    }
}

/// Everything the renderer needs, and nothing it does not.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ToastSpec {
    pub content: Content,
    pub style: Style,
    pub behaviour: Behaviour,
}

impl ToastSpec {
    /// Bring every field inside its documented range.
    ///
    /// Called after building a spec from anywhere — a template, a settings
    /// file, a preview pane — so the renderer never has to ask whether its
    /// input is sane. Returns the spec so it reads as part of construction.
    #[must_use]
    pub fn clamped(mut self) -> Self {
        self.style.width = WIDTH.clamp(self.style.width, "style.width");
        self.style.height = self
            .style
            .height
            .map(|height| HEIGHT.clamp(height, "style.height"));
        self.style.radius = RADIUS.clamp(self.style.radius, "style.radius");
        self.style.opacity = OPACITY.clamp(self.style.opacity, "style.opacity");
        self.style.border = BORDER.clamp(self.style.border, "style.border");

        self.behaviour.duration_secs = if self.behaviour.persistent {
            // A persistent toast is capped, not endless — but it may outlast
            // the ordinary ceiling, which is the point of the flag.
            Range::new(
                DURATION_SECS.min,
                PERSISTENT_CAP_SECS,
                DURATION_SECS.default,
            )
            .clamp(self.behaviour.duration_secs, "behaviour.duration_secs")
        } else {
            DURATION_SECS.clamp(self.behaviour.duration_secs, "behaviour.duration_secs")
        };
        self.behaviour.max_visible =
            MAX_VISIBLE.clamp(self.behaviour.max_visible, "behaviour.max_visible");
        self.behaviour.actions = ACTIONS.clamp(self.behaviour.actions, "behaviour.actions");
        self.behaviour.stack_gap = STACK_GAP.clamp(self.behaviour.stack_gap, "behaviour.stack_gap");
        self
    }

    /// The radius this spec's shape actually draws with, at `height`.
    pub fn drawn_radius(&self, height: f32) -> f32 {
        self.style.shape.radius_for(self.style.radius, height)
    }

    /// The accent this toast draws with: its own, or the default.
    pub fn accent(&self) -> Rgba {
        self.content.accent.unwrap_or(self.style.accent)
    }

    /// Opacity as the 0.0–1.0 the renderer wants.
    pub fn alpha(&self) -> f32 {
        self.style.opacity / 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_the_ones_the_spec_table_names() {
        let spec = ToastSpec::default();
        assert_eq!(spec.style.width, 360.0, "docs/specs/toast.md §4");
        assert_eq!(spec.behaviour.duration_secs, 8.0);
        assert_eq!(spec.style.opacity, 100.0);
        assert_eq!(spec.style.shape, Shape::RoundedRectangle);
        // A spec with nothing set must already be a usable toast.
        assert_eq!(spec.clone().clamped(), spec);
    }

    #[test]
    fn every_limit_holds_at_both_ends() {
        let mut spec = ToastSpec::default();
        spec.style.width = 20.0;
        spec.style.height = Some(5_000.0);
        spec.style.radius = -3.0;
        spec.style.opacity = 0.0;
        spec.style.border = 99.0;
        spec.behaviour.duration_secs = 0.5;
        spec.behaviour.max_visible = 50;
        spec.behaviour.actions = 0;

        let spec = spec.clamped();
        assert_eq!(spec.style.width, WIDTH.min, "a hand-edited width 20");
        assert_eq!(spec.style.height, Some(HEIGHT.max));
        assert_eq!(spec.style.radius, RADIUS.min);
        assert_eq!(spec.style.opacity, OPACITY.min);
        assert_eq!(spec.style.border, BORDER.max);
        assert_eq!(spec.behaviour.duration_secs, DURATION_SECS.min);
        assert_eq!(spec.behaviour.max_visible, MAX_VISIBLE.max);
        assert_eq!(spec.behaviour.actions, ACTIONS.min);

        // And the other end.
        let mut spec = ToastSpec::default();
        spec.style.width = 10_000.0;
        spec.behaviour.duration_secs = 10_000.0;
        let spec = spec.clamped();
        assert_eq!(spec.style.width, WIDTH.max);
        assert_eq!(spec.behaviour.duration_secs, DURATION_SECS.max);
    }

    #[test]
    fn a_persistent_toast_is_capped_rather_than_endless() {
        let mut spec = ToastSpec::default();
        spec.behaviour.persistent = true;
        spec.behaviour.duration_secs = 60.0 * 60.0;
        let spec = spec.clamped();
        assert_eq!(
            spec.behaviour.duration_secs, PERSISTENT_CAP_SECS,
            "ten minutes is the cap, not an hour"
        );

        // A persistent toast may still outlast the ordinary 60-second ceiling,
        // which is the whole point of the flag.
        let mut spec = ToastSpec::default();
        spec.behaviour.persistent = true;
        spec.behaviour.duration_secs = 120.0;
        assert_eq!(spec.clamped().behaviour.duration_secs, 120.0);
    }

    #[test]
    fn a_non_finite_value_becomes_the_default_rather_than_a_toast_of_no_size() {
        // `f32::clamp` keeps a NaN and panics on a NaN bound, so a NaN width
        // would reach the renderer intact.
        let mut spec = ToastSpec::default();
        spec.style.width = f32::NAN;
        spec.style.opacity = f32::INFINITY;
        let spec = spec.clamped();
        assert_eq!(spec.style.width, WIDTH.default);
        assert!(spec.style.width.is_finite());
        assert_eq!(spec.style.opacity, OPACITY.default);
    }

    #[test]
    fn a_pill_stays_a_pill_at_any_height_and_a_rectangle_stays_square() {
        // This is the spec's first open question answered: a pill is not
        // radius 40 under another name, because 40 stops being half the
        // height as soon as the toast grows.
        for height in [80.0_f32, 120.0, 300.0] {
            let radius = Shape::Pill.radius_for(0.0, height);
            assert_eq!(radius, height / 2.0, "a pill at {height}px");
            assert!(
                radius > RADIUS.max || height <= RADIUS.max * 2.0,
                "a {height}px pill needs r={radius}, past the slider's own ceiling"
            );
        }
        assert_eq!(Shape::Rectangle.radius_for(40.0, 120.0), 0.0);
        assert_eq!(Shape::RoundedRectangle.radius_for(12.0, 120.0), 12.0);
        // The rounded rectangle is the only one that obeys the slider, and it
        // obeys the slider's limits too.
        assert_eq!(Shape::RoundedRectangle.radius_for(999.0, 120.0), RADIUS.max);
    }

    #[test]
    fn a_spec_round_trips_through_toml_keeping_what_it_was_given() {
        let mut spec = ToastSpec::default();
        spec.content.title = "Solat Zohor".to_string();
        spec.content.body = "Waktu Zohor telah masuk.".to_string();
        spec.style.shape = Shape::Pill;
        spec.style.width = 420.0;
        spec.style.background_to = Some(Rgba::new(0x12, 0x34, 0x56, 0xff));
        spec.behaviour.persistent = true;

        let text = toml::to_string_pretty(&spec).expect("a spec encodes");
        let back: ToastSpec = toml::from_str(&text).expect("and decodes");
        assert_eq!(back, spec);
    }

    #[test]
    fn a_hand_edited_file_loads_clamped_rather_than_refused() {
        // The settings file is the user's to edit, so an out-of-range value is
        // brought into range rather than stopping the app
        // (`docs/specs/toast.md` §4).
        let text = r#"
[style]
width = 20.0
opacity = 1.0

[behaviour]
duration_secs = 900.0
"#;
        let spec: ToastSpec = toml::from_str(text).expect("it parses");
        let spec = spec.clamped();
        assert_eq!(spec.style.width, WIDTH.min);
        assert_eq!(spec.style.opacity, OPACITY.min);
        assert_eq!(spec.behaviour.duration_secs, DURATION_SECS.max);
    }

    #[test]
    fn an_unknown_key_is_refused_rather_than_ignored() {
        let text = "[style]\nshpe = \"pill\"\n";
        assert!(
            toml::from_str::<ToastSpec>(text).is_err(),
            "a typo'd key must not load as a default"
        );
    }

    #[test]
    fn a_colour_survives_being_stored_as_a_number() {
        let colour = Rgba::new(0x3b, 0x9e, 0xff, 0x80);
        assert_eq!(colour.parts(), (0x3b, 0x9e, 0xff, 0x80));
        let text = toml::to_string(&ToastSpec {
            style: Style {
                accent: colour,
                ..Style::default()
            },
            ..ToastSpec::default()
        })
        .expect("encodes");
        let back: ToastSpec = toml::from_str(&text).expect("decodes");
        assert_eq!(back.style.accent, colour);
    }

    #[test]
    fn a_notifications_own_accent_overrides_the_default_and_absent_falls_back() {
        let mut spec = ToastSpec::default();
        assert_eq!(spec.accent(), spec.style.accent, "no override: the default");
        let prayer_green = Rgba::new(0x1f, 0x9d, 0x55, 0xff);
        spec.content.accent = Some(prayer_green);
        assert_eq!(spec.accent(), prayer_green, "the notification's own wins");
        assert_ne!(spec.accent(), spec.style.accent);
    }

    #[test]
    fn opacity_stops_at_seventy_percent_so_text_stays_readable() {
        // CR-0005: the whole toast fades, text included. A settings file
        // edited down to the old floor of 40% is clamped to the new one.
        let mut spec = ToastSpec::default();
        spec.style.opacity = 40.0;
        assert_eq!(spec.clamped().style.opacity, 70.0);
    }

    #[test]
    fn alpha_is_the_fraction_the_renderer_wants() {
        let mut spec = ToastSpec::default();
        assert_eq!(spec.alpha(), 1.0);
        spec.style.opacity = 40.0;
        assert_eq!(spec.alpha(), 0.4);
    }

    #[test]
    fn a_shadow_shows_only_on_a_rectangle_and_old_settings_still_read() {
        assert!(Shadow::On.shows(Shape::Rectangle));
        assert!(!Shadow::On.shows(Shape::RoundedRectangle));
        assert!(!Shadow::On.shows(Shape::Pill));
        assert!(!Shadow::Off.shows(Shape::Rectangle));

        // Before CR-0006 a settings file said soft or strong.
        for old in ["soft", "strong"] {
            let table: toml::Table = toml::from_str(&format!("shadow = '{old}'")).expect("toml");
            let read: Shadow = table["shadow"].clone().try_into().expect(old);
            assert_eq!(read, Shadow::On, "{old}");
        }
    }

    #[test]
    fn every_shape_names_a_key_and_none_share_one() {
        let keys: Vec<&str> = Shape::all().iter().map(|s| s.name_key()).collect();
        let mut unique = keys.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(
            unique.len(),
            keys.len(),
            "two options share a key: {keys:?}"
        );
    }
}
