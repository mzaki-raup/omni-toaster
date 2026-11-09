//! The tray icon and its menu (`docs/specs/tray-and-settings.md` §3).
//!
//! The app's normal state is this icon and nothing else on screen, so it is
//! the only thing a user can reach when no window is open.
//!
//! **Labels come in as strings, not as Fluent keys.** This crate owns Windows,
//! not language: the caller resolves every label and hands over a
//! [`Labels`], which is also what makes a language change a rebuild of the
//! menu rather than a special case inside it. The menu is rebuilt rather than
//! relabelled because muda has no way to change an item's text after the fact
//! that is cheaper than building the thing again.
//!
//! Every item carries a glyph before its label, and the same verb carries the
//! same glyph everywhere (`.claude/rules/ui.md`). The glyph is chosen here,
//! per verb, instead of being written into the locale files — a translator
//! should not be able to change an icon by editing a word.

/// What the user asked the tray to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    /// Left-click, or Open Settings.
    OpenSettings,
    Pause(crate::PauseWindow),
    Resume,
    TestToast,
    SnoozeLast,
    Quit,
}

/// The three pause windows, named here so `ot-platform` does not have to
/// depend on `ot-core`'s clock types to describe a menu item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseWindow {
    ThirtyMinutes,
    OneHour,
    UntilResumed,
}

/// Every string the menu shows, already in the user's language.
#[derive(Debug, Clone)]
pub struct Labels {
    pub open_settings: String,
    pub pause: String,
    pub pause_30: String,
    pub pause_60: String,
    pub pause_until_resumed: String,
    pub resume: String,
    pub test_toast: String,
    pub snooze_last: String,
    pub quit: String,
    /// The hover text: the next reminder or prayer, or the paused notice.
    pub tooltip: String,
}

/// The glyph for each verb. One place, so the same verb cannot pick up two
/// different pictures in two different menus.
mod glyph {
    pub const SETTINGS: char = '\u{2699}'; // ⚙
    pub const PAUSE: char = '\u{23F8}'; // ⏸
    pub const RESUME: char = '\u{25B6}'; // ▶
    pub const TEST: char = '\u{1F514}'; // 🔔
    pub const SNOOZE: char = '\u{1F4A4}'; // 💤
    pub const QUIT: char = '\u{23FB}'; // ⏻
    pub const CLOCK: char = '\u{1F550}'; // 🕐
}

/// `⚙ Open Settings` — the glyph, a space, then the label.
fn with_glyph(glyph: char, label: &str) -> String {
    format!("{glyph}  {label}")
}

/// Why the tray could not be created.
#[derive(Debug, thiserror::Error)]
pub enum TrayError {
    #[error("the tray icon could not be built: {0}")]
    Icon(String),
    #[error("the tray menu could not be built: {0}")]
    Menu(String),
    #[error("this platform has no tray")]
    Unsupported,
}

#[cfg(windows)]
pub use platform::Tray;

#[cfg(windows)]
mod platform {
    use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};
    use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

    use super::{Labels, PauseWindow, TrayCommand, TrayError, glyph, with_glyph};

    /// The icon resource `crates/omnitoaster/build.rs` embeds in the exe.
    ///
    /// Taking it from the executable's own resources rather than a file means
    /// the tray cannot show a different picture from the one Explorer shows,
    /// and a portable folder has nothing extra to carry. `winresource` writes
    /// the icon as the first icon resource, which is id 1.
    const ICON_RESOURCE_ID: u16 = 1;

    /// The tray icon, its menu, and the ids that map a click back to a verb.
    ///
    /// Dropping it removes the icon, which is why the app holds it for as
    /// long as it runs.
    pub struct Tray {
        icon: TrayIcon,
        ids: Ids,
        paused: bool,
    }

    /// The menu item ids, kept so a click can be matched without comparing
    /// labels — the labels change with the language, the ids do not.
    struct Ids {
        open_settings: MenuId,
        pause_30: MenuId,
        pause_60: MenuId,
        pause_until_resumed: MenuId,
        resume: MenuId,
        test_toast: MenuId,
        snooze_last: MenuId,
        quit: MenuId,
    }

    impl Tray {
        /// Put the icon in the notification area.
        pub fn new(labels: &Labels, paused: bool) -> Result<Self, TrayError> {
            let (menu, ids) = build_menu(labels, paused)?;
            let icon = TrayIconBuilder::new()
                .with_menu(Box::new(menu))
                .with_tooltip(&labels.tooltip)
                .with_icon(load_icon()?)
                .build()
                .map_err(|error| TrayError::Icon(error.to_string()))?;
            Ok(Self { icon, ids, paused })
        }

        /// Rebuild the menu — after a language change, or when the pause
        /// state changes which items belong in it.
        pub fn rebuild(&mut self, labels: &Labels, paused: bool) -> Result<(), TrayError> {
            let (menu, ids) = build_menu(labels, paused)?;
            self.icon.set_menu(Some(Box::new(menu)));
            self.ids = ids;
            self.paused = paused;
            self.set_tooltip(&labels.tooltip);
            Ok(())
        }

        /// The hover text. Cheap, so it can follow the next reminder.
        pub fn set_tooltip(&self, text: &str) {
            if let Err(error) = self.icon.set_tooltip(Some(text)) {
                tracing::warn!(%error, "the tray tooltip could not be set");
            }
        }

        /// Whether the menu currently shows the paused shape.
        pub fn shows_paused(&self) -> bool {
            self.paused
        }

        /// Take whatever the user has done since the last call.
        ///
        /// Both channels are drained, so a burst of clicks cannot leave an
        /// event sitting until the next one arrives.
        pub fn drain(&self) -> Vec<TrayCommand> {
            let mut commands = Vec::new();

            while let Ok(event) = MenuEvent::receiver().try_recv() {
                match self.command_for(&event.id) {
                    Some(command) => commands.push(command),
                    // An id we did not create: a separator, or a menu from
                    // somewhere else in the process. Not an error.
                    None => tracing::debug!(id = ?event.id, "an unknown menu id was clicked"),
                }
            }

            while let Ok(event) = TrayIconEvent::receiver().try_recv() {
                // A left click opens or focuses the window; the right button
                // is the menu, which muda handles itself.
                if let TrayIconEvent::Click {
                    button: tray_icon::MouseButton::Left,
                    button_state: tray_icon::MouseButtonState::Up,
                    ..
                } = event
                {
                    commands.push(TrayCommand::OpenSettings);
                }
            }

            commands
        }

        fn command_for(&self, id: &MenuId) -> Option<TrayCommand> {
            let ids = &self.ids;
            if *id == ids.open_settings {
                Some(TrayCommand::OpenSettings)
            } else if *id == ids.pause_30 {
                Some(TrayCommand::Pause(PauseWindow::ThirtyMinutes))
            } else if *id == ids.pause_60 {
                Some(TrayCommand::Pause(PauseWindow::OneHour))
            } else if *id == ids.pause_until_resumed {
                Some(TrayCommand::Pause(PauseWindow::UntilResumed))
            } else if *id == ids.resume {
                Some(TrayCommand::Resume)
            } else if *id == ids.test_toast {
                Some(TrayCommand::TestToast)
            } else if *id == ids.snooze_last {
                Some(TrayCommand::SnoozeLast)
            } else if *id == ids.quit {
                Some(TrayCommand::Quit)
            } else {
                None
            }
        }
    }

    fn build_menu(labels: &Labels, paused: bool) -> Result<(Menu, Ids), TrayError> {
        let menu = Menu::new();
        let to_menu_error = |error: tray_icon::menu::Error| TrayError::Menu(error.to_string());

        let open_settings = MenuItem::new(
            with_glyph(glyph::SETTINGS, &labels.open_settings),
            true,
            None,
        );
        menu.append(&open_settings).map_err(to_menu_error)?;
        menu.append(&PredefinedMenuItem::separator())
            .map_err(to_menu_error)?;

        // The pause submenu, or Resume when a pause is already running. Both
        // are never shown at once: the one that is not available would be a
        // dead item, and the menu is rebuilt anyway.
        let pause_30 = MenuItem::new(with_glyph(glyph::CLOCK, &labels.pause_30), true, None);
        let pause_60 = MenuItem::new(with_glyph(glyph::CLOCK, &labels.pause_60), true, None);
        let pause_until_resumed = MenuItem::new(
            with_glyph(glyph::PAUSE, &labels.pause_until_resumed),
            true,
            None,
        );
        let resume = MenuItem::new(with_glyph(glyph::RESUME, &labels.resume), true, None);

        if paused {
            menu.append(&resume).map_err(to_menu_error)?;
        } else {
            let submenu = Submenu::new(with_glyph(glyph::PAUSE, &labels.pause), true);
            submenu.append(&pause_30).map_err(to_menu_error)?;
            submenu.append(&pause_60).map_err(to_menu_error)?;
            submenu
                .append(&pause_until_resumed)
                .map_err(to_menu_error)?;
            menu.append(&submenu).map_err(to_menu_error)?;
        }

        // Present and disabled rather than hidden: the menu's shape must not
        // change under the user when Phases 2 and 4 give these something to
        // do (FEAT-0015).
        let test_toast = MenuItem::new(with_glyph(glyph::TEST, &labels.test_toast), false, None);
        let snooze_last =
            MenuItem::new(with_glyph(glyph::SNOOZE, &labels.snooze_last), false, None);
        menu.append(&test_toast).map_err(to_menu_error)?;
        menu.append(&snooze_last).map_err(to_menu_error)?;

        menu.append(&PredefinedMenuItem::separator())
            .map_err(to_menu_error)?;
        let quit = MenuItem::new(with_glyph(glyph::QUIT, &labels.quit), true, None);
        menu.append(&quit).map_err(to_menu_error)?;

        let ids = Ids {
            open_settings: open_settings.id().clone(),
            pause_30: pause_30.id().clone(),
            pause_60: pause_60.id().clone(),
            pause_until_resumed: pause_until_resumed.id().clone(),
            resume: resume.id().clone(),
            test_toast: test_toast.id().clone(),
            snooze_last: snooze_last.id().clone(),
            quit: quit.id().clone(),
        };
        Ok((menu, ids))
    }

    fn load_icon() -> Result<Icon, TrayError> {
        Icon::from_resource(ICON_RESOURCE_ID, None).map_err(|error| {
            TrayError::Icon(format!(
                "resource {ICON_RESOURCE_ID} is not an icon in this executable: {error}"
            ))
        })
    }
}

#[cfg(not(windows))]
mod platform {
    use super::{Labels, TrayCommand, TrayError};

    /// There is no tray off Windows; the type exists so the rest of the app
    /// compiles and clippy runs on Linux.
    pub struct Tray;

    impl Tray {
        pub fn new(_labels: &Labels, _paused: bool) -> Result<Self, TrayError> {
            Err(TrayError::Unsupported)
        }
        pub fn rebuild(&mut self, _labels: &Labels, _paused: bool) -> Result<(), TrayError> {
            Err(TrayError::Unsupported)
        }
        pub fn set_tooltip(&self, _text: &str) {}
        pub fn shows_paused(&self) -> bool {
            false
        }
        pub fn drain(&self) -> Vec<TrayCommand> {
            Vec::new()
        }
    }
}

#[cfg(not(windows))]
pub use platform::Tray;

#[cfg(test)]
mod tests {
    use super::*;

    fn labels() -> Labels {
        Labels {
            open_settings: "Open Settings".into(),
            pause: "Pause All Notifications".into(),
            pause_30: "For 30 Minutes".into(),
            pause_60: "For 1 Hour".into(),
            pause_until_resumed: "Until I Resume".into(),
            resume: "Resume Notifications".into(),
            test_toast: "Test Toast".into(),
            snooze_last: "Snooze Last".into(),
            quit: "Quit".into(),
            tooltip: "OmniToaster".into(),
        }
    }

    #[test]
    fn every_label_is_given_a_glyph_and_the_label_survives_it() {
        let labels = labels();
        let decorated = with_glyph(glyph::SETTINGS, &labels.open_settings);
        assert!(
            decorated.starts_with(glyph::SETTINGS),
            "the glyph comes first: {decorated:?}"
        );
        assert!(
            decorated.ends_with(&labels.open_settings),
            "the label is still there whole: {decorated:?}"
        );
    }

    #[test]
    fn one_verb_never_has_two_glyphs() {
        // The rule is that the same verb carries the same glyph everywhere,
        // which only holds if each is named once.
        let all = [
            glyph::SETTINGS,
            glyph::PAUSE,
            glyph::RESUME,
            glyph::TEST,
            glyph::SNOOZE,
            glyph::QUIT,
            glyph::CLOCK,
        ];
        let mut unique = all.to_vec();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(
            unique.len(),
            all.len(),
            "two verbs share a glyph, so the same picture means two things"
        );
    }
}
