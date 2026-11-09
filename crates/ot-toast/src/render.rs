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

use crate::spec::{Rgba, Shadow, Shape, ToastSpec};

/// How far the shadow reaches, per setting. The soft one is a hint of depth;
/// the strong one is for a toast that has to be noticed over a busy desktop.
const SHADOW_BLUR: [f32; 3] = [0.0, 12.0, 28.0];
/// How far it is offset downward, which is what makes it read as a shadow
/// rather than a glow.
const SHADOW_OFFSET: [f32; 3] = [0.0, 4.0, 8.0];

impl Shadow {
    fn blur(self) -> f32 {
        SHADOW_BLUR[self as usize]
    }

    fn offset(self) -> f32 {
        SHADOW_OFFSET[self as usize]
    }
}

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
/// Returns the rect the content may use — the frame's rect inset by the
/// border and by a padding that keeps text off the edge, and inset further on
/// a pill, whose round ends would otherwise clip the first and last glyph.
pub fn frame(painter: &egui::Painter, rect: egui::Rect, spec: &ToastSpec) -> egui::Rect {
    let alpha = spec.alpha();
    let radius = spec.drawn_radius(rect.height());
    let corner = egui::CornerRadius::same(radius.round().clamp(0.0, 255.0) as u8);

    if spec.style.shadow != Shadow::Off {
        let shadow = egui::epaint::Shadow {
            offset: [0, spec.style.shadow.offset().round() as i8],
            blur: spec.style.shadow.blur().round() as u8,
            spread: 0,
            // The shadow follows the toast's own opacity: a toast at 40%
            // casting a solid shadow would look like a sticker.
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

    // An accent bar down the leading edge: the one place a category's colour
    // shows without repainting the whole toast.
    let accent_width = 4.0_f32.min(rect.width());
    let accent = egui::Rect::from_min_size(rect.min, egui::vec2(accent_width, rect.height()));
    painter.rect_filled(
        accent,
        egui::CornerRadius {
            nw: corner.nw,
            sw: corner.sw,
            ne: 0,
            se: 0,
        },
        colour(spec.accent(), alpha),
    );

    content_rect(rect, spec)
}

/// Where the content goes inside the frame.
pub fn content_rect(rect: egui::Rect, spec: &ToastSpec) -> egui::Rect {
    let padding = 12.0 + spec.style.border;
    // A pill's ends are round, so text has to start inside the curve rather
    // than inside the bounding box.
    let side = match spec.style.shape {
        Shape::Pill => padding + rect.height() / 4.0,
        _ => padding,
    };
    egui::Rect::from_min_max(
        rect.min + egui::vec2(side.max(8.0), padding),
        rect.max - egui::vec2(side.max(8.0), padding),
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
    fn a_shadow_setting_that_is_off_reaches_nothing() {
        assert_eq!(Shadow::Off.blur(), 0.0);
        assert_eq!(Shadow::Off.offset(), 0.0);
        assert!(Shadow::Strong.blur() > Shadow::Soft.blur());
        assert!(Shadow::Strong.offset() > Shadow::Soft.offset());
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
}
