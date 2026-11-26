//! Settings ▸ Prayer (FEAT-0057, `docs/11-prayer-and-azan.md` §1, §3, §6, §8).
//!
//! The pane draws and nothing in it blocks: the lookups, the search and the day's
//! times run on the threads of [`crate::prayer::Controller`], and each frame
//! [`State::tick`] drains what has come back. What is derived — the plan, the
//! zone matches, the formatted times — is cached and rebuilt when its inputs
//! change, never every frame (`.claude/rules/ui.md`, "Per-frame cost").

use jiff::Timestamp;
use jiff::tz::TimeZone;
use ot_core::{LocationSettings, PrayerSettings};
use ot_prayer::calculated::{Asr, Preset};
use ot_prayer::chain::{Resolution, Resolved, Source, SourceChoice};
use ot_prayer::jakim::Prayer;
use ot_prayer::location::{
    self, Consent, CoordinateError, Located, LookupError, MALAYSIA_TZ, Origin as LocationOrigin,
    Step,
};
use ot_prayer::zones::{self, Zone};
use ot_store::location::{Origin, StoredLocation};

use crate::layout::{self, Columns};
use crate::prayer::{self, Controller, Plan, Reply, Warning};
use crate::theme::Tokens;

/// The prayers listed, in the order of the day. Imsak and Dhuha stay hidden
/// until they can be switched on (spec §1).
const LISTED: [(Prayer, &str); 6] = [
    (Prayer::Fajr, "prayer-name-fajr"),
    (Prayer::Syuruk, "prayer-name-syuruk"),
    (Prayer::Dhuhr, "prayer-name-dhuhr"),
    (Prayer::Asr, "prayer-name-asr"),
    (Prayer::Maghrib, "prayer-name-maghrib"),
    (Prayer::Isha, "prayer-name-isha"),
];

/// The source label in the active language, with when the times were fetched in
/// Malaysian time, which is the zone JAKIM's times are in.
pub fn source_label(source: Source) -> String {
    let tz = TimeZone::get(MALAYSIA_TZ).unwrap_or(TimeZone::UTC);
    let fetched = source
        .fetched_at()
        .map(|at| {
            ot_i18n::format::format_datetime(
                &at.to_zoned_datetime(tz.clone()).datetime(),
                ot_i18n::format::TimePrefs::current(),
            )
        })
        .unwrap_or_default();
    ot_i18n::text_with(source.label_key(), &[("time", fetched.into())])
}

/// Every prayer with a template, by the scheduler's key and its name's key, in
/// the order of the day. Jumu'ah is one of them though it has no time of its
/// own: it is Dhuhr's, on Fridays.
const TEMPLATES: [(&str, &str); 7] = [
    ("fajr", "prayer-name-fajr"),
    ("syuruk", "prayer-name-syuruk"),
    ("dhuhr", "prayer-name-dhuhr"),
    ("jumuah", "prayer-name-jumuah"),
    ("asr", "prayer-name-asr"),
    ("maghrib", "prayer-name-maghrib"),
    ("isha", "prayer-name-isha"),
];

/// A prayer's name in both languages, from the scheduler's key.
pub fn prayer_name(key: &str) -> String {
    TEMPLATES
        .iter()
        .find(|(scheduler_key, _)| *scheduler_key == key)
        .map_or_else(|| key.to_owned(), |(_, fluent)| ot_i18n::text(fluent))
}

/// Where a city search stands.
#[derive(Debug)]
enum Search {
    Idle,
    Loading,
    Results(Vec<Located>),
    Failed(&'static str),
}

/// Where today's times stand.
#[derive(Debug)]
enum Day {
    /// No place and no zone to work from.
    NoPlace,
    Loading,
    Times,
    Unavailable,
}

/// Everything the pane remembers between frames.
pub struct State {
    controller: Controller,
    saved: Option<StoredLocation>,
    loaded: bool,
    detecting: bool,
    /// Whether this run has asked the lookup service already, so a failure is
    /// not retried every frame.
    detect_attempted: bool,
    detect_error: Option<&'static str>,
    plan_dirty: bool,
    plan: Option<Plan>,
    day: Day,
    resolved: Option<Resolved>,
    /// The day's rows, as formatted text: name, time.
    rows: Vec<(String, String)>,
    label: String,
    editing: bool,
    city: String,
    search: Search,
    coords: String,
    coords_error: Option<&'static str>,
    coords_note: Option<String>,
    zone_open: bool,
    zone_query: String,
    /// The zone list for `zone_query`, rebuilt only when the text changes.
    zone_rows: Vec<(String, String)>,
    /// The query and language `zone_rows` was built for.
    zone_rows_for: Option<(String, bool)>,
    system_tz: String,
    /// The saved place changed: the prayer service is told once.
    changed: bool,
    /// Which prayer's template is being edited (an index into `LISTED`), what
    /// is typed, and whether that prayer's saved template has been asked for.
    template_prayer: usize,
    edit: TemplateEdit,
    template_loaded: Option<usize>,
    template_saved: bool,
    /// Why the last chosen clip was refused, as a Fluent key.
    sound_error: Option<&'static str>,
    /// A file dialog or an import is under way.
    importing: bool,
}

/// A template as typed: empty text is the built-in text.
#[derive(Debug, Clone, Default)]
struct TemplateEdit {
    title: String,
    message: String,
    accent_on: bool,
    accent: egui::Color32,
    /// The picture's path under the data folder, or `None` for the built-in.
    picture: Option<String>,
    sound_mode: SoundMode,
    /// The imported clip's path under the data folder.
    sound_file: Option<String>,
}

/// Which sound a prayer plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum SoundMode {
    #[default]
    Chime,
    Azan,
    File,
    Silent,
}

impl SoundMode {
    const ALL: [Self; 4] = [Self::Chime, Self::Azan, Self::File, Self::Silent];

    fn as_str(self) -> &'static str {
        match self {
            Self::Chime => "chime",
            Self::Azan => "azan",
            Self::File => "file",
            Self::Silent => "none",
        }
    }

    fn parse(text: Option<&str>) -> Self {
        Self::ALL
            .into_iter()
            .find(|mode| Some(mode.as_str()) == text)
            .unwrap_or_default()
    }

    fn key(self) -> &'static str {
        match self {
            Self::Chime => "prayer-sound-chime",
            Self::Azan => "prayer-sound-azan",
            Self::File => "prayer-sound-file",
            Self::Silent => "prayer-sound-none",
        }
    }
}

/// A picture in the library, as the template's picker lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureChoice {
    pub sha256: String,
    pub relative_path: String,
    pub label: String,
}

/// A request to show the toast as it would look, from the editor's Preview.
#[derive(Debug, Clone)]
pub struct Preview {
    pub prayer: String,
    pub template: ot_store::reminders::Template,
}

impl State {
    /// Start reading the saved place; nothing is drawn until it arrives.
    pub fn new(root: &std::path::Path) -> Self {
        let state = Self::new_idle(root);
        state.controller.load_location();
        state
    }

    fn new_idle(root: &std::path::Path) -> Self {
        let controller = Controller::new(root);
        Self {
            controller,
            saved: None,
            loaded: false,
            detecting: false,
            detect_attempted: false,
            detect_error: None,
            plan_dirty: true,
            plan: None,
            day: Day::NoPlace,
            resolved: None,
            rows: Vec::new(),
            label: String::new(),
            editing: false,
            city: String::new(),
            search: Search::Idle,
            coords: String::new(),
            coords_error: None,
            coords_note: None,
            zone_open: false,
            zone_query: String::new(),
            zone_rows: Vec::new(),
            zone_rows_for: None,
            system_tz: TimeZone::system()
                .iana_name()
                .unwrap_or_default()
                .to_string(),
            changed: false,
            template_prayer: 0,
            edit: TemplateEdit::default(),
            template_loaded: None,
            template_saved: false,
            sound_error: None,
            importing: false,
        }
    }

    /// A state with every part of the pane on show, for the layout tests: the
    /// place, the entry controls with results and errors, the zone list and a
    /// full day. Nothing is started and nothing is read from disk.
    #[doc(hidden)]
    pub fn for_layout_test(
        root: &std::path::Path,
        settings: &PrayerSettings,
        place: Option<StoredLocation>,
        results: Vec<Located>,
    ) -> Self {
        let mut state = Self::new_idle(root);
        state.loaded = true;
        state.saved = place;
        let plan = prayer::plan(settings, state.saved.as_ref());
        state.plan = Some(plan);
        state.editing = true;
        state.city = "Shah Alam".to_string();
        state.search = Search::Results(results);
        state.coords = "3.14, 101.69".to_string();
        state.coords_error = Some("prayer-coords-error-range");
        state.coords_note = Some(ot_i18n::text_with(
            "prayer-coords-zone-note",
            &[("zone", "America/Argentina/ComodRivadavia".into())],
        ));
        state.detect_error = Some("prayer-error-service");
        state.zone_open = true;
        // The template editor at its fullest: text as long as the field holds, a
        // custom accent and a picture whose label is far longer than the field.
        // Text longer than the field is clipped by egui, which the accessibility
        // bounds this test reads do not show: that is looked at in the app.
        state.edit = TemplateEdit {
            title: "Masuk waktu Subuh, bangunlah".to_string(),
            message: "Jangan lewat, selepas azan".to_string(),
            accent_on: true,
            accent: egui::Color32::from_rgb(0x3b, 0x9e, 0xff),
            picture: Some("assets/long-label.png".to_string()),
            sound_mode: SoundMode::File,
            sound_file: Some(
                "assets/0123456789abcdef0123456789abcdef0123456789abcdef.mp3".to_string(),
            ),
        };
        state.sound_error = Some("error-import-audio-too-large");
        state.template_saved = true;
        state.day = Day::Times;
        state.rows = LISTED
            .iter()
            .map(|(_, key)| (ot_i18n::text(key), "12:34 pm".to_string()))
            .collect();
        state.label = ot_i18n::text_with(
            "prayer-source-calculated-fallback-approximate",
            &[("time", "26/11/2026 at 11:26 am".into())],
        );
        state
    }

    /// Whether the saved place changed since last asked, so the prayer service
    /// can be told. Reading it clears it.
    pub fn take_changed(&mut self) -> bool {
        std::mem::take(&mut self.changed)
    }

    /// The settings changed (or the place did): work out the plan again.
    pub fn mark_dirty(&mut self) {
        self.plan_dirty = true;
    }

    /// Stop the work in flight, for a window that is closing. The day is worked
    /// out again the next time the pane is drawn, so a cancelled resolve never
    /// leaves it stuck loading.
    pub fn cancel(&mut self) {
        self.controller.cancel();
        self.plan_dirty = true;
    }

    /// Whether an answer is still to come, so the window keeps repainting.
    pub fn busy(&self) -> bool {
        !self.loaded
            || self.detecting
            || matches!(self.day, Day::Loading)
            || matches!(self.search, Search::Loading)
    }

    /// A language or clock change: the rows and the label are text, so they are
    /// built again from what was resolved.
    pub fn reformat(&mut self) {
        self.rebuild_rows();
    }

    /// The step the rule gives for this consent and this saved place.
    fn step(&self, consent: Consent, now: Timestamp) -> Step {
        let saved = self.saved.as_ref().map(|place| {
            let origin = match place.origin {
                Origin::Ip => LocationOrigin::Ip,
                Origin::Manual => LocationOrigin::Manual,
            };
            (origin, place.updated_at)
        });
        location::next_step(consent, saved, now)
    }

    /// Drain the replies, start a lookup the rule calls for, and resolve the day
    /// when its inputs changed. Called once a frame, before drawing.
    pub fn tick(&mut self, prayer_settings: &PrayerSettings, consent: Consent, now: Timestamp) {
        for reply in self.controller.drain() {
            self.apply(reply);
        }
        if self.template_loaded != Some(self.template_prayer) {
            self.template_loaded = Some(self.template_prayer);
            self.controller
                .load_template(TEMPLATES[self.template_prayer].0.to_owned());
        }
        if !self.loaded {
            return;
        }
        match self.step(consent, now) {
            Step::Lookup if !self.detecting && !self.detect_attempted => {
                self.detecting = true;
                self.detect_attempted = true;
                self.detect_error = None;
                self.controller.detect(Consent::Granted);
            }
            Step::EnterManually if self.saved.is_none() => self.editing = true,
            _ => {}
        }
        if self.plan_dirty {
            self.plan_dirty = false;
            let plan = prayer::plan(prayer_settings, self.saved.as_ref());
            if plan.zone.is_some() || plan.calculation.is_some() {
                self.day = Day::Loading;
                self.controller.resolve(plan.clone());
            } else {
                self.controller.cancel();
                self.day = Day::NoPlace;
                self.resolved = None;
                self.rows.clear();
            }
            self.plan = Some(plan);
        }
    }

    fn apply(&mut self, reply: Reply) {
        match reply {
            Reply::Location(place) => {
                self.changed |= self.loaded && self.saved != place;
                self.saved = place;
                self.loaded = true;
                self.detecting = false;
                self.plan_dirty = true;
            }
            Reply::DetectFailed(error) => {
                self.detecting = false;
                self.detect_error = Some(error_key(&error));
            }
            Reply::Search(Ok(found)) => self.search = Search::Results(found),
            Reply::Search(Err(error)) => self.search = Search::Failed(error_key(&error)),
            Reply::Day(Resolution::Times(resolved)) => {
                self.resolved = Some(resolved);
                self.day = Day::Times;
                self.rebuild_rows();
            }
            Reply::Day(Resolution::Nothing) => {
                self.day = Day::Unavailable;
                self.resolved = None;
                self.rows.clear();
            }
            Reply::Template { prayer, template } => {
                if prayer == TEMPLATES[self.template_prayer].0 {
                    let template = template.unwrap_or_default();
                    self.edit = TemplateEdit {
                        title: template.custom_title.unwrap_or_default(),
                        message: template.custom_body.unwrap_or_default(),
                        accent_on: template.accent_color.is_some(),
                        accent: template
                            .accent_color
                            .as_deref()
                            .and_then(parse_hex)
                            .unwrap_or(egui::Color32::from_rgb(0x3b, 0x9e, 0xff)),
                        picture: template.image,
                        sound_mode: SoundMode::parse(template.sound_mode.as_deref()),
                        sound_file: template.sound,
                    };
                }
            }
            Reply::AudioImported { relative_path } => {
                self.edit.sound_file = Some(relative_path);
                self.edit.sound_mode = SoundMode::File;
                self.sound_error = None;
                self.importing = false;
            }
            Reply::AudioRefused(key) => {
                self.sound_error = Some(key);
                self.importing = false;
            }
            Reply::AudioCancelled => self.importing = false,
            Reply::Failed(reason) => {
                tracing::warn!(%reason, "a prayer pane request failed");
                self.detecting = false;
                self.loaded = true;
                if matches!(self.day, Day::Loading) {
                    self.day = Day::Unavailable;
                }
            }
        }
    }

    /// The rows and the label, formatted for the language and clock in force.
    fn rebuild_rows(&mut self) {
        self.rows.clear();
        self.label.clear();
        let (Some(resolved), Some(plan)) = (&self.resolved, &self.plan) else {
            return;
        };
        // JAKIM's times are Malaysian clock values; a calculated day is read in
        // the place's own zone.
        let tz_name = match resolved.source {
            Source::Calculated { .. } => plan.tz.as_str(),
            _ => MALAYSIA_TZ,
        };
        let tz = TimeZone::get(tz_name).unwrap_or(TimeZone::UTC);
        let prefs = ot_i18n::format::TimePrefs::current();
        for (prayer, key) in LISTED {
            if let Some((_, at)) = resolved.times.iter().find(|(p, _)| *p == prayer) {
                let local = at.to_zoned_datetime(tz.clone()).datetime();
                self.rows.push((
                    ot_i18n::text(key),
                    ot_i18n::format::format_time(&local, prefs),
                ));
            }
        }
        self.label = source_label(resolved.source);
    }

    /// The zone list as formatted rows (code, line), rebuilt only when the search
    /// text or the language changed — never formatted again each frame.
    fn refresh_zone_rows(&mut self) {
        let english = english();
        let current = self
            .zone_rows_for
            .as_ref()
            .is_some_and(|(query, language)| *query == self.zone_query && *language == english);
        if !current {
            self.zone_rows = zones::search(&self.zone_query)
                .into_iter()
                .map(|zone| (zone.code.clone(), zone_line("prayer-zone-row", zone)))
                .collect();
            self.zone_rows_for = Some((self.zone_query.clone(), english));
        }
    }
}

/// The Fluent key of what went wrong with a lookup or a search.
fn error_key(error: &LookupError) -> &'static str {
    match error {
        LookupError::Network(_) | LookupError::Http(500..) => "prayer-error-network",
        LookupError::Service(_) | LookupError::Http(_) => "prayer-error-service",
        _ => "prayer-error-answer",
    }
}

fn coordinate_error_key(error: &CoordinateError) -> &'static str {
    match error {
        CoordinateError::Format => "prayer-coords-error-format",
        CoordinateError::Range => "prayer-coords-error-range",
        CoordinateError::TimeZone(_) => "prayer-coords-error-zone",
    }
}

fn english() -> bool {
    ot_i18n::current_language() != Some(ot_i18n::Language::Malay)
}

fn language_code() -> &'static str {
    if english() { "en" } else { "ms" }
}

/// A zone as one wrapped line: code, state and areas, in the active language.
fn zone_line(key: &str, zone: &Zone) -> String {
    let english = english();
    ot_i18n::text_with(
        key,
        &[
            ("code", zone.code.clone().into()),
            ("state", zone.state(english).to_string().into()),
            ("areas", zone.areas(english).to_string().into()),
        ],
    )
}

/// A button as tall as the fields beside it.
fn button(ui: &mut egui::Ui, text: String) -> egui::Response {
    // A minimum height and no fixed width: a zero width squeezes the label to a
    // letter a line.
    ui.add(egui::Button::new(text).min_size(egui::vec2(0.0, layout::field_height(ui))))
}

/// A box and a label that wraps: a long Malay sentence must not run past the
/// pane, and clicking the words toggles the box. Returns whether it changed.
fn wrapped_checkbox(ui: &mut egui::Ui, indent: f32, on: &mut bool, text: &str) -> bool {
    let mut toggled = false;
    // On one middle line with the label: the row opens as tall as a field, and
    // the box is made that tall too, so the words are centred on it. Raising
    // the height inside the closure would leave the row on its old height.
    let height = layout::field_height(ui);
    ui.spacing_mut().interact_size.y = height;
    ui.horizontal(|ui| {
        ui.set_min_height(height);
        ui.add_space(indent);
        toggled |= ui.checkbox(on, "").changed();
        let words = ui.add(egui::Label::new(text).wrap().sense(egui::Sense::click()));
        if words.clicked() {
            *on = !*on;
            toggled = true;
        }
    });
    toggled
}

fn subheading(ui: &mut egui::Ui, key: &str) {
    ui.label(egui::RichText::new(ot_i18n::text(key)).strong());
    ui.add_space(layout::ROW_GAP);
}

fn rule(ui: &mut egui::Ui) {
    ui.add_space(layout::ROW_GAP);
    ui.separator();
    ui.add_space(layout::ROW_GAP);
}

/// A sentence in the pane, wrapped to the width it has, in a colour.
fn note(ui: &mut egui::Ui, text: &str, colour: egui::Color32) {
    ui.add(egui::Label::new(egui::RichText::new(text).color(colour)).wrap());
}

/// What a frame of the pane changed.
#[derive(Debug, Default, Clone)]
pub struct Outcome {
    /// `settings.toml` should be saved.
    pub settings_changed: bool,
    /// Show this toast: the editor's Preview.
    pub preview: Option<Preview>,
    /// Play this prayer's sound as the template has it now: the Play Sound button.
    pub play_sound: Option<Preview>,
    /// Stop the sound.
    pub stop_sound: bool,
}

/// Draw the pane. The window owns the settings and saves them when
/// [`Outcome::settings_changed`] is set.
pub fn show(
    ui: &mut egui::Ui,
    state: &mut State,
    prayer_settings: &mut PrayerSettings,
    location_settings: &mut LocationSettings,
    pictures: &[PictureChoice],
    tokens: Tokens,
    now: Timestamp,
) -> Outcome {
    let mut outcome = Outcome::default();
    let columns = layout::columns(ui);
    let consent = Consent::from(location_settings.ip_consent);

    location_block(
        ui,
        columns,
        state,
        location_settings,
        consent,
        tokens,
        now,
        &mut outcome,
    );
    rule(ui);
    source_block(ui, columns, state, prayer_settings, tokens, &mut outcome);
    rule(ui);
    notification_block(ui, columns, prayer_settings, tokens, &mut outcome);
    rule(ui);
    template_block(ui, columns, state, pictures, tokens, &mut outcome);
    rule(ui);
    today_block(ui, state, tokens);
    outcome
}

#[allow(
    clippy::too_many_arguments,
    reason = "one pane block, each argument a distinct input"
)]
fn location_block(
    ui: &mut egui::Ui,
    columns: Columns,
    state: &mut State,
    settings: &mut LocationSettings,
    consent: Consent,
    tokens: Tokens,
    now: Timestamp,
    outcome: &mut Outcome,
) {
    subheading(ui, "prayer-location-heading");
    if !state.loaded {
        note(
            ui,
            &ot_i18n::text("prayer-location-looking"),
            tokens.text_dim,
        );
        return;
    }

    if state.step(consent, now) == Step::AskConsent {
        egui::Frame::new()
            .fill(tokens.surface_raised)
            .stroke(egui::Stroke::new(1.0_f32, tokens.border))
            .corner_radius(6.0_f32)
            .inner_margin(layout::PANE_MARGIN)
            .show(ui, |ui| {
                ui.label(egui::RichText::new(ot_i18n::text("prayer-consent-title")).strong());
                ui.add_space(layout::ROW_GAP);
                note(ui, &ot_i18n::text("prayer-consent-body"), tokens.text);
                ui.add_space(layout::ROW_GAP);
                ui.horizontal_wrapped(|ui| {
                    if button(ui, ot_i18n::text("prayer-consent-allow")).clicked() {
                        settings.ip_consent = Some(true);
                        outcome.settings_changed = true;
                        state.detect_attempted = false;
                    }
                    ui.add_space(layout::OPTION_GAP - ui.spacing().item_spacing.x);
                    if button(ui, ot_i18n::text("prayer-consent-decline")).clicked() {
                        settings.ip_consent = Some(false);
                        outcome.settings_changed = true;
                        state.editing = true;
                    }
                });
            });
        ui.add_space(layout::ROW_GAP);
    } else {
        // The saved place, or the lookup in progress, or why it failed.
        if state.detecting {
            note(
                ui,
                &ot_i18n::text("prayer-location-looking"),
                tokens.text_dim,
            );
        } else if let Some(place) = &state.saved {
            let key = match place.origin {
                Origin::Ip => "prayer-location-detected",
                Origin::Manual => "prayer-location-manual",
            };
            note(
                ui,
                &ot_i18n::text_with(key, &[("place", place.name.clone().into())]),
                tokens.text,
            );
        } else {
            note(ui, &ot_i18n::text("prayer-location-none"), tokens.text_dim);
        }
        // Typed coordinates outside Malaysia took this PC's time zone, which may
        // not be the place's: said where the place is shown, until it changes.
        if let Some(text) = &state.coords_note {
            ui.add_space(layout::ROW_GAP / 2.0);
            note(ui, text, tokens.danger);
        }
        if let Some(key) = state.detect_error {
            ui.add_space(layout::ROW_GAP / 2.0);
            note(ui, &ot_i18n::text(key), tokens.danger);
        }
        ui.add_space(layout::ROW_GAP);

        // The consent can be changed after the first answer.
        let mut agreed = consent == Consent::Granted;
        if wrapped_checkbox(
            ui,
            columns.label,
            &mut agreed,
            &ot_i18n::text("prayer-detect-checkbox"),
        ) {
            settings.ip_consent = Some(agreed);
            outcome.settings_changed = true;
            state.detect_attempted = false;
        }
        layout::field_hint(
            ui,
            columns,
            &ot_i18n::text("prayer-detect-hint"),
            tokens.text_dim,
        );
        ui.add_space(layout::ROW_GAP);
    }

    if !state.editing {
        if button(ui, ot_i18n::text("prayer-location-change")).clicked() {
            state.editing = true;
        }
        return;
    }
    entry_controls(ui, columns, state, tokens);
}

/// City search and typed coordinates.
fn entry_controls(ui: &mut egui::Ui, columns: Columns, state: &mut State, tokens: Tokens) {
    // City.
    let mut run_search = false;
    layout::labelled_row(ui, columns, &ot_i18n::text("prayer-search-label"), |ui| {
        let field = (columns.field - 96.0).max(120.0);
        let response = ui.add_sized(
            [field, layout::field_height(ui)],
            egui::TextEdit::singleline(&mut state.city)
                .hint_text(ot_i18n::text("prayer-search-placeholder")),
        );
        let enter = response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
        if button(ui, ot_i18n::text("prayer-search-button")).clicked() || enter {
            run_search = true;
        }
    });
    if run_search && !state.city.trim().is_empty() {
        state.search = Search::Loading;
        state.controller.search(state.city.clone(), language_code());
    }
    let mut chosen = None;
    match &state.search {
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
        state.controller.save_manual(place);
        state.search = Search::Idle;
        state.editing = false;
        state.coords_note = None;
    }
    ui.add_space(layout::ROW_GAP);

    // Coordinates.
    let mut use_coords = false;
    layout::labelled_row(ui, columns, &ot_i18n::text("prayer-coords-label"), |ui| {
        let field = (columns.field - 150.0).max(120.0);
        ui.add_sized(
            [field, layout::field_height(ui)],
            egui::TextEdit::singleline(&mut state.coords)
                .hint_text(ot_i18n::text("prayer-coords-placeholder")),
        );
        if button(ui, ot_i18n::text("prayer-coords-use")).clicked() {
            use_coords = true;
        }
    });
    if use_coords {
        match location::from_coordinates(&state.coords, &state.system_tz) {
            Ok(place) => {
                state.coords_error = None;
                // Outside Malaysia the zone is this PC's, which is a guess.
                state.coords_note = (place.tz != MALAYSIA_TZ).then(|| {
                    ot_i18n::text_with(
                        "prayer-coords-zone-note",
                        &[("zone", place.tz.clone().into())],
                    )
                });
                state.controller.save_manual(place);
                state.editing = false;
            }
            Err(error) => {
                state.coords_note = None;
                state.coords_error = Some(coordinate_error_key(&error));
            }
        }
    }
    if let Some(key) = state.coords_error {
        ui.add_space(layout::ROW_GAP / 2.0);
        note(ui, &ot_i18n::text(key), tokens.danger);
    }
    ui.add_space(layout::ROW_GAP);
    if button(ui, ot_i18n::text("prayer-close")).clicked() {
        state.editing = false;
        state.search = Search::Idle;
    }
}

fn source_block(
    ui: &mut egui::Ui,
    columns: Columns,
    state: &mut State,
    settings: &mut PrayerSettings,
    tokens: Tokens,
    outcome: &mut Outcome,
) {
    // Source.
    let source = SourceChoice::parse(&settings.source);
    let mut chosen = source;
    layout::labelled_row(ui, columns, &ot_i18n::text("prayer-source-label"), |ui| {
        egui::ComboBox::from_id_salt("prayer-source")
            .width(columns.field)
            .height(layout::field_height(ui))
            .selected_text(ot_i18n::text(source.name_key()))
            .show_ui(ui, |ui| {
                for option in SourceChoice::all() {
                    ui.selectable_value(&mut chosen, option, ot_i18n::text(option.name_key()));
                }
            });
    });
    if chosen != source {
        settings.source = chosen.as_str().to_string();
        outcome.settings_changed = true;
        state.mark_dirty();
    }
    layout::field_hint(
        ui,
        columns,
        &ot_i18n::text("prayer-source-hint"),
        tokens.text_dim,
    );
    if let Some(warning) = state.plan.as_ref().and_then(|plan| plan.warning) {
        ui.add_space(layout::ROW_GAP / 2.0);
        let key = match warning {
            Warning::JakimNeedsAZone => "prayer-warning-needs-zone",
            Warning::JakimAbroad => "prayer-warning-abroad",
        };
        note(ui, &ot_i18n::text(key), tokens.danger);
    }
    ui.add_space(layout::ROW_GAP);

    if chosen != SourceChoice::Calculated {
        zone_block(ui, columns, state, settings, tokens, outcome);
    }
    if chosen != SourceChoice::Jakim {
        calculation_block(ui, columns, state, settings, tokens, outcome);
    }
}

fn zone_block(
    ui: &mut egui::Ui,
    columns: Columns,
    state: &mut State,
    settings: &mut PrayerSettings,
    tokens: Tokens,
    outcome: &mut Outcome,
) {
    ui.label(egui::RichText::new(ot_i18n::text("prayer-zone-label")).strong());
    ui.add_space(layout::ROW_GAP / 2.0);
    match state
        .plan
        .as_ref()
        .and_then(|plan| plan.zone.map(|z| (z, plan.zone_detected)))
    {
        Some((zone, detected)) => {
            let key = if detected {
                "prayer-zone-detected"
            } else {
                "prayer-zone-chosen"
            };
            note(ui, &zone_line(key, zone), tokens.text);
        }
        None => note(ui, &ot_i18n::text("prayer-zone-none"), tokens.text_dim),
    }
    ui.add_space(layout::ROW_GAP);
    ui.horizontal_wrapped(|ui| {
        if button(ui, ot_i18n::text("prayer-zone-change")).clicked() {
            state.zone_open = !state.zone_open;
        }
        if settings.zone.is_some() {
            ui.add_space(layout::OPTION_GAP - ui.spacing().item_spacing.x);
            if button(ui, ot_i18n::text("prayer-zone-use-detected")).clicked() {
                settings.zone = None;
                outcome.settings_changed = true;
                state.mark_dirty();
            }
        }
    });
    if state.zone_open {
        ui.add_space(layout::ROW_GAP);
        ui.add_sized(
            [
                columns.field.max(columns.label + 120.0),
                layout::field_height(ui),
            ],
            egui::TextEdit::singleline(&mut state.zone_query)
                .hint_text(ot_i18n::text("prayer-zone-search-placeholder")),
        );
        ui.add_space(layout::ROW_GAP);
        state.refresh_zone_rows();
        // Taken out for the drawing and put back, so the rows are neither cloned
        // nor formatted again.
        let rows = std::mem::take(&mut state.zone_rows);
        if rows.is_empty() {
            note(ui, &ot_i18n::text("prayer-zone-no-match"), tokens.text_dim);
        } else {
            let mut picked = None;
            egui::ScrollArea::vertical()
                .id_salt("prayer-zones")
                .min_scrolled_height(220.0)
                .max_height(220.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    for (code, line) in &rows {
                        let selected = settings.zone.as_deref() == Some(code.as_str());
                        if ui
                            .add(egui::Button::selectable(selected, line.as_str()).wrap())
                            .clicked()
                        {
                            picked = Some(code.clone());
                        }
                    }
                });
            if let Some(code) = picked {
                settings.zone = Some(code);
                outcome.settings_changed = true;
                state.zone_open = false;
                state.mark_dirty();
            }
        }
        state.zone_rows = rows;
    }
    ui.add_space(layout::ROW_GAP);
}

fn calculation_block(
    ui: &mut egui::Ui,
    columns: Columns,
    state: &mut State,
    settings: &mut PrayerSettings,
    tokens: Tokens,
    outcome: &mut Outcome,
) {
    // The method shown is the one in force, which may be the default for where
    // the user is.
    let in_malaysia = state.plan.as_ref().is_some_and(|plan| plan.in_malaysia);
    let preset = settings
        .preset
        .as_deref()
        .and_then(Preset::parse)
        .unwrap_or_else(|| Preset::default_for(in_malaysia));
    let mut chosen = preset;
    layout::labelled_row(ui, columns, &ot_i18n::text("prayer-method-label"), |ui| {
        egui::ComboBox::from_id_salt("prayer-method")
            .width(columns.field)
            // The popup's own cap: four methods need room, or the last one is
            // hidden behind a scroll bar nobody looks for.
            .height(layout::field_height(ui) * 6.0)
            .selected_text(ot_i18n::text(preset.name_key()))
            .show_ui(ui, |ui| {
                for option in Preset::all() {
                    ui.selectable_value(&mut chosen, option, ot_i18n::text(option.name_key()));
                }
            });
    });
    if chosen != preset {
        settings.preset = Some(chosen.as_str().to_string());
        outcome.settings_changed = true;
        state.mark_dirty();
    }
    ui.add_space(layout::ROW_GAP);

    let asr = Asr::parse(&settings.asr);
    let mut chosen_asr = asr;
    layout::labelled_row(ui, columns, &ot_i18n::text("prayer-asr-label"), |ui| {
        egui::ComboBox::from_id_salt("prayer-asr")
            .width(columns.field)
            .height(layout::field_height(ui))
            .selected_text(ot_i18n::text(asr.name_key()))
            .show_ui(ui, |ui| {
                for option in Asr::all() {
                    ui.selectable_value(&mut chosen_asr, option, ot_i18n::text(option.name_key()));
                }
            });
    });
    if chosen_asr != asr {
        settings.asr = chosen_asr.as_str().to_string();
        outcome.settings_changed = true;
        state.mark_dirty();
    }
    layout::field_hint(
        ui,
        columns,
        &ot_i18n::text("prayer-asr-hint"),
        tokens.text_dim,
    );
}

/// Which prayers announce themselves, and how late a missed one may be and
/// still be listed (spec §9, `docs/specs/reminders.md` §6).
fn notification_block(
    ui: &mut egui::Ui,
    columns: Columns,
    settings: &mut PrayerSettings,
    tokens: Tokens,
    outcome: &mut Outcome,
) {
    subheading(ui, "prayer-notify-heading");
    let mut syuruk = settings.notify_syuruk;
    if wrapped_checkbox(
        ui,
        columns.label,
        &mut syuruk,
        &ot_i18n::text("prayer-notify-syuruk"),
    ) {
        settings.notify_syuruk = syuruk;
        outcome.settings_changed = true;
    }
    let mut jumuah = settings.notify_jumuah;
    if wrapped_checkbox(
        ui,
        columns.label,
        &mut jumuah,
        &ot_i18n::text("prayer-notify-jumuah"),
    ) {
        settings.notify_jumuah = jumuah;
        outcome.settings_changed = true;
    }
    layout::field_hint(
        ui,
        columns,
        &ot_i18n::text("prayer-notify-hint"),
        tokens.text_dim,
    );
    ui.add_space(layout::ROW_GAP);

    let mut minutes = settings.azan_window();
    layout::labelled_row(
        ui,
        columns,
        &ot_i18n::text("prayer-azan-window-label"),
        |ui| {
            ui.add(
                egui::DragValue::new(&mut minutes)
                    .range(ot_core::AZAN_WINDOW_MINUTES)
                    .speed(1.0),
            );
        },
    );
    if minutes != settings.azan_window() {
        settings.azan_window_minutes = minutes;
        outcome.settings_changed = true;
    }
    layout::field_hint(
        ui,
        columns,
        &ot_i18n::text("prayer-azan-window-hint"),
        tokens.text_dim,
    );
    ui.add_space(layout::ROW_GAP);

    // A reminder ahead of each prayer that has a notification (spec §9).
    let mut lead = settings.pre_prayer_lead();
    layout::labelled_row(ui, columns, &ot_i18n::text("prayer-before-label"), |ui| {
        ui.add(
            egui::DragValue::new(&mut lead)
                .range(ot_core::PRE_PRAYER_MINUTES)
                .speed(1.0),
        );
    });
    if lead != settings.pre_prayer_lead() {
        settings.pre_prayer_minutes = lead;
        outcome.settings_changed = true;
    }
    for key in settings.enabled_prayers() {
        let mut on = settings.pre_prayer.iter().any(|chosen| chosen == key);
        if wrapped_checkbox(ui, columns.label, &mut on, &prayer_name(key)) {
            settings.pre_prayer.retain(|chosen| chosen != key);
            if on {
                settings.pre_prayer.push(key.to_owned());
            }
            outcome.settings_changed = true;
        }
    }
    layout::field_hint(
        ui,
        columns,
        &ot_i18n::text("prayer-before-hint"),
        tokens.text_dim,
    );
}

/// `text` cut to `max` characters with an ellipsis: for a name the user gave,
/// which can be any length, and never for the app's own labels.
fn ellipsize(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let kept: String = text.chars().take(max.saturating_sub(1)).collect();
    format!("{kept}…")
}

fn parse_hex(text: &str) -> Option<egui::Color32> {
    let hex = text.strip_prefix('#')?;
    if hex.len() != 6 || !hex.is_ascii() {
        return None;
    }
    let channel = |at: usize| u8::from_str_radix(&hex[at..at + 2], 16).ok();
    Some(egui::Color32::from_rgb(
        channel(0)?,
        channel(2)?,
        channel(4)?,
    ))
}

fn hex(colour: egui::Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", colour.r(), colour.g(), colour.b())
}

/// What a prayer's notification says and shows (spec toast.md §8): its title,
/// message, accent and picture, each empty meaning the built-in.
fn template_block(
    ui: &mut egui::Ui,
    columns: Columns,
    state: &mut State,
    pictures: &[PictureChoice],
    tokens: Tokens,
    outcome: &mut Outcome,
) {
    subheading(ui, "prayer-template-heading");

    // Which prayer.
    let mut chosen = state.template_prayer;
    layout::labelled_row(
        ui,
        columns,
        &ot_i18n::text("prayer-template-prayer"),
        |ui| {
            egui::ComboBox::from_id_salt("prayer-template-prayer")
                .width(columns.field)
                .height(layout::field_height(ui) * 6.0)
                .selected_text(ot_i18n::text(TEMPLATES[chosen].1))
                .show_ui(ui, |ui| {
                    for (index, (_, key)) in TEMPLATES.iter().enumerate() {
                        ui.selectable_value(&mut chosen, index, ot_i18n::text(key));
                    }
                });
        },
    );
    if chosen != state.template_prayer {
        state.template_prayer = chosen;
        state.template_saved = false;
    }
    ui.add_space(layout::ROW_GAP);

    let prayer = TEMPLATES[state.template_prayer].0;
    let name = prayer_name(prayer);
    let field = columns.field;

    // Title and message: empty is the built-in text.
    layout::labelled_row(ui, columns, &ot_i18n::text("prayer-template-title"), |ui| {
        ui.add_sized(
            [field, layout::field_height(ui)],
            egui::TextEdit::singleline(&mut state.edit.title)
                .desired_width(field)
                .clip_text(true)
                .hint_text(name.clone()),
        );
    });
    ui.add_space(layout::ROW_GAP);
    layout::labelled_row(
        ui,
        columns,
        &ot_i18n::text("prayer-template-message"),
        |ui| {
            ui.add_sized(
                [field, layout::field_height(ui)],
                egui::TextEdit::singleline(&mut state.edit.message)
                    .desired_width(field)
                    .clip_text(true)
                    .hint_text(ot_i18n::text_with(
                        "prayer-toast-body",
                        &[("prayer", name.clone().into())],
                    )),
            );
        },
    );
    ui.add_space(layout::ROW_GAP);

    // The accent.
    wrapped_checkbox(
        ui,
        columns.label,
        &mut state.edit.accent_on,
        &ot_i18n::text("prayer-template-accent-use"),
    );
    if state.edit.accent_on {
        ui.horizontal(|ui| {
            ui.add_space(columns.label);
            egui::color_picker::color_edit_button_srgba(
                ui,
                &mut state.edit.accent,
                egui::color_picker::Alpha::Opaque,
            );
        });
    }
    ui.add_space(layout::ROW_GAP);

    // The picture: the built-in one for this prayer, or one from the library.
    let shown = state
        .edit
        .picture
        .as_deref()
        .and_then(|path| pictures.iter().find(|p| p.relative_path == path))
        .map_or_else(
            || ot_i18n::text("prayer-template-picture-builtin"),
            |picture| ellipsize(&picture.label, (columns.field / 9.0) as usize),
        );
    layout::labelled_row(
        ui,
        columns,
        &ot_i18n::text("prayer-template-picture"),
        |ui| {
            egui::ComboBox::from_id_salt("prayer-template-picture")
                .truncate()
                .width(columns.field)
                .height(layout::field_height(ui) * 6.0)
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut state.edit.picture,
                        None,
                        ot_i18n::text("prayer-template-picture-builtin"),
                    );
                    for picture in pictures {
                        ui.selectable_value(
                            &mut state.edit.picture,
                            Some(picture.relative_path.clone()),
                            picture.label.clone(),
                        );
                    }
                });
        },
    );
    layout::field_hint(
        ui,
        columns,
        &ot_i18n::text("prayer-template-hint"),
        tokens.text_dim,
    );
    ui.add_space(layout::ROW_GAP);

    // The sound: the chime by default, the full Azan, a clip of the user's own,
    // or none.
    let mut mode = state.edit.sound_mode;
    layout::labelled_row(ui, columns, &ot_i18n::text("prayer-sound-label"), |ui| {
        egui::ComboBox::from_id_salt("prayer-sound")
            .width(columns.field)
            .height(layout::field_height(ui) * 6.0)
            .selected_text(ot_i18n::text(mode.key()))
            .show_ui(ui, |ui| {
                for option in SoundMode::ALL {
                    ui.selectable_value(&mut mode, option, ot_i18n::text(option.key()));
                }
            });
    });
    state.edit.sound_mode = mode;
    if mode == SoundMode::File {
        ui.add_space(layout::ROW_GAP);
        let shown = state.edit.sound_file.as_deref().map_or_else(
            || ot_i18n::text("prayer-sound-none-chosen"),
            |path| ellipsize(path, (columns.field / 9.0) as usize),
        );
        ui.horizontal_wrapped(|ui| {
            ui.add_space(columns.label);
            note(ui, &shown, tokens.text_dim);
        });
        ui.horizontal_wrapped(|ui| {
            ui.add_space(columns.label);
            ui.add_enabled_ui(!state.importing, |ui| {
                if button(ui, ot_i18n::text("prayer-sound-choose")).clicked() {
                    state.importing = true;
                    state.sound_error = None;
                    state.controller.import_audio(
                        ot_i18n::text("prayer-sound-dialog-title"),
                        ot_i18n::text("prayer-sound-filter"),
                    );
                }
            });
        });
    }
    if let Some(key) = state.sound_error {
        ui.add_space(layout::ROW_GAP / 2.0);
        note(ui, &ot_i18n::text(key), tokens.danger);
    }
    ui.add_space(layout::ROW_GAP);

    // Save, restore, preview.
    let template = |state: &State| ot_store::reminders::Template {
        prayer_key: Some(prayer.to_owned()),
        custom_title: Some(state.edit.title.trim().to_owned()).filter(|t| !t.is_empty()),
        custom_body: Some(state.edit.message.trim().to_owned()).filter(|t| !t.is_empty()),
        accent_color: state.edit.accent_on.then(|| hex(state.edit.accent)),
        image: state.edit.picture.clone(),
        sound_mode: Some(state.edit.sound_mode.as_str().to_owned()),
        sound: state.edit.sound_file.clone(),
        ..Default::default()
    };
    let save = |state: &mut State| {
        let sha = state.edit.picture.as_deref().and_then(|path| {
            pictures
                .iter()
                .find(|p| p.relative_path == path)
                .map(|p| p.sha256.clone())
        });
        let language = if english() { "en-US" } else { "ms-MY" };
        let saved = template(state);
        state.controller.save_template(
            prayer.to_owned(),
            ot_store::reminders::PrayerTemplateEdit {
                title: saved.custom_title,
                body: saved.custom_body,
                accent: saved.accent_color,
                image: sha,
                authored_lang: language,
                sound_mode: (state.edit.sound_mode != SoundMode::Chime)
                    .then(|| state.edit.sound_mode.as_str().to_owned()),
                // The clip's name is its hash: `assets/<sha256>.<ext>`.
                sound: state
                    .edit
                    .sound_file
                    .as_deref()
                    .filter(|_| state.edit.sound_mode == SoundMode::File)
                    .and_then(|path| std::path::Path::new(path).file_stem())
                    .map(|stem| stem.to_string_lossy().into_owned()),
            },
        );
        state.template_saved = true;
    };
    ui.horizontal_wrapped(|ui| {
        if button(ui, ot_i18n::text("prayer-template-save")).clicked() {
            save(state);
        }
        ui.add_space(layout::OPTION_GAP - ui.spacing().item_spacing.x);
        if button(ui, ot_i18n::text("prayer-template-default")).clicked() {
            state.edit.title.clear();
            state.edit.message.clear();
            save(state);
        }
        ui.add_space(layout::OPTION_GAP - ui.spacing().item_spacing.x);
        if button(ui, ot_i18n::text("prayer-template-preview")).clicked() {
            outcome.preview = Some(Preview {
                prayer: prayer.to_owned(),
                template: template(state),
            });
        }
        ui.add_space(layout::OPTION_GAP - ui.spacing().item_spacing.x);
        if button(ui, ot_i18n::text("prayer-sound-play")).clicked() {
            outcome.play_sound = Some(Preview {
                prayer: prayer.to_owned(),
                template: template(state),
            });
        }
        ui.add_space(layout::OPTION_GAP - ui.spacing().item_spacing.x);
        if button(ui, ot_i18n::text("prayer-sound-stop")).clicked() {
            outcome.stop_sound = true;
        }
    });
    if state.template_saved {
        ui.add_space(layout::ROW_GAP / 2.0);
        note(ui, &ot_i18n::text("prayer-template-saved"), tokens.text_dim);
    }
}

fn today_block(ui: &mut egui::Ui, state: &State, tokens: Tokens) {
    subheading(ui, "prayer-today-heading");
    match state.day {
        Day::NoPlace => note(ui, &ot_i18n::text("prayer-today-none-yet"), tokens.text_dim),
        Day::Loading => note(ui, &ot_i18n::text("prayer-today-loading"), tokens.text_dim),
        Day::Unavailable => note(ui, &ot_i18n::text("prayer-today-nothing"), tokens.danger),
        Day::Times => {
            // The source label is always beside the times (spec §7).
            note(ui, &state.label, tokens.text_dim);
            ui.add_space(layout::ROW_GAP);
            egui::Grid::new("prayer-times")
                .num_columns(2)
                .spacing([layout::OPTION_GAP, layout::ROW_GAP / 2.0])
                .show(ui, |ui| {
                    for (name, time) in &state.rows {
                        ui.label(name);
                        ui.label(egui::RichText::new(time).strong());
                        ui.end_row();
                    }
                });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every `"prayer-…"` literal in this file, plus the keys
    /// the enums and the source label name — so a key added later is held to
    /// the same rule without anyone remembering to list it.
    fn keys_used() -> Vec<String> {
        let mut keys: Vec<String> = include_str!("prayer_pane.rs")
            .lines()
            // A widget id looks like a key and is not one.
            .filter(|line| !line.contains("id_salt") && !line.contains("Grid::new"))
            .flat_map(|line| line.split('"'))
            .filter(|token| {
                token.starts_with("prayer-")
                    && !token.ends_with('-')
                    && token
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            })
            .map(str::to_owned)
            .collect();
        keys.push("settings-section-prayer".to_owned());
        keys.extend(SourceChoice::all().map(|c| c.name_key().to_owned()));
        keys.extend(Preset::all().map(|p| p.name_key().to_owned()));
        keys.extend(Asr::all().map(|a| a.name_key().to_owned()));
        keys.extend(ot_prayer::chain::LABEL_KEYS.map(str::to_owned));
        // The reasons an audio clip is refused are named through `name_key`.
        for error in [
            ot_media::audio::AudioError::TooLarge(0),
            ot_media::audio::AudioError::NotAudio,
            ot_media::audio::AudioError::Undecodable(String::new()),
            ot_media::audio::AudioError::TooLong,
        ] {
            keys.push(error.name_key().to_owned());
        }
        keys.push("error-import-unreadable".to_owned());
        keys.sort();
        keys.dedup();
        keys
    }

    #[test]
    fn every_key_the_pane_uses_exists_in_both_catalogues() {
        let keys = keys_used();
        assert!(keys.len() > 50, "the scan found only {} keys", keys.len());
        for tag in ["en-US", "ms-MY"] {
            let present = ot_i18n::message_keys(tag).expect("the catalogue loads");
            for key in &keys {
                assert!(
                    present.contains(key.as_str()),
                    "{tag} has no message {key}, so the pane would draw its key"
                );
            }
        }
    }

    #[test]
    fn the_zone_rows_are_formatted_once_per_query_and_language_not_each_frame() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        let mut state =
            State::for_layout_test(dir.path(), &PrayerSettings::default(), None, vec![]);
        state.refresh_zone_rows();
        assert_eq!(state.zone_rows.len(), zones::zones().len());
        // Prove it is cached: a change made to the cached rows survives another
        // refresh for the same query, and goes when the query changes.
        state.zone_rows[0].1 = "cached".to_string();
        state.refresh_zone_rows();
        assert_eq!(
            state.zone_rows[0].1, "cached",
            "the same query rebuilds nothing"
        );
        state.zone_query = "selangor".to_string();
        state.refresh_zone_rows();
        assert!(
            state
                .zone_rows
                .iter()
                .all(|(code, _)| code.starts_with("SGR"))
        );
        assert!(state.zone_rows.len() < zones::zones().len());
    }

    #[test]
    fn closing_the_window_cancels_and_the_next_draw_works_the_day_out_again() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        std::fs::create_dir_all(dir.path().join("data")).expect("data folder");
        // The app opens the store in `main` before any window thread exists; two
        // threads creating a new database at once can fail on the WAL switch.
        drop(ot_store::Store::open(dir.path()).expect("the store is created first"));
        // Calculated, so no network is touched.
        let settings = PrayerSettings {
            source: "calculated".to_string(),
            ..PrayerSettings::default()
        };
        let place = StoredLocation {
            name: "Shah Alam".to_string(),
            latitude: 3.09,
            longitude: 101.53,
            tz: MALAYSIA_TZ.to_string(),
            country_code: Some("MY".to_string()),
            origin: Origin::Manual,
            updated_at: "2026-11-26T01:00:00Z".parse().expect("instant"),
        };
        let now: Timestamp = "2026-11-26T04:00:00Z".parse().expect("instant");
        let mut state = State::for_layout_test(dir.path(), &settings, Some(place), vec![]);
        state.plan_dirty = true;
        state.tick(&settings, Consent::Granted, now);
        assert!(matches!(state.day, Day::Loading), "a resolve was started");

        // The window closes: the work is cancelled and the pane must not stay
        // on "loading" for ever when it is shown again.
        state.cancel();
        assert!(state.plan_dirty, "the day is to be worked out again");
        state.day = Day::Unavailable;
        state.tick(&settings, Consent::Granted, now);
        assert!(
            matches!(state.day, Day::Loading),
            "the next draw starts the resolve again"
        );
        // And the answer of that second resolve does arrive.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !matches!(state.day, Day::Times) && std::time::Instant::now() < deadline {
            state.tick(&settings, Consent::Granted, now);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(matches!(state.day, Day::Times), "the times arrive");
        assert_eq!(state.rows.len(), 6, "the six prayers");
    }

    #[test]
    fn a_lookup_error_maps_to_a_sentence_the_user_can_act_on() {
        assert_eq!(
            error_key(&LookupError::Network("x".into())),
            "prayer-error-network"
        );
        assert_eq!(error_key(&LookupError::Http(503)), "prayer-error-network");
        assert_eq!(
            error_key(&LookupError::Service("RateLimited".into())),
            "prayer-error-service"
        );
        assert_eq!(error_key(&LookupError::Http(429)), "prayer-error-service");
        assert_eq!(error_key(&LookupError::TooLarge), "prayer-error-answer");
        assert_eq!(
            error_key(&LookupError::Shape("x".into())),
            "prayer-error-answer"
        );
    }
}
