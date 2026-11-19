//! The window a toast lives in (`docs/specs/toast.md` §1).
//!
//! Borderless, transparent, always on top, and **never the focused window**.
//! It is not a Windows Action Center notification: the app paints its own,
//! which is what makes the customization possible at all.
//!
//! A toast that takes focus is worse than no toast. Someone mid-sentence in
//! another application must keep typing into that application, so the viewport
//! is built inactive and stays inactive — and it is absent from the taskbar
//! and from Alt+Tab, where a notification has no business appearing.
//!
//! Several can be alive at once: that is what stacking is
//! (`docs/specs/toast.md` §7). Each gets its own viewport id, derived from the
//! toast's own id so the same toast keeps its window across frames.

use std::time::{Duration, Instant};

use crate::spec::ToastSpec;

/// What every toast window's title starts with.
///
/// Never shown — there is no title bar — but it is what a screen reader
/// reads, and it is how `ot_platform::toast_window` finds the windows to take
/// out of Alt+Tab. It is the app's own name and does not change with the
/// language, so that lookup cannot break when the UI switches. Each toast
/// adds its own id ([`ToastId::window_title`]), so one window can be cut to
/// its own shape.
pub const WINDOW_TITLE: &str = "OmniToaster";

/// The id of one live toast, and of the viewport that draws it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ToastId(pub u64);

impl ToastId {
    /// The egui viewport id for this toast.
    ///
    /// Derived from the toast's own id rather than allocated per frame, so a
    /// toast keeps its window for as long as it is on screen instead of
    /// getting a new one — and a new one would flash.
    pub fn viewport_id(self) -> egui::ViewportId {
        egui::ViewportId(egui::Id::new(("ot-toast", self.0)))
    }

    /// This toast's window title: [`WINDOW_TITLE`] and the id.
    pub fn window_title(self) -> String {
        format!("{WINDOW_TITLE} {}", self.0)
    }
}

/// Build the viewport for a toast.
///
/// Every flag here is a requirement from the spec rather than a preference,
/// and `the_viewport_carries_every_flag_the_spec_names` is what stops one
/// being dropped by a later edit — each of them fails silently and visibly
/// only to a user.
pub fn builder(spec: &ToastSpec, title: &str) -> egui::ViewportBuilder {
    let height = spec.style.toast_height();
    egui::ViewportBuilder::default()
        // The title is never shown — there is no title bar — but it is what
        // a screen reader and the window list would read.
        .with_title(title)
        .with_inner_size([spec.style.width, height])
        // The app paints its own frame, including its own corners.
        .with_decorations(false)
        // Corners, shadow and any opacity below 100% need the window itself
        // to be transparent, not just the pixels we draw.
        .with_transparent(true)
        // A notification is above the work the user is doing, by definition.
        .with_always_on_top()
        // Never steal focus: someone typing must keep typing where they were.
        .with_active(false)
        // A notification belongs in neither the taskbar nor Alt+Tab.
        .with_taskbar(false)
        .with_resizable(false)
}

/// Draw every visible toast as its own viewport.
///
/// Each one is an **immediate** viewport, painted inside the root window's own
/// frame (ADR-016). Deferred viewports, each repainting on its own, starve one
/// another: of three that each asked for every frame, only the newest was ever
/// repainted. Painted from the root's frame, every toast is repainted in the
/// same frame, which is also the one clock every animation runs on.
///
/// `tray` is the tray icon's centre in points, when Windows says where it is:
/// what a toast shrinks into.
///
/// `paint` is given the ui, the spec, the toast's id, whose
/// [`ToastId::window_title`] names the window it is painting, and its
/// [`Look`](crate::effects::Look) this frame: where the toast sits in its
/// window, the transform and alpha its effect draws it with, and the part
/// that shows.
pub fn show_all(
    ctx: &egui::Context,
    toasts: &Toasts,
    work_area: egui::Rect,
    tray: Option<egui::Pos2>,
    now: Instant,
    mut paint: impl FnMut(&mut egui::Ui, &ToastSpec, ToastId, &crate::effects::Look),
) {
    let scene = crate::effects::Scene { work_area, tray };
    let slots = toasts.slots(now);
    for (toast, slot) in toasts.visible().iter().zip(slots) {
        let spec = &toast.spec;
        let id = toast.id;
        let presence = toast.presence(now);
        let size = egui::vec2(spec.style.width, spec.style.toast_height());
        // Worked out every frame against the work area in use, rather than
        // once when the toast appeared: the monitor can change underneath it
        // (`docs/specs/toast.md` §7.2).
        let at = crate::placement::place_at(
            work_area,
            size,
            spec.behaviour.position,
            slot,
            spec.behaviour.stack_gap,
        );
        // The effect on the way in, or its mirror on the way out.
        let look = crate::effects::look(
            toast.animation(toasts.reduced_motion),
            toast.is_leaving(),
            presence,
            at,
            size,
            spec.behaviour.position.accent_edge(),
            scene,
        );
        ctx.show_viewport_immediate(
            toast.id.viewport_id(),
            // Hidden as far as egui knows, always: the app shows the window
            // itself once it is shaped (`ot_platform::toast_window::reveal`).
            // Shown through egui, winit rewrote the window's extended style
            // and took its alpha with it (BUG-0012).
            builder(spec, &id.window_title())
                .with_position(look.window.min)
                .with_inner_size(look.window.size())
                .with_visible(false),
            |ctx, _class| {
                // No frame of egui's own: the toast paints its own
                // background, its own corners and its own shadow, and a panel
                // fill underneath would show through the transparent corners.
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(ctx, |ui| {
                        // The effect's motion and scale, on everything the
                        // toast paints (§5).
                        ctx.set_transform_layer(ui.layer_id(), look.transform);
                        paint(ui, spec, id, &look);
                    });
            },
        );
        // egui keeps one map of layer transforms for the whole context, and
        // every viewport's panel has the same layer id, the settings
        // window's included. The toast's window is painted by now, so its
        // transform is cleared before it can move the next window's panel.
        ctx.set_transform_layer(
            egui::LayerId::background(),
            egui::emath::TSTransform::IDENTITY,
        );
        // And a Genie's bands, each painted on a layer of its own.
        if look.genie.is_some() {
            for band in 0..crate::effects::GENIE_BANDS {
                ctx.set_transform_layer(
                    crate::effects::genie_layer(id.0, band),
                    egui::emath::TSTransform::IDENTITY,
                );
            }
        }
    }
}

/// Where a toast is in its life (`docs/specs/toast.md` §5): it enters, holds,
/// and leaves. The two animated stages carry their linear progress, 0.0–1.0,
/// before any easing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Stage {
    /// Arriving: 0 has just appeared, 1 is fully there.
    Entering(f32),
    /// On screen, its countdown running unless the pointer is over it.
    Holding,
    /// Going: 1 is still fully there, 0 is gone. A toast dismissed while it
    /// was entering starts from wherever it had got to, not from 1.
    Leaving(f32),
    /// Its out-animation has finished; the window can close.
    Gone,
}

/// How many of a toast's last frame intervals the frame-time budget looks at.
pub const BUDGET_FRAMES: usize = 6;
/// The slowest interval between frames an effect may keep: 30 fps.
pub const BUDGET_INTERVAL: f32 = 1.0 / 30.0;

/// How long a toast waits for its window before it starts anyway.
const APPEAR_WAIT: Duration = Duration::from_secs(1);

/// One toast on screen: its id, its spec, and where it is in its timeline.
///
/// Every method that depends on the time takes it, so the whole timeline can
/// be tested exactly, without sleeping.
#[derive(Debug, Clone)]
pub struct LiveToast {
    pub id: ToastId,
    pub spec: ToastSpec,
    /// When it appeared on screen and its in-animation began. Until its
    /// window exists and has been shaped — a frame or more after it is asked
    /// for — it has not appeared: a window shown before its alpha is set
    /// flashes at full strength first.
    entered: Instant,
    /// Whether [`LiveToast::appear`] has been called.
    on_screen: bool,
    /// When it was created, so a window that never turns up does not keep a
    /// toast waiting for ever.
    created: Instant,
    /// Hold time lost to the pointer, from pauses that have ended.
    paused: Duration,
    /// When the pointer arrived, while it is still there.
    hovered_since: Option<Instant>,
    /// When it began to leave, and how far in it was at that moment.
    leaving: Option<(Instant, f32)>,
    /// Frames painted during the current animation, and when it began, for
    /// the frame-rate log.
    frames: u32,
    counting_since: Instant,
    /// Plays its effects as fades from now on: its frames fell below 30 fps,
    /// or the toast before it had them fall (FEAT-0035).
    faded: bool,
    /// The intervals between its last frames while it animated, in seconds,
    /// for the frame-time budget; the newest last.
    intervals: Vec<f32>,
    /// When its last animated frame was painted.
    last_frame: Option<Instant>,
}

impl LiveToast {
    pub fn new(id: ToastId, spec: ToastSpec, now: Instant) -> Self {
        Self {
            id,
            spec: spec.clamped(),
            entered: now,
            on_screen: false,
            created: now,
            paused: Duration::ZERO,
            hovered_since: None,
            leaving: None,
            frames: 0,
            counting_since: now,
            faded: false,
            intervals: Vec::with_capacity(BUDGET_FRAMES),
            last_frame: None,
        }
    }

    /// Its window is on screen and shaped: the in-animation starts now.
    /// Calling it again changes nothing.
    pub fn appear(&mut self, now: Instant) {
        if !self.on_screen {
            self.on_screen = true;
            self.entered = now;
            self.counting_since = now;
            tracing::debug!(id = self.id.0, "a toast appeared");
        }
    }

    pub fn is_on_screen(&self) -> bool {
        self.on_screen
    }

    fn animation_length(&self) -> Duration {
        self.spec.animation.duration()
    }

    /// The moment the in-animation ends and the countdown starts.
    fn hold_start(&self) -> Instant {
        self.entered + self.animation_length()
    }

    /// How long its countdown has run: from the end of the in-animation,
    /// not counting time under the pointer (`docs/specs/toast.md` §5, §6).
    pub fn held(&self, now: Instant) -> Duration {
        let start = self.hold_start();
        let current_pause = self.hovered_since.map_or(Duration::ZERO, |since| {
            now.saturating_duration_since(since.max(start))
        });
        now.saturating_duration_since(start)
            .saturating_sub(self.paused)
            .saturating_sub(current_pause)
    }

    /// Stop or restart the countdown as the pointer comes and goes. Time
    /// under the pointer while the toast is still entering costs the
    /// countdown nothing, because the countdown has not started.
    pub fn set_hovered(&mut self, hovered: bool, now: Instant) {
        match (hovered, self.hovered_since) {
            (true, None) => self.hovered_since = Some(now),
            (false, Some(since)) => {
                self.paused += now.saturating_duration_since(since.max(self.hold_start()));
                self.hovered_since = None;
            }
            _ => {}
        }
    }

    pub fn is_paused(&self) -> bool {
        self.hovered_since.is_some()
    }

    /// Where it is in its life at `now`.
    pub fn stage(&self, now: Instant) -> Stage {
        let length = self.animation_length().as_secs_f32().max(f32::EPSILON);
        if let Some((since, from)) = self.leaving {
            let left = from - now.saturating_duration_since(since).as_secs_f32() / length;
            return if left <= 0.0 {
                Stage::Gone
            } else {
                Stage::Leaving(left)
            };
        }
        if !self.on_screen {
            return Stage::Entering(0.0);
        }
        let entering = now.saturating_duration_since(self.entered).as_secs_f32() / length;
        if entering < 1.0 {
            Stage::Entering(entering)
        } else {
            Stage::Holding
        }
    }

    /// How much of the toast is there at `now`, 0.0–1.0, before easing: 1
    /// while it holds, rising while it enters, falling while it leaves.
    pub fn presence(&self, now: Instant) -> f32 {
        match self.stage(now) {
            Stage::Entering(t) | Stage::Leaving(t) => t.clamp(0.0, 1.0),
            Stage::Holding => 1.0,
            Stage::Gone => 0.0,
        }
    }

    /// Whether it is moving at `now`, so the next frame is wanted at once.
    pub fn is_animating(&self, now: Instant) -> bool {
        matches!(self.stage(now), Stage::Entering(_) | Stage::Leaving(_))
    }

    /// Begin the out-animation from wherever it is. Leaving again changes
    /// nothing.
    pub fn leave(&mut self, now: Instant) {
        if self.leaving.is_none() {
            let from = self.presence(now);
            self.end_animation_log("entered", now);
            self.leaving = Some((now, from));
            self.counting_since = now;
        }
    }

    pub fn is_leaving(&self) -> bool {
        self.leaving.is_some()
    }

    /// Whether its countdown has run out at `now`.
    ///
    /// A persistent toast waits to be acknowledged, but not for ever: its cap
    /// is already in the spec's own duration by the time it gets here
    /// (`ToastSpec::clamped`).
    fn expired(&self, now: Instant) -> bool {
        let allowed = Duration::from_secs_f32(self.spec.behaviour.duration_secs.max(0.0));
        self.stage(now) == Stage::Holding && self.held(now) >= allowed
    }

    /// When something next changes without anyone touching it: the moment
    /// its countdown runs out. `None` while it animates (every frame is
    /// wanted then) or while the pointer holds it.
    pub fn next_change(&self, now: Instant) -> Option<Instant> {
        if self.is_animating(now) || self.is_paused() || self.leaving.is_some() {
            return None;
        }
        let allowed = Duration::from_secs_f32(self.spec.behaviour.duration_secs.max(0.0));
        let start = self.hold_start().max(now);
        Some(start + allowed.saturating_sub(self.held(now)))
    }

    /// Count a painted frame of an animation; when the animation has just
    /// ended, log how many frames it took. The log is the frame-rate
    /// measurement the phase's exit criterion asks for (ADR-016).
    /// The animation it plays: its own, or every effect a fade when motion
    /// is reduced or its frames could not keep up (`docs/specs/toast.md` §5).
    pub fn animation(&self, reduced: bool) -> crate::spec::Animation {
        if reduced || self.faded || self.spec.animation.reduce_motion {
            self.spec.animation.as_fade()
        } else {
            self.spec.animation
        }
    }

    pub fn is_faded(&self) -> bool {
        self.faded
    }

    /// Count a painted frame against the frame-time budget, with at least
    /// `floor` seconds between frames (a way to force the fallback for
    /// checking). Returns true on the frame its effect gives up and
    /// finishes as a fade: the median of its last [`BUDGET_FRAMES`]
    /// intervals is slower than 30 fps. A median, so one slow frame — a
    /// window being created, a hitch — does not trigger it.
    pub fn budget_frame(&mut self, now: Instant, floor: f32) -> bool {
        if !self.is_animating(now) {
            self.intervals.clear();
            self.last_frame = None;
            return false;
        }
        if let Some(last) = self.last_frame {
            let interval = now.saturating_duration_since(last).as_secs_f32().max(floor);
            if self.intervals.len() == BUDGET_FRAMES {
                self.intervals.remove(0);
            }
            self.intervals.push(interval);
        }
        self.last_frame = Some(now);
        if self.faded || self.intervals.len() < BUDGET_FRAMES {
            return false;
        }
        let mut sorted = self.intervals.clone();
        sorted.sort_by(f32::total_cmp);
        let median = sorted[BUDGET_FRAMES / 2];
        if median > BUDGET_INTERVAL {
            self.faded = true;
            tracing::warn!(
                id = self.id.0,
                frame_ms = median * 1000.0,
                fps = 1.0 / median,
                "a toast animation fell below 30 fps; it finishes as a fade, and so does the next toast"
            );
            return true;
        }
        false
    }

    pub fn note_frame(&mut self, now: Instant) {
        match self.stage(now) {
            Stage::Entering(_) | Stage::Leaving(_) => self.frames += 1,
            Stage::Holding => self.end_animation_log("entered", now),
            Stage::Gone => self.end_animation_log("left", now),
        }
    }

    fn end_animation_log(&mut self, what: &'static str, now: Instant) {
        if self.frames > 0 {
            let ms = now
                .saturating_duration_since(self.counting_since)
                .as_secs_f32()
                * 1000.0;
            let fps = self.frames as f32 * 1000.0 / ms.max(1.0);
            tracing::debug!(
                id = self.id.0,
                what,
                frames = self.frames,
                ms,
                fps,
                "a toast animation ended"
            );
            self.frames = 0;
        }
    }
}

/// Every toast on screen, and the ones waiting for room.
///
/// When more fire than `max_visible`, the surplus **queues** and appears as
/// the earlier ones retire, rather than being dropped — a fired reminder that
/// silently never appeared is the one failure a notification app may not have
/// (`docs/specs/toast.md` §7).
#[derive(Debug, Default)]
pub struct Toasts {
    visible: Vec<LiveToast>,
    queued: std::collections::VecDeque<LiveToast>,
    next_id: u64,
    /// Reduced motion — Windows' "Show animations" off, or forced — so
    /// every toast fades (FEAT-0035). Set by the app each frame.
    pub reduced_motion: bool,
    /// The next toast to arrive starts as a fade: the one before it fell
    /// below 30 fps.
    fade_next: bool,
    /// The least time between frames the budget counts, in seconds: 0 in
    /// use, raised to force the fallback for checking.
    pub frame_floor: f32,
}

impl Toasts {
    pub fn new() -> Self {
        Self::default()
    }

    /// Show a toast, or queue it when the screen is full.
    pub fn push(&mut self, spec: ToastSpec, now: Instant) -> ToastId {
        self.push_at(spec, now, false)
    }

    /// Show an urgent toast — an alarm — or, when the screen is full, queue
    /// it ahead of everything already waiting (`docs/specs/reminders.md` §8).
    /// It does not push a toast already on screen off it.
    pub fn push_first(&mut self, spec: ToastSpec, now: Instant) -> ToastId {
        self.push_at(spec, now, true)
    }

    fn push_at(&mut self, spec: ToastSpec, now: Instant, first: bool) -> ToastId {
        let id = ToastId(self.next_id);
        self.next_id += 1;
        let mut toast = LiveToast::new(id, spec, now);
        // The toast before fell below 30 fps: this one starts as a fade.
        if std::mem::take(&mut self.fade_next) {
            toast.faded = true;
        }

        if self.staying() < self.max_visible() {
            self.visible.push(toast);
        } else if first {
            self.queued.push_front(toast);
        } else {
            self.queued.push_back(toast);
        }
        id
    }

    /// The visible toasts that are not on their way out: a leaving toast
    /// has given up its place, so the next in the queue may start to enter.
    fn staying(&self) -> usize {
        self.visible.iter().filter(|t| !t.is_leaving()).count()
    }

    /// How many may be on screen at once.
    ///
    /// Taken from the oldest visible toast's own spec, because the setting is
    /// global and every spec carries the same clamped copy of it; with none on
    /// screen the default applies.
    fn max_visible(&self) -> usize {
        self.visible
            .first()
            .map_or(crate::spec::MAX_VISIBLE.default, |t| {
                t.spec.behaviour.max_visible
            }) as usize
    }

    /// Start the out-animation of whatever has had its time, drop what has
    /// finished leaving, and promote from the queue.
    ///
    /// Returns the ids that are gone, so their viewports can be closed.
    pub fn tick(&mut self, now: Instant) -> Vec<ToastId> {
        for toast in &mut self.visible {
            // A window that has not turned up in a second is not going to be
            // shaped first; the toast starts anyway rather than wait unseen.
            if !toast.on_screen && now.saturating_duration_since(toast.created) >= APPEAR_WAIT {
                tracing::warn!(
                    id = toast.id.0,
                    "a toast's window was not ready in time; it starts anyway"
                );
                toast.appear(now);
            }
            if toast.expired(now) {
                toast.leave(now);
            }
            if toast.stage(now) == Stage::Gone {
                toast.end_animation_log("left", now);
            }
        }
        let mut gone = Vec::new();
        self.visible.retain(|toast| {
            if toast.stage(now) == Stage::Gone {
                gone.push(toast.id);
                false
            } else {
                true
            }
        });

        while self.staying() < self.max_visible() {
            match self.queued.pop_front() {
                // It enters when it appears, not when it was pushed: a
                // queued toast nobody has seen has not been shown.
                Some(toast) => self.visible.push(LiveToast::new(toast.id, toast.spec, now)),
                None => break,
            }
        }
        gone
    }

    /// The toast's window is on screen and shaped: its in-animation starts.
    pub fn appear(&mut self, id: ToastId, now: Instant) {
        if let Some(toast) = self.visible.iter_mut().find(|toast| toast.id == id) {
            toast.appear(now);
        }
    }

    /// Whether the pointer is over this toast's window: hovering pauses its
    /// countdown (`docs/specs/toast.md` §6). Called every frame for every
    /// visible toast; telling it what it already knows changes nothing.
    pub fn set_hovered(&mut self, id: ToastId, hovered: bool, now: Instant) {
        if let Some(toast) = self.visible.iter_mut().find(|toast| toast.id == id) {
            toast.set_hovered(hovered, now);
        }
    }

    /// Dismiss one by id: a visible toast plays its out-animation from
    /// wherever it is, and a queued one never appears.
    pub fn dismiss(&mut self, id: ToastId, now: Instant) {
        if let Some(toast) = self.visible.iter_mut().find(|toast| toast.id == id) {
            toast.leave(now);
        }
        self.queued.retain(|toast| toast.id != id);
    }

    /// Count this frame against every toast's frame-time budget. A toast
    /// whose effect gives up finishes as a fade, and the next to arrive
    /// starts as one.
    pub fn budget_frame(&mut self, now: Instant) {
        let floor = self.frame_floor;
        for toast in &mut self.visible {
            if toast.budget_frame(now, floor) {
                self.fade_next = true;
            }
        }
    }

    /// Whether any toast is moving at `now`, so every frame is wanted.
    pub fn is_animating(&self, now: Instant) -> bool {
        self.visible.iter().any(|toast| toast.is_animating(now))
    }

    /// The soonest moment something changes on its own — a countdown
    /// running out — so the app can sleep until then rather than poll.
    pub fn next_change(&self, now: Instant) -> Option<Instant> {
        self.visible
            .iter()
            .filter_map(|toast| toast.next_change(now))
            .min()
    }

    /// Each visible toast's place in the stack at `now`, in toasts from the
    /// anchor. A leaving toast's own share shrinks with its eased presence,
    /// so the ones beyond it move up with the same easing rather than
    /// jumping (`docs/specs/toast.md` §5).
    pub fn slots(&self, now: Instant) -> Vec<f32> {
        let mut slot = 0.0;
        self.visible
            .iter()
            .map(|toast| {
                let here = slot;
                let share = if toast.is_leaving() {
                    toast
                        .spec
                        .animation
                        .easing
                        .apply(toast.presence(now))
                        .max(0.0)
                } else {
                    1.0
                };
                slot += share;
                here
            })
            .collect()
    }

    pub fn visible(&self) -> &[LiveToast] {
        &self.visible
    }

    pub fn visible_mut(&mut self) -> &mut [LiveToast] {
        &mut self.visible
    }

    pub fn queued_count(&self) -> usize {
        self.queued.len()
    }

    pub fn is_empty(&self) -> bool {
        self.visible.is_empty() && self.queued.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::MAX_VISIBLE;

    fn spec_lasting(secs: f32) -> ToastSpec {
        let mut spec = ToastSpec::default();
        spec.behaviour.duration_secs = secs;
        spec.clamped()
    }

    #[test]
    fn the_viewport_carries_every_flag_the_spec_names() {
        // Each of these fails silently, and visibly only to a user: a toast
        // that steals focus, or shows a title bar, or turns up in Alt+Tab.
        let built = builder(&ToastSpec::default(), "OmniToaster");
        assert_eq!(built.decorations, Some(false), "borderless");
        assert_eq!(built.transparent, Some(true), "transparent");
        assert_eq!(
            built.window_level,
            Some(egui::viewport::WindowLevel::AlwaysOnTop),
            "always on top"
        );
        assert_eq!(built.active, Some(false), "must never take focus");
        assert_eq!(built.taskbar, Some(false), "absent from the taskbar");
        assert_eq!(built.resizable, Some(false));
        assert_eq!(
            built.inner_size,
            Some(egui::vec2(
                ToastSpec::default().style.width,
                crate::spec::AUTO_HEIGHT + crate::spec::ACTION_ROW
            )),
            "an auto-height toast grows by its action row"
        );
    }

    #[test]
    fn the_viewport_takes_its_size_from_the_spec_including_a_fixed_height() {
        let mut spec = ToastSpec::default();
        spec.style.width = 480.0;
        spec.style.height = Some(200.0);
        let built = builder(&spec.clamped(), "t");
        assert_eq!(built.inner_size, Some(egui::vec2(480.0, 200.0)));
    }

    #[test]
    fn a_toast_keeps_one_viewport_id_and_two_toasts_never_share_one() {
        assert_eq!(ToastId(7).viewport_id(), ToastId(7).viewport_id());
        assert_ne!(ToastId(7).viewport_id(), ToastId(8).viewport_id());
    }

    /// A fixed start, so every time in these tests is exact.
    fn t0() -> Instant {
        Instant::now()
    }

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    /// The default animation is 350 ms each way.
    const ANIM: u64 = 350;

    /// Push a toast and let its window appear at once, as the app does once
    /// it has shaped the window.
    fn shown(toasts: &mut Toasts, spec: ToastSpec, now: Instant) -> ToastId {
        let id = toasts.push(spec, now);
        toasts.appear(id, now);
        id
    }

    #[test]
    fn a_toast_waits_for_its_window_and_starts_anyway_after_a_second() {
        let start = t0();
        let mut toasts = Toasts::new();
        let id = toasts.push(spec_lasting(3.0), start);
        let toast = &toasts.visible()[0];
        assert!(!toast.is_on_screen());
        assert_eq!(
            toast.stage(start + ms(500)),
            Stage::Entering(0.0),
            "held at the start"
        );
        assert!(
            toasts.is_animating(start + ms(500)),
            "and frames keep coming until it shows"
        );

        // Its window never turned up: after a second it starts regardless.
        toasts.tick(start + APPEAR_WAIT);
        let toast = &toasts.visible()[0];
        assert!(toast.is_on_screen());
        assert_eq!(toast.stage(start + APPEAR_WAIT), Stage::Entering(0.0));
        assert_eq!(toast.id, id);

        // Appearing again changes nothing.
        toasts.appear(id, start + ms(5_000));
        assert_eq!(
            toasts.visible()[0].stage(start + APPEAR_WAIT + ms(ANIM)),
            Stage::Holding
        );
    }

    #[test]
    fn the_surplus_queues_rather_than_being_dropped() {
        // A fired reminder that silently never appeared is the one failure a
        // notification app may not have.
        let now = t0();
        let mut toasts = Toasts::new();
        let max = MAX_VISIBLE.default as usize;
        for _ in 0..max + 2 {
            shown(&mut toasts, spec_lasting(60.0), now);
        }
        assert_eq!(toasts.visible().len(), max);
        assert_eq!(toasts.queued_count(), 2, "the surplus is kept, not dropped");
    }

    #[test]
    fn a_toast_enters_holds_for_its_whole_duration_then_leaves() {
        let start = t0();
        let mut toast = LiveToast::new(ToastId(1), spec_lasting(3.0), start);
        toast.appear(start);
        assert_eq!(toast.stage(start), Stage::Entering(0.0));
        assert_eq!(toast.stage(start + ms(ANIM / 2)), Stage::Entering(0.5));
        assert_eq!(toast.stage(start + ms(ANIM)), Stage::Holding);

        // The countdown starts when the in-animation ends, so the whole
        // three seconds are on screen in full.
        assert_eq!(toast.held(start + ms(ANIM)), Duration::ZERO);
        assert_eq!(toast.held(start + ms(ANIM + 1_000)), ms(1_000));
        assert!(!toast.expired(start + ms(ANIM + 2_999)));
        assert!(toast.expired(start + ms(ANIM + 3_000)));
    }

    #[test]
    fn expiry_plays_the_out_animation_before_the_window_closes() {
        let start = t0();
        let mut toasts = Toasts::new();
        let id = shown(&mut toasts, spec_lasting(3.0), start);
        let expires = start + ms(ANIM + 3_000);

        assert!(toasts.tick(expires).is_empty(), "not gone: leaving");
        let toast = &toasts.visible()[0];
        assert_eq!(toast.stage(expires), Stage::Leaving(1.0));
        assert_eq!(toast.stage(expires + ms(ANIM / 2)), Stage::Leaving(0.5));

        assert!(toasts.tick(expires + ms(ANIM / 2)).is_empty());
        assert_eq!(
            toasts.tick(expires + ms(ANIM)),
            vec![id],
            "gone once it has left"
        );
        assert!(toasts.is_empty());
    }

    #[test]
    fn a_toast_dismissed_while_entering_leaves_from_where_it_is() {
        let start = t0();
        let mut toast = LiveToast::new(ToastId(1), spec_lasting(8.0), start);
        toast.appear(start);
        let at = start + ms(ANIM * 2 / 5);
        let reached = toast.presence(at);
        assert!((0.39..0.41).contains(&reached), "{reached}");

        toast.leave(at);
        assert_eq!(
            toast.stage(at),
            Stage::Leaving(reached),
            "no jump to fully there"
        );
        // And it takes only as long as it had come: 40% of the way back.
        assert!(matches!(
            toast.stage(at + ms(ANIM * 2 / 5 - 5)),
            Stage::Leaving(_)
        ));
        assert_eq!(toast.stage(at + ms(ANIM * 2 / 5 + 5)), Stage::Gone);

        // Leaving twice does not restart it.
        toast.leave(at + ms(50));
        assert_eq!(toast.stage(at), Stage::Leaving(reached));
    }

    #[test]
    fn hovering_pauses_the_countdown_but_not_the_entry() {
        let start = t0();
        let mut toast = LiveToast::new(ToastId(1), spec_lasting(60.0), start);
        toast.appear(start);
        let holding = start + ms(ANIM);

        // The pointer arrives while it enters: the entry carries on, and the
        // countdown, which has not started, loses nothing to it yet.
        toast.set_hovered(true, start + ms(100));
        assert!(toast.is_paused());
        assert_eq!(
            toast.stage(start + ms(200)),
            Stage::Entering(200.0 / ANIM as f32)
        );
        assert_eq!(
            toast.held(holding + ms(500)),
            Duration::ZERO,
            "paused all along"
        );

        toast.set_hovered(false, holding + ms(500));
        assert!(!toast.is_paused());
        assert_eq!(
            toast.held(holding + ms(800)),
            ms(300),
            "resumes where it stopped"
        );

        // A pause during the hold costs exactly its own length.
        toast.set_hovered(true, holding + ms(1_000));
        toast.set_hovered(false, holding + ms(4_000));
        assert_eq!(toast.held(holding + ms(5_000)), ms(1_500));
        assert_eq!(
            toast.next_change(holding + ms(5_000)),
            Some(holding + ms(5_000) + ms(58_500))
        );
    }

    #[test]
    fn an_urgent_toast_waits_ahead_of_the_queue() {
        let start = t0();
        let mut toasts = Toasts::new();
        let max = MAX_VISIBLE.default as usize;
        let first = shown(&mut toasts, spec_lasting(60.0), start);
        for _ in 1..max {
            shown(&mut toasts, spec_lasting(60.0), start);
        }
        let ordinary = toasts.push(spec_lasting(60.0), start);
        let alarm = toasts.push_first(spec_lasting(60.0), start);
        assert_eq!(
            toasts.visible().len(),
            max,
            "nothing on screen is displaced"
        );

        let later = start + ms(1_000);
        toasts.dismiss(first, later);
        toasts.tick(later);
        assert!(
            toasts.visible().iter().any(|t| t.id == alarm),
            "the alarm first"
        );
        assert!(toasts.visible().iter().all(|t| t.id != ordinary));
        assert_eq!(toasts.queued_count(), 1);
    }

    #[test]
    fn a_queued_toast_enters_when_it_is_promoted_not_when_it_was_pushed() {
        // Otherwise a toast that waited behind others would arrive already
        // expired and flash past unread.
        let start = t0();
        let mut toasts = Toasts::new();
        let max = MAX_VISIBLE.default as usize;
        let first = shown(&mut toasts, spec_lasting(60.0), start);
        for _ in 1..max {
            shown(&mut toasts, spec_lasting(60.0), start);
        }
        let queued = toasts.push(spec_lasting(60.0), start);
        assert_eq!(toasts.queued_count(), 1);

        // Dismissing one gives up its place at once: the queued toast starts
        // to enter while the dismissed one is still leaving.
        let later = start + ms(30_000);
        toasts.dismiss(first, later);
        toasts.tick(later);
        assert_eq!(toasts.queued_count(), 0);
        let promoted = toasts
            .visible()
            .iter()
            .find(|t| t.id == queued)
            .expect("promoted");
        assert_eq!(
            promoted.stage(later),
            Stage::Entering(0.0),
            "entering from the start"
        );
    }

    #[test]
    fn the_toasts_beyond_a_leaving_one_move_up_with_its_easing() {
        let start = t0();
        let mut toasts = Toasts::new();
        let first = shown(&mut toasts, spec_lasting(60.0), start);
        shown(&mut toasts, spec_lasting(60.0), start);
        shown(&mut toasts, spec_lasting(60.0), start);
        let settled = start + ms(ANIM);
        assert_eq!(toasts.slots(settled), vec![0.0, 1.0, 2.0]);

        toasts.dismiss(first, settled);
        assert_eq!(
            toasts.slots(settled),
            vec![0.0, 1.0, 2.0],
            "nothing has moved yet"
        );
        let halfway = toasts.slots(settled + ms(ANIM / 2));
        let eased = crate::spec::Easing::EaseOut.apply(0.5);
        assert!((halfway[1] - eased).abs() < 1e-4, "{halfway:?}");
        assert!((halfway[2] - (eased + 1.0)).abs() < 1e-4);
        let gone = toasts.slots(settled + ms(ANIM));
        assert_eq!(&gone[1..], &[0.0, 1.0], "in the room it left");
    }

    #[test]
    fn dismissing_a_queued_toast_means_it_never_appears() {
        let start = t0();
        let mut toasts = Toasts::new();
        for _ in 0..MAX_VISIBLE.default {
            shown(&mut toasts, spec_lasting(60.0), start);
        }
        let queued = toasts.push(spec_lasting(60.0), start);
        toasts.dismiss(queued, start);
        assert_eq!(toasts.queued_count(), 0);
        let first = toasts.visible()[0].id;
        toasts.dismiss(first, start + ms(ANIM));
        toasts.tick(start + ms(ANIM * 3));
        assert!(
            toasts.visible().iter().all(|t| t.id != queued),
            "a dismissed toast must not arrive later from the queue"
        );
    }

    #[test]
    fn the_pointer_over_one_toast_pauses_that_toast_only() {
        // BUG-0009: the model could pause, but nothing told it which toast
        // the pointer was over, so on screen a hovered toast ran out anyway.
        let start = t0();
        let mut toasts = Toasts::new();
        let under = shown(&mut toasts, spec_lasting(3.0), start);
        let other = shown(&mut toasts, spec_lasting(3.0), start);
        let holding = start + ms(ANIM);

        toasts.set_hovered(under, true, holding);
        toasts.set_hovered(other, false, holding);
        let past_its_time = holding + ms(3_500);
        toasts.tick(past_its_time);
        let find = |toasts: &Toasts, id| toasts.visible().iter().find(|t| t.id == id).cloned();
        let hovered = find(&toasts, under).expect("still here");
        assert_eq!(
            hovered.stage(past_its_time),
            Stage::Holding,
            "held by the pointer"
        );
        assert!(
            find(&toasts, other).expect("leaving").is_leaving(),
            "the other ran out"
        );

        // The pointer moves away: its remaining three seconds run from there.
        toasts.set_hovered(under, false, past_its_time);
        toasts.tick(past_its_time + ms(2_900));
        assert!(!find(&toasts, under).expect("here").is_leaving());
        toasts.tick(past_its_time + ms(3_000));
        assert!(find(&toasts, under).expect("leaving now").is_leaving());
    }

    #[test]
    fn only_a_moving_toast_wants_every_frame_and_a_still_one_says_when_it_changes() {
        let start = t0();
        let mut toasts = Toasts::new();
        shown(&mut toasts, spec_lasting(3.0), start);
        assert!(toasts.is_animating(start));
        assert_eq!(
            toasts.next_change(start),
            None,
            "every frame while it enters"
        );
        let holding = start + ms(ANIM);
        assert!(!toasts.is_animating(holding));
        assert_eq!(toasts.next_change(holding), Some(holding + ms(3_000)));
    }

    /// A toast that is entering at `start`, with a long animation so every
    /// frame below is mid-entry.
    fn entering(toasts: &mut Toasts, start: Instant) -> ToastId {
        let mut spec = spec_lasting(60.0);
        spec.animation.duration_ms = 1000;
        shown(toasts, spec, start)
    }

    #[test]
    fn frames_steadily_below_30_fps_fall_back_to_a_fade_and_the_next_toast_starts_as_one() {
        let start = t0();
        let mut toasts = Toasts::new();
        let slow = entering(&mut toasts, start);
        // 50 ms a frame: 20 fps.
        for frame in 0..=BUDGET_FRAMES as u64 {
            toasts.budget_frame(start + ms(50 * frame));
        }
        let toast = toasts
            .visible()
            .iter()
            .find(|t| t.id == slow)
            .expect("here");
        assert!(toast.is_faded(), "it finishes as a fade");
        assert_eq!(toast.animation(false).enter, crate::spec::Effect::Appear);

        // The next toast to arrive starts as one; the one after does not.
        let next = toasts.push(spec_lasting(60.0), start + ms(400));
        let after = toasts.push(spec_lasting(60.0), start + ms(400));
        let faded = |id| {
            toasts
                .visible()
                .iter()
                .find(|t| t.id == id)
                .map(LiveToast::is_faded)
        };
        assert_eq!(faded(next), Some(true));
        assert_eq!(faded(after), Some(false));
    }

    #[test]
    fn one_slow_frame_does_not_trigger_the_fallback() {
        let start = t0();
        let mut toasts = Toasts::new();
        entering(&mut toasts, start);
        // 16 ms frames with one of 200 ms among them — a window being made.
        let mut at = start;
        for frame in 0..20 {
            at += if frame == 3 { ms(200) } else { ms(16) };
            toasts.budget_frame(at);
        }
        assert!(!toasts.visible()[0].is_faded());
    }

    #[test]
    fn the_floor_forces_the_fallback_for_checking() {
        let start = t0();
        let mut toasts = Toasts::new();
        entering(&mut toasts, start);
        toasts.frame_floor = 0.05;
        for frame in 0..=BUDGET_FRAMES as u64 {
            toasts.budget_frame(start + ms(16 * frame));
        }
        assert!(
            toasts.visible()[0].is_faded(),
            "fast frames counted as slow"
        );
    }

    #[test]
    fn reduced_motion_makes_every_effect_a_fade() {
        use crate::spec::Effect;
        for effect in Effect::all_out() {
            let mut spec = spec_lasting(8.0);
            spec.animation.enter = effect;
            spec.animation.leave = effect;
            let toast = LiveToast::new(ToastId(1), spec.clone().clamped(), t0());
            let reduced = toast.animation(true);
            assert_eq!(
                (reduced.enter, reduced.leave),
                (Effect::Appear, Effect::Appear),
                "{effect:?}"
            );
            // The setting does the same without Windows.
            spec.animation.reduce_motion = true;
            let own = LiveToast::new(ToastId(2), spec.clamped(), t0()).animation(false);
            assert_eq!(
                (own.enter, own.leave),
                (Effect::Appear, Effect::Appear),
                "{effect:?}"
            );
        }
        // Neither: the toast's own effect.
        let mut spec = spec_lasting(8.0);
        spec.animation.enter = Effect::Glow;
        assert_eq!(
            LiveToast::new(ToastId(3), spec, t0())
                .animation(false)
                .enter,
            Effect::Glow
        );
    }
}
