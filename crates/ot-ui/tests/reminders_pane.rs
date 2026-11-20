//! Settings ▸ Reminders, driven through `egui_kittest` (FEAT-0046): the
//! guards on Delete and Clear History, and the list's per-frame cost with a
//! thousand reminders.
//!
//! In its own process because it installs the process-wide language; every
//! test here uses English and tolerates a sibling having installed it first.

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use ot_i18n::Language;
use ot_ui::reminder_editor::{Editor, EditorAction};

fn english() {
    let _ = ot_i18n::init(Language::English);
    assert_eq!(ot_i18n::current_language(), Some(Language::English));
}

fn fonts(harness: &mut Harness<'_, impl Sized>) {
    harness
        .ctx
        .set_fonts(ot_ui::settings_window::font_definitions().expect("the Windows UI font"));
}

fn saved_editor() -> Editor {
    let mut editor = Editor::new_at(
        "2026-11-19T00:00Z".parse().expect("instant"),
        &jiff::tz::TimeZone::UTC,
    );
    editor.id = Some(7);
    editor.title = "Bayar bil".to_owned();
    editor
}

#[test]
fn delete_asks_first_and_keep_it_backs_out() {
    english();
    let state = (saved_editor(), None::<EditorAction>);
    let mut harness = Harness::builder().with_size([700.0, 900.0]).build_ui_state(
        |ui, (editor, action): &mut (Editor, Option<EditorAction>)| {
            let columns = ot_ui::layout::columns(ui);
            if let Some(pressed) =
                ot_ui::reminder_editor::editor(ui, columns, editor, &[], ot_ui::Tokens::DARK)
            {
                *action = Some(pressed);
            }
        },
        state,
    );
    fonts(&mut harness);
    harness.run();

    // The first Delete only asks.
    harness.get_by_label("Delete").click();
    harness.run();
    assert_eq!(harness.state().1, None, "nothing deleted yet");
    assert!(harness.state().0.confirming_delete);

    // Keep It backs out, and nothing was deleted.
    harness.get_by_label("Keep It").click();
    harness.run();
    assert!(!harness.state().0.confirming_delete);
    assert_eq!(harness.state().1, None);

    // Asked again and answered: now it is a delete.
    harness.get_by_label("Delete").click();
    harness.run();
    harness.get_by_label("Delete").click();
    harness.run();
    assert_eq!(harness.state().1, Some(EditorAction::Delete));
}

#[test]
fn clear_history_asks_first() {
    english();
    let rows = vec!["19/11/2026 at 10:00 am — Shown".to_owned()];
    let mut harness = Harness::builder().with_size([700.0, 400.0]).build_ui_state(
        |ui, (confirming, cleared): &mut (bool, bool)| {
            if ot_ui::reminders::history(ui, &rows, confirming, ot_ui::Tokens::DARK) {
                *cleared = true;
            }
        },
        (false, false),
    );
    fonts(&mut harness);
    harness.run();

    harness.get_by_label("Clear History").click();
    harness.run();
    assert_eq!(*harness.state(), (true, false), "asked, not cleared");
    harness.get_by_label("Clear History").click();
    harness.run();
    assert_eq!(*harness.state(), (false, true), "cleared once answered");
}

#[test]
fn a_thousand_reminders_lay_out_only_the_rows_in_view() {
    // Per-frame cost (ui.md): the list lays out what is visible, not all of
    // it, so the widget count stays that of a short list.
    english();
    let rows: Vec<ot_ui::reminders::ListRow> = (0..1000)
        .map(|id| ot_ui::reminders::ListRow {
            id,
            title: format!("Reminder {id}"),
            detail: "Custom · next 19/11/2026 at 10:00 am".to_owned(),
        })
        .collect();
    let mut harness = Harness::builder().with_size([700.0, 900.0]).build_ui(|ui| {
        let _ = ot_ui::reminders::reminder_list(ui, Some(&rows), None, ot_ui::Tokens::DARK);
    });
    fonts(&mut harness);
    harness.run();

    let drawn = harness
        .root()
        .children_recursive()
        .filter(|node| {
            node.accesskit_node()
                .label()
                .is_some_and(|label| label.starts_with("Reminder "))
        })
        .count();
    assert!(drawn > 0, "the list drew nothing at all");
    assert!(drawn <= 12, "{drawn} of 1000 rows were laid out");
}

#[test]
fn the_list_says_when_it_is_loading_and_when_it_is_empty() {
    english();
    for (rows, expected) in [
        (None, "Loading your reminders…"),
        (
            Some(Vec::new()),
            "You have no reminders yet. New Reminder makes one.",
        ),
    ] {
        let mut harness = Harness::builder().with_size([700.0, 300.0]).build_ui(|ui| {
            let _ = ot_ui::reminders::reminder_list(ui, rows.as_deref(), None, ot_ui::Tokens::DARK);
        });
        fonts(&mut harness);
        harness.run();
        assert!(harness.query_by_label(expected).is_some(), "{expected}");
    }
}
