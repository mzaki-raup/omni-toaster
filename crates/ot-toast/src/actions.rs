//! The action row (`docs/specs/toast.md` §4.3).
//!
//! Right-aligned along the bottom of the content, Dismiss last. The layout is
//! a pure function of the room and the labels' widths, so it can be tested
//! for every shape and language without a window; drawing and hit-testing
//! both use the rects it returns, so what is drawn is what is pressed.

use crate::spec::{ACTION_ROW, Action};

/// A button's height. The rest of [`ACTION_ROW`] is the gap above the row.
pub const BUTTON_HEIGHT: f32 = 26.0;
/// Space on each side of a label inside its button.
const PADDING: f32 = 12.0;
/// The narrowest a button is drawn, so a short label is still a
/// comfortable target.
const MIN_WIDTH: f32 = 64.0;
/// Between two buttons.
const GAP: f32 = 8.0;
/// The label's size, in points: the body's, a touch smaller.
pub const LABEL_SIZE: f32 = 12.5;

/// Split the content rect into the room left for the body and the row, at
/// its bottom. A rect too short for a row gives it none.
pub fn split(content: egui::Rect) -> (egui::Rect, Option<egui::Rect>) {
    if content.height() < ACTION_ROW + BUTTON_HEIGHT {
        return (content, None);
    }
    let row = egui::Rect::from_min_max(
        egui::pos2(content.left(), content.bottom() - BUTTON_HEIGHT),
        content.right_bottom(),
    );
    let body = egui::Rect::from_min_max(
        content.left_top(),
        egui::pos2(content.right(), content.bottom() - ACTION_ROW),
    );
    (body, Some(row))
}

/// Where each button goes in `row`, given each label's width, left to right.
///
/// Each button is its label plus padding, at least [`MIN_WIDTH`], and the
/// row is right-aligned. When that is wider than the row, the buttons share
/// the row equally instead — and the caller truncates a label that still
/// does not fit. Nothing is ever placed outside `row`.
pub fn layout(row: egui::Rect, label_widths: &[f32]) -> Vec<egui::Rect> {
    let count = label_widths.len();
    if count == 0 || row.width() <= 0.0 {
        return Vec::new();
    }
    let gaps = GAP * (count - 1) as f32;
    let natural: Vec<f32> = label_widths
        .iter()
        .map(|width| (width + 2.0 * PADDING).max(MIN_WIDTH))
        .collect();
    let total: f32 = natural.iter().sum::<f32>() + gaps;
    let widths = if total <= row.width() {
        natural
    } else {
        vec![((row.width() - gaps) / count as f32).max(0.0); count]
    };
    let used: f32 = widths.iter().sum::<f32>() + gaps;
    let mut x = row.right() - used.min(row.width());
    widths
        .into_iter()
        .map(|width| {
            let rect = egui::Rect::from_min_size(
                egui::pos2(x, row.top()),
                egui::vec2(width, row.height()),
            );
            x += width + GAP;
            rect
        })
        .collect()
}

/// Draw the buttons and return where each one is, for hit-testing.
///
/// `text` and `dim` are the toast's own text colours from the theme, so the
/// row hard-codes no colour. `pointer` is where the pointer is, in the
/// painter's coordinates, if it is over this toast: hover fills a working
/// button more strongly and never grows or moves it.
pub fn draw(
    painter: &egui::Painter,
    row: egui::Rect,
    actions: &[Action],
    label: impl Fn(Action) -> String,
    text: egui::Color32,
    dim: egui::Color32,
    pointer: Option<egui::Pos2>,
) -> Vec<(Action, egui::Rect)> {
    let font = egui::FontId::proportional(LABEL_SIZE);
    let widths: Vec<f32> = actions
        .iter()
        .map(|action| {
            painter
                .layout_no_wrap(label(*action), font.clone(), text)
                .size()
                .x
        })
        .collect();
    let rects = layout(row, &widths);
    let corner = egui::CornerRadius::same(6);
    for (action, rect) in actions.iter().zip(&rects) {
        let works = action.works();
        let hovered = works && pointer.is_some_and(|at| rect.contains(at));
        let fill = match (works, hovered) {
            (true, true) => 0.24,
            (true, false) => 0.12,
            (false, _) => 0.05,
        };
        painter.rect_filled(*rect, corner, text.gamma_multiply(fill));
        let colour = if works { text } else { dim.gamma_multiply(0.7) };
        let mut job = egui::text::LayoutJob::simple(
            label(*action),
            font.clone(),
            colour,
            (rect.width() - 2.0 * 4.0).max(1.0),
        );
        job.wrap = egui::text::TextWrapping {
            max_width: (rect.width() - 2.0 * 4.0).max(1.0),
            max_rows: 1,
            break_anywhere: true,
            overflow_character: Some('…'),
        };
        let galley = painter.layout_job(job);
        let at = rect.center() - galley.size() / 2.0;
        painter.galley(at, galley, colour);
    }
    actions.iter().copied().zip(rects).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(width: f32) -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(10.0, 100.0), egui::vec2(width, BUTTON_HEIGHT))
    }

    #[test]
    fn the_row_is_right_aligned_and_dismiss_is_last() {
        let row = row(300.0);
        let rects = layout(row, &[30.0, 40.0]);
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[1].right(), row.right(), "flush with the right edge");
        assert!(rects[0].right() < rects[1].left(), "apart, in order");
        assert_eq!(
            Action::for_count(2),
            &[Action::Snooze, Action::Dismiss],
            "Dismiss last"
        );
        assert_eq!(Action::for_count(1), &[Action::Dismiss]);
        assert_eq!(
            Action::for_count(3),
            &[Action::Open, Action::Snooze, Action::Dismiss]
        );
    }

    #[test]
    fn a_short_label_still_gets_a_comfortable_button() {
        let rects = layout(row(300.0), &[10.0]);
        assert_eq!(rects[0].width(), MIN_WIDTH);
    }

    #[test]
    fn buttons_never_leave_the_row_however_long_the_labels() {
        for width in [120.0, 200.0, 300.0] {
            for labels in [vec![40.0], vec![300.0, 300.0], vec![500.0, 10.0, 500.0]] {
                let row = row(width);
                let rects = layout(row, &labels);
                for rect in &rects {
                    assert!(
                        row.expand(0.01).contains_rect(*rect),
                        "{rect:?} outside {row:?} for {labels:?}"
                    );
                }
                for pair in rects.windows(2) {
                    assert!(pair[0].right() <= pair[1].left(), "overlap: {pair:?}");
                }
            }
        }
    }

    #[test]
    fn the_body_gives_up_exactly_the_row() {
        let content = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(300.0, 128.0));
        let (body, row) = split(content);
        let row = row.expect("room for a row");
        assert_eq!(body.bottom(), content.bottom() - ACTION_ROW);
        assert_eq!(row.bottom(), content.bottom());
        assert_eq!(row.height(), BUTTON_HEIGHT);
        assert!(body.bottom() < row.top(), "a gap between them");

        // Too short for a row: it gets none, and the body keeps it all.
        let short = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(300.0, 40.0));
        assert_eq!(split(short), (short, None));
    }

    #[test]
    fn dismiss_and_open_work_and_snooze_waits_for_its_item() {
        assert!(Action::Dismiss.works());
        assert!(Action::Open.works());
        assert!(!Action::Snooze.works());
    }
}
