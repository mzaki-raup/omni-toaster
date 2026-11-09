//! The measurements and the row helper every pane uses.
//!
//! The UI rules (`.claude/rules/ui.md`) are specific about three things that
//! are easy to get wrong in an immediate-mode UI, so they live here rather
//! than being retyped per pane:
//!
//! - nothing touches a pane edge, and sections are set apart by a consistent
//!   gap;
//! - every control in a row is the same height and on one middle line;
//! - a field is wide enough for the values it holds, **in Malay**, which runs
//!   up to 30% wider than English.

/// The inset between a pane's content and its edge.
pub const PANE_MARGIN: f32 = 16.0;
/// Between the sections of a pane — a form, a table, the status line.
pub const PANE_GAP: f32 = 12.0;
/// Between the rows of one form.
pub const ROW_GAP: f32 = 8.0;
/// Between two options sharing a row, so each reads as its own choice.
pub const OPTION_GAP: f32 = 18.0;
/// A field is at least this wide: enough for a long Malay value.
pub const FIELD_MIN_WIDTH: f32 = 260.0;
/// And no wider, so a form does not become a row of stretched boxes.
pub const FIELD_MAX_WIDTH: f32 = 560.0;

// The measurements have to stay in that order for a form to read correctly:
// rows sit closer together than sections, two options sharing a row need more
// air than two rows do, and a field's floor is below its ceiling. Checked at
// compile time rather than in a test, because they are constants.
const _: () = assert!(ROW_GAP < PANE_GAP);
const _: () = assert!(OPTION_GAP > ROW_GAP);
const _: () = assert!(FIELD_MIN_WIDTH < FIELD_MAX_WIDTH);

/// One height for every control in a row — about 32 px at the default font.
///
/// Derived from the font rather than fixed, so it scales with the user's text
/// size and with DPI instead of being a raw pixel count
/// (`.claude/rules/ui.md`).
pub fn field_height(ui: &egui::Ui) -> f32 {
    height_for(egui::TextStyle::Body.resolve(ui.style()).size)
}

/// The height a given body font size earns. Separated so the rule can be
/// asserted without standing up an egui context.
fn height_for(body_size: f32) -> f32 {
    (body_size * 2.0).max(32.0)
}

/// How a form row divides at the width it has.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Columns {
    /// The label column, or 0 when the label sits above the field.
    pub label: f32,
    /// What the field gets.
    pub field: f32,
    /// Whether the label is above the field rather than beside it.
    pub stacked: bool,
}

/// Work out the two columns for the width this `ui` has.
///
/// Call it once per pane and pass it to each row, so every row divides the
/// same way and the fields line up in one column.
pub fn columns(ui: &egui::Ui) -> Columns {
    columns_for(ui.available_width())
}

/// The rule itself, separated so it can be asserted at every width without
/// standing up an egui context.
fn columns_for(available: f32) -> Columns {
    let label = (available * 0.35).clamp(0.0, 200.0);
    let beside = available - label - PANE_MARGIN;
    if beside >= FIELD_MIN_WIDTH {
        return Columns {
            label,
            field: beside.min(FIELD_MAX_WIDTH),
            stacked: false,
        };
    }
    // Not enough width for a label and a usable field side by side. The label
    // goes above, which keeps the field wide enough to show its value instead
    // of letting it run off the pane — the narrow-window case the rules
    // require a screen to survive (`.claude/rules/ui.md`).
    Columns {
        label: 0.0,
        field: (available - PANE_MARGIN).clamp(0.0, FIELD_MAX_WIDTH),
        stacked: true,
    }
}

/// A label and its controls: beside each other on one middle line, or the
/// label above them when the pane is too narrow for both.
///
/// The field height is set **before** the row opens. egui fixes a horizontal
/// row's height when the row opens, so raising the controls inside the closure
/// leaves the label centred on the old, shorter row — it sits high, and it is
/// the single most common layout bug in this UI (`.claude/rules/ui.md`).
pub fn labelled_row<R>(
    ui: &mut egui::Ui,
    columns: Columns,
    label: &str,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let height = field_height(ui);
    ui.spacing_mut().interact_size.y = height;

    if columns.stacked {
        ui.label(label);
        return ui
            .horizontal(|ui| {
                ui.set_min_height(height);
                add_contents(ui)
            })
            .inner;
    }

    ui.horizontal(|ui| {
        ui.set_min_height(height);
        // Left-aligned and on the field's middle line. `add_sized` centres
        // what it is given, which puts the label in the middle of its own
        // column — a layout nobody asked for and the rules do not describe.
        ui.allocate_ui_with_layout(
            egui::vec2(columns.label, height),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                // Without this the column shrinks to the text, so every row
                // starts its field at a different x and the form reads as a
                // ragged edge rather than a column.
                ui.set_min_width(columns.label);
                ui.label(label);
            },
        );
        add_contents(ui)
    })
    .inner
}

/// A hint under a field, on its own row.
///
/// Never in the field's own cell: that grows the row and drops the label to
/// the middle of the pair (`.claude/rules/ui.md`).
pub fn field_hint(ui: &mut egui::Ui, columns: Columns, text: &str, colour: egui::Color32) {
    // The hint sits in the field's column and wraps inside it. A plain
    // `horizontal_wrapped` wraps against the *pane*, so a long Malay hint
    // starts its second line under the label — which reads as a different
    // paragraph. Malay runs up to 30% wider than English, so this is the
    // normal case there, not an edge case.
    ui.horizontal_top(|ui| {
        ui.add_space(columns.label);
        ui.allocate_ui_with_layout(
            egui::vec2(columns.field, 0.0),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.set_max_width(columns.field);
                ui.label(egui::RichText::new(text).color(colour).size(12.0));
            },
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_field_height_scales_with_the_font_and_never_goes_below_the_floor() {
        // Nothing may be a raw pixel count that should have scaled with the
        // font and with DPI (`.claude/rules/ui.md`).
        for size in [8.0_f32, 12.0, 14.0, 16.0, 24.0, 32.0] {
            let height = height_for(size);
            assert!(
                height >= 32.0,
                "a {size}px font gave a {height}px field, under the floor"
            );
        }
        assert!(
            height_for(32.0) > height_for(14.0),
            "the height did not follow the font"
        );
        // The default font must land on the ~32 px the rules name.
        assert!((height_for(14.0) - 32.0).abs() < 2.0);
    }

    #[test]
    fn a_field_never_runs_past_the_pane_at_any_width() {
        // The narrow window is a rule, not a nicety: "nothing overlaps,
        // nothing is cut off" (`.claude/rules/ui.md`).
        for available in [120.0_f32, 200.0, 320.0, 360.0, 420.0, 700.0, 1600.0] {
            let c = columns_for(available);
            assert!(
                c.label + c.field <= available,
                "at {available}px the row wants {} + {} px",
                c.label,
                c.field
            );
            assert!(c.field >= 0.0);
        }
    }

    #[test]
    fn a_narrow_pane_stacks_the_label_rather_than_shrinking_the_field() {
        // Below the point where a label and a usable field fit side by side,
        // the label goes above and the field keeps its width.
        let narrow = columns_for(360.0);
        assert!(narrow.stacked, "360px should have stacked");
        assert_eq!(narrow.label, 0.0, "a stacked row has no label column");
        assert!(
            narrow.field >= FIELD_MIN_WIDTH || narrow.field == 360.0 - PANE_MARGIN,
            "the field should take what is left: {}",
            narrow.field
        );

        let wide = columns_for(900.0);
        assert!(!wide.stacked, "900px has room for both");
        assert!(wide.label > 0.0);
        assert!(wide.field >= FIELD_MIN_WIDTH);
    }

    #[test]
    fn every_row_in_one_pane_divides_the_same_way() {
        // The fields must line up in one column, which they only do if the
        // split depends on the pane's width and not on the label's text.
        assert_eq!(columns_for(880.0), columns_for(880.0));
    }
}
