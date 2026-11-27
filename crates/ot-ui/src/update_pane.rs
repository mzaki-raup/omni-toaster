//! Settings ▸ Updates (FEAT-0080, `docs/specs/updater.md` §7 and §9).
//!
//! The running version, which forge this copy gets its updates from (shown, not
//! editable: it was decided when the copy was built), the two switches, Check Now,
//! what the last check found, and the install button with what it will install.
//! The pane draws and nothing in it blocks: the network work is the
//! [`update_service`](crate::update_service)'s, and its notes arrive through
//! [`State::apply`].

use std::path::Path;

use jiff::{SignedDuration, Timestamp};
use ot_core::UpdateSettings;
use ot_update::check::Outcome;
use ot_update::release::Release;
use ot_update::source;
use ot_update::swap::{self, Layout};
use semver::Version;

use crate::layout;
use crate::prayer_pane::{button, ellipsize, note, rule, subheading, wrapped_checkbox};
use crate::theme::Tokens;
use crate::update_service::{Note, Problem};
use crate::weather_pane::age_text;

/// What the pane is showing about the update.
#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    /// Nothing asked yet, or what was asked was stopped.
    Idle,
    Checking,
    UpToDate,
    Available(Release),
    /// A newer release that cannot be installed from here: its page is offered.
    NotInstallable(Release),
    Downloading {
        version: Version,
        done: u64,
        total: u64,
    },
    /// Staged beside the running program, waiting for a restart.
    Staged(Version),
    Failed(Problem),
}

/// What a frame of the pane asks the window to do.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Asked {
    /// `settings.toml` should be saved.
    pub settings_changed: bool,
    /// Look for a newer release now.
    pub check: bool,
    /// Verify, download and stage this release.
    pub install: Option<Release>,
    /// Stop the check or the download.
    pub cancel: bool,
    /// Restart into the staged version.
    pub restart: Option<Version>,
}

/// The pane's state.
pub struct State {
    phase: Phase,
    /// The release last offered, kept so cancelling a download goes back to it.
    offered: Option<Release>,
    last_checked: Option<Timestamp>,
    running: Version,
    source: String,
}

impl State {
    /// A state for the running program. An update that was rolled back at the last
    /// start is read here, once, so the user is told.
    pub fn new(root: &Path) -> Self {
        let mut state = Self::with_phase(Phase::Idle);
        let layout = Layout::new(Path::new("OmniToaster.exe"), root);
        if let Some(reason) = swap::take_rollback_note(&layout) {
            tracing::warn!(%reason, "the last update was rolled back");
            state.phase = Phase::Failed(Problem::RolledBack);
        }
        state
    }

    fn with_phase(phase: Phase) -> Self {
        Self {
            phase,
            offered: None,
            last_checked: None,
            running: ot_update::release::running(),
            source: source::this_repo().web_url(),
        }
    }

    /// A state in `phase`, for the layout test.
    pub fn for_layout_test(phase: Phase) -> Self {
        let mut state = Self::with_phase(phase);
        state.last_checked = Some(Timestamp::now());
        state
    }

    pub fn phase(&self) -> &Phase {
        &self.phase
    }

    /// Whether a request or a download is in flight.
    pub fn busy(&self) -> bool {
        matches!(self.phase, Phase::Checking | Phase::Downloading { .. })
    }

    /// Take what the service said.
    pub fn apply(&mut self, said: Note) {
        match said {
            Note::Checking => self.phase = Phase::Checking,
            Note::Checked { outcome, at } => {
                self.last_checked = Some(at);
                self.offered = None;
                self.phase = match outcome {
                    Outcome::UpToDate { .. } => Phase::UpToDate,
                    Outcome::Available(release) => {
                        self.offered = Some(release.clone());
                        Phase::Available(release)
                    }
                    Outcome::NotInstallable(release) => {
                        self.offered = Some(release.clone());
                        Phase::NotInstallable(release)
                    }
                };
            }
            Note::Progress {
                version,
                done,
                total,
            } => {
                self.phase = Phase::Downloading {
                    version,
                    done,
                    total,
                }
            }
            Note::Staged(version) => self.phase = Phase::Staged(version),
            Note::Failed(problem) => self.phase = Phase::Failed(problem),
            Note::Stopped => {
                self.phase = match self.offered.clone() {
                    Some(release) => Phase::Available(release),
                    None => Phase::Idle,
                };
            }
        }
    }

    /// The window has asked for a check or an install: show it at once rather than
    /// when the thread's first note arrives.
    pub fn requested(&mut self, checking: bool) {
        if checking {
            self.phase = Phase::Checking;
        }
    }

    /// The release page of what is on offer, for the link.
    fn page(&self) -> Option<&str> {
        match &self.phase {
            Phase::Available(release) | Phase::NotInstallable(release) => Some(&release.page_url),
            Phase::Failed(Problem::TooOld(_) | Problem::FolderNotWritable) => self
                .offered
                .as_ref()
                .map(|release| release.page_url.as_str()),
            _ => None,
        }
    }
}

/// The sentence for a failure, in the active language.
pub fn problem_text(problem: &Problem, source: &str) -> String {
    match problem {
        Problem::Network => {
            ot_i18n::text_with("update-failed-network", &[("source", source.into())])
        }
        Problem::NoRelease => ot_i18n::text("update-failed-none"),
        Problem::Rejected => ot_i18n::text("update-failed-rejected"),
        Problem::BadDownload => ot_i18n::text("update-failed-download"),
        Problem::FolderNotWritable => ot_i18n::text("update-failed-folder"),
        Problem::TooOld(version) => ot_i18n::text_with(
            "update-failed-too-old",
            &[("version", version.to_string().into())],
        ),
        Problem::RolledBack => ot_i18n::text("update-rolled-back"),
        Problem::Other => ot_i18n::text("update-failed-other"),
    }
}

/// The whole pane.
pub fn show(
    ui: &mut egui::Ui,
    state: &mut State,
    settings: &mut UpdateSettings,
    tokens: Tokens,
    now: Timestamp,
) -> Asked {
    let mut outcome = Asked::default();
    let columns = layout::columns(ui);

    subheading(ui, "update-heading");
    layout::labelled_row(ui, columns, &ot_i18n::text("update-version-label"), |ui| {
        ui.label(state.running.to_string());
    });
    layout::labelled_row(ui, columns, &ot_i18n::text("update-source-label"), |ui| {
        ui.label(ellipsize(&state.source, (columns.field / 8.0) as usize));
    });
    layout::field_hint(
        ui,
        columns,
        &ot_i18n::text("update-source-hint"),
        tokens.text_dim,
    );
    rule(ui);

    let mut enabled = settings.enabled;
    if wrapped_checkbox(
        ui,
        columns.label,
        &mut enabled,
        &ot_i18n::text("update-enabled"),
    ) {
        settings.enabled = enabled;
        outcome.settings_changed = true;
    }
    layout::field_hint(
        ui,
        columns,
        &ot_i18n::text("update-enabled-hint"),
        tokens.text_dim,
    );
    ui.add_space(layout::ROW_GAP);
    let mut automatic = settings.automatic;
    ui.add_enabled_ui(settings.enabled, |ui| {
        if wrapped_checkbox(
            ui,
            columns.label,
            &mut automatic,
            &ot_i18n::text("update-automatic"),
        ) {
            settings.automatic = automatic;
            outcome.settings_changed = true;
        }
    });
    layout::field_hint(
        ui,
        columns,
        &ot_i18n::text("update-automatic-hint"),
        tokens.text_dim,
    );
    rule(ui);

    status_block(ui, state, settings.enabled, tokens, now);
    ui.add_space(layout::ROW_GAP);
    buttons(ui, state, settings.enabled, &mut outcome);
    if let Some(page) = state.page() {
        ui.add_space(layout::ROW_GAP);
        ui.hyperlink_to(ot_i18n::text("update-release-notes"), page);
    }
    outcome
}

/// What is known, in one or two sentences.
fn status_block(ui: &mut egui::Ui, state: &State, enabled: bool, tokens: Tokens, now: Timestamp) {
    match &state.phase {
        Phase::Idle => {}
        Phase::Checking => note(ui, &ot_i18n::text("update-checking"), tokens.text),
        Phase::UpToDate => note(
            ui,
            &ot_i18n::text_with(
                "update-up-to-date",
                &[("version", state.running.to_string().into())],
            ),
            tokens.text,
        ),
        Phase::Available(release) => note(
            ui,
            &ot_i18n::text_with(
                "update-available",
                &[("version", release.version.to_string().into())],
            ),
            tokens.text,
        ),
        Phase::NotInstallable(release) => note(
            ui,
            &ot_i18n::text_with(
                "update-not-installable",
                &[("version", release.version.to_string().into())],
            ),
            tokens.text,
        ),
        Phase::Downloading {
            version,
            done,
            total,
        } => {
            let fraction = if *total == 0 {
                0.0
            } else {
                (*done as f32 / *total as f32).clamp(0.0, 1.0)
            };
            note(
                ui,
                &ot_i18n::text_with(
                    "update-downloading",
                    &[
                        ("version", version.to_string().into()),
                        (
                            "percent",
                            format!("{}", (fraction * 100.0).round() as u32).into(),
                        ),
                    ],
                ),
                tokens.text,
            );
            ui.add(egui::ProgressBar::new(fraction));
        }
        Phase::Staged(version) => note(
            ui,
            &ot_i18n::text_with("update-staged", &[("version", version.to_string().into())]),
            tokens.text,
        ),
        Phase::Failed(problem) => note(ui, &problem_text(problem, &state.source), tokens.danger),
    }
    if !enabled {
        note(ui, &ot_i18n::text("update-off"), tokens.text_dim);
    } else if !matches!(state.phase, Phase::Checking) {
        let line = match state.last_checked {
            Some(at) => ot_i18n::text_with(
                "update-last-checked",
                &[(
                    "age",
                    age_text(SignedDuration::from_secs(now.as_second() - at.as_second())).into(),
                )],
            ),
            None => ot_i18n::text("update-never-checked"),
        };
        note(ui, &line, tokens.text_dim);
    }
}

/// The buttons that apply, wrapping rather than running past the pane.
fn buttons(ui: &mut egui::Ui, state: &State, enabled: bool, outcome: &mut Asked) {
    ui.horizontal_wrapped(|ui| {
        let busy = state.busy();
        if ui
            .add_enabled(
                enabled && !busy,
                egui::Button::new(ot_i18n::text("update-check-now"))
                    .min_size(egui::vec2(0.0, layout::field_height(ui))),
            )
            .clicked()
        {
            outcome.check = true;
        }
        if busy && button(ui, ot_i18n::text("update-cancel")).clicked() {
            outcome.cancel = true;
        }
        if let Phase::Available(release) = &state.phase
            && enabled
            && button(ui, ot_i18n::text("update-install")).clicked()
        {
            outcome.install = Some(release.clone());
        }
        if let Phase::Staged(version) = &state.phase
            && button(ui, ot_i18n::text("update-restart")).clicked()
        {
            outcome.restart = Some(version.clone());
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(version: &str) -> Release {
        Release {
            version: Version::parse(version).expect("version"),
            tag: format!("v{version}"),
            page_url: "https://example.test/r".into(),
            assets: Vec::new(),
        }
    }

    fn checked(outcome: Outcome) -> Note {
        Note::Checked {
            outcome,
            at: Timestamp::now(),
        }
    }

    #[test]
    fn notes_move_the_pane_through_its_states() {
        let mut state = State::with_phase(Phase::Idle);
        state.apply(Note::Checking);
        assert!(state.busy());
        state.apply(checked(Outcome::Available(release("9.0.0"))));
        assert_eq!(state.phase(), &Phase::Available(release("9.0.0")));
        assert!(state.last_checked.is_some());
        state.apply(Note::Progress {
            version: Version::new(9, 0, 0),
            done: 5,
            total: 10,
        });
        assert!(state.busy());
        state.apply(Note::Staged(Version::new(9, 0, 0)));
        assert_eq!(state.phase(), &Phase::Staged(Version::new(9, 0, 0)));
        assert!(!state.busy());
    }

    #[test]
    fn cancelling_a_download_goes_back_to_the_offer_and_a_cancelled_check_to_idle() {
        let mut state = State::with_phase(Phase::Idle);
        state.apply(Note::Checking);
        state.apply(Note::Stopped);
        assert_eq!(state.phase(), &Phase::Idle);

        state.apply(checked(Outcome::Available(release("9.0.0"))));
        state.apply(Note::Progress {
            version: Version::new(9, 0, 0),
            done: 1,
            total: 2,
        });
        state.apply(Note::Stopped);
        assert_eq!(state.phase(), &Phase::Available(release("9.0.0")));
    }

    #[test]
    fn a_failure_is_kept_and_a_later_check_clears_it() {
        let mut state = State::with_phase(Phase::Idle);
        state.apply(Note::Failed(Problem::Network));
        assert_eq!(state.phase(), &Phase::Failed(Problem::Network));
        state.apply(Note::Checking);
        state.apply(checked(Outcome::UpToDate {
            latest: Version::new(0, 1, 0),
        }));
        assert_eq!(state.phase(), &Phase::UpToDate);
    }

    #[test]
    fn a_rolled_back_update_is_reported_once() {
        let dir = tempfile::tempdir().expect("temp dir");
        let cache = dir.path().join("cache").join("update");
        std::fs::create_dir_all(&cache).expect("cache");
        std::fs::write(cache.join("rolled-back"), "the new program did not start").expect("note");
        assert_eq!(
            State::new(dir.path()).phase(),
            &Phase::Failed(Problem::RolledBack)
        );
        assert_eq!(State::new(dir.path()).phase(), &Phase::Idle, "told once");
    }
}
