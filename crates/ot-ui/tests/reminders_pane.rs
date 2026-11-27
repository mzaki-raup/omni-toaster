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
        let _ = ot_ui::reminders::reminder_list(ui, Some(&rows), None, ot_ui::Tokens::DARK, 6);
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
            let _ =
                ot_ui::reminders::reminder_list(ui, rows.as_deref(), None, ot_ui::Tokens::DARK, 6);
        });
        fonts(&mut harness);
        harness.run();
        assert!(harness.query_by_label(expected).is_some(), "{expected}");
    }
}

type PaneState = (
    Editor,
    Option<ot_ui::reminders::ListAction>,
    Option<EditorAction>,
);

fn two_column_harness(width: f32) -> Harness<'static, PaneState> {
    english();
    let rows = vec![
        ot_ui::reminders::ListRow {
            id: 7,
            title: "Bayar bil".to_owned(),
            detail: "Event · next 19/11/2026 at 10:00 am".to_owned(),
        },
        ot_ui::reminders::ListRow {
            id: 9,
            title: "Jumpa doktor".to_owned(),
            detail: "Appointment · next 20/11/2026 at 9:00 am".to_owned(),
        },
    ];
    let mut harness = Harness::builder()
        .with_size([width, 1200.0])
        .build_ui_state(
            move |ui, (editor, listed, edited): &mut PaneState| {
                let master = ot_ui::reminders::list_and_details(
                    ui,
                    Some(&rows),
                    editor,
                    &[],
                    &[],
                    &mut false,
                    ot_ui::Tokens::DARK,
                );
                if master.listed.is_some() {
                    *listed = master.listed;
                }
                if master.edited.is_some() {
                    *edited = master.edited;
                }
            },
            (saved_editor(), None, None),
        );
    fonts(&mut harness);
    harness.run();
    harness
}

#[test]
fn wide_the_titles_are_a_column_on_the_left_and_the_form_is_on_the_right() {
    let harness = two_column_harness(900.0);
    let title = harness.get_by_label("Bayar bil");
    let new = harness.get_by_label("New Reminder");
    let form = harness.get_by_label("Save");
    assert!(
        new.rect().right() < form.rect().left(),
        "New Reminder {:?} is left of the form's Save {:?}",
        new.rect(),
        form.rect()
    );
    assert!(
        title.rect().right() < form.rect().left(),
        "the titles are left of the form"
    );
    // Side by side, not stacked: the form's own first label starts to the right
    // of the titles, where stacked it would start at the pane's left edge.
    let first = harness.get_by_label("Title");
    assert!(
        first.rect().left() > title.rect().right(),
        "the form's Title {:?} is right of the list's titles {:?}",
        first.rect(),
        title.rect()
    );
    assert!(
        harness.query_by_label("Jumpa doktor").is_some(),
        "both titles are listed"
    );
}

#[test]
fn narrow_the_list_sits_above_the_form_and_nothing_overlaps() {
    let harness = two_column_harness(500.0);
    let list = harness.get_by_label("Jumpa doktor");
    let form = harness.get_by_label("Save");
    assert!(
        list.rect().bottom() < form.rect().top(),
        "the list {:?} is above the form {:?}",
        list.rect(),
        form.rect()
    );
}

#[test]
fn choosing_a_title_new_reminder_and_the_forms_buttons_each_say_so() {
    let mut harness = two_column_harness(900.0);
    harness.get_by_label("Jumpa doktor").click();
    harness.run();
    assert_eq!(
        harness.state().1,
        Some(ot_ui::reminders::ListAction::Select(9))
    );
    harness.get_by_label("New Reminder").click();
    harness.run();
    assert_eq!(harness.state().1, Some(ot_ui::reminders::ListAction::New));
    harness.get_by_label("Cancel").click();
    harness.run();
    assert_eq!(harness.state().2, Some(EditorAction::Cancel));
}
