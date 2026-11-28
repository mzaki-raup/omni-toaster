//! The dropdown that chooses an entry from a library (FEAT-0088, `docs/specs/toast.md`
//! §8.1).
//!
//! One shape for every picker that takes something from Images, Icons or Audio: the
//! built-in choices **first** (for a picture or an icon that is the one empty choice, for
//! a sound it is the short chime, the Azan and silence), then the user's own entries by
//! name, then "Upload a New …" **last**, so nobody has to leave the editor to add one.
//! The picker draws and reports what was clicked; what a choice *means* (a path in a
//! template, a sound mode) and what an upload starts are the caller's.
//!
//! The individual built-in pictures are deliberately not offered: they have no `asset`
//! row to refer to (`docs/specs/toast.md` §8.2), so a template could not store one.

use crate::layout;

/// An entry in a library, as a picker lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    /// What a template stores: the entry's relative path under the data folder.
    pub value: String,
    /// The name the user gave it.
    pub label: String,
}

/// What the user clicked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Picked {
    /// The built-in choice at this index of [`Menu::builtins`].
    Builtin(usize),
    /// The entry whose [`Choice::value`] this is.
    Entry(String),
    /// "Upload a New …".
    Upload,
}

/// What to draw.
pub struct Menu<'a> {
    /// The id the dropdown's open state is kept under.
    pub id: &'a str,
    pub width: f32,
    /// What the closed dropdown says: what will actually show or play, so it never
    /// contradicts the Preview.
    pub shown: String,
    /// The built-in choices, each with whether it is the current one.
    pub builtins: &'a [(String, bool)],
    pub entries: &'a [Choice],
    /// The [`Choice::value`] that is current, if an entry is.
    pub current: Option<&'a str>,
    /// The last item, if the caller can upload.
    pub upload: Option<String>,
}

/// Draw the dropdown and say what was clicked, if anything.
pub fn show(ui: &mut egui::Ui, menu: &Menu<'_>) -> Option<Picked> {
    let mut picked = None;
    egui::ComboBox::from_id_salt(menu.id)
        .truncate()
        .width(menu.width)
        .height(layout::field_height(ui) * 6.0)
        .selected_text(menu.shown.clone())
        .show_ui(ui, |ui| {
            for (index, (label, current)) in menu.builtins.iter().enumerate() {
                if ui.selectable_label(*current, label).clicked() {
                    picked = Some(Picked::Builtin(index));
                }
            }
            for entry in menu.entries {
                let current = menu.current == Some(entry.value.as_str());
                if ui.selectable_label(current, &entry.label).clicked() {
                    picked = Some(Picked::Entry(entry.value.clone()));
                }
            }
            if let Some(label) = &menu.upload {
                ui.separator();
                if ui.selectable_label(false, label).clicked() {
                    picked = Some(Picked::Upload);
                }
            }
        });
    picked
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable as _;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn entries() -> Vec<Choice> {
        vec![
            Choice {
                value: "assets/a.png".into(),
                label: "Mosque".into(),
            },
            Choice {
                value: "assets/b.png".into(),
                label: "Alarm".into(),
            },
        ]
    }

    /// Open the dropdown, click `item`, and say what the picker reported.
    fn click_item(item: &str, current: Option<&'static str>) -> Option<Picked> {
        let seen: Rc<RefCell<Option<Picked>>> = Rc::new(RefCell::new(None));
        let sink = Rc::clone(&seen);
        let choices = entries();
        let builtins = vec![("Built-in".to_owned(), current.is_none())];
        let mut harness = Harness::new_ui(move |ui| {
            let menu = Menu {
                id: "test-picker",
                width: 200.0,
                shown: "Built-in".to_owned(),
                builtins: &builtins,
                entries: &choices,
                current,
                upload: Some("Upload a New Picture".to_owned()),
            };
            if let Some(picked) = show(ui, &menu) {
                *sink.borrow_mut() = Some(picked);
            }
        });
        harness.get_by_value("Built-in").click();
        harness.run();
        harness.get_by_label(item).click();
        harness.run();
        seen.take()
    }

    #[test]
    fn the_built_in_choice_an_entry_and_upload_each_report_themselves() {
        assert_eq!(
            click_item("Alarm", None),
            Some(Picked::Entry("assets/b.png".into()))
        );
        assert_eq!(
            click_item("Upload a New Picture", None),
            Some(Picked::Upload)
        );
        // Choosing the built-in again, with an entry current, goes back to it.
        assert_eq!(
            click_item("Built-in", Some("assets/a.png")),
            Some(Picked::Builtin(0))
        );
    }
}
