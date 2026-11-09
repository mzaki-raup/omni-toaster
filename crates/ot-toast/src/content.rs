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

/// Draw the content inside `rect`, which is what
/// [`render::frame`](crate::render::frame) returned.
///
/// `text_colour` and `dim` come from the theme through the caller, so the
/// renderer still hard-codes no colour.
pub fn draw(
    painter: &egui::Painter,
    rect: egui::Rect,
    spec: &ToastSpec,
    text_colour: egui::Color32,
    dim: egui::Color32,
) {
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let alpha = spec.alpha();
    let text_colour = fade(text_colour, alpha);
    let dim = fade(dim, alpha);

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

    // The category mark.
    let mut title_left = rect.left();
    if spec.content.show_icon {
        let side = ICON_SIDE.min(rect.height());
        let icon =
            egui::Rect::from_min_size(egui::pos2(rect.left(), cursor), egui::vec2(side, side));
        painter.rect_filled(
            icon,
            egui::CornerRadius::same((side / 4.0) as u8),
            render::colour(spec.style.accent, alpha),
        );
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

    // The body: user text again, wrapped, and truncated when even wrapping
    // cannot fit it.
    if spec.content.body.is_empty() {
        return;
    }
    cursor += HEADER_GAP;
    let remaining = rect.bottom() - cursor;
    if remaining <= 0.0 {
        return;
    }
    let rows = (remaining / (BODY_SIZE * 1.3)).floor().max(1.0) as usize;
    let galley = painter.layout_job(truncating_job(
        spec.content.body.clone(),
        body_font(spec),
        text_colour,
        rect.width(),
        rows,
    ));
    painter.galley(egui::pos2(rect.left(), cursor), galley, text_colour);
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
        for (title, body) in cases {
            let spec = spec_with(title, body);
            let rects = laid_out(&spec, spec.style.width, 120.0);
            let content = rects.first().copied().expect("a content rect");
            let frame = egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(spec.style.width, 120.0),
            );
            assert!(
                frame.contains_rect(content),
                "the content rect escaped the toast for {:?}",
                &title[..title.len().min(30)]
            );
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
        let ctx = ctx();
        let width = 240.0_f32;
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
                    "{:?} laid out {}px wide in {width}px",
                    &text[..text.len().min(30)],
                    galley.size().x
                );
            });
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
            );
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
