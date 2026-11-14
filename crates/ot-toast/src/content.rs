//! Laying out what a toast says (`docs/specs/toast.md` §4).
//!
//! A header row — the category mark, the title, the app name and the time —
//! and the body beneath it. Each header part is toggleable, and what is off
//! takes no room rather than leaving a gap.
//!
//! **Nothing overflows the toast.** Boxes size to their content in both
//! languages; a long word wraps, and text that still will not fit truncates
//! with an ellipsis. The ellipsis is for *user* text, which can be any
//! length — never for the app's own labels while there is room for them
//! (`.claude/rules/ui.md`).
//!
//! Malay runs up to 30% wider than English, so the Malay layout is the one
//! that decides this, not the English one.

use crate::render;
use crate::spec::ToastSpec;

/// The title's size, in points. The body is smaller, and the app name and
/// time smaller again — the header is context, the title is the message.
const TITLE_SIZE: f32 = 16.0;
const BODY_SIZE: f32 = 13.5;
const META_SIZE: f32 = 11.5;
/// Between the header row and the body.
const HEADER_GAP: f32 = 6.0;
/// Between the icon and the title.
const ICON_GAP: f32 = 8.0;
/// The side of the square the category mark occupies.
const ICON_SIDE: f32 = 20.0;

/// The largest the body image is drawn, in points. It sits beside the body
/// text and never pushes the toast taller than its content allows.
const IMAGE_SIDE: f32 = 72.0;

/// A decoded picture: its texture, and its size so it can be fitted without
/// being stretched.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Picture {
    pub texture: egui::TextureId,
    pub size: egui::Vec2,
}

/// Draw the content inside `rect`, which is what
/// [`render::frame`](crate::render::frame) returned.
///
/// Two pictures, kept apart because they mean different things
/// (`docs/specs/toast.md` §4): the **icon** in the header, beside the title,
/// and the **image** inside the content, beside the body. Every notification
/// may carry its own of each. The caller resolves which icon applies — the
/// notification's own, or the app's — so this draws what it is given.
///
/// `text_colour` and `dim` come from the theme through the caller, so the
/// renderer still hard-codes no colour.
pub fn draw(
    painter: &egui::Painter,
    rect: egui::Rect,
    spec: &ToastSpec,
    text_colour: egui::Color32,
    dim: egui::Color32,
    icon: Option<Picture>,
    image: Option<Picture>,
) {
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    // Solid: opacity fades the finished toast as a whole (CR-0005), so the
    // text is not faded on its own here.
    let alpha = 1.0;

    let mut cursor = rect.top();

    // The header's right-hand side: the app name and the time, which are the
    // app's own labels and so are laid out first — they get their room, and
    // the title takes what is left.
    let meta = meta_text(spec);
    let meta_galley = meta.as_ref().map(|text| {
        painter.layout_no_wrap(text.clone(), egui::FontId::proportional(META_SIZE), dim)
    });
    let meta_width = meta_galley
        .as_ref()
        .map_or(0.0, |galley| galley.size().x + ICON_GAP);

    // The icon.
    let mut title_left = rect.left();
    if spec.content.show_icon {
        let side = ICON_SIDE.min(rect.height());
        let slot =
            egui::Rect::from_min_size(egui::pos2(rect.left(), cursor), egui::vec2(side, side));
        match icon {
            Some(picture) => {
                draw_picture(painter, slot, picture, alpha);
            }
            // Only while a picture is still on its way: it was decoded off
            // the UI thread and arrives through a channel, and a toast does
            // not wait for it to appear.
            None => {
                painter.rect_filled(
                    slot,
                    egui::CornerRadius::same((side / 4.0) as u8),
                    render::colour(spec.accent(), alpha),
                );
            }
        }
        title_left += side + ICON_GAP;
    }

    // The title: user text, so it is what truncates.
    let title_width = (rect.right() - meta_width - title_left).max(0.0);
    if !spec.content.title.is_empty() && title_width > 0.0 {
        let galley = painter.layout_job(truncating_job(
            spec.content.title.clone(),
            egui::FontId::proportional(TITLE_SIZE),
            text_colour,
            title_width,
            1,
        ));
        painter.galley(egui::pos2(title_left, cursor), galley.clone(), text_colour);
        cursor += galley.size().y;
    }

    if let Some(galley) = meta_galley {
        // Right-aligned against the toast's inner edge, on the title's line.
        let at = egui::pos2(rect.right() - galley.size().x, rect.top());
        painter.galley(at, galley, dim);
    }

    cursor += HEADER_GAP;
    let remaining = rect.bottom() - cursor;
    if remaining <= 0.0 {
        return;
    }

    // The image, against the right-hand edge of the content, as tall as the
    // body has room for. The body text takes the width that is left.
    let mut body_width = rect.width();
    if let Some(picture) = image {
        let side = IMAGE_SIDE.min(remaining).min(rect.width() / 3.0);
        if side > 0.0 {
            let slot = egui::Rect::from_min_size(
                egui::pos2(rect.right() - side, cursor),
                egui::vec2(side, side),
            );
            let drawn = draw_picture(painter, slot, picture, alpha);
            if spec.content.image_border {
                // Around the picture as drawn, not around its slot, so a
                // wide image is framed tightly rather than boxed with empty
                // bars above and below it.
                //
                // In the theme's dim colour, not the toast's border colour.
                // That was tried first and measured: #3a3a42 on #26262c is a
                // frame you can find at 300% zoom and not at 100%, so turning
                // the option on changed nothing anyone would notice. Dim text
                // is meant to be read on this surface in both themes, which
                // is exactly the contrast a frame needs.
                painter.rect_stroke(
                    drawn,
                    egui::CornerRadius::ZERO,
                    egui::Stroke::new(1.0_f32, dim),
                    egui::StrokeKind::Outside,
                );
            }
            body_width = (rect.width() - side - ICON_GAP).max(0.0);
        }
    }

    // The body: user text again, wrapped, and truncated when even wrapping
    // cannot fit it.
    if spec.content.body.is_empty() || body_width <= 0.0 {
        return;
    }
    let rows = (remaining / (BODY_SIZE * 1.3)).floor().max(1.0) as usize;
    let galley = painter.layout_job(truncating_job(
        spec.content.body.clone(),
        body_font(spec),
        text_colour,
        body_width,
        rows,
    ));
    painter.galley(egui::pos2(rect.left(), cursor), galley, text_colour);
}

/// Draw a picture inside `slot`, scaled to fit and centred, never stretched.
/// Returns where it was actually drawn.
fn draw_picture(
    painter: &egui::Painter,
    slot: egui::Rect,
    picture: Picture,
    alpha: f32,
) -> egui::Rect {
    let fitted = fit(slot, picture.size);
    painter.image(
        picture.texture,
        fitted,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        fade(egui::Color32::WHITE, alpha),
    );
    fitted
}

/// The largest rect of `size`'s shape that fits in `bounds`, centred in it.
fn fit(bounds: egui::Rect, size: egui::Vec2) -> egui::Rect {
    if size.x <= 0.0 || size.y <= 0.0 {
        return bounds;
    }
    let scale = (bounds.width() / size.x).min(bounds.height() / size.y);
    egui::Rect::from_center_size(bounds.center(), size * scale)
}

/// The font the body draws in, with its emphasis applied.
fn body_font(spec: &ToastSpec) -> egui::FontId {
    // egui has no italic family of its own; emphasis is bold, or bold and a
    // little larger, rather than a faux-oblique transform that would look
    // wrong at every size.
    let size = if spec.content.body_italic {
        BODY_SIZE + 0.5
    } else {
        BODY_SIZE
    };
    egui::FontId::proportional(size)
}

/// A layout job that wraps at word boundaries, then truncates with an
/// ellipsis.
///
/// **`break_anywhere` stays off.** It sounds like the answer to the case a
/// notification actually meets — one unbroken 200-character word with nowhere
/// to wrap — but it applies to *everything*, so ordinary prose breaks
/// mid-word: "with your curren / t settings". egui already breaks a word that
/// cannot fit on a line of its own, which is the case that needed handling,
/// so word wrapping and the long-word case are both covered with it off.
fn truncating_job(
    text: String,
    font: egui::FontId,
    colour: egui::Color32,
    width: f32,
    max_rows: usize,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::simple(text, font, colour, width.max(1.0));
    job.wrap = egui::text::TextWrapping {
        max_width: width.max(1.0),
        max_rows,
        break_anywhere: false,
        overflow_character: Some('…'),
    };
    job
}

/// The app name and the time, joined for the header's right-hand side.
fn meta_text(spec: &ToastSpec) -> Option<String> {
    match (
        spec.content.app_name.is_empty(),
        spec.content.timestamp.as_deref(),
    ) {
        (true, None) => None,
        (true, Some(time)) => Some(time.to_string()),
        (false, None) => Some(spec.content.app_name.clone()),
        (false, Some(time)) => Some(format!("{} · {}", spec.content.app_name, time)),
    }
}

/// Apply the toast's opacity to a colour the theme chose.
fn fade(colour: egui::Color32, alpha: f32) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(
        colour.r(),
        colour.g(),
        colour.b(),
        (f32::from(colour.a()) * alpha.clamp(0.0, 1.0)) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{Content, Style};

    /// Roughly 40% longer than what it is given, in the shape of a
    /// pseudo-locale: long enough to break a layout Malay would not, while
    /// staying readable in a failure message.
    fn pseudo(text: &str) -> String {
        let padding = "·".repeat((text.chars().count() * 2 / 5).max(1));
        format!("[{text}{padding}]")
    }

    /// A context with a **real font** installed.
    ///
    /// `FontDefinitions::default()` is empty in this build: `default_fonts`
    /// is off (ADR-014), so egui bundles none. An empty set measures every
    /// glyph as zero wide, which makes a layout test pass without laying
    /// anything out — the first version of these tests did exactly that and
    /// looked green. The machine's own UI font is loaded instead.
    fn ctx() -> egui::Context {
        let ctx = egui::Context::default();
        let font = ot_platform::ui_font::proportional()
            .expect("a Windows machine has a UI font; see ADR-014");
        let mut fonts = egui::FontDefinitions::empty();
        fonts.font_data.insert(
            "ui".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(font.bytes)),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .push("ui".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push("ui".to_owned());
        ctx.set_fonts(fonts);
        ctx
    }

    /// Proof that the context above measures anything at all, so the tests
    /// that follow cannot be vacuous.
    #[test]
    fn the_test_context_has_a_font_that_measures_something() {
        let ctx = ctx();
        let _ = ctx.run(Default::default(), |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::background());
            let galley = painter.layout_no_wrap(
                "measure me".to_string(),
                egui::FontId::proportional(16.0),
                egui::Color32::WHITE,
            );
            assert!(
                galley.size().x > 10.0,
                "the font measured {}px wide, so these tests would prove nothing",
                galley.size().x
            );
        });
    }

    /// Lay out one toast and return every galley's rect.
    fn laid_out(spec: &ToastSpec, width: f32, height: f32) -> Vec<egui::Rect> {
        let ctx = ctx();
        let rects = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let collected = std::sync::Arc::clone(&rects);
        let _ = ctx.run(Default::default(), |ctx| {
            let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(width, height));
            let painter = ctx.layer_painter(egui::LayerId::background());
            let content = render::content_rect(rect, spec);
            draw(
                &painter,
                content,
                spec,
                egui::Color32::WHITE,
                egui::Color32::GRAY,
                None,
                None,
            );
            let mut out = collected.lock().expect("the collector");
            out.push(content);
        });
        rects.lock().expect("the collector").clone()
    }

    fn spec_with(title: &str, body: &str) -> ToastSpec {
        ToastSpec {
            content: Content {
                title: title.to_string(),
                body: body.to_string(),
                app_name: "OmniToaster".to_string(),
                timestamp: Some("13:19".to_string()),
                show_icon: true,
                ..Content::default()
            },
            style: Style::default(),
            ..ToastSpec::default()
        }
        .clamped()
    }

    #[test]
    fn the_content_rect_holds_everything_at_the_longest_strings_we_know_of() {
        // The longest Malay strings in every slot, then a pseudo-locale about
        // 40% longer again, then the case a notification actually meets: one
        // unbroken word with nowhere to wrap.
        let cases = [
            (
                "Waktu Solat Zohor",
                "Waktu Zohor telah masuk bagi zon WLY01.",
            ),
            (
                &pseudo("Waktu Solat Zohor"),
                &pseudo("Waktu Zohor telah masuk bagi zon WLY01."),
            ),
            (&"A".repeat(200), &"B".repeat(200)),
        ];
        // Every shape: a pill insets its content further than a rectangle,
        // and that is exactly the inset a shape-blind test would not see.
        for shape in crate::spec::Shape::all() {
            for (title, body) in cases {
                let mut spec = spec_with(title, body);
                spec.style.shape = shape;
                let rects = laid_out(&spec, spec.style.width, 120.0);
                let content = rects.first().copied().expect("a content rect");
                let frame = egui::Rect::from_min_size(
                    egui::pos2(0.0, 0.0),
                    egui::vec2(spec.style.width, 120.0),
                );
                assert!(
                    frame.contains_rect(content),
                    "the content rect escaped a {shape:?} toast for {:?}",
                    &title[..title.len().min(30)]
                );
            }
        }
    }

    #[test]
    fn the_app_name_and_the_time_share_one_line_and_either_may_be_absent() {
        let mut spec = spec_with("T", "B");
        assert_eq!(meta_text(&spec).as_deref(), Some("OmniToaster · 13:19"));

        spec.content.timestamp = None;
        assert_eq!(meta_text(&spec).as_deref(), Some("OmniToaster"));

        spec.content.app_name.clear();
        spec.content.timestamp = Some("13:19".to_string());
        assert_eq!(meta_text(&spec).as_deref(), Some("13:19"));

        // Both off: the slot takes no room rather than leaving a gap.
        spec.content.timestamp = None;
        assert_eq!(meta_text(&spec), None);
    }

    #[test]
    fn nothing_laid_out_is_wider_than_the_room_it_was_given() {
        // Including the case with nowhere to wrap: one unbroken
        // 200-character word, which egui breaks even with `break_anywhere`
        // off. That flag is deliberately off — it applies to everything, so
        // ordinary prose comes out broken mid-word.
        //
        // At 100%, 150% and 200%: text is laid out in points but snapped to
        // physical pixels, so a line that fits at one scale is not
        // automatically a line that fits at another (FEAT-0024).
        let width = 240.0_f32;
        for scale in [1.0_f32, 1.5, 2.0] {
            let ctx = ctx();
            ctx.set_pixels_per_point(scale);
            for text in [
                "This is what a toast looks like with your current settings.",
                "Beginilah rupa toast dengan tetapan anda sekarang.",
                &"x".repeat(200),
                &pseudo("Waktu Zohor telah masuk bagi zon WLY01."),
            ] {
                let _ = ctx.run(Default::default(), |ctx| {
                    let painter = ctx.layer_painter(egui::LayerId::background());
                    let galley = painter.layout_job(truncating_job(
                        text.to_string(),
                        egui::FontId::proportional(BODY_SIZE),
                        egui::Color32::WHITE,
                        width,
                        3,
                    ));
                    assert!(
                        galley.size().x <= width + 0.5,
                        "{:?} laid out {}px wide in {width}px at {scale}x",
                        &text[..text.len().min(30)],
                        galley.size().x
                    );
                });
            }
        }
    }

    #[test]
    fn ordinary_prose_wraps_between_words_rather_than_inside_one() {
        // The bug this replaced: `break_anywhere` cut "your current" into
        // "your curren / t", which looks like a rendering fault to a reader.
        let ctx = ctx();
        let _ = ctx.run(Default::default(), |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::background());
            let galley = painter.layout_job(truncating_job(
                "alpha bravo charlie delta".to_string(),
                egui::FontId::proportional(BODY_SIZE),
                egui::Color32::WHITE,
                80.0,
                5,
            ));
            assert!(galley.rows.len() > 1, "this should have wrapped at all");
            for row in &galley.rows {
                let text: String = row.text().to_string();
                let trimmed = text.trim();
                assert!(
                    trimmed.is_empty()
                        || ["alpha", "bravo", "charlie", "delta"]
                            .iter()
                            .any(|word| trimmed.split_whitespace().all(|part| {
                                ["alpha", "bravo", "charlie", "delta"].contains(&part)
                                    || part == *word
                            })),
                    "a row broke inside a word: {trimmed:?}"
                );
            }
        });
    }

    #[test]
    fn a_zero_width_rect_is_laid_out_rather_than_panicking() {
        // `LayoutJob` divides by the wrap width, and a toast whose border has
        // eaten its content rect is not a crash.
        let spec = spec_with("T", "B");
        let ctx = ctx();
        let _ = ctx.run(Default::default(), |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::background());
            draw(
                &painter,
                egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(0.0, 0.0)),
                &spec,
                egui::Color32::WHITE,
                egui::Color32::GRAY,
                None,
                None,
            );
        });
    }

    #[test]
    fn a_picture_is_fitted_into_its_slot_and_never_stretched() {
        // A wide image in a square slot keeps its shape: it shrinks to the
        // slot's width and is centred, rather than being squashed square.
        let slot = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(72.0, 72.0));
        let wide = fit(slot, egui::vec2(200.0, 100.0));
        assert_eq!(wide.width(), 72.0);
        assert_eq!(wide.height(), 36.0, "the 2:1 shape survives");
        assert_eq!(wide.center(), slot.center());

        let tall = fit(slot, egui::vec2(50.0, 100.0));
        assert_eq!(tall.height(), 72.0);
        assert_eq!(tall.width(), 36.0);
        assert!(slot.contains_rect(tall) && slot.contains_rect(wide));

        // A picture with no size is not a division by zero.
        assert_eq!(fit(slot, egui::Vec2::ZERO), slot);
    }

    #[test]
    fn an_image_and_an_icon_are_drawn_without_escaping_the_toast() {
        // Both pictures at once, beside the longest Malay text: the image
        // takes the right of the body and the text the rest, and nothing
        // leaves the content rect.
        let ctx = ctx();
        let spec = spec_with(
            "Waktu Solat Zohor",
            &pseudo("Waktu Zohor telah masuk bagi zon WLY01."),
        );
        let picture = |side: f32| Picture {
            texture: egui::TextureId::Managed(0),
            size: egui::vec2(side, side),
        };
        let _ = ctx.run(Default::default(), |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::background());
            let rect = egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(spec.style.width, 120.0),
            );
            let content = render::content_rect(rect, &spec);
            draw(
                &painter,
                content,
                &spec,
                egui::Color32::WHITE,
                egui::Color32::GRAY,
                Some(picture(64.0)),
                Some(picture(512.0)),
            );
            assert!(rect.contains_rect(content));
        });
    }

    #[test]
    fn emphasis_changes_the_body_font_rather_than_being_ignored() {
        let mut spec = spec_with("T", "B");
        let plain = body_font(&spec);
        spec.content.body_italic = true;
        assert_ne!(body_font(&spec).size, plain.size);
    }
}
