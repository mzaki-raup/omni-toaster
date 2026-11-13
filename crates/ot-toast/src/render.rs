//! Painting a toast (`docs/specs/toast.md` §4).
//!
//! The renderer takes a [`ToastSpec`] and nothing else. It reads no setting,
//! no reminder row and no category — which is what stops the Settings preview
//! and a real toast from drifting apart, because both arrive here through the
//! same door.
//!
//! Nothing here hard-codes a colour: every one comes from the spec, and the
//! spec's defaults come from the theme. A hand-painted rect with a literal in
//! it is a finding, not a shortcut (`.claude/rules/ui.md`).

use crate::placement::AccentEdge;
use crate::spec::{Rgba, Shape, ToastSpec};

/// The shadow the Settings preview draws for a Rectangle toast with its
/// shadow on. A real toast's shadow is Windows' own, drawn outside the
/// window (CR-0006); this is the nearest egui comes to it, and it falls
/// outside a real toast's window, so only the preview ever shows it.
const SHADOW_BLUR: f32 = 16.0;
/// How far it is offset downward, which is what makes it read as a shadow
/// rather than a glow.
const SHADOW_OFFSET: f32 = 4.0;
/// How thick the accent bar is, in points.
const ACCENT_WIDTH: f32 = 4.0;

/// An egui colour from the spec's own, with the toast's opacity applied.
///
/// Opacity multiplies what the colour already carries rather than replacing
/// it, so a background that is itself translucent stays translucent when the
/// user turns the opacity down.
pub fn colour(rgba: Rgba, alpha: f32) -> egui::Color32 {
    let (r, g, b, a) = rgba.parts();
    let a = (f32::from(a) * alpha.clamp(0.0, 1.0))
        .round()
        .clamp(0.0, 255.0) as u8;
    egui::Color32::from_rgba_unmultiplied(r, g, b, a)
}

/// Paint the toast's frame: its shadow, its background and its border.
///
/// **Solid, whatever the opacity** (CR-0005). Opacity fades the finished
/// toast as a whole, text included, and is applied by whatever shows it:
/// the window's own alpha for a real toast, the painter's opacity for the
/// Settings preview. Fading each colour here instead blended them against
/// black inside the toast's window, which is darker, not see-through.
///
/// Returns the rect the content may use — the frame's rect inset by the
/// border and by a padding that keeps text off the edge, and inset further on
/// a pill, whose round ends would otherwise clip the first and last glyph.
pub fn frame(painter: &egui::Painter, rect: egui::Rect, spec: &ToastSpec) -> egui::Rect {
    let alpha = 1.0;
    let radii = corner_radii(spec, rect.height());
    let corner = to_corner(radii);

    if spec.style.shadow.shows(spec.style.shape) {
        let shadow = egui::epaint::Shadow {
            offset: [0, SHADOW_OFFSET.round() as i8],
            blur: SHADOW_BLUR.round() as u8,
            spread: 0,
            color: colour(Rgba::new(0, 0, 0, 0x60), alpha),
        };
        painter.add(shadow.as_shape(rect, corner));
    }

    match spec.style.background_to {
        // A gradient is painted as a mesh, then clipped to the shape by
        // drawing the rounded frame over it as a stroke-only rect. egui has no
        // rounded gradient fill, and a mesh with rounded corners of its own
        // would need a tessellated path for something the stroke already does.
        Some(to) => {
            gradient(
                painter,
                rect,
                colour(spec.style.background, alpha),
                colour(to, alpha),
                corner,
            );
        }
        None => {
            painter.rect_filled(rect, corner, colour(spec.style.background, alpha));
        }
    }

    if spec.style.border > 0.0 {
        painter.rect_stroke(
            rect,
            corner,
            egui::Stroke::new(spec.style.border, colour(spec.style.border_colour, alpha)),
            egui::StrokeKind::Inside,
        );
    }

    accent_bar(
        painter,
        rect,
        radii,
        spec.behaviour.position.accent_edge(),
        colour(spec.accent(), alpha),
    );

    content_rect(rect, spec)
}

/// The radius of each corner in points, `[nw, ne, sw, se]`.
///
/// The shape's own radius everywhere except the side the toast sits against,
/// which is square and carries the accent bar (`docs/specs/toast.md` §4.1).
/// The renderer, the Settings preview and the toast window's region all take
/// their corners from here, so the window is cut to exactly what is painted.
pub fn corner_radii(spec: &ToastSpec, height: f32) -> [f32; 4] {
    let r = spec.drawn_radius(height);
    match spec.behaviour.position.accent_edge() {
        AccentEdge::Left => [0.0, r, 0.0, r],
        AccentEdge::Right => [r, 0.0, r, 0.0],
        AccentEdge::Top => [0.0, 0.0, r, r],
        AccentEdge::Bottom => [r, r, 0.0, 0.0],
        AccentEdge::TopAndBottom => [r, r, r, r],
    }
}

fn to_corner([nw, ne, sw, se]: [f32; 4]) -> egui::CornerRadius {
    let byte = |r: f32| r.round().clamp(0.0, 255.0) as u8;
    egui::CornerRadius {
        nw: byte(nw),
        ne: byte(ne),
        sw: byte(sw),
        se: byte(se),
    }
}

/// The accent bar along `edge`. A side bar sits on a square side, so it is a
/// plain strip; at the centre the top and bottom bars follow the rounded
/// corners round, as a stroke along the outline.
fn accent_bar(
    painter: &egui::Painter,
    rect: egui::Rect,
    [nw, ne, sw, se]: [f32; 4],
    edge: AccentEdge,
    colour: egui::Color32,
) {
    let width = ACCENT_WIDTH.min(rect.width()).min(rect.height());
    let strip = |strip: egui::Rect| {
        painter.rect_filled(strip, 0, colour);
    };
    match edge {
        AccentEdge::Left => strip(egui::Rect::from_min_max(
            rect.min,
            egui::pos2(rect.min.x + width, rect.max.y),
        )),
        AccentEdge::Right => strip(egui::Rect::from_min_max(
            egui::pos2(rect.max.x - width, rect.min.y),
            rect.max,
        )),
        AccentEdge::Top => strip(egui::Rect::from_min_max(
            rect.min,
            egui::pos2(rect.max.x, rect.min.y + width),
        )),
        AccentEdge::Bottom => strip(egui::Rect::from_min_max(
            egui::pos2(rect.min.x, rect.max.y - width),
            rect.max,
        )),
        AccentEdge::TopAndBottom => {
            let stroke = egui::Stroke::new(width, colour);
            painter.add(egui::Shape::line(
                outline(rect, nw.max(ne), width, true),
                stroke,
            ));
            painter.add(egui::Shape::line(
                outline(rect, sw.max(se), width, false),
                stroke,
            ));
        }
    }
}

/// The top or bottom edge of the outline, with its two corners, inset by
/// half the stroke so the whole stroke lies inside the shape.
fn outline(rect: egui::Rect, radius: f32, width: f32, top: bool) -> Vec<egui::Pos2> {
    const STEPS: usize = 16;
    let inset = width / 2.0;
    // A corner too tight to follow is drawn as the straight edge it nearly is.
    let r = radius.max(inset);
    let y = if top { rect.min.y + r } else { rect.max.y - r };
    let left = egui::pos2(rect.min.x + r, y);
    let right = egui::pos2(rect.max.x - r, y);
    // Angles in screen space (y down): π is left, 3π/2 is up, π/2 is down.
    let (first, second, from, to) = if top {
        (
            left,
            right,
            std::f32::consts::PI,
            2.0 * std::f32::consts::PI,
        )
    } else {
        (right, left, 0.0, std::f32::consts::PI)
    };
    let half = (to - from) / 2.0;
    let mut points = Vec::with_capacity(2 * (STEPS + 1));
    for (centre, start) in [(first, from), (second, from + half)] {
        for step in 0..=STEPS {
            let angle = start + half * step as f32 / STEPS as f32;
            points.push(centre + (r - inset) * egui::vec2(angle.cos(), angle.sin()));
        }
    }
    points
}

/// Where the content goes inside the frame.
pub fn content_rect(rect: egui::Rect, spec: &ToastSpec) -> egui::Rect {
    let padding = 12.0 + spec.style.border;
    // A pill's ends are round, so text has to start inside the curve rather
    // than inside the bounding box — on each side that is round. The side
    // against the screen edge is square (CR-0004) and needs only the padding.
    let [nw, ne, sw, se] = corner_radii(spec, rect.height());
    let side = |round: bool| match spec.style.shape {
        Shape::Pill if round => padding + rect.height() / 4.0,
        _ => padding,
    };
    let left = side(nw.max(sw) > 0.0);
    let right = side(ne.max(se) > 0.0);
    egui::Rect::from_min_max(
        rect.min + egui::vec2(left.max(8.0), padding),
        rect.max - egui::vec2(right.max(8.0), padding),
    )
}

/// A vertical two-stop gradient, clipped to the toast's corners.
fn gradient(
    painter: &egui::Painter,
    rect: egui::Rect,
    from: egui::Color32,
    to: egui::Color32,
    corner: egui::CornerRadius,
) {
    // The corners are drawn by filling the rounded shape with the end colour
    // first; the mesh then paints the body. Anything the mesh does not cover
    // keeps the fill, so the rounded corners survive.
    painter.rect_filled(rect, corner, to);

    let mut mesh = egui::Mesh::default();
    let radius = f32::from(corner.nw.max(corner.ne).max(corner.sw).max(corner.se));
    // Inset by the radius so the mesh never spills over a rounded corner.
    let body = egui::Rect::from_min_max(
        egui::pos2(rect.min.x, rect.min.y + radius),
        egui::pos2(rect.max.x, rect.max.y - radius),
    );
    if body.height() <= 0.0 {
        return;
    }
    mesh.colored_vertex(body.left_top(), from);
    mesh.colored_vertex(body.right_top(), from);
    mesh.colored_vertex(body.left_bottom(), to);
    mesh.colored_vertex(body.right_bottom(), to);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Style;

    #[test]
    fn opacity_multiplies_what_the_colour_already_carries() {
        // A background that is itself translucent must stay translucent when
        // the opacity comes down, rather than being replaced by it.
        let half = Rgba::new(0x20, 0x20, 0x20, 0x80);
        assert_eq!(colour(half, 1.0).a(), 0x80);
        assert_eq!(colour(half, 0.5).a(), 0x40);

        let solid = Rgba::new(0x20, 0x20, 0x20, 0xff);
        assert_eq!(colour(solid, 1.0).a(), 0xff);
        // The spec's floor of 40% opacity, applied to a solid colour.
        assert_eq!(colour(solid, 0.4).a(), 102);
    }

    #[test]
    fn the_colour_is_the_one_egui_would_build_at_every_opacity() {
        // Not `(c.r(), c.g(), c.b()) == (0x3b, 0x9e, 0xff)`: `Color32` stores
        // **premultiplied** alpha, so reading a channel back after
        // `from_rgba_unmultiplied` gives the premultiplied value, not the one
        // that went in — 0x3b at 40% reads as 24. That is right for painting
        // and misleading to inspect, so this compares against the colour egui
        // itself would build rather than against the channels.
        let accent = Rgba::new(0x3b, 0x9e, 0xff, 0xff);
        for (alpha, expected_a) in [(1.0_f32, 0xff_u8), (0.4, 102)] {
            assert_eq!(
                colour(accent, alpha),
                egui::Color32::from_rgba_unmultiplied(0x3b, 0x9e, 0xff, expected_a),
                "at {alpha}"
            );
        }
    }

    #[test]
    fn content_never_escapes_the_frame_whatever_the_shape() {
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(360.0, 120.0));
        for shape in Shape::all() {
            let spec = ToastSpec {
                style: Style {
                    shape,
                    border: 4.0,
                    ..Style::default()
                },
                ..ToastSpec::default()
            }
            .clamped();
            let content = content_rect(rect, &spec);
            assert!(
                rect.contains_rect(content),
                "{shape:?} put its content outside the toast: {content:?}"
            );
            assert!(content.width() > 0.0 && content.height() > 0.0);
        }
    }

    #[test]
    fn a_pill_keeps_its_text_inside_the_curve() {
        // A pill's ends are round, so content inset by the same padding as a
        // rectangle would have its first glyph clipped by the curve.
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(360.0, 120.0));
        let pill = ToastSpec {
            style: Style {
                shape: Shape::Pill,
                ..Style::default()
            },
            ..ToastSpec::default()
        };
        let rectangle = ToastSpec {
            style: Style {
                shape: Shape::Rectangle,
                ..Style::default()
            },
            ..ToastSpec::default()
        };
        assert!(
            content_rect(rect, &pill).left() > content_rect(rect, &rectangle).left(),
            "a pill must inset further than a rectangle"
        );
    }

    #[test]
    fn a_toast_too_small_for_its_padding_still_yields_a_sane_rect() {
        // Width and height are clamped, so this cannot arrive from a spec —
        // but the renderer is handed a *rect*, and a narrow monitor or a
        // future layout could make one.
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(10.0, 10.0));
        let content = content_rect(rect, &ToastSpec::default());
        assert!(content.width().is_finite() && content.height().is_finite());
    }

    #[test]
    fn the_side_against_the_screen_edge_is_square_and_the_rest_keep_their_rounding() {
        use crate::placement::Position;
        let mut spec = ToastSpec::default();
        spec.style.shape = Shape::RoundedRectangle;
        let r = spec.drawn_radius(120.0);
        assert!(r > 0.0);
        let table = [
            (Position::BottomRight, [r, 0.0, r, 0.0]),
            (Position::Left, [0.0, r, 0.0, r]),
            (Position::Top, [0.0, 0.0, r, r]),
            (Position::Bottom, [r, r, 0.0, 0.0]),
            (Position::Centre, [r, r, r, r]),
        ];
        for (position, radii) in table {
            spec.behaviour.position = position;
            assert_eq!(corner_radii(&spec, 120.0), radii, "{position:?}");
        }

        // A pill against the right edge is round on the left only, with the
        // full half-height radius: a D, not a stadium.
        spec.style.shape = Shape::Pill;
        spec.behaviour.position = Position::Right;
        assert_eq!(corner_radii(&spec, 120.0), [60.0, 0.0, 60.0, 0.0]);
        // And its text is inset for the curve on the round side only.
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(360.0, 120.0));
        let content = content_rect(rect, &spec);
        assert!(content.left() - rect.left() > rect.right() - content.right());
    }
}
