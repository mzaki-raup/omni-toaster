//! The shared date field and time field (`.claude/rules/ui.md`, "The right
//! control for the value"). Every pane that edits a date or a time of day
//! uses these, so they look and behave the same everywhere.
//!
//! - **Date:** day, then the month by name, then the year — the same order in
//!   both languages, and a month name cannot be misread the way 03/04 can.
//!   The day never exceeds the month's length; changing the month or the year
//!   brings it back inside.
//! - **Time:** hour and minute, 24-hour. An input, not a display: shown
//!   times still go through `ot_i18n::format`, which honours the 12-hour
//!   choice.

use jiff::civil::{Date, Time};

/// The years a date field offers.
pub const YEARS: std::ops::RangeInclusive<i16> = 2000..=2100;

/// Edit a date in place. Returns whether it changed. `id` keeps two date
/// fields on one pane apart.
pub fn date_field(ui: &mut egui::Ui, id: &str, date: &mut Date) -> bool {
    let (mut year, mut month, mut day) = (date.year(), date.month(), date.day());
    let mut changed = false;
    let height = crate::layout::field_height(ui);
    ui.horizontal(|ui| {
        ui.spacing_mut().interact_size.y = height;
        changed |= ui
            .add(egui::DragValue::new(&mut day).range(1..=31).speed(0.2))
            .changed();
        egui::ComboBox::from_id_salt(format!("{id}-month"))
            .height(height)
            .selected_text(ot_i18n::format::month_name(
                u32::try_from(month).unwrap_or(1),
            ))
            .show_ui(ui, |ui| {
                for candidate in 1..=12_i8 {
                    changed |= ui
                        .selectable_value(
                            &mut month,
                            candidate,
                            ot_i18n::format::month_name(u32::try_from(candidate).unwrap_or(1)),
                        )
                        .changed();
                }
            });
        changed |= ui
            .add(egui::DragValue::new(&mut year).range(YEARS).speed(0.2))
            .changed();
    });
    if changed {
        *date = clamped_date(year, month, day);
    }
    changed
}

/// A date from its parts, with the day brought inside the month: 31
/// February becomes 28 or 29 February.
pub fn clamped_date(year: i16, month: i8, day: i8) -> Date {
    let first = Date::new(year, month, 1).unwrap_or_default();
    let last = first.days_in_month();
    Date::new(year, month, day.clamp(1, last)).unwrap_or(first)
}

/// Edit a time of day in place, to the minute. Returns whether it changed.
pub fn time_field(ui: &mut egui::Ui, time: &mut Time) -> bool {
    let (mut hour, mut minute) = (time.hour(), time.minute());
    let mut changed = false;
    let height = crate::layout::field_height(ui);
    ui.horizontal(|ui| {
        ui.spacing_mut().interact_size.y = height;
        // Two digits each, as a clock shows them; this is an input control,
        // not a time being displayed.
        let two_digits = |value: f64, _| format!("{value:02.0}");
        changed |= ui
            .add(
                egui::DragValue::new(&mut hour)
                    .range(0..=23)
                    .speed(0.1)
                    .custom_formatter(two_digits),
            )
            .changed();
        ui.label(":");
        changed |= ui
            .add(
                egui::DragValue::new(&mut minute)
                    .range(0..=59)
                    .speed(0.2)
                    .custom_formatter(two_digits),
            )
            .changed();
    });
    if changed {
        *time = Time::new(hour, minute, 0, 0).unwrap_or(*time);
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_day_past_the_months_end_is_brought_inside_it() {
        assert_eq!(clamped_date(2026, 2, 31), jiff::civil::date(2026, 2, 28));
        assert_eq!(clamped_date(2028, 2, 31), jiff::civil::date(2028, 2, 29));
        assert_eq!(clamped_date(2026, 4, 31), jiff::civil::date(2026, 4, 30));
        assert_eq!(clamped_date(2026, 1, 15), jiff::civil::date(2026, 1, 15));
        assert_eq!(clamped_date(2026, 1, 0), jiff::civil::date(2026, 1, 1));
    }
}
