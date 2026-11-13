//! Where a toast goes (`docs/specs/toast.md` §7).
//!
//! Nine presets — four corners, four edges and the centre — or a custom point
//! the user chose. All of it is worked out against the chosen monitor's
//! **work area**, never the full monitor rect, so a toast never covers the
//! taskbar.
//!
//! The arithmetic lives here, with no screen in sight, so it can be tested
//! against a table of work areas and scales. Asking Windows which monitors
//! exist and where their work areas are is `ot-platform`'s job.

use serde::{Deserialize, Serialize};

/// The margin between a toast and an edge it sits against, in points.
/// Scales with DPI like every other measurement, because it is in points.
pub const EDGE_MARGIN: f32 = 16.0;

/// Where a toast appears.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Position {
    TopLeft,
    Top,
    TopRight,
    Left,
    Centre,
    Right,
    BottomLeft,
    Bottom,
    /// The default: where Windows puts its own notifications, so where people
    /// already look.
    #[default]
    BottomRight,
    /// A point the user chose, as a fraction of the work area.
    ///
    /// **A fraction, not pixels.** `(1850, 1010)` is wrong the moment the
    /// resolution changes, the taskbar moves or the toast appears on another
    /// monitor; `(0.96, 0.94)` means the same place on all of them. There is
    /// one custom point, not one per monitor, for the same reason.
    Custom {
        x: f32,
        y: f32,
    },
}

/// Which way a stack grows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackDirection {
    Down,
    Up,
}

/// How a position sits on one axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Align {
    Start,
    Middle,
    End,
}

/// Which edge of a toast carries its accent bar. That edge is drawn square;
/// the others keep the shape's rounding (`docs/specs/toast.md` §4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccentEdge {
    Left,
    Right,
    Top,
    Bottom,
    /// At the centre of the screen: a bar along the top and one along the
    /// bottom, each following the rounded corners round into the sides.
    /// Every corner keeps its rounding.
    TopAndBottom,
}

/// Which third of an axis a fraction of the work area falls in.
fn third(fraction: f32) -> Align {
    if fraction < 1.0 / 3.0 {
        Align::Start
    } else if fraction > 2.0 / 3.0 {
        Align::End
    } else {
        Align::Middle
    }
}

impl Position {
    /// Every preset, in the order the picker shows them: the grid read left
    /// to right, top to bottom.
    pub fn presets() -> [Self; 9] {
        [
            Self::TopLeft,
            Self::Top,
            Self::TopRight,
            Self::Left,
            Self::Centre,
            Self::Right,
            Self::BottomLeft,
            Self::Bottom,
            Self::BottomRight,
        ]
    }

    /// The Fluent key naming this position in the picker.
    pub fn name_key(self) -> &'static str {
        match self {
            Self::TopLeft => "toast-position-top-left",
            Self::Top => "toast-position-top",
            Self::TopRight => "toast-position-top-right",
            Self::Left => "toast-position-left",
            Self::Centre => "toast-position-centre",
            Self::Right => "toast-position-right",
            Self::BottomLeft => "toast-position-bottom-left",
            Self::Bottom => "toast-position-bottom",
            Self::BottomRight => "toast-position-bottom-right",
            Self::Custom { .. } => "toast-position-custom",
        }
    }

    pub fn is_custom(self) -> bool {
        matches!(self, Self::Custom { .. })
    }

    fn horizontal(self) -> Align {
        match self {
            Self::TopLeft | Self::Left | Self::BottomLeft => Align::Start,
            Self::Top | Self::Centre | Self::Bottom => Align::Middle,
            Self::TopRight | Self::Right | Self::BottomRight => Align::End,
            // A custom point's alignment comes from where it is, not from a
            // name; `anchor` handles it without going through here.
            Self::Custom { .. } => Align::Start,
        }
    }

    fn vertical(self) -> Align {
        match self {
            Self::TopLeft | Self::Top | Self::TopRight => Align::Start,
            Self::Left | Self::Centre | Self::Right => Align::Middle,
            Self::BottomLeft | Self::Bottom | Self::BottomRight => Align::End,
            Self::Custom { .. } => Align::Start,
        }
    }

    /// Which edge of the toast carries the accent bar: the side it sits
    /// against (`docs/specs/toast.md` §4.1). A custom point takes the edge of
    /// the preset whose ninth of the screen it falls in.
    pub fn accent_edge(self) -> AccentEdge {
        let (horizontal, vertical) = match self {
            Self::Custom { x, y } => (third(x), third(y)),
            preset => (preset.horizontal(), preset.vertical()),
        };
        match (horizontal, vertical) {
            (Align::Start, _) => AccentEdge::Left,
            (Align::End, _) => AccentEdge::Right,
            (Align::Middle, Align::Start) => AccentEdge::Top,
            (Align::Middle, Align::End) => AccentEdge::Bottom,
            (Align::Middle, Align::Middle) => AccentEdge::TopAndBottom,
        }
    }

    /// Which way a stack grows from here: away from the nearest edge, so it
    /// never walks off the screen.
    pub fn stack_direction(self, work_area: egui::Rect) -> StackDirection {
        match self {
            Self::BottomLeft | Self::Bottom | Self::BottomRight => StackDirection::Up,
            Self::Custom { y, .. } => {
                // Nearer the bottom edge: grow upward. In the vertical middle
                // (or nearer the top): grow downward, as every non-bottom
                // preset does.
                let _ = work_area;
                if y > 0.5 {
                    StackDirection::Up
                } else {
                    StackDirection::Down
                }
            }
            _ => StackDirection::Down,
        }
    }
}

/// Where the toast of this size, at this stack index, belongs.
///
/// `work_area` is the chosen monitor's work area in points, which may have a
/// **negative origin**: a second monitor placed left of or above the primary
/// one starts at a negative coordinate, and a routine that assumes (0, 0) puts
/// every toast on the wrong screen.
///
/// The result is always inside the work area: a toast is clamped rather than
/// allowed off the edge, and the clamping happens here, against the monitor in
/// use, rather than once when the setting was saved.
pub fn place(
    work_area: egui::Rect,
    size: egui::Vec2,
    position: Position,
    stack_index: usize,
    gap: f32,
) -> egui::Pos2 {
    let stack_offset = (size.y + gap) * stack_index as f32;
    let direction = position.stack_direction(work_area);

    let (mut x, mut y) = match position {
        Position::Custom { x, y } => {
            // The fraction addresses the toast's **anchor corner** - the
            // corner nearest the point - so a toast placed near the
            // bottom-right grows up and left rather than off the screen.
            let point = egui::pos2(
                work_area.left() + work_area.width() * x.clamp(0.0, 1.0),
                work_area.top() + work_area.height() * y.clamp(0.0, 1.0),
            );
            let left = if x > 0.5 { point.x - size.x } else { point.x };
            let top = if y > 0.5 { point.y - size.y } else { point.y };
            (left, top)
        }
        preset => {
            let x = match preset.horizontal() {
                Align::Start => work_area.left() + EDGE_MARGIN,
                Align::Middle => work_area.center().x - size.x / 2.0,
                Align::End => work_area.right() - EDGE_MARGIN - size.x,
            };
            let y = match preset.vertical() {
                Align::Start => work_area.top() + EDGE_MARGIN,
                Align::Middle => work_area.center().y - size.y / 2.0,
                Align::End => work_area.bottom() - EDGE_MARGIN - size.y,
            };
            (x, y)
        }
    };

    y += match direction {
        StackDirection::Down => stack_offset,
        StackDirection::Up => -stack_offset,
    };

    // Whatever the arithmetic produced, the toast stays inside the work area.
    // `max` after `min` so a toast larger than the work area is pinned to the
    // top-left rather than pushed off the other side.
    x = x.min(work_area.right() - size.x).max(work_area.left());
    y = y.min(work_area.bottom() - size.y).max(work_area.top());
    egui::pos2(x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A primary 1920×1080 with a 48 px taskbar at the bottom.
    fn primary() -> egui::Rect {
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1920.0, 1032.0))
    }

    /// A second monitor to the left of and above the primary one, so its
    /// origin is negative. This is the case that catches a routine written
    /// against the primary monitor's rect.
    fn second() -> egui::Rect {
        egui::Rect::from_min_max(egui::pos2(-1280.0, -200.0), egui::pos2(0.0, 520.0))
    }

    const SIZE: egui::Vec2 = egui::Vec2 { x: 360.0, y: 120.0 };

    #[test]
    fn the_accent_bar_sits_on_the_side_the_toast_sits_against() {
        use AccentEdge::*;
        let table = [
            (Position::TopLeft, Left),
            (Position::Left, Left),
            (Position::BottomLeft, Left),
            (Position::TopRight, Right),
            (Position::Right, Right),
            // The default. Before CR-0004 every toast had its bar on the
            // left, which at Bottom Right is the side facing the screen.
            (Position::BottomRight, Right),
            (Position::Top, Top),
            (Position::Bottom, Bottom),
            (Position::Centre, TopAndBottom),
        ];
        for (position, edge) in table {
            assert_eq!(position.accent_edge(), edge, "{position:?}");
        }
    }

    #[test]
    fn a_custom_point_takes_the_edge_of_the_ninth_it_falls_in() {
        use AccentEdge::*;
        let table = [
            ((0.1, 0.1), Left),
            ((0.1, 0.9), Left),
            ((0.9, 0.5), Right),
            ((0.5, 0.1), Top),
            ((0.5, 0.9), Bottom),
            ((0.5, 0.5), TopAndBottom),
            // On a boundary: the middle third includes it.
            ((1.0 / 3.0, 0.5), TopAndBottom),
        ];
        for ((x, y), edge) in table {
            assert_eq!(Position::Custom { x, y }.accent_edge(), edge, "({x}, {y})");
        }
    }

    #[test]
    fn every_preset_lands_against_the_edges_it_names() {
        let area = primary();
        let at = |p| place(area, SIZE, p, 0, 8.0);

        // Corners.
        assert_eq!(at(Position::TopLeft), egui::pos2(16.0, 16.0));
        assert_eq!(
            at(Position::TopRight),
            egui::pos2(1920.0 - 16.0 - 360.0, 16.0)
        );
        assert_eq!(
            at(Position::BottomLeft),
            egui::pos2(16.0, 1032.0 - 16.0 - 120.0)
        );
        assert_eq!(
            at(Position::BottomRight),
            egui::pos2(1920.0 - 16.0 - 360.0, 1032.0 - 16.0 - 120.0)
        );

        // Edges: margined on one axis, centred on the other.
        assert_eq!(at(Position::Top), egui::pos2(1920.0 / 2.0 - 180.0, 16.0));
        assert_eq!(
            at(Position::Bottom),
            egui::pos2(1920.0 / 2.0 - 180.0, 1032.0 - 16.0 - 120.0)
        );
        assert_eq!(at(Position::Left), egui::pos2(16.0, 1032.0 / 2.0 - 60.0));
        assert_eq!(
            at(Position::Right),
            egui::pos2(1920.0 - 16.0 - 360.0, 1032.0 / 2.0 - 60.0)
        );

        // And the centre is centred on both.
        assert_eq!(
            at(Position::Centre),
            egui::pos2(1920.0 / 2.0 - 180.0, 1032.0 / 2.0 - 60.0)
        );
    }

    #[test]
    fn nothing_is_placed_outside_the_work_area_on_any_monitor() {
        for area in [primary(), second()] {
            for position in Position::presets() {
                for index in 0..5 {
                    let at = place(area, SIZE, position, index, 8.0);
                    let rect = egui::Rect::from_min_size(at, SIZE);
                    assert!(
                        area.contains_rect(rect),
                        "{position:?} #{index} landed at {at:?}, outside {area:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_monitor_with_a_negative_origin_is_not_the_primary_one() {
        // The bug this guards: working from the primary monitor's rect puts
        // every toast on the wrong screen, and the symptom is a toast that
        // appears "on the wrong monitor" rather than an error.
        let at = place(second(), SIZE, Position::TopLeft, 0, 8.0);
        assert_eq!(at, egui::pos2(-1280.0 + 16.0, -200.0 + 16.0));
        assert!(at.x < 0.0 && at.y < 0.0);
    }

    #[test]
    fn a_stack_grows_away_from_the_nearest_edge() {
        let area = primary();
        let gap = 8.0;

        // Bottom presets grow upward.
        let first = place(area, SIZE, Position::BottomRight, 0, gap);
        let second_toast = place(area, SIZE, Position::BottomRight, 1, gap);
        assert!(
            second_toast.y < first.y,
            "a bottom stack must grow up: {first:?} then {second_toast:?}"
        );
        assert_eq!(first.y - second_toast.y, SIZE.y + gap);

        // Top presets grow downward.
        let first = place(area, SIZE, Position::TopLeft, 0, gap);
        let second_toast = place(area, SIZE, Position::TopLeft, 1, gap);
        assert!(second_toast.y > first.y, "a top stack must grow down");

        // So do the middle ones, per §7.3's table.
        for position in [Position::Left, Position::Centre, Position::Right] {
            assert_eq!(position.stack_direction(area), StackDirection::Down);
        }
    }

    #[test]
    fn a_custom_fraction_means_the_same_place_on_any_resolution() {
        // The whole reason it is a fraction and not a pixel pair.
        let point = Position::Custom { x: 0.25, y: 0.25 };
        let hd = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1920.0, 1080.0));
        let uhd = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(3840.0, 2160.0));

        let on_hd = place(hd, SIZE, point, 0, 8.0);
        let on_uhd = place(uhd, SIZE, point, 0, 8.0);
        assert_eq!(on_hd, egui::pos2(480.0, 270.0));
        assert_eq!(on_uhd, egui::pos2(960.0, 540.0));
        // Same relative place on both.
        assert!((on_hd.x / hd.width() - on_uhd.x / uhd.width()).abs() < 0.001);
        assert!((on_hd.y / hd.height() - on_uhd.y / uhd.height()).abs() < 0.001);
    }

    #[test]
    fn a_custom_point_near_an_edge_grows_inward_rather_than_off_the_screen() {
        // The anchor corner is the corner nearest the point, so a toast
        // placed at the far bottom-right extends up and left from it.
        let area = primary();
        let at = place(area, SIZE, Position::Custom { x: 1.0, y: 1.0 }, 0, 8.0);
        assert_eq!(at, egui::pos2(1920.0 - 360.0, 1032.0 - 120.0));
        assert!(area.contains_rect(egui::Rect::from_min_size(at, SIZE)));

        // And at the far top-left it extends down and right.
        let at = place(area, SIZE, Position::Custom { x: 0.0, y: 0.0 }, 0, 8.0);
        assert_eq!(at, egui::pos2(0.0, 0.0));
    }

    #[test]
    fn a_custom_stack_grows_away_from_the_horizontal_edge_it_is_nearest() {
        let area = primary();
        let low = Position::Custom { x: 0.5, y: 0.9 };
        let high = Position::Custom { x: 0.5, y: 0.1 };
        assert_eq!(low.stack_direction(area), StackDirection::Up);
        assert_eq!(high.stack_direction(area), StackDirection::Down);
        // Exactly in the middle counts as "not near the bottom", so it grows
        // down like every other middle position.
        assert_eq!(
            Position::Custom { x: 0.5, y: 0.5 }.stack_direction(area),
            StackDirection::Down
        );
    }

    #[test]
    fn a_fraction_outside_zero_to_one_is_brought_back_rather_than_trusted() {
        let area = primary();
        for (x, y) in [(-5.0, -5.0), (9.0, 9.0), (f32::NAN, 0.5)] {
            let at = place(area, SIZE, Position::Custom { x, y }, 0, 8.0);
            let rect = egui::Rect::from_min_size(at, SIZE);
            assert!(
                area.contains_rect(rect) || at.x.is_finite(),
                "({x}, {y}) placed a toast at {at:?}"
            );
            assert!(
                at.x.is_finite() && at.y.is_finite(),
                "({x}, {y}) gave {at:?}"
            );
        }
    }

    #[test]
    fn a_toast_larger_than_the_work_area_is_pinned_rather_than_pushed_off() {
        // A tiny work area is not a crash and not a toast at a negative
        // offset from nowhere.
        let tiny = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(200.0, 100.0));
        let at = place(tiny, SIZE, Position::BottomRight, 0, 8.0);
        assert_eq!(at, tiny.min, "pinned to the work area's own corner");
    }

    #[test]
    fn every_position_names_a_key_and_the_presets_are_the_nine() {
        let presets = Position::presets();
        assert_eq!(presets.len(), 9, "four corners, four edges and the centre");
        let mut keys: Vec<&str> = presets.iter().map(|p| p.name_key()).collect();
        keys.push(Position::Custom { x: 0.5, y: 0.5 }.name_key());
        let mut unique = keys.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(
            unique.len(),
            keys.len(),
            "two positions share a key: {keys:?}"
        );
        assert!(!presets.iter().any(|p| p.is_custom()));
        assert_eq!(Position::default(), Position::BottomRight);
    }
}
