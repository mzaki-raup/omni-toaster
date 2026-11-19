//! Settings ▸ Reminders. For now the missed reminders (FEAT-0045) and the
//! Snooze section — what one press of the toast's Snooze does, how many times
//! one fire may be snoozed, and how far back a missed reminder is still
//! listed (`docs/specs/reminders.md` §5, §6). The reminder list and its
//! editor join them with FEAT-0046.

use ot_core::Reminders;
use ot_core::settings::{MISSED_WINDOW_HOURS, SNOOZE_LIMIT, SNOOZE_MINUTES, SNOOZE_PRESETS};

use crate::layout::{self, Columns};
use crate::theme::Tokens;

/// A custom length to start from when Custom is picked, so the choice reads
/// as Custom rather than snapping back to a preset.
const CUSTOM_START: u32 = 30;

/// The Fluent key naming a preset length.
fn preset_key(minutes: u32) -> &'static str {
    match minutes {
        5 => "settings-snooze-5-minutes",
        15 => "settings-snooze-15-minutes",
        60 => "settings-snooze-1-hour",
        _ => "settings-snooze-1-day",
    }
}

/// The Snooze controls. Returns whether anything changed.
pub fn snooze_controls(
    ui: &mut egui::Ui,
    columns: Columns,
    reminders: &mut Reminders,
    tokens: Tokens,
) -> bool {
    let mut changed = false;
    let custom = !SNOOZE_PRESETS.contains(&reminders.snooze_minutes);

    layout::labelled_row(
        ui,
        columns,
        &ot_i18n::text("settings-snooze-length"),
        |ui| {
            let selected = if custom {
                ot_i18n::text("settings-snooze-custom")
            } else {
                ot_i18n::text(preset_key(reminders.snooze_minutes))
            };
            egui::ComboBox::from_id_salt("snooze-length")
                .width(columns.field)
                .height(layout::field_height(ui))
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    for minutes in SNOOZE_PRESETS {
                        changed |= ui
                            .selectable_value(
                                &mut reminders.snooze_minutes,
                                minutes,
                                ot_i18n::text(preset_key(minutes)),
                            )
                            .changed();
                    }
                    if ui
                        .selectable_label(custom, ot_i18n::text("settings-snooze-custom"))
                        .clicked()
                        && !custom
                    {
                        reminders.snooze_minutes = CUSTOM_START;
                        changed = true;
                    }
                });
        },
    );
    layout::field_hint(
        ui,
        columns,
        &ot_i18n::text("settings-snooze-length-hint"),
        tokens.text_dim,
    );
    ui.add_space(layout::ROW_GAP);

    if !SNOOZE_PRESETS.contains(&reminders.snooze_minutes) {
        layout::labelled_row(
            ui,
            columns,
            &ot_i18n::text("settings-snooze-custom-length"),
            |ui| {
                changed |= ui
                    .add(
                        // The unit is in the label, "(Minutes)": a suffix
                        // after the number would read "1 minutes".
                        egui::DragValue::new(&mut reminders.snooze_minutes)
                            .range(SNOOZE_MINUTES)
                            .speed(1.0),
                    )
                    .changed();
            },
        );
        ui.add_space(layout::ROW_GAP);
    }

    layout::labelled_row(ui, columns, &ot_i18n::text("settings-snooze-limit"), |ui| {
        ui.spacing_mut().slider_width = (columns.field - 60.0).max(80.0);
        changed |= ui
            .add(egui::Slider::new(&mut reminders.snooze_limit, SNOOZE_LIMIT))
            .changed();
    });
    layout::field_hint(
        ui,
        columns,
        &ot_i18n::text("settings-snooze-limit-hint"),
        tokens.text_dim,
    );
    ui.add_space(layout::ROW_GAP);

    layout::labelled_row(
        ui,
        columns,
        &ot_i18n::text("settings-missed-window"),
        |ui| {
            // The unit is in the label, "(Hours)", as for the custom length.
            ui.spacing_mut().slider_width = (columns.field - 60.0).max(80.0);
            changed |= ui
                .add(egui::Slider::new(
                    &mut reminders.missed_window_hours,
                    MISSED_WINDOW_HOURS,
                ))
                .changed();
        },
    );
    layout::field_hint(
        ui,
        columns,
        &ot_i18n::text("settings-missed-window-hint"),
        tokens.text_dim,
    );
    ui.add_space(layout::ROW_GAP);

    changed
}

/// One missed reminder as the list shows it: text already in the user's
/// language and already formatted, so nothing is formatted per frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissedRow {
    /// The reminder's own title as typed, or its type's name.
    pub title: String,
    /// "Due …", through the one formatter.
    pub due: String,
    /// False once the fire has been snoozed as often as allowed.
    pub can_snooze: bool,
}

/// What a button in the missed list asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissedAction {
    Snooze,
    Dismiss,
}

/// The missed reminders, each with Snooze and Dismiss, or a sentence saying
/// there are none (`docs/specs/reminders.md` §6). Returns the row whose button
/// was pressed, and which.
pub fn missed_list(
    ui: &mut egui::Ui,
    rows: &[MissedRow],
    tokens: Tokens,
) -> Option<(usize, MissedAction)> {
    ui.label(egui::RichText::new(ot_i18n::text("settings-missed-heading")).strong());
    ui.add_space(layout::ROW_GAP);
    if rows.is_empty() {
        ui.label(
            egui::RichText::new(ot_i18n::text("settings-missed-empty")).color(tokens.text_dim),
        );
        return None;
    }
    let mut pressed = None;
    for (index, row) in rows.iter().enumerate() {
        // The title is the user's, any length: it truncates rather than
        // pushing the buttons off the pane.
        ui.add(egui::Label::new(&row.title).truncate());
        // The due line on its own row, under the title, and the buttons on
        // the next: a text beside taller buttons would not share their middle
        // line (ui.md). The buttons wrap on a narrow pane.
        ui.label(egui::RichText::new(&row.due).color(tokens.text_dim));
        let height = layout::field_height(ui);
        ui.horizontal_wrapped(|ui| {
            let snooze = egui::Button::new(ot_i18n::text("toast-action-snooze"))
                .min_size(egui::vec2(0.0, height));
            if ui.add_enabled(row.can_snooze, snooze).clicked() {
                pressed = Some((index, MissedAction::Snooze));
            }
            let dismiss = egui::Button::new(ot_i18n::text("toast-action-dismiss"))
                .min_size(egui::vec2(0.0, height));
            if ui.add(dismiss).clicked() {
                pressed = Some((index, MissedAction::Dismiss));
            }
        });
        ui.add_space(layout::ROW_GAP);
    }
    pressed
}
