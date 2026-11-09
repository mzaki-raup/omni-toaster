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

use crate::spec::ToastSpec;

/// The height a toast takes when it sizes to its content.
///
/// A provisional number that the renderer replaces once it knows what it is
/// drawing (FEAT-0020); a viewport has to be given *some* size when it opens.
pub const AUTO_HEIGHT: f32 = 120.0;

/// The title every toast window carries.
///
/// Never shown — there is no title bar — but it is what a screen reader
/// reads, and it is how `ot_platform::toast_window` finds the window to take
/// out of Alt+Tab. It is the app's own name and does not change with the
/// language, so that lookup cannot break when the UI switches.
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
}

/// Build the viewport for a toast.
///
/// Every flag here is a requirement from the spec rather than a preference,
/// and `the_viewport_carries_every_flag_the_spec_names` is what stops one
/// being dropped by a later edit — each of them fails silently and visibly
/// only to a user.
pub fn builder(spec: &ToastSpec, title: &str) -> egui::ViewportBuilder {
    let height = spec.style.height.unwrap_or(AUTO_HEIGHT);
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
/// Each one is a **deferred** viewport: egui owns its window and calls back to
/// paint it, which is what lets several exist at once without the app running
/// a window loop of its own. The callback cannot borrow the app, so each
/// toast's spec is cloned into it — a `ToastSpec` is small and cloning it once
/// per frame per visible toast is nothing beside painting one.
///
/// `paint` is given the spec and the ui. Until FEAT-0019 that is a fill; the
/// renderer replaces it.
pub fn show_all(
    ctx: &egui::Context,
    toasts: &Toasts,
    paint: impl Fn(&mut egui::Ui, &ToastSpec) + Send + Sync + Clone + 'static,
) {
    for toast in toasts.visible() {
        let spec = toast.spec.clone();
        let paint = paint.clone();
        ctx.show_viewport_deferred(
            toast.id.viewport_id(),
            builder(&spec, WINDOW_TITLE),
            move |ctx, _class| {
                // No frame of egui's own: the toast paints its own
                // background, its own corners and its own shadow, and a panel
                // fill underneath would show through the transparent corners.
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(ctx, |ui| paint(ui, &spec));
            },
        );
    }
}

/// What should happen to a toast this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    /// Keep it on screen.
    Keep,
    /// Its time is up, or it was dismissed.
    Retire,
}

/// One toast on screen: its id, its spec, and when it appeared.
#[derive(Debug, Clone)]
pub struct LiveToast {
    pub id: ToastId,
    pub spec: ToastSpec,
    /// Set to `None` while the pointer is over it: hovering pauses the
    /// countdown (`docs/specs/toast.md` §6), and a paused countdown is one
    /// that is not running rather than one that is running slowly.
    shown_at: Option<std::time::Instant>,
    /// How much of its life it had used when the pointer arrived.
    elapsed_before_hover: std::time::Duration,
}

impl LiveToast {
    pub fn new(id: ToastId, spec: ToastSpec) -> Self {
        Self {
            id,
            spec: spec.clamped(),
            shown_at: Some(std::time::Instant::now()),
            elapsed_before_hover: std::time::Duration::ZERO,
        }
    }

    /// How long it has been on screen, not counting time under the pointer.
    pub fn elapsed(&self) -> std::time::Duration {
        match self.shown_at {
            Some(at) => self.elapsed_before_hover + at.elapsed(),
            None => self.elapsed_before_hover,
        }
    }

    /// Stop or restart the countdown as the pointer comes and goes.
    pub fn set_hovered(&mut self, hovered: bool) {
        match (hovered, self.shown_at) {
            (true, Some(at)) => {
                self.elapsed_before_hover += at.elapsed();
                self.shown_at = None;
            }
            (false, None) => self.shown_at = Some(std::time::Instant::now()),
            _ => {}
        }
    }

    pub fn is_paused(&self) -> bool {
        self.shown_at.is_none()
    }

    /// Whether this toast has had its time.
    ///
    /// A persistent toast waits to be acknowledged, but not for ever: its cap
    /// is already in the spec's own duration by the time it gets here
    /// (`ToastSpec::clamped`).
    pub fn disposition(&self) -> Disposition {
        let allowed =
            std::time::Duration::from_secs_f32(self.spec.behaviour.duration_secs.max(0.0));
        if self.elapsed() >= allowed {
            Disposition::Retire
        } else {
            Disposition::Keep
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
}

impl Toasts {
    pub fn new() -> Self {
        Self::default()
    }

    /// Show a toast, or queue it when the screen is full.
    pub fn push(&mut self, spec: ToastSpec) -> ToastId {
        let id = ToastId(self.next_id);
        self.next_id += 1;
        let toast = LiveToast::new(id, spec);

        if self.visible.len() < self.max_visible() {
            self.visible.push(toast);
        } else {
            self.queued.push_back(toast);
        }
        id
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

    /// Retire whatever has had its time and promote from the queue.
    ///
    /// Returns the ids that left, so their viewports can be closed.
    pub fn tick(&mut self) -> Vec<ToastId> {
        let mut retired = Vec::new();
        self.visible.retain(|toast| {
            if toast.disposition() == Disposition::Retire {
                retired.push(toast.id);
                false
            } else {
                true
            }
        });

        while self.visible.len() < self.max_visible() {
            match self.queued.pop_front() {
                // Its countdown starts when it appears, not when it was
                // pushed: a queued toast nobody has seen has not been shown.
                Some(mut toast) => {
                    toast.shown_at = Some(std::time::Instant::now());
                    toast.elapsed_before_hover = std::time::Duration::ZERO;
                    self.visible.push(toast);
                }
                None => break,
            }
        }
        retired
    }

    /// Dismiss one by id, wherever it is.
    pub fn dismiss(&mut self, id: ToastId) {
        self.visible.retain(|toast| toast.id != id);
        self.queued.retain(|toast| toast.id != id);
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
            Some(egui::vec2(ToastSpec::default().style.width, AUTO_HEIGHT))
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

    #[test]
    fn the_surplus_queues_rather_than_being_dropped() {
        // A fired reminder that silently never appeared is the one failure a
        // notification app may not have.
        let mut toasts = Toasts::new();
        let max = MAX_VISIBLE.default as usize;
        for _ in 0..max + 2 {
            toasts.push(spec_lasting(60.0));
        }
        assert_eq!(toasts.visible().len(), max);
        assert_eq!(toasts.queued_count(), 2, "the surplus is kept, not dropped");
    }

    #[test]
    fn a_queued_toast_starts_its_countdown_when_it_appears() {
        // Otherwise a toast that waited behind four others would arrive
        // already expired and flash past unread.
        let mut toasts = Toasts::new();
        let max = MAX_VISIBLE.default as usize;
        for _ in 0..max {
            // Already over: these retire on the first tick.
            toasts.push(spec_lasting(DURATION_FLOOR));
        }
        let queued = toasts.push(spec_lasting(60.0));
        assert_eq!(toasts.queued_count(), 1);

        std::thread::sleep(std::time::Duration::from_millis(DURATION_FLOOR_MS + 20));
        let retired = toasts.tick();
        assert_eq!(retired.len(), max, "the expired ones left");
        assert_eq!(toasts.queued_count(), 0, "and the queued one was promoted");

        let promoted = toasts
            .visible()
            .iter()
            .find(|t| t.id == queued)
            .expect("the promoted toast");
        assert!(
            promoted.elapsed() < std::time::Duration::from_millis(50),
            "its countdown started on arrival, not on being pushed: {:?}",
            promoted.elapsed()
        );
    }

    /// The shortest a toast may last, which is what makes these tests quick.
    const DURATION_FLOOR: f32 = 3.0;
    const DURATION_FLOOR_MS: u64 = 3_000;

    #[test]
    fn hovering_pauses_the_countdown_and_leaving_resumes_it() {
        let mut toasts = Toasts::new();
        let id = toasts.push(spec_lasting(60.0));
        std::thread::sleep(std::time::Duration::from_millis(30));

        let toast = &mut toasts.visible_mut()[0];
        toast.set_hovered(true);
        assert!(toast.is_paused());
        let at_hover = toast.elapsed();

        std::thread::sleep(std::time::Duration::from_millis(60));
        assert_eq!(
            toast.elapsed(),
            at_hover,
            "time under the pointer must not count"
        );

        toast.set_hovered(false);
        assert!(!toast.is_paused());
        std::thread::sleep(std::time::Duration::from_millis(30));
        assert!(
            toast.elapsed() > at_hover,
            "and it resumes where it stopped"
        );
        assert_eq!(toasts.visible()[0].id, id);
    }

    #[test]
    fn dismissing_removes_it_from_wherever_it_is() {
        let mut toasts = Toasts::new();
        let first = toasts.push(spec_lasting(60.0));
        let mut queued_ids = Vec::new();
        for _ in 0..MAX_VISIBLE.default + 1 {
            queued_ids.push(toasts.push(spec_lasting(60.0)));
        }
        let last_queued = *queued_ids.last().expect("a queued toast");

        toasts.dismiss(first);
        assert!(toasts.visible().iter().all(|t| t.id != first));

        toasts.dismiss(last_queued);
        toasts.tick();
        assert!(
            toasts.visible().iter().all(|t| t.id != last_queued),
            "a dismissed toast must not arrive later from the queue"
        );
    }
}
