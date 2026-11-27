//! The two ways to name a place by hand: search a city, or type coordinates
//! (FEAT-0056, shared by Settings ▸ Prayer and Settings ▸ Weather since FEAT-0072).
//!
//! The picker draws the controls and remembers what is typed and what the search
//! found. It starts nothing: it hands back what the user asked for in an [`Action`],
//! and the pane runs the search its own way and feeds the answer back with
//! [`Picker::set_results`]. What a place is *for* — saved as Prayer's, saved as
//! Weather's, a time-zone note for typed coordinates — is the caller's.
//!
//! Both panes use the same `prayer-search-*` and `prayer-coords-*` words: they are
//! the same controls.

use jiff::tz::TimeZone;
use ot_prayer::location::{self, CoordinateError, Located};

use crate::layout::{self, Columns};
use crate::prayer_pane::{button, note};
use crate::theme::Tokens;

/// Where a city search stands.
#[derive(Debug)]
pub enum Search {
    Idle,
    Loading,
    Results(Vec<Located>),
    /// The Fluent key of what went wrong.
    Failed(&'static str),
}

/// A place the user chose.
#[derive(Debug, Clone, PartialEq)]
pub struct Chosen {
    pub place: Located,
    /// Whether it came from typed coordinates rather than a search result: outside
    /// Malaysia those take this PC's time zone, which is a guess worth saying so.
    pub typed_coordinates: bool,
}

/// What a frame of the picker asks for.
#[derive(Debug, Default)]
pub struct Action {
    /// A city to search for.
    pub search: Option<String>,
    pub chosen: Option<Chosen>,
    /// The Close button.
    pub close: bool,
}

/// What is typed and what was found.
pub struct Picker {
    pub(crate) city: String,
    pub(crate) search: Search,
    pub(crate) coords: String,
    pub(crate) coords_error: Option<&'static str>,
    /// This PC's zone, which typed coordinates outside Malaysia fall back to.
    system_tz: String,
}

impl Picker {
    pub fn new() -> Self {
        Self {
            city: String::new(),
            search: Search::Idle,
            coords: String::new(),
            coords_error: None,
            system_tz: TimeZone::system()
                .iana_name()
                .unwrap_or_default()
                .to_string(),
        }
    }

    /// Whether a search is out, so the window keeps repainting.
    pub fn searching(&self) -> bool {
        matches!(self.search, Search::Loading)
    }

    /// The search came back.
    pub fn set_results(&mut self, found: Result<Vec<Located>, &'static str>) {
        self.search = match found {
            Ok(found) => Search::Results(found),
            Err(key) => Search::Failed(key),
        };
    }

    /// Draw the controls. `language` is not needed here: the caller searches.
    pub fn show(&mut self, ui: &mut egui::Ui, columns: Columns, tokens: Tokens) -> Action {
        let mut action = Action::default();

        // City.
        let mut run_search = false;
        layout::labelled_row(ui, columns, &ot_i18n::text("prayer-search-label"), |ui| {
            let field = (columns.field - 96.0).max(120.0);
            let response = ui.add_sized(
                [field, layout::field_height(ui)],
                egui::TextEdit::singleline(&mut self.city)
                    .hint_text(ot_i18n::text("prayer-search-placeholder")),
            );
            let enter =
                response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
            if button(ui, ot_i18n::text("prayer-search-button")).clicked() || enter {
                run_search = true;
            }
        });
        if run_search && !self.city.trim().is_empty() {
            self.search = Search::Loading;
            action.search = Some(self.city.clone());
        }
        let mut chosen = None;
        match &self.search {
            Search::Idle => {}
            Search::Loading => {
                ui.add_space(layout::ROW_GAP / 2.0);
                note(ui, &ot_i18n::text("prayer-search-loading"), tokens.text_dim);
            }
            Search::Failed(key) => {
                ui.add_space(layout::ROW_GAP / 2.0);
                note(ui, &ot_i18n::text(key), tokens.danger);
            }
            Search::Results(found) if found.is_empty() => {
                ui.add_space(layout::ROW_GAP / 2.0);
                note(ui, &ot_i18n::text("prayer-search-empty"), tokens.text_dim);
            }
            Search::Results(found) => {
                ui.add_space(layout::ROW_GAP / 2.0);
                for place in found {
                    let text = ot_i18n::text_with(
                        "prayer-search-result",
                        &[
                            ("place", place.name.clone().into()),
                            ("zone", place.tz.clone().into()),
                        ],
                    );
                    if ui
                        .add(egui::Button::selectable(false, text).wrap())
                        .clicked()
                    {
                        chosen = Some(place.clone());
                    }
                }
            }
        }
        if let Some(place) = chosen {
            self.search = Search::Idle;
            action.chosen = Some(Chosen {
                place,
                typed_coordinates: false,
            });
        }
        ui.add_space(layout::ROW_GAP);

        // Coordinates.
        let mut use_coords = false;
        layout::labelled_row(ui, columns, &ot_i18n::text("prayer-coords-label"), |ui| {
            let field = (columns.field - 150.0).max(120.0);
            ui.add_sized(
                [field, layout::field_height(ui)],
                egui::TextEdit::singleline(&mut self.coords)
                    .hint_text(ot_i18n::text("prayer-coords-placeholder")),
            );
            if button(ui, ot_i18n::text("prayer-coords-use")).clicked() {
                use_coords = true;
            }
        });
        if use_coords {
            match location::from_coordinates(&self.coords, &self.system_tz) {
                Ok(place) => {
                    self.coords_error = None;
                    action.chosen = Some(Chosen {
                        place,
                        typed_coordinates: true,
                    });
                }
                Err(error) => self.coords_error = Some(coordinate_error_key(&error)),
            }
        }
        if let Some(key) = self.coords_error {
            ui.add_space(layout::ROW_GAP / 2.0);
            note(ui, &ot_i18n::text(key), tokens.danger);
        }
        ui.add_space(layout::ROW_GAP);
        if button(ui, ot_i18n::text("prayer-close")).clicked() {
            self.search = Search::Idle;
            action.close = true;
        }
        action
    }
}

impl Default for Picker {
    fn default() -> Self {
        Self::new()
    }
}

/// The Fluent key of what is wrong with typed coordinates.
pub(crate) fn coordinate_error_key(error: &CoordinateError) -> &'static str {
    match error {
        CoordinateError::Format => "prayer-coords-error-format",
        CoordinateError::Range => "prayer-coords-error-range",
        CoordinateError::TimeZone(_) => "prayer-coords-error-zone",
    }
}
