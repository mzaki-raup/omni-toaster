//! Settings ▸ Weather (FEAT-0070, `docs/specs/weather.md` §6 and §9).
//!
//! The switch, the place it uses (the one saved under Prayer), how often to look,
//! which kinds of weather and which sudden temperature moves to tell about, and a
//! template for each condition: its title, message, accent, picture and sound,
//! each empty meaning the built-in. The pane draws and nothing in it blocks: the
//! place and the templates are read and written on short-lived threads and
//! drained each frame by [`State::tick`], like the Prayer pane's.
//!
//! The templates reuse the generic words of the Prayer ones (`prayer-template-*`,
//! `prayer-sound-*`) rather than a second copy of each.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};

use ot_core::WeatherSettings;
use ot_store::Store;
use ot_store::reminders::{PrayerTemplateEdit, Template};
use ot_weather::condition::Condition;

use crate::weather_service::Snapshot;

use crate::layout::{self, Columns};
use crate::prayer_pane::{PictureChoice, ellipsize, hex, parse_hex};
use crate::prayer_pane::{button, note, rule, subheading, wrapped_checkbox};
use crate::theme::Tokens;

/// What a thread read or wrote.
enum Reply {
    Place(Option<String>),
    Template {
        condition: String,
        template: Option<Box<Template>>,
    },
    Failed(String),
}

/// What the user is writing for the chosen condition.
#[derive(Debug, Clone)]
struct TemplateEdit {
    title: String,
    message: String,
    accent_on: bool,
    accent: egui::Color32,
    /// The picture's path under the data folder, or `None` for the built-in.
    picture: Option<String>,
    /// The short chime when it appears; silence otherwise, the default.
    chime: bool,
}

impl Default for TemplateEdit {
    fn default() -> Self {
        Self {
            title: String::new(),
            message: String::new(),
            accent_on: false,
            accent: egui::Color32::from_rgb(0x3b, 0x9e, 0xff),
            picture: None,
            chime: false,
        }
    }
}

/// What a frame of the pane changed.
#[derive(Debug, Default, Clone)]
pub struct Outcome {
    /// `settings.toml` should be saved, and the weather service told.
    pub settings_changed: bool,
    /// Show a toast for this condition with this template: the editor's Preview.
    pub preview: Option<(Condition, Template)>,
    /// Play the short chime: the Play Sound button.
    pub play_sound: bool,
    /// Stop the sound.
    pub stop_sound: bool,
}

/// The pane's own state and its threads' door.
pub struct State {
    root: PathBuf,
    sender: SyncSender<Reply>,
    receiver: Receiver<Reply>,
    pending: u32,
    /// Which condition's template is being edited (an index into
    /// [`Condition::ALL`]).
    condition: usize,
    edit: TemplateEdit,
    saved: bool,
    /// The saved place's name once read; `None` inside means there is none.
    place: Option<Option<String>>,
    /// The last error, as text for the status note.
    error: Option<String>,
    /// The weather as the service last found it, with how old it is.
    snapshot: Option<Snapshot>,
}

impl State {
    pub fn new(root: &Path) -> Self {
        let state = Self::idle(root);
        state.spawn("weather-place", |root, out| {
            let reply = match Store::open(root).and_then(|store| store.location()) {
                Ok(place) => Reply::Place(place.map(|place| place.name)),
                Err(error) => Reply::Failed(error.to_string()),
            };
            let _ = out.send(reply);
        });
        state.load_template(Condition::ALL[0]);
        let mut state = state;
        state.pending = 2;
        state
    }

    fn idle(root: &Path) -> Self {
        let (sender, receiver) = sync_channel(16);
        Self {
            root: root.to_path_buf(),
            sender,
            receiver,
            pending: 0,
            condition: 0,
            edit: TemplateEdit::default(),
            saved: false,
            place: None,
            error: None,
            snapshot: None,
        }
    }

    /// A state with nothing loaded and the longest sample text in every field,
    /// for the layout test.
    pub fn for_layout_test(root: &Path) -> Self {
        let mut state = Self::idle(root);
        state.place = Some(Some(
            "Kuala Lumpur, Wilayah Persekutuan Kuala Lumpur".to_string(),
        ));
        state.condition = 3;
        state.edit = TemplateEdit {
            title: "Ribut petir di sini, berhati-hati".to_string(),
            message: "Hujan lebat dan petir dijangka, simpan barang jemuran".to_string(),
            accent_on: true,
            picture: Some("assets/long-label.png".to_string()),
            chime: true,
            ..TemplateEdit::default()
        };
        state.saved = true;
        state.error = Some("the database is locked".to_string());
        // The stale marker at its longest: days old.
        state.snapshot = Some(Snapshot {
            condition: Condition::Storm,
            temperature_c: 24.6,
            fetched_at: jiff::Timestamp::UNIX_EPOCH,
            stale: true,
            ttl: jiff::SignedDuration::from_hours(2),
        });
        state
    }

    fn spawn(&self, name: &str, work: impl FnOnce(&Path, &SyncSender<Reply>) + Send + 'static) {
        let root = self.root.clone();
        let sender = self.sender.clone();
        if let Err(error) = std::thread::Builder::new()
            .name(name.to_string())
            .spawn(move || work(&root, &sender))
        {
            tracing::error!(%error, thread = name, "a weather pane thread could not be started");
        }
    }

    fn load_template(&self, condition: Condition) {
        let Some(name) = condition.name() else {
            return;
        };
        self.spawn("weather-template", move |root, out| {
            let reply = match Store::open(root).and_then(|store| store.template_for_weather(name)) {
                Ok(template) => Reply::Template {
                    condition: name.to_owned(),
                    template: template.map(Box::new),
                },
                Err(error) => Reply::Failed(error.to_string()),
            };
            let _ = out.send(reply);
        });
    }

    fn save_template(&self, condition: Condition, edit: PrayerTemplateEdit) {
        let Some(name) = condition.name() else {
            return;
        };
        self.spawn("weather-template-save", move |root, out| {
            let saved = Store::open(root).and_then(|store| {
                store.set_weather_template(name, &edit)?;
                store.template_for_weather(name)
            });
            let reply = match saved {
                Ok(template) => Reply::Template {
                    condition: name.to_owned(),
                    template: template.map(Box::new),
                },
                Err(error) => Reply::Failed(error.to_string()),
            };
            let _ = out.send(reply);
        });
    }

    /// The service found the weather: show it.
    pub fn set_snapshot(&mut self, snapshot: Snapshot) {
        self.snapshot = Some(snapshot);
    }

    /// The saved place's name once read, for the preview toast.
    pub fn place_name(&self) -> Option<&str> {
        self.place.as_ref()?.as_deref()
    }

    /// Whether there is weather to show, so its age can be kept current.
    pub fn has_snapshot(&self) -> bool {
        self.snapshot.is_some()
    }

    /// Whether a thread's answer is still awaited, so the pane asks to be drawn
    /// again soon.
    pub fn busy(&self) -> bool {
        self.pending > 0
    }

    /// Take what the threads have answered since the last frame.
    pub fn tick(&mut self) {
        while let Ok(reply) = self.receiver.try_recv() {
            self.pending = self.pending.saturating_sub(1);
            match reply {
                Reply::Place(place) => self.place = Some(place),
                Reply::Template {
                    condition,
                    template,
                } => {
                    if Some(condition.as_str()) == Condition::ALL[self.condition].name() {
                        let template = template.map(|boxed| *boxed).unwrap_or_default();
                        self.edit = TemplateEdit {
                            title: template.custom_title.unwrap_or_default(),
                            message: template.custom_body.unwrap_or_default(),
                            accent_on: template.accent_color.is_some(),
                            accent: template
                                .accent_color
                                .as_deref()
                                .and_then(parse_hex)
                                .unwrap_or(TemplateEdit::default().accent),
                            picture: template.image,
                            chime: template.sound_mode.as_deref() == Some("chime"),
                        };
                        self.error = None;
                    }
                }
                Reply::Failed(error) => {
                    tracing::warn!(%error, "a weather pane thread failed");
                    self.error = Some(error);
                }
            }
        }
    }

    /// The place was changed under Prayer: read it again.
    pub fn place_changed(&mut self) {
        self.pending += 1;
        self.spawn("weather-place", |root, out| {
            let reply = match Store::open(root).and_then(|store| store.location()) {
                Ok(place) => Reply::Place(place.map(|place| place.name)),
                Err(error) => Reply::Failed(error.to_string()),
            };
            let _ = out.send(reply);
        });
    }
}

/// Draw the pane. The window owns the settings and saves them when
/// [`Outcome::settings_changed`] is set.
pub fn show(
    ui: &mut egui::Ui,
    state: &mut State,
    settings: &mut WeatherSettings,
    pictures: &[PictureChoice],
    tokens: Tokens,
    now: jiff::Timestamp,
) -> Outcome {
    let mut outcome = Outcome::default();
    let columns = layout::columns(ui);
    switch_block(ui, columns, state, settings, tokens, &mut outcome);
    rule(ui);
    now_block(ui, columns, state, settings.enabled, tokens, now);
    rule(ui);
    classes_block(ui, columns, settings, tokens, &mut outcome);
    rule(ui);
    temperature_block(ui, columns, settings, tokens, &mut outcome);
    rule(ui);
    template_block(ui, columns, state, pictures, tokens, &mut outcome);
    rule(ui);
    ui.horizontal_wrapped(|ui| {
        note(ui, &ot_i18n::text("weather-credit"), tokens.text_dim);
    });
    outcome
}

/// How long ago, in words: "just now", "5 minutes ago", "3 hours ago", "2 days
/// ago", from `ot-i18n`'s Fluent plurals so both languages say it their own way.
pub fn age_text(age: jiff::SignedDuration) -> String {
    let minutes = age.as_mins().max(0);
    let count = |key: &str, n: i64| {
        ot_i18n::text_with_count(key, u32::try_from(n).unwrap_or(u32::MAX), &[])
    };
    match minutes {
        0 => ot_i18n::text("weather-age-now"),
        1..=59 => count("weather-age-minutes", minutes),
        60..=1439 => count("weather-age-hours", minutes / 60),
        _ => count("weather-age-days", minutes / 1440),
    }
}

/// The weather as it stands, and when it is stale, which says so and how old it
/// is (`docs/specs/weather.md` §7): never old data shown as if it were current.
fn now_block(
    ui: &mut egui::Ui,
    columns: Columns,
    state: &State,
    enabled: bool,
    tokens: Tokens,
    now: jiff::Timestamp,
) {
    subheading(ui, "weather-now-heading");
    let Some(snapshot) = &state.snapshot else {
        let key = if enabled {
            "weather-now-none"
        } else {
            "weather-now-off"
        };
        layout::field_hint(ui, columns, &ot_i18n::text(key), tokens.text_dim);
        return;
    };
    let age = age_text(now.duration_since(snapshot.fetched_at));
    layout::labelled_row(ui, columns, &ot_i18n::text("weather-now-label"), |ui| {
        ui.label(ot_i18n::text_with(
            "weather-now",
            &[
                ("condition", ot_i18n::text(snapshot.condition.key()).into()),
                (
                    "temperature",
                    format!("{}", snapshot.temperature_c.round() as i64).into(),
                ),
            ],
        ));
    });
    if snapshot.is_stale(now) {
        ui.horizontal_wrapped(|ui| {
            ui.add_space(columns.label);
            ui.colored_label(
                tokens.accent,
                ot_i18n::text_with("weather-stale", &[("age", age.into())]),
            );
        });
    } else {
        layout::field_hint(
            ui,
            columns,
            &ot_i18n::text_with("weather-updated", &[("age", age.into())]),
            tokens.text_dim,
        );
    }
}

/// The switch, the place and how often to look.
fn switch_block(
    ui: &mut egui::Ui,
    columns: Columns,
    state: &State,
    settings: &mut WeatherSettings,
    tokens: Tokens,
    outcome: &mut Outcome,
) {
    subheading(ui, "weather-heading");
    let mut enabled = settings.enabled;
    if wrapped_checkbox(
        ui,
        columns.label,
        &mut enabled,
        &ot_i18n::text("weather-enable"),
    ) {
        settings.enabled = enabled;
        outcome.settings_changed = true;
    }
    layout::field_hint(
        ui,
        columns,
        &ot_i18n::text("weather-enable-hint"),
        tokens.text_dim,
    );
    ui.add_space(layout::ROW_GAP);

    layout::labelled_row(
        ui,
        columns,
        &ot_i18n::text("weather-place-label"),
        |ui| match &state.place {
            None => {
                ui.label(ot_i18n::text("prayer-location-looking"));
            }
            Some(Some(name)) => {
                ui.label(ellipsize(name, (columns.field / 8.0) as usize));
            }
            Some(None) => {
                ui.label(ot_i18n::text("weather-place-none"));
            }
        },
    );
    layout::field_hint(
        ui,
        columns,
        &ot_i18n::text("weather-place-hint"),
        tokens.text_dim,
    );
    ui.add_space(layout::ROW_GAP);

    let mut minutes = settings.poll_minutes();
    layout::labelled_row(ui, columns, &ot_i18n::text("weather-poll-label"), |ui| {
        ui.add(
            egui::DragValue::new(&mut minutes)
                .range(ot_core::WEATHER_POLL_MINUTES)
                .speed(1.0),
        );
    });
    if minutes != settings.poll_minutes() {
        settings.poll_minutes = minutes;
        outcome.settings_changed = true;
    }
    layout::field_hint(
        ui,
        columns,
        &ot_i18n::text("weather-poll-hint"),
        tokens.text_dim,
    );
}

/// Which kinds of weather a change **to** is worth telling about.
fn classes_block(
    ui: &mut egui::Ui,
    columns: Columns,
    settings: &mut WeatherSettings,
    tokens: Tokens,
    outcome: &mut Outcome,
) {
    subheading(ui, "weather-notify-heading");
    for condition in Condition::ALL {
        let Some(name) = condition.name() else {
            continue;
        };
        let mut on = settings.notifies(name);
        if wrapped_checkbox(ui, columns.label, &mut on, &ot_i18n::text(condition.key())) {
            settings.notify_classes.retain(|chosen| chosen != name);
            if on {
                settings.notify_classes.push(name.to_owned());
            }
            outcome.settings_changed = true;
        }
    }
    layout::field_hint(
        ui,
        columns,
        &ot_i18n::text("weather-notify-hint"),
        tokens.text_dim,
    );
}

/// The two sudden-change thresholds.
fn temperature_block(
    ui: &mut egui::Ui,
    columns: Columns,
    settings: &mut WeatherSettings,
    tokens: Tokens,
    outcome: &mut Outcome,
) {
    subheading(ui, "weather-temperature-heading");
    let mut changed = false;
    threshold(
        ui,
        columns,
        "weather-fall-enable",
        (
            &mut settings.fall_enabled,
            &mut settings.fall_delta_c,
            &mut settings.fall_window_hours,
        ),
        &mut changed,
    );
    ui.add_space(layout::ROW_GAP);
    threshold(
        ui,
        columns,
        "weather-rise-enable",
        (
            &mut settings.rise_enabled,
            &mut settings.rise_delta_c,
            &mut settings.rise_window_hours,
        ),
        &mut changed,
    );
    layout::field_hint(
        ui,
        columns,
        &ot_i18n::text("weather-temperature-hint"),
        tokens.text_dim,
    );
    outcome.settings_changed |= changed;
}

fn threshold(
    ui: &mut egui::Ui,
    columns: Columns,
    label_key: &str,
    (on, delta, hours): (&mut bool, &mut u32, &mut u32),
    changed: &mut bool,
) {
    let mut now = *on;
    if wrapped_checkbox(ui, columns.label, &mut now, &ot_i18n::text(label_key)) {
        *on = now;
        *changed = true;
    }
    let (mut by, mut within) = (*delta, *hours);
    layout::labelled_row(ui, columns, &ot_i18n::text("weather-delta-label"), |ui| {
        ui.add_enabled(
            *on,
            egui::DragValue::new(&mut by).range(ot_core::WEATHER_DELTA_C),
        );
    });
    layout::labelled_row(ui, columns, &ot_i18n::text("weather-window-label"), |ui| {
        ui.add_enabled(
            *on,
            egui::DragValue::new(&mut within).range(ot_core::WEATHER_WINDOW_HOURS),
        );
    });
    if (by, within) != (*delta, *hours) {
        (*delta, *hours) = (by, within);
        *changed = true;
    }
}

/// A different condition to put before `condition` in a sample sentence, for the
/// hint and the preview: "from Clear to Rain", never "from Clear to Clear".
pub fn sample_before(condition: Condition) -> Condition {
    if condition == Condition::Clear {
        Condition::Cloudy
    } else {
        Condition::Clear
    }
}

/// What a condition's notification says and shows: its title, message, accent,
/// picture and sound, each empty meaning the built-in (spec toast.md §8).
fn template_block(
    ui: &mut egui::Ui,
    columns: Columns,
    state: &mut State,
    pictures: &[PictureChoice],
    tokens: Tokens,
    outcome: &mut Outcome,
) {
    subheading(ui, "weather-template-heading");

    let mut chosen = state.condition;
    layout::labelled_row(
        ui,
        columns,
        &ot_i18n::text("weather-template-condition"),
        |ui| {
            egui::ComboBox::from_id_salt("weather-template-condition")
                .width(columns.field)
                .height(layout::field_height(ui) * 6.0)
                .selected_text(ot_i18n::text(Condition::ALL[chosen].key()))
                .show_ui(ui, |ui| {
                    for (index, condition) in Condition::ALL.iter().enumerate() {
                        ui.selectable_value(&mut chosen, index, ot_i18n::text(condition.key()));
                    }
                });
        },
    );
    if chosen != state.condition {
        state.condition = chosen;
        state.saved = false;
        state.pending += 1;
        state.load_template(Condition::ALL[chosen]);
    }
    ui.add_space(layout::ROW_GAP);

    let condition = Condition::ALL[state.condition];
    let name = ot_i18n::text(condition.key());
    let field = columns.field;

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
                        "weather-toast-condition-body",
                        &[
                            ("from", ot_i18n::text(sample_before(condition).key()).into()),
                            ("to", name.clone().into()),
                        ],
                    )),
            );
        },
    );
    ui.add_space(layout::ROW_GAP);

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
            egui::ComboBox::from_id_salt("weather-template-picture")
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

    // The sound: silent by default (a change of weather is not an Azan), or the
    // short chime.
    let mut chime = state.edit.chime;
    layout::labelled_row(ui, columns, &ot_i18n::text("prayer-sound-label"), |ui| {
        egui::ComboBox::from_id_salt("weather-sound")
            .width(columns.field)
            .selected_text(ot_i18n::text(if chime {
                "prayer-sound-chime"
            } else {
                "prayer-sound-none"
            }))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut chime, false, ot_i18n::text("prayer-sound-none"));
                ui.selectable_value(&mut chime, true, ot_i18n::text("prayer-sound-chime"));
            });
    });
    state.edit.chime = chime;
    ui.add_space(layout::ROW_GAP);

    let template = |state: &State| Template {
        weather_key: condition.name().map(str::to_owned),
        custom_title: Some(state.edit.title.trim().to_owned()).filter(|t| !t.is_empty()),
        custom_body: Some(state.edit.message.trim().to_owned()).filter(|t| !t.is_empty()),
        accent_color: state.edit.accent_on.then(|| hex(state.edit.accent)),
        image: state.edit.picture.clone(),
        sound_mode: state.edit.chime.then(|| "chime".to_owned()),
        ..Template::default()
    };
    let save = |state: &mut State| {
        let sha = state.edit.picture.as_deref().and_then(|path| {
            pictures
                .iter()
                .find(|p| p.relative_path == path)
                .map(|p| p.sha256.clone())
        });
        let language = if ot_i18n::current_language() == Some(ot_i18n::Language::Malay) {
            "ms-MY"
        } else {
            "en-US"
        };
        let saved = template(state);
        state.pending += 1;
        state.save_template(
            condition,
            PrayerTemplateEdit {
                title: saved.custom_title,
                body: saved.custom_body,
                accent: saved.accent_color,
                image: sha,
                authored_lang: language,
                sound_mode: saved.sound_mode,
                sound: None,
            },
        );
        state.saved = true;
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
            outcome.preview = Some((condition, template(state)));
        }
        ui.add_space(layout::OPTION_GAP - ui.spacing().item_spacing.x);
        if button(ui, ot_i18n::text("prayer-sound-play")).clicked() {
            outcome.play_sound = true;
        }
        ui.add_space(layout::OPTION_GAP - ui.spacing().item_spacing.x);
        if button(ui, ot_i18n::text("prayer-sound-stop")).clicked() {
            outcome.stop_sound = true;
        }
    });
    if state.saved {
        ui.add_space(layout::ROW_GAP / 2.0);
        note(ui, &ot_i18n::text("prayer-template-saved"), tokens.text_dim);
    }
    if state.error.is_some() {
        // The cause is in the log; the pane says it in the user's language.
        ui.add_space(layout::ROW_GAP / 2.0);
        note(ui, &ot_i18n::text("weather-error"), tokens.danger);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ot_core::DataRoot;
    use ot_store::location::{Origin, StoredLocation};
    use std::time::{Duration, Instant};

    #[test]
    fn the_pane_reads_the_place_and_the_first_template_off_the_calling_thread() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        DataRoot::resolve(Some(dir.path())).expect("resolves");
        {
            let store = Store::open(dir.path()).expect("created");
            store
                .save_manual_location(&StoredLocation {
                    name: "Kuala Lumpur".into(),
                    latitude: 3.15,
                    longitude: 101.69,
                    tz: "Asia/Kuala_Lumpur".into(),
                    country_code: Some("MY".into()),
                    origin: Origin::Manual,
                    updated_at: "2026-11-26T00:00:00Z".parse().expect("t"),
                })
                .expect("saved");
            store
                .set_weather_template(
                    "clear",
                    &PrayerTemplateEdit {
                        title: Some("Fine day".into()),
                        body: None,
                        accent: Some("#336699".into()),
                        image: None,
                        authored_lang: "en-US",
                        sound_mode: Some("chime".into()),
                        sound: None,
                    },
                )
                .expect("saved");
        }
        let mut state = State::new(dir.path());
        let deadline = Instant::now() + Duration::from_secs(10);
        while state.busy() && Instant::now() < deadline {
            state.tick();
            std::thread::sleep(Duration::from_millis(10));
        }
        state.tick();
        assert!(!state.busy(), "the threads answered");
        assert_eq!(state.place, Some(Some("Kuala Lumpur".to_owned())));
        assert_eq!(state.edit.title, "Fine day");
        assert!(state.edit.chime && state.edit.accent_on);
        assert_eq!(state.edit.accent, parse_hex("#336699").expect("a colour"));
    }

    #[test]
    fn saving_through_the_pane_reaches_the_store_and_a_clear_condition_reads_back() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        DataRoot::resolve(Some(dir.path())).expect("resolves");
        drop(Store::open(dir.path()).expect("created"));
        let mut state = State::new(dir.path());
        state.save_template(
            Condition::Rain,
            PrayerTemplateEdit {
                title: Some("Umbrella!".into()),
                body: None,
                accent: None,
                image: None,
                authored_lang: "en-US",
                sound_mode: None,
                sound: None,
            },
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        let store = Store::open(dir.path()).expect("opens");
        while Instant::now() < deadline {
            let saved = store
                .template_for_weather("rain")
                .expect("reads")
                .expect("row");
            if saved.custom_title.as_deref() == Some("Umbrella!") {
                state.tick();
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("the template was not saved in time");
    }
}
