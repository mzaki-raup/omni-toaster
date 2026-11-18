//! What each animation does to a toast at a point in its timeline
//! (`docs/specs/toast.md` §5).
//!
//! One pure function turns an effect, the toast's eased progress and where it
//! belongs into a [`Look`]: where its window is and how big, where the toast
//! sits inside it, the transform its content is drawn with, the part that is
//! visible, and its alpha. The viewport places the window from it, the
//! painter draws through its transform, and the window's region is cut to its
//! visible part — so everything that moves is decided here, once, and tested
//! here without a window.
//!
//! Progress runs 0 → 1 on the way in and 1 → 0 on the way out, so every
//! out-animation is its in-animation's mirror for free.

use crate::placement::AccentEdge;
use crate::spec::{Animation, Effect};

/// How far a popping toast starts from its full size.
const POP_FROM: f32 = 0.3;
/// The transparent margin a popping toast's window has on each side, as a
/// fraction of the toast's size: room for a spring's overshoot, which peaks
/// about 14% past full size, so 7% on each side.
const POP_MARGIN: f32 = 0.08;
/// The transparent margin a glowing toast's window has on each side, in
/// points: room for the halo at its fullest ([`HALO_BLUR`] + [`HALO_SPREAD`]).
const GLOW_MARGIN: f32 = 18.0;
/// The halo at its fullest: how far it blurs, and how far it spreads before
/// blurring, in points.
pub const HALO_BLUR: f32 = 14.0;
pub const HALO_SPREAD: f32 = 2.0;
/// The side of a dissolve's square cells, in points: small enough to read as
/// grain, large enough that a toast is a few hundred of them, not thousands.
pub const DISSOLVE_CELL: f32 = 6.0;

/// Everything an effect decides about one frame of one toast. Positions are
/// in points: `window` on the screen, the rest in the window's own space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    /// The window's rect on the screen.
    pub window: egui::Rect,
    /// Where the toast is laid out inside the window, before `transform`.
    pub toast: egui::Rect,
    /// Applied to everything the toast paints: the motion of a drawer, the
    /// scale of a pop.
    pub transform: egui::emath::TSTransform,
    /// The part of the window that shows: the window's region is cut to the
    /// toast as transformed, intersected with this.
    pub clip: egui::Rect,
    /// Multiplies the toast's own opacity, 0.0–1.0.
    pub alpha: f32,
    /// Multiplies the toast's background, border and accent bar — not its
    /// text — 0.0–1.0: alpha to solid fades the background in behind text
    /// that is already there.
    pub background: f32,
    /// How strong the glow's halo is, 0.0 (none) to 1.0 (fullest).
    pub halo: f32,
    /// A dissolve's progress, when there is one: the cells whose noise is
    /// below it show ([`dissolve_cells`]).
    pub dissolve: Option<f32>,
    /// Whether Windows' shadow may be on this frame. Off while an effect
    /// needs the window see-through — a halo, a background not yet solid —
    /// since the shadow makes a Rectangle's window opaque (ADR-016).
    pub shadow_ok: bool,
}

impl Look {
    /// The toast as it is drawn this frame, in the window's space.
    pub fn drawn(&self) -> egui::Rect {
        self.transform.mul_rect(self.toast)
    }

    /// How much the transform scales, for the corner radii.
    pub fn scale(&self) -> f32 {
        self.transform.scaling
    }

    /// How far past the toast's edge its halo reaches this frame, in points:
    /// the window's region has to take that in, or the margin the halo is
    /// painted in is cut away with the rest.
    pub fn halo_reach(&self) -> f32 {
        (HALO_SPREAD + HALO_BLUR) * self.halo
    }
}

/// The transparent margin `effects` need around the toast, in points per
/// side: room for a pop's overshoot or a glow's halo, and none otherwise.
pub fn margin(enter: Effect, leave: Effect, size: egui::Vec2) -> egui::Vec2 {
    let uses = |effect: Effect| enter == effect || leave == effect;
    let mut margin = egui::Vec2::ZERO;
    if uses(Effect::PopOut) {
        margin = margin.max(size * POP_MARGIN);
    }
    if uses(Effect::Glow) {
        margin = margin.max(egui::Vec2::splat(GLOW_MARGIN));
    }
    margin
}

/// The look of a toast at `presence` (0 gone, 1 fully there, before easing)
/// with its `animation` — its in-effect, or its out-effect while `leaving` —
/// whose resting place is `at` (the toast's top-left on the screen).
///
/// `edge` is the side the toast sits against, which a drawer opens from;
/// `work_area` is what a slide comes in from beyond and is clipped to.
pub fn look(
    animation: Animation,
    leaving: bool,
    presence: f32,
    at: egui::Pos2,
    size: egui::Vec2,
    edge: AccentEdge,
    work_area: egui::Rect,
) -> Look {
    let effect = if leaving {
        animation.leave
    } else {
        animation.enter
    };
    let easing = animation.easing;
    let margin = margin(animation.enter, animation.leave, size);
    let motion = easing.apply(presence);
    let fade = easing.apply_alpha(presence);
    let window_size = size + 2.0 * margin;
    let toast = egui::Rect::from_min_size(margin.to_pos2(), size);
    let whole = egui::Rect::from_min_size(egui::Pos2::ZERO, window_size);
    let still = |alpha: f32| Look {
        window: egui::Rect::from_min_size(at - margin, window_size),
        toast,
        transform: egui::emath::TSTransform::IDENTITY,
        clip: whole,
        alpha,
        background: 1.0,
        halo: 0.0,
        dissolve: None,
        shadow_ok: true,
    };
    let settled = presence >= 1.0;

    match effect {
        Effect::SlideFromRight | Effect::SlideFromBottom => {
            // From just beyond the work area's edge to where it belongs; the
            // overshoot of a spring carries it briefly past.
            let travel = if effect == Effect::SlideFromRight {
                egui::vec2(work_area.right() - (at.x - margin.x), 0.0)
            } else {
                egui::vec2(0.0, work_area.bottom() - (at.y - margin.y))
            };
            let window =
                egui::Rect::from_min_size(at - margin + travel * (1.0 - motion), window_size);
            // Whatever lies beyond the work area is not shown: not over the
            // taskbar, not on the monitor next door.
            let clip = window.intersect(work_area).translate(-window.min.to_vec2());
            Look {
                window,
                clip: if clip.is_positive() {
                    clip
                } else {
                    egui::Rect::NOTHING
                },
                ..still(1.0)
            }
        }
        Effect::Drawer => {
            // Out of the edge it sits against: the toast moves, and only the
            // part that is out shows.
            let shown = motion.max(0.0);
            let (shift, clip) = match edge {
                AccentEdge::Right => (
                    egui::vec2(size.x * (1.0 - shown), 0.0),
                    egui::Rect::from_min_max(
                        egui::pos2(toast.right() - size.x * shown, toast.top()),
                        toast.max,
                    ),
                ),
                AccentEdge::Left => (
                    egui::vec2(-size.x * (1.0 - shown), 0.0),
                    egui::Rect::from_min_max(
                        toast.min,
                        egui::pos2(toast.left() + size.x * shown, toast.bottom()),
                    ),
                ),
                AccentEdge::Bottom => (
                    egui::vec2(0.0, size.y * (1.0 - shown)),
                    egui::Rect::from_min_max(
                        egui::pos2(toast.left(), toast.bottom() - size.y * shown),
                        toast.max,
                    ),
                ),
                // The top edge, and the centre, which has bars top and bottom
                // and opens downward like the top.
                AccentEdge::Top | AccentEdge::TopAndBottom => (
                    egui::vec2(0.0, -size.y * (1.0 - shown)),
                    egui::Rect::from_min_max(
                        toast.min,
                        egui::pos2(toast.right(), toast.top() + size.y * shown),
                    ),
                ),
            };
            Look {
                transform: egui::emath::TSTransform::from_translation(shift),
                clip: clip.intersect(whole),
                ..still(1.0)
            }
        }
        Effect::PopOut => {
            // From the centre, with the spring's overshoot inside the margin.
            let scale = (POP_FROM + (1.0 - POP_FROM) * motion).max(0.0);
            let centre = toast.center().to_vec2();
            let transform = egui::emath::TSTransform::new(centre - centre * scale, scale);
            Look {
                transform,
                ..still(fade)
            }
        }
        Effect::Dissolve => Look {
            // Grain by grain; the window is cut to the cells that show.
            dissolve: Some(motion.clamp(0.0, 1.0)),
            ..still(1.0)
        },
        Effect::Glow => Look {
            // A halo in the accent colour that swells while the toast fades
            // in, and has settled to nothing when it has arrived.
            halo: (std::f32::consts::PI * presence.clamp(0.0, 1.0)).sin(),
            shadow_ok: settled,
            ..still(fade)
        },
        Effect::AlphaToSolid => Look {
            // The text is there from the start; the background fades in
            // behind it.
            background: fade,
            shadow_ok: settled,
            ..still(1.0)
        },
        // Appear, and every effect not built yet: a fade.
        _ => still(fade),
    }
}

/// The cells of a dissolve that show at `progress`, in the window's space,
/// over the toast laid out at `toast` — as runs along each row, neighbouring
/// cells joined into one rect.
///
/// Each cell has a fixed noise value; a cell shows once `progress` passes
/// it, so the toast grows grain by grain on the way in and loses the same
/// grains in reverse on the way out. The noise is made once for each size of
/// toast and kept (`noise_for`), never per frame (`docs/specs/toast.md` §5).
pub fn dissolve_cells(toast: egui::Rect, progress: f32) -> Vec<egui::Rect> {
    let columns = (toast.width() / DISSOLVE_CELL).ceil().max(1.0) as usize;
    let rows = (toast.height() / DISSOLVE_CELL).ceil().max(1.0) as usize;
    let noise = noise_for(columns, rows);
    // Neighbouring cells that both show are one rect: the window's region is
    // rebuilt from these on every frame, and a run per row is far fewer
    // rects than a cell each (three dissolving toasts fell just under 60 fps
    // with a rect per cell).
    let mut runs = Vec::with_capacity(rows * 8);
    for row in 0..rows {
        let mut start = None;
        for column in 0..=columns {
            let shows = column < columns && noise[row * columns + column] < progress;
            match (shows, start) {
                (true, None) => start = Some(column),
                (false, Some(first)) => {
                    let min = toast.min + egui::vec2(first as f32, row as f32) * DISSOLVE_CELL;
                    let size = egui::vec2((column - first) as f32, 1.0) * DISSOLVE_CELL;
                    runs.push(egui::Rect::from_min_size(min, size).intersect(toast));
                    start = None;
                }
                _ => {}
            }
        }
    }
    runs
}

/// The dissolve's noise for a `columns` × `rows` grid: one value in [0, 1)
/// per cell, the same every time for the same grid. Made once per grid and
/// shared from then on.
fn noise_for(columns: usize, rows: usize) -> std::sync::Arc<[f32]> {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    /// Each grid's noise, by (columns, rows).
    type Made = HashMap<(usize, usize), Arc<[f32]>>;
    static MADE: Mutex<Option<Made>> = Mutex::new(None);
    let mut made = MADE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let made = made.get_or_insert_with(HashMap::new);
    Arc::clone(made.entry((columns, rows)).or_insert_with(|| {
        // A small integer hash per cell: grain, not a pattern, and nothing
        // random — the same toast dissolves the same way every time.
        (0..columns * rows)
            .map(|index| {
                let mut x = (index as u32).wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
                x ^= x >> 16;
                x = x.wrapping_mul(0x7FEB_352D);
                x ^= x >> 15;
                x = x.wrapping_mul(0x846C_A68B);
                x ^= x >> 16;
                (x >> 8) as f32 / (1u32 << 24) as f32
            })
            .collect()
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Easing;

    const SIZE: egui::Vec2 = egui::vec2(360.0, 152.0);

    fn area() -> egui::Rect {
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1032.0))
    }

    /// The default Bottom Right resting place.
    fn bottom_right() -> egui::Pos2 {
        egui::pos2(1920.0 - 16.0 - SIZE.x, 1032.0 - 16.0 - SIZE.y)
    }

    fn animation(effect: Effect, easing: Easing) -> Animation {
        Animation {
            enter: effect,
            leave: Effect::Appear,
            easing,
            ..Animation::default()
        }
    }

    fn at(effect: Effect, presence: f32, edge: AccentEdge) -> Look {
        look(
            animation(effect, Easing::Linear),
            false,
            presence,
            bottom_right(),
            SIZE,
            edge,
            area(),
        )
    }

    #[test]
    fn every_effect_rests_exactly_where_the_toast_belongs() {
        for effect in Effect::all_out() {
            let look = at(effect, 1.0, AccentEdge::Right);
            let drawn = look.drawn().translate(look.window.min.to_vec2());
            assert!(
                (drawn.min - bottom_right()).length() < 1e-3,
                "{effect:?} rests at {:?}",
                drawn.min
            );
            assert_eq!(drawn.size(), SIZE, "{effect:?} at full size");
            assert_eq!(look.alpha, 1.0, "{effect:?} fully there");
            assert!(
                look.clip.contains_rect(look.drawn()),
                "{effect:?} wholly shown"
            );
        }
    }

    #[test]
    fn a_slide_starts_beyond_the_edge_and_is_clipped_to_the_work_area() {
        let start = at(Effect::SlideFromRight, 0.0, AccentEdge::Right);
        assert_eq!(
            start.window.left(),
            area().right(),
            "just beyond the right edge"
        );
        assert!(!start.clip.is_positive(), "nothing shows yet");

        let half = at(Effect::SlideFromRight, 0.5, AccentEdge::Right);
        let rest = bottom_right().x;
        assert_eq!(half.window.left(), rest + (area().right() - rest) / 2.0);
        let shown = half.clip.translate(half.window.min.to_vec2());
        assert!(
            area().contains_rect(shown),
            "only what is inside the work area"
        );
        assert!(shown.width() < SIZE.x && shown.width() > 0.0);

        let up = at(Effect::SlideFromBottom, 0.0, AccentEdge::Right);
        assert_eq!(
            up.window.top(),
            area().bottom(),
            "just below the work area: not over the taskbar"
        );
        let half_up = at(Effect::SlideFromBottom, 0.5, AccentEdge::Right);
        let shown = half_up.clip.translate(half_up.window.min.to_vec2());
        assert!(shown.bottom() <= area().bottom());
    }

    #[test]
    fn a_drawer_opens_from_the_edge_the_toast_sits_against() {
        // A table of edges: at half open, the visible half is the one against
        // that edge, and the toast has moved out of it by the other half.
        let table = [
            (AccentEdge::Right, egui::vec2(SIZE.x / 2.0, 0.0)),
            (AccentEdge::Left, egui::vec2(-SIZE.x / 2.0, 0.0)),
            (AccentEdge::Bottom, egui::vec2(0.0, SIZE.y / 2.0)),
            (AccentEdge::Top, egui::vec2(0.0, -SIZE.y / 2.0)),
            (AccentEdge::TopAndBottom, egui::vec2(0.0, -SIZE.y / 2.0)),
        ];
        for (edge, shift) in table {
            let look = at(Effect::Drawer, 0.5, edge);
            assert_eq!(look.transform.translation, shift, "{edge:?}");
            assert!(
                (look.clip.area() - SIZE.x * SIZE.y / 2.0).abs() < 1.0,
                "{edge:?} half shown"
            );
            let closed = at(Effect::Drawer, 0.0, edge);
            assert!(closed.clip.area() < 1.0, "{edge:?} shut at the start");
        }
        let right = at(Effect::Drawer, 0.5, AccentEdge::Right);
        assert_eq!(
            right.clip.right(),
            SIZE.x,
            "the half against the right edge"
        );
    }

    #[test]
    fn a_pop_scales_from_the_centre_and_its_overshoot_stays_in_the_window() {
        let margin = margin(Effect::PopOut, Effect::Appear, SIZE);
        assert!(margin.x > 0.0 && margin.y > 0.0);
        let look_at = |easing: Easing, presence: f32| {
            look(
                animation(Effect::PopOut, easing),
                false,
                presence,
                bottom_right(),
                SIZE,
                AccentEdge::Right,
                area(),
            )
        };
        let start = look_at(Easing::Linear, 0.0);
        assert_eq!(start.scale(), POP_FROM);
        assert_eq!(
            start.drawn().center(),
            start.toast.center(),
            "from the centre"
        );
        assert_eq!(start.alpha, 0.0);
        let half = look_at(Easing::Linear, 0.5);
        assert!((half.scale() - (POP_FROM + 1.0) / 2.0).abs() < 1e-5);

        // A spring overshoots, and every frame of it fits the window.
        let window = egui::Rect::from_min_size(egui::Pos2::ZERO, start.window.size());
        let mut biggest = 0.0_f32;
        for i in 0..=100 {
            let look = look_at(Easing::Spring, i as f32 / 100.0);
            biggest = biggest.max(look.scale());
            assert!(
                window.expand(0.01).contains_rect(look.drawn()),
                "frame {i} left the window"
            );
            assert!(look.alpha <= 1.0);
        }
        assert!(biggest > 1.05, "the spring overshoots: {biggest}");
    }

    #[test]
    fn appear_and_the_effects_not_built_yet_only_fade() {
        for effect in [
            Effect::Appear,
            Effect::GenieFromBottom,
            Effect::ShrinkToTray,
        ] {
            let look = at(effect, 0.5, AccentEdge::Right);
            assert_eq!(look.alpha, 0.5, "{effect:?}");
            assert_eq!(look.transform, egui::emath::TSTransform::IDENTITY);
            assert_eq!(look.drawn().min + look.window.min.to_vec2(), bottom_right());
        }
    }

    #[test]
    fn at_every_preset_each_effect_starts_hidden_or_small_and_rests_in_place() {
        use crate::placement::{Position, place};
        for preset in Position::presets() {
            let rest = place(area(), SIZE, preset, 0, 8.0);
            let edge = preset.accent_edge();
            for effect in Effect::all_out() {
                let look_at = |presence: f32| {
                    look(
                        animation(effect, Easing::EaseOut),
                        false,
                        presence,
                        rest,
                        SIZE,
                        edge,
                        area(),
                    )
                };
                let end = look_at(1.0);
                let drawn = end.drawn().translate(end.window.min.to_vec2());
                assert!(
                    (drawn.min - rest).length() < 1e-3 && end.clip.contains_rect(end.drawn()),
                    "{effect:?} at {preset:?} does not rest where it belongs"
                );
                let start = look_at(0.0);
                let shown = start.clip.intersect(start.drawn());
                let hidden = !shown.is_positive() || shown.area() < 1.0;
                match effect {
                    Effect::SlideFromRight | Effect::SlideFromBottom | Effect::Drawer => {
                        assert!(
                            hidden,
                            "{effect:?} at {preset:?} shows at its start: {shown:?}"
                        );
                    }
                    Effect::PopOut => assert_eq!(start.scale(), POP_FROM),
                    Effect::Dissolve => assert_eq!(start.dissolve, Some(0.0), "no grain yet"),
                    Effect::AlphaToSolid => {
                        assert_eq!(start.background, 0.0, "no background yet");
                        assert_eq!(start.alpha, 1.0, "the text is already there");
                    }
                    _ => assert_eq!(start.alpha, 0.0, "{effect:?} starts invisible"),
                }
            }
        }
    }

    #[test]
    fn the_dissolve_noise_is_made_once_per_size_and_its_cells_grow_in_order() {
        let toast = egui::Rect::from_min_size(egui::pos2(4.0, 4.0), SIZE);
        let columns = (SIZE.x / DISSOLVE_CELL).ceil() as usize;
        let rows = (SIZE.y / DISSOLVE_CELL).ceil() as usize;
        assert!(
            std::sync::Arc::ptr_eq(&noise_for(columns, rows), &noise_for(columns, rows)),
            "the same grid shares one noise, not one per frame"
        );
        let area = |runs: &[egui::Rect]| runs.iter().map(|run| run.area()).sum::<f32>();
        assert!(
            dissolve_cells(toast, 0.0).is_empty(),
            "nothing at the start"
        );
        let all = dissolve_cells(toast, 1.0);
        assert_eq!(all.len(), rows, "the whole toast: one run per row");
        assert!(
            (area(&all) - toast.area()).abs() < 1.0,
            "together they are the toast"
        );
        assert!(all.iter().all(|run| toast.contains_rect(*run)));
        let half = area(&dissolve_cells(toast, 0.5)) / toast.area();
        assert!((0.4..0.6).contains(&half), "about half at half way: {half}");
        // Runs never overlap, and joining them saves rects.
        let some = dissolve_cells(toast, 0.5);
        for (i, a) in some.iter().enumerate() {
            for b in &some[i + 1..] {
                assert!(a.intersect(*b).area() <= 0.0, "{a:?} overlaps {b:?}");
            }
        }
        let cells_at_half = noise_for(columns, rows)
            .iter()
            .filter(|v| **v < 0.5)
            .count();
        assert!(
            some.len() < cells_at_half,
            "{} runs for {cells_at_half} cells",
            some.len()
        );
        // In order: every run showing at a third still shows at two thirds.
        let third = dissolve_cells(toast, 0.33);
        let two_thirds = dissolve_cells(toast, 0.66);
        for run in &third {
            let centre = run.center();
            assert!(
                two_thirds.iter().any(|r| r.contains(centre)),
                "{run:?} vanished"
            );
        }
    }

    #[test]
    fn the_halo_stays_inside_the_window_and_has_settled_on_arrival() {
        for (enter, leave) in [
            (Effect::Glow, Effect::Glow),
            (Effect::Glow, Effect::PopOut),
            (Effect::PopOut, Effect::Glow),
        ] {
            let animation = Animation {
                enter,
                leave,
                easing: Easing::Spring,
                ..Animation::default()
            };
            for i in 0..=100 {
                let presence = i as f32 / 100.0;
                let glow = look(
                    animation,
                    enter != Effect::Glow,
                    presence,
                    bottom_right(),
                    SIZE,
                    AccentEdge::Right,
                    area(),
                );
                let reach = (HALO_SPREAD + HALO_BLUR) * glow.halo;
                let window = egui::Rect::from_min_size(egui::Pos2::ZERO, glow.window.size());
                assert!(
                    window
                        .expand(0.01)
                        .contains_rect(glow.drawn().expand(reach)),
                    "the halo left the window at {presence} for {enter:?}/{leave:?}"
                );
            }
        }
        let arrived = at(Effect::Glow, 1.0, AccentEdge::Right);
        assert!(arrived.halo.abs() < 1e-5, "no halo once it has arrived");
        let midway = at(Effect::Glow, 0.5, AccentEdge::Right);
        assert!(midway.halo > 0.99, "fullest half way");
        assert!(!midway.shadow_ok, "no shadow behind a halo");
        assert!(arrived.shadow_ok);
    }

    #[test]
    fn alpha_to_solid_keeps_its_text_and_ends_at_the_toasts_own_opacity() {
        for i in 0..=10 {
            let look = at(Effect::AlphaToSolid, i as f32 / 10.0, AccentEdge::Right);
            assert_eq!(look.alpha, 1.0, "the text never fades");
            assert!(
                (look.background - i as f32 / 10.0).abs() < 1e-5,
                "the background does"
            );
        }
        let end = at(Effect::AlphaToSolid, 1.0, AccentEdge::Right);
        assert_eq!(
            (end.alpha, end.background),
            (1.0, 1.0),
            "solid: the toast's own opacity applies"
        );
        assert!(end.shadow_ok);
        assert!(!at(Effect::AlphaToSolid, 0.5, AccentEdge::Right).shadow_ok);
    }

    #[test]
    fn only_a_pop_needs_a_margin() {
        assert_eq!(
            margin(Effect::SlideFromRight, Effect::Drawer, SIZE),
            egui::Vec2::ZERO
        );
        assert!(
            margin(Effect::Appear, Effect::PopOut, SIZE).x > 0.0,
            "a pop on the way out too"
        );
        assert_eq!(
            margin(Effect::Glow, Effect::Appear, SIZE),
            egui::Vec2::splat(GLOW_MARGIN)
        );
    }
}
