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
/// The height of a toast that sizes to its content, before its action row.
pub const AUTO_HEIGHT: f32 = 120.0;
/// The action row and the gap above it, which an auto-height toast grows by.
pub const ACTION_ROW: f32 = 32.0;
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
    /// Whether a toast of `shape` and `glass` actually shows a shadow.
    ///
    /// Glass turns it off: Windows' shadow needs non-client rendering, which
    /// leaves the client area opaque, and glass needs it see-through
    /// (ADR-016).
    pub fn shows(self, shape: Shape, glass: Glass) -> bool {
        self == Self::On && shape == Shape::Rectangle && glass == Glass::None
    }
}

/// The desktop blurred behind the toast (`docs/specs/toast.md` §4.2).
///
/// All three kinds are the one blur Windows draws for an inactive window —
/// the accent policy's Acrylic (ADR-016) — and differ only in how much of the
/// toast's background colour lies over it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Glass {
    /// The solid background, as in Phase 2.
    #[default]
    None,
    Blur,
    Acrylic,
    MicaLike,
}

impl Glass {
    pub fn all() -> [Self; 4] {
        [Self::None, Self::Blur, Self::Acrylic, Self::MicaLike]
    }

    /// The glass a toast of `shape` actually gets: Rectangle only.
    ///
    /// Windows draws the blur over the whole window rectangle and ignores the
    /// region that cuts a rounded toast's corners, so a rounded rectangle or a
    /// pill with glass would show blurred square corners past its edge. The
    /// owner chose Rectangle-only on 2026-11-18, like the shadow; the setting
    /// is kept, and does nothing on the other shapes.
    pub fn on(self, shape: Shape) -> Self {
        if shape == Shape::Rectangle {
            self
        } else {
            Self::None
        }
    }

    /// How much of the background colour lies over the blur, 0.0–1.0, or
    /// `None` for a solid toast.
    pub fn tint(self) -> Option<f32> {
        match self {
            Self::None => None,
            // The lightest of the three. Not "almost none", as first
            // specified: at 10% the text, chosen for the toast's colour,
            // vanished over a desktop of the opposite lightness, so the owner
            // set 45% on 2026-11-18.
            Self::Blur => Some(0.45),
            Self::Acrylic => Some(0.6),
            // Mostly opaque: the desktop shows only as a hint of colour.
            Self::MicaLike => Some(0.88),
        }
    }

    /// The Fluent key naming this kind in the picker.
    pub fn name_key(self) -> &'static str {
        match self {
            Self::None => "toast-glass-none",
            Self::Blur => "toast-glass-blur",
            Self::Acrylic => "toast-glass-acrylic",
            Self::MicaLike => "toast-glass-mica-like",
        }
    }
}

/// An unknown kind loads as `None` with a warning, rather than throwing away
/// the whole `[toast]` table over one word (`docs/specs/toast.md` §4.2).
impl<'de> Deserialize<'de> for Glass {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        Ok(match name.as_str() {
            "none" => Self::None,
            "blur" => Self::Blur,
            "acrylic" => Self::Acrylic,
            "mica-like" => Self::MicaLike,
            other => {
                tracing::warn!(value = other, "an unknown glass kind was read as none");
                Self::None
            }
        })
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

/// One button in the action row (`docs/specs/toast.md` §4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Open,
    Snooze,
    Dismiss,
}

impl Action {
    /// The buttons for the actions setting, left to right: Dismiss is always
    /// there and always last.
    pub fn for_count(count: u32) -> &'static [Self] {
        match count {
            0 | 1 => &[Self::Dismiss],
            2 => &[Self::Snooze, Self::Dismiss],
            _ => &[Self::Open, Self::Snooze, Self::Dismiss],
        }
    }

    /// Whether pressing it does anything yet. Snooze and Open act on a
    /// reminder, and reminders arrive in Phase 4; until then they are drawn
    /// greyed (the owner's choice, 2026-11-18).
    pub fn works(self) -> bool {
        self == Self::Dismiss
    }
}

/// The buttons' labels, already in the user's language: a spec holds text,
/// never keys, like every other string in it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ActionLabels {
    pub open: String,
    pub snooze: String,
    pub dismiss: String,
}

impl ActionLabels {
    pub fn get(&self, action: Action) -> &str {
        match action {
            Action::Open => &self.open,
            Action::Snooze => &self.snooze,
            Action::Dismiss => &self.dismiss,
        }
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
    /// The action row's labels; which buttons show is the actions setting
    /// (`Behaviour::actions`).
    pub action_labels: ActionLabels,
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
    pub glass: Glass,
    /// The one colour that marks this toast's category.
    pub accent: Rgba,
}

impl Style {
    /// The toast's height in points: the fixed one, or the content's own
    /// with the action row added, so the row never costs the body a line
    /// (`docs/specs/toast.md` §4.3). The window, the stack, the preview and
    /// the placement map all ask here.
    pub fn toast_height(&self) -> f32 {
        self.height.unwrap_or(AUTO_HEIGHT + ACTION_ROW)
    }
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
            glass: Glass::default(),
            accent: Rgba::new(0x3b, 0x9e, 0xff, 0xff),
        }
    }
}

/// How a toast arrives and leaves (`docs/specs/toast.md` §5).
///
/// The out list is the in list plus "shrink to tray", Genie's own way out.
/// Until each effect is built (FEAT-0032 to FEAT-0034) it plays as a fade,
/// which is also what reduced motion and a slow frame fall back to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Effect {
    /// A fade: the alpha ramp every other effect falls back to.
    #[default]
    Appear,
    SlideFromRight,
    SlideFromBottom,
    GenieFromBottom,
    PopOut,
    Drawer,
    Dissolve,
    Glow,
    AlphaToSolid,
    /// Out only: Genie's mirror, towards the tray icon.
    ShrinkToTray,
}

impl Effect {
    /// Every effect a toast may arrive with.
    pub fn all_in() -> [Self; 9] {
        [
            Self::SlideFromRight,
            Self::SlideFromBottom,
            Self::GenieFromBottom,
            Self::PopOut,
            Self::Drawer,
            Self::Appear,
            Self::Dissolve,
            Self::Glow,
            Self::AlphaToSolid,
        ]
    }

    /// Every effect a toast may leave with: the same, plus shrink to tray.
    pub fn all_out() -> [Self; 10] {
        let [a, b, c, d, e, f, g, h, i] = Self::all_in();
        [a, b, c, d, e, f, g, h, i, Self::ShrinkToTray]
    }

    /// The effect `settings.toml` spells `name`, kebab-case.
    fn from_name(name: &str) -> Option<Self> {
        Self::all_out().into_iter().find(|effect| {
            toml::Value::try_from(effect).is_ok_and(|value| value.as_str() == Some(name))
        })
    }
}

/// An unknown effect loads as Appear with a warning, rather than throwing
/// away the whole `[toast]` table over one word.
impl<'de> Deserialize<'de> for Effect {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        Ok(Self::from_name(&name).unwrap_or_else(|| {
            tracing::warn!(value = %name, "an unknown animation was read as appear");
            Self::Appear
        }))
    }
}

/// How an animation's progress is shaped over its duration (§5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Easing {
    Linear,
    /// Cubic: fast at first, settling at the end.
    #[default]
    EaseOut,
    /// A damped overshoot that settles within the duration.
    Spring,
}

impl Easing {
    /// The shaped progress for `t`, 0.0–1.0. Exactly 0 at 0 and exactly 1
    /// at 1; a spring may pass 1 on the way, which is right for motion and
    /// scale.
    pub fn apply(self, t: f32) -> f32 {
        let t = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            1.0
        };
        match self {
            Self::Linear => t,
            Self::EaseOut => 1.0 - (1.0 - t).powi(3),
            Self::Spring if t >= 1.0 => 1.0,
            // Decays to within a quarter of a percent by the end, and peaks
            // about 14% past the target a third of the way in.
            Self::Spring => 1.0 - (-6.0 * t).exp() * (3.0 * std::f32::consts::PI * t).cos(),
        }
    }

    /// The same, for an alpha, which never overshoots past solid (§5).
    pub fn apply_alpha(self, t: f32) -> f32 {
        self.apply(t).clamp(0.0, 1.0)
    }
}

/// An unknown easing loads as ease-out with a warning.
impl<'de> Deserialize<'de> for Easing {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        Ok(match name.as_str() {
            "linear" => Self::Linear,
            "ease-out" => Self::EaseOut,
            "spring" => Self::Spring,
            other => {
                tracing::warn!(value = other, "an unknown easing was read as ease-out");
                Self::EaseOut
            }
        })
    }
}

/// How the toast arrives and leaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Animation {
    pub enter: Effect,
    pub leave: Effect,
    /// The length of each of the two, in milliseconds: 150–1000.
    pub duration_ms: u32,
    pub easing: Easing,
    /// Play every effect as a fade, whatever Windows says. Windows' own
    /// "Show animations" being off does the same on its own; this is the
    /// way to ask for it here (`docs/specs/toast.md` §5).
    pub reduce_motion: bool,
}

impl Default for Animation {
    fn default() -> Self {
        Self {
            enter: Effect::default(),
            leave: Effect::default(),
            duration_ms: ANIMATION_MS.default,
            easing: Easing::default(),
            reduce_motion: false,
        }
    }
}

impl Animation {
    /// The same, with every effect a fade: what reduced motion and the
    /// frame-time fallback play.
    #[must_use]
    pub fn as_fade(self) -> Self {
        Self {
            enter: Effect::Appear,
            leave: Effect::Appear,
            ..self
        }
    }

    pub fn duration(&self) -> std::time::Duration {
        std::time::Duration::from_millis(u64::from(self.duration_ms))
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
    pub animation: Animation,
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
        self.animation.duration_ms =
            ANIMATION_MS.clamp(self.animation.duration_ms, "animation.duration_ms");
        // Shrink to tray is Genie's way out; nothing arrives from the tray.
        if self.animation.enter == Effect::ShrinkToTray {
            tracing::warn!(
                "shrink to tray is an out-animation only; the toast arrives with appear"
            );
            self.animation.enter = Effect::Appear;
        }

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

    /// The buttons in this toast's action row, left to right.
    pub fn actions(&self) -> &'static [Action] {
        Action::for_count(self.behaviour.actions)
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
        assert!(Shadow::On.shows(Shape::Rectangle, Glass::None));
        assert!(!Shadow::On.shows(Shape::RoundedRectangle, Glass::None));
        assert!(!Shadow::On.shows(Shape::Pill, Glass::None));
        assert!(!Shadow::Off.shows(Shape::Rectangle, Glass::None));

        // Before CR-0006 a settings file said soft or strong.
        for old in ["soft", "strong"] {
            let table: toml::Table = toml::from_str(&format!("shadow = '{old}'")).expect("toml");
            let read: Shadow = table["shadow"].clone().try_into().expect(old);
            assert_eq!(read, Shadow::On, "{old}");
        }
    }

    #[test]
    fn glass_turns_the_shadow_off() {
        // ADR-016: the shadow keeps the client area opaque, glass needs it
        // see-through.
        for glass in [Glass::Blur, Glass::Acrylic, Glass::MicaLike] {
            assert!(!Shadow::On.shows(Shape::Rectangle, glass), "{glass:?}");
        }
    }

    #[test]
    fn every_easing_starts_at_zero_and_ends_at_one() {
        for easing in [Easing::Linear, Easing::EaseOut, Easing::Spring] {
            assert_eq!(easing.apply(0.0), 0.0, "{easing:?} at 0");
            assert_eq!(easing.apply(1.0), 1.0, "{easing:?} at 1");
            // Out of range and not a number: held to the ends, never NaN.
            assert_eq!(easing.apply(-1.0), 0.0);
            assert_eq!(easing.apply(2.0), 1.0);
            assert_eq!(easing.apply(f32::NAN), 1.0);
        }
        assert_eq!(Easing::Linear.apply(0.5), 0.5);
        assert_eq!(Easing::EaseOut.apply(0.5), 0.875, "1 - 0.5³");
        let spring_half = Easing::Spring.apply(0.5);
        assert!((0.9..1.1).contains(&spring_half), "{spring_half}");
    }

    #[test]
    fn a_spring_overshoots_but_an_alpha_never_passes_solid() {
        let samples = (0..=100).map(|i| i as f32 / 100.0);
        let peak = samples
            .clone()
            .map(|t| Easing::Spring.apply(t))
            .fold(0.0_f32, f32::max);
        assert!(peak > 1.05, "a spring overshoots: peak {peak}");
        assert!(
            samples
                .map(|t| Easing::Spring.apply_alpha(t))
                .all(|a| (0.0..=1.0).contains(&a)),
            "an alpha never overshoots past solid (docs/specs/toast.md §5)"
        );
        // Settled by the end: the last stretch is already within 1%.
        assert!((Easing::Spring.apply(0.95) - 1.0).abs() < 0.01);
    }

    #[test]
    fn the_animation_is_clamped_and_nothing_arrives_from_the_tray() {
        let mut spec = ToastSpec::default();
        assert_eq!(spec.animation.duration_ms, 350, "the spec's default");
        spec.animation.duration_ms = 5;
        assert_eq!(
            spec.clone().clamped().animation.duration_ms,
            ANIMATION_MS.min
        );
        spec.animation.duration_ms = 60_000;
        assert_eq!(
            spec.clone().clamped().animation.duration_ms,
            ANIMATION_MS.max
        );

        spec.animation.enter = Effect::ShrinkToTray;
        spec.animation.leave = Effect::ShrinkToTray;
        let spec = spec.clamped();
        assert_eq!(spec.animation.enter, Effect::Appear);
        assert_eq!(spec.animation.leave, Effect::ShrinkToTray, "a fine way out");
    }

    #[test]
    fn the_animation_round_trips_and_unknown_words_load_as_defaults() {
        for effect in Effect::all_out() {
            let spec = ToastSpec {
                animation: Animation {
                    leave: effect,
                    easing: Easing::Spring,
                    ..Animation::default()
                },
                ..ToastSpec::default()
            };
            let text = toml::to_string(&spec).expect("encodes");
            let back: ToastSpec = toml::from_str(&text).expect("decodes");
            assert_eq!(back.animation, spec.animation, "{effect:?}");
        }
        let text = "[animation]\nenter = \"slide-from-right\"\nleave = \"warp\"\neasing = \"bouncy\"\nduration_ms = 500\n";
        let spec: ToastSpec = toml::from_str(text).expect("unknown words still parse");
        assert_eq!(spec.animation.enter, Effect::SlideFromRight);
        assert_eq!(spec.animation.leave, Effect::Appear);
        assert_eq!(spec.animation.easing, Easing::EaseOut);
        assert_eq!(spec.animation.duration_ms, 500, "and the rest is kept");
    }

    #[test]
    fn glass_is_for_the_rectangle_only() {
        // Windows blurs the whole window rectangle, past a rounded toast's
        // cut corners, so only the Rectangle gets it.
        assert_eq!(Glass::Acrylic.on(Shape::Rectangle), Glass::Acrylic);
        assert_eq!(Glass::Acrylic.on(Shape::RoundedRectangle), Glass::None);
        assert_eq!(Glass::MicaLike.on(Shape::Pill), Glass::None);
        assert_eq!(Glass::None.on(Shape::Rectangle), Glass::None);
    }

    #[test]
    fn each_glass_kind_tints_more_than_the_last_and_none_is_solid() {
        assert_eq!(Glass::None.tint(), None);
        let tints: Vec<f32> = [Glass::Blur, Glass::Acrylic, Glass::MicaLike]
            .iter()
            .map(|glass| glass.tint().expect("a glass kind has a tint"))
            .collect();
        assert!(tints.windows(2).all(|pair| pair[0] < pair[1]), "{tints:?}");
        assert!(tints.iter().all(|tint| (0.0..1.0).contains(tint)));
    }

    #[test]
    fn glass_round_trips_and_an_unknown_kind_loads_as_none() {
        for glass in Glass::all() {
            let spec = ToastSpec {
                style: Style {
                    glass,
                    ..Style::default()
                },
                ..ToastSpec::default()
            };
            let text = toml::to_string(&spec).expect("encodes");
            let back: ToastSpec = toml::from_str(&text).expect("decodes");
            assert_eq!(back.style.glass, glass);
        }
        let text = "[style]\nglass = \"mica-like\"\n";
        let spec: ToastSpec = toml::from_str(text).expect("parses");
        assert_eq!(spec.style.glass, Glass::MicaLike, "kebab-case on disk");

        // One unknown word must not cost the user the rest of their style.
        let text = "[style]\nwidth = 420.0\nglass = \"frosted\"\n";
        let spec: ToastSpec = toml::from_str(text).expect("an unknown kind still parses");
        assert_eq!(spec.style.glass, Glass::None);
        assert_eq!(spec.style.width, 420.0);
    }

    #[test]
    fn every_glass_kind_names_its_own_key() {
        let mut keys: Vec<&str> = Glass::all().iter().map(|g| g.name_key()).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), Glass::all().len());
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
