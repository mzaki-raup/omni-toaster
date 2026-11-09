//! Whether Windows is currently in light or dark mode.
//!
//! Read from `HKCU\…\Themes\Personalize\AppsUseLightTheme`, which is the
//! setting that governs *application* chrome. There is a sibling,
//! `SystemUsesLightTheme`, for the taskbar and Start menu — the two can
//! differ, and following the wrong one means OmniToaster goes light while
//! every other app stays dark.
//!
//! ADR-006 chose reading the registry over the `dark-light` crate: one value,
//! no dependency, and the polling interval stays ours.
//!
//! There is no cheap notification for this worth running a message loop for
//! here, so the caller polls — every 5 seconds, per
//! `docs/specs/tray-and-settings.md` §8.

/// How often System mode should re-read the setting.
pub const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);

/// What Windows is set to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemTheme {
    Light,
    Dark,
}

/// Read the current Windows app theme.
///
/// Returns `None` when the value cannot be read — on a Windows build old
/// enough to lack it, or a profile that has never had it set. The caller
/// should fall back to the app's default rather than treat this as an error:
/// Dark is OmniToaster's default (`docs/04-ui-design.md`).
pub fn system_theme() -> Option<SystemTheme> {
    platform::apps_use_light_theme().map(|light| {
        if light {
            SystemTheme::Light
        } else {
            SystemTheme::Dark
        }
    })
}

#[cfg(windows)]
mod platform {
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
    use windows::core::HSTRING;

    const PERSONALIZE: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
    /// Application chrome. Deliberately not `SystemUsesLightTheme`, which is
    /// the taskbar and can differ.
    const VALUE: &str = "AppsUseLightTheme";

    #[allow(
        unsafe_code,
        reason = "RegGetValueW is a Win32 call; the buffer contract is argued at the call"
    )]
    pub(super) fn apps_use_light_theme() -> Option<bool> {
        let key = HSTRING::from(PERSONALIZE);
        let value = HSTRING::from(VALUE);
        let mut data: u32 = 0;
        let mut size = u32::try_from(size_of::<u32>()).ok()?;

        // SAFETY: `data` is a live u32 and `size` says it is 4 bytes, which
        // is what bounds the write. RRF_RT_REG_DWORD makes the API refuse
        // anything that is not a DWORD rather than reinterpreting it.
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                &key,
                &value,
                RRF_RT_REG_DWORD,
                None,
                Some(std::ptr::from_mut(&mut data).cast()),
                Some(&mut size),
            )
        };

        if status != ERROR_SUCCESS {
            // Absent on older builds and on profiles that never set it.
            // Not an error: the caller falls back to the app default.
            return None;
        }
        // 1 means light, 0 means dark. The name reads backwards from how it
        // is usually spoken about, which is worth saying out loud.
        Some(data != 0)
    }
}

#[cfg(not(windows))]
mod platform {
    pub(super) fn apps_use_light_theme() -> Option<bool> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_poll_interval_is_the_one_the_spec_fixes() {
        assert_eq!(POLL_INTERVAL.as_secs(), 5);
    }

    #[test]
    fn reading_the_theme_either_answers_or_declines_without_panicking() {
        // What it is depends on the machine, so this asserts only that it is
        // one of the two and never panics — a profile without the value is a
        // supported state, not a failure.
        match system_theme() {
            Some(SystemTheme::Light) | Some(SystemTheme::Dark) | None => {}
        }
    }

    #[cfg(windows)]
    #[test]
    fn the_reading_agrees_with_the_registry() {
        // Cross-check against the value itself, so a wrong polarity - light
        // and dark swapped - cannot pass. The name is the trap:
        // AppsUseLightTheme = 1 means light.
        let out = std::process::Command::new("reg")
            .args([
                "query",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
                "/v",
                "AppsUseLightTheme",
            ])
            .output();

        let Ok(out) = out else { return };
        let text = String::from_utf8_lossy(&out.stdout);
        let Some(line) = text.lines().find(|l| l.contains("AppsUseLightTheme")) else {
            // Value absent: system_theme must say None, not guess.
            assert_eq!(
                system_theme(),
                None,
                "the value is absent, so the reading should decline"
            );
            return;
        };

        let expected = if line.trim_end().ends_with("0x1") {
            SystemTheme::Light
        } else {
            SystemTheme::Dark
        };
        assert_eq!(
            system_theme(),
            Some(expected),
            "the reading disagrees with the registry: {line}"
        );
    }
}
