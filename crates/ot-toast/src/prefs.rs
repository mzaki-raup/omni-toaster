//! The user's toast settings, as Settings ▸ Appearance edits them.
//!
//! One function turns these plus a toast's content into a [`ToastSpec`], and
//! the preview and every real toast both go through it. That is the whole of
//! the rule that keeps them from diverging (`docs/specs/toast.md` §3): there
//! is one path from what the user chose to what is drawn, not two.

use serde::{Deserialize, Serialize};

use crate::spec::{Behaviour, Content, Style, ToastSpec};

/// Which parts of the header show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Header {
    pub icon: bool,
    pub app_name: bool,
    pub timestamp: bool,
}

impl Default for Header {
    fn default() -> Self {
        Self {
            icon: true,
            app_name: true,
            timestamp: true,
        }
    }
}

/// The monitor toasts appear on, when it is not simply the primary one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MonitorChoice {
    /// The monitor's device path, which survives a reboot and a re-plug
    /// (`ot_platform::monitors::Monitor::id`).
    pub id: String,
    /// What the monitor was called when it was chosen, and its display
    /// number, so a monitor that is not connected can still be named in the
    /// picker. The label is the monitor's own (EDID) name, which does not
    /// change with the language; it may be empty.
    pub label: String,
    #[serde(default)]
    pub number: u32,
}

/// Everything Settings ▸ Appearance holds.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Prefs {
    pub style: Style,
    pub behaviour: Behaviour,
    pub header: Header,
    /// `None` is the primary monitor, whichever that is at the time — the
    /// default (`docs/specs/toast.md` §7). A chosen monitor that is not
    /// connected also means the primary one.
    pub monitor: Option<MonitorChoice>,
}

impl Prefs {
    /// Read the `[toast]` table of `settings.toml`.
    ///
    /// A table that does not fit — a typo, a value of the wrong type — is
    /// reported and the defaults are used, so a hand-edit cannot stop toasts
    /// appearing. Values that fit but are out of range are clamped later, by
    /// [`ToastSpec::clamped`], which is where every limit lives.
    pub fn from_table(table: &toml::Table) -> Self {
        if table.is_empty() {
            return Self::default();
        }
        match toml::Value::Table(table.clone()).try_into::<Self>() {
            Ok(prefs) => prefs,
            Err(error) => {
                tracing::warn!(%error, "the [toast] settings could not be read; using the defaults");
                Self::default()
            }
        }
    }

    /// Write them back as a `[toast]` table.
    pub fn to_table(&self) -> toml::Table {
        match toml::Value::try_from(self) {
            Ok(toml::Value::Table(table)) => table,
            // A struct always serializes to a table; anything else would be a
            // serde bug, and losing the user's style to it is worse than
            // saving the defaults with a line in the log.
            Ok(_) | Err(_) => {
                tracing::error!("the toast settings did not serialize to a table");
                toml::Table::new()
            }
        }
    }

    /// The one function that turns a choice of style into a toast.
    ///
    /// `content` arrives with its text already in the user's language — a
    /// spec holds text, never keys — and the header switches decide which of
    /// it shows. The result is clamped, so nothing downstream has to ask
    /// whether it is sane.
    pub fn spec(&self, mut content: Content) -> ToastSpec {
        // The icon, the image, the title and the body are the notification's
        // own and pass through untouched: every notification may carry
        // different ones (docs/specs/toast.md §8). These prefs are the global
        // *style* only, plus whether the header shows an icon at all.
        content.show_icon = self.header.icon;
        if !self.header.app_name {
            content.app_name.clear();
        }
        if !self.header.timestamp {
            content.timestamp = None;
        }
        ToastSpec {
            content,
            style: self.style,
            behaviour: self.behaviour,
        }
        .clamped()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::placement::Position;
    use crate::spec::Shape;

    fn content() -> Content {
        Content {
            title: "Zohor".to_string(),
            body: "Waktu Zohor telah masuk.".to_string(),
            app_name: "OmniToaster".to_string(),
            timestamp: Some("13:19".to_string()),
            ..Content::default()
        }
    }

    #[test]
    fn prefs_survive_the_settings_file() {
        let mut prefs = Prefs::default();
        prefs.style.shape = Shape::Pill;
        prefs.style.width = 420.0;
        prefs.behaviour.position = Position::Custom { x: 0.25, y: 0.75 };
        prefs.header.timestamp = false;
        prefs.monitor = Some(MonitorChoice {
            id: r"\\?\DISPLAY#DEL4321#5&1a2b3c&0&UID4352#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}"
                .to_owned(),
            label: "DELL U2720Q".to_owned(),
            number: 2,
        });

        let back = Prefs::from_table(&prefs.to_table());
        assert_eq!(back, prefs);
    }

    #[test]
    fn an_empty_table_is_a_first_run_and_gets_the_defaults() {
        assert_eq!(Prefs::from_table(&toml::Table::new()), Prefs::default());
    }

    #[test]
    fn a_table_that_does_not_fit_falls_back_rather_than_failing() {
        let mut table = toml::Table::new();
        table.insert("style".into(), toml::Value::String("not a table".into()));
        assert_eq!(Prefs::from_table(&table), Prefs::default());
    }

    #[test]
    fn the_header_switches_decide_what_shows() {
        let prefs = Prefs {
            header: Header {
                icon: false,
                app_name: false,
                timestamp: false,
            },
            ..Prefs::default()
        };
        let spec = prefs.spec(content());
        assert!(!spec.content.show_icon);
        assert!(spec.content.app_name.is_empty());
        assert_eq!(spec.content.timestamp, None);
        // The user's own words are never touched by a switch.
        assert_eq!(spec.content.title, "Zohor");
    }

    #[test]
    fn the_preview_and_a_real_toast_are_the_same_spec_for_the_same_choices() {
        // The rule the whole phase hangs on, stated as a test: one function,
        // two callers, identical output.
        let mut prefs = Prefs::default();
        prefs.style.shape = Shape::Rectangle;
        prefs.style.opacity = 70.0;
        let preview = prefs.spec(content());
        let real = prefs.spec(content());
        assert_eq!(preview, real);
    }

    #[test]
    fn an_out_of_range_choice_is_clamped_on_the_way_to_a_spec() {
        let mut prefs = Prefs::default();
        prefs.style.width = 20.0;
        assert_eq!(prefs.spec(content()).style.width, crate::spec::WIDTH.min);
    }
}
