//! Settings ▸ Reminders. For now the Snooze section — what one press of the
//! toast's Snooze does, and how many times one fire may be snoozed
//! (`docs/specs/reminders.md` §5, FEAT-0043). The reminder list and its
//! editor join it with FEAT-0046.

use ot_core::Reminders;
use ot_core::settings::{SNOOZE_LIMIT, SNOOZE_MINUTES, SNOOZE_PRESETS};

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
                        egui::DragValue::new(&mut reminders.snooze_minutes)
                            .range(SNOOZE_MINUTES)
                            .speed(1.0)
                            .suffix(format!(
                                " {}",
                                ot_i18n::text("settings-snooze-minutes-unit")
                            )),
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

    changed
}
