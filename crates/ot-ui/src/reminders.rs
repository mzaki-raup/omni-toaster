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

/// One reminder as the list shows it, built once when the list arrives —
/// never formatted per frame (`.claude/rules/ui.md`, "Per-frame cost").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListRow {
    pub id: i64,
    /// The reminder's own title, as typed.
    pub title: String,
    /// Its type and when it next fires, or that it is done.
    pub detail: String,
}

/// What the reminder list asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListAction {
    New,
    Select(i64),
}

/// How tall the list may grow before it scrolls, in rows.
const LIST_ROWS: usize = 6;

/// The reminders, newest last, with New Reminder above them. `rows` is
/// `None` while they load. Only the rows in view are laid out, so a list of
/// a thousand costs what a list of six does.
pub fn reminder_list(
    ui: &mut egui::Ui,
    rows: Option<&[ListRow]>,
    selected: Option<i64>,
    tokens: Tokens,
) -> Option<ListAction> {
    let mut action = None;
    // The heading and its button on one middle line (ui.md): the row's
    // height is set before it opens, or the label centres on a shorter row
    // than the button makes.
    let height = layout::field_height(ui);
    let row = egui::vec2(ui.available_width(), height);
    ui.allocate_ui_with_layout(
        row,
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.label(egui::RichText::new(ot_i18n::text("settings-reminders-heading")).strong());
            if ui
                .add(
                    egui::Button::new(ot_i18n::text("settings-reminders-new"))
                        .min_size(egui::vec2(0.0, height)),
                )
                .clicked()
            {
                action = Some(ListAction::New);
            }
        },
    );
    ui.add_space(layout::ROW_GAP);
    let Some(rows) = rows else {
        ui.label(
            egui::RichText::new(ot_i18n::text("settings-reminders-loading")).color(tokens.text_dim),
        );
        return action;
    };
    if rows.is_empty() {
        ui.label(
            egui::RichText::new(ot_i18n::text("settings-reminders-empty")).color(tokens.text_dim),
        );
        return action;
    }
    let row_height =
        2.0 * ui.text_style_height(&egui::TextStyle::Body) + ui.spacing().item_spacing.y * 2.0;
    egui::ScrollArea::vertical()
        .id_salt("reminder-list")
        .max_height(row_height * LIST_ROWS as f32)
        .auto_shrink([false, true])
        .show_rows(ui, row_height, rows.len(), |ui, range| {
            for row in &rows[range] {
                let response = ui
                    .push_id(row.id, |ui| {
                        ui.add(
                            egui::Button::selectable(
                                selected == Some(row.id),
                                egui::RichText::new(&row.title),
                            )
                            .truncate(),
                        )
                    })
                    .inner;
                if response.clicked() {
                    action = Some(ListAction::Select(row.id));
                }
                ui.add(
                    egui::Label::new(egui::RichText::new(&row.detail).color(tokens.text_dim))
                        .truncate(),
                );
            }
        });
    action
}

/// A reminder's history, newest last, already formatted, with Clear
/// History behind a confirmation. Returns true when the clearing was
/// confirmed.
pub fn history(ui: &mut egui::Ui, rows: &[String], confirming: &mut bool, tokens: Tokens) -> bool {
    let mut cleared = false;
    ui.label(egui::RichText::new(ot_i18n::text("settings-reminder-history")).strong());
    ui.add_space(layout::ROW_GAP);
    if rows.is_empty() {
        ui.label(
            egui::RichText::new(ot_i18n::text("settings-reminder-history-empty"))
                .color(tokens.text_dim),
        );
        return false;
    }
    // The last twenty: the list is for "did it fire?", not an archive.
    for row in rows.iter().rev().take(20).rev() {
        ui.add(egui::Label::new(egui::RichText::new(row).color(tokens.text_dim)).truncate());
    }
    ui.add_space(layout::ROW_GAP);
    ui.horizontal_wrapped(|ui| {
        let height = layout::field_height(ui);
        let button = |text: String| egui::Button::new(text).min_size(egui::vec2(0.0, height));
        if *confirming {
            ui.label(ot_i18n::text("settings-reminder-history-clear-confirm"));
            if ui
                .add(button(ot_i18n::text("settings-reminder-history-clear")))
                .clicked()
            {
                cleared = true;
                *confirming = false;
            }
            if ui
                .add(button(ot_i18n::text("settings-reminder-keep")))
                .clicked()
            {
                *confirming = false;
            }
        } else if ui
            .add(button(ot_i18n::text("settings-reminder-history-clear")))
            .clicked()
        {
            *confirming = true;
        }
    });
    cleared
}
