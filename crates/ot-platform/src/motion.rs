//! Whether Windows wants animations shown: Settings ▸ Accessibility ▸ Visual
//! effects ▸ Animation effects, the "Show animations in Windows" of older
//! builds (`SPI_GETCLIENTAREAANIMATION`).
//!
//! Off means a toast plays every effect as a fade (`docs/specs/toast.md` §5).
//! There is no cheap notification worth running a message loop for here, so
//! the caller polls, like the theme.

/// How often the setting is re-read.
pub const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);

/// Whether Windows wants animations shown. `None` when it will not say, and
/// then the caller shows them, as Windows does by default.
pub fn show_animations() -> Option<bool> {
    platform::show_animations()
}

#[cfg(windows)]
mod platform {
    use windows::Win32::UI::WindowsAndMessaging::{
        SPI_GETCLIENTAREAANIMATION, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
    };
    use windows::core::BOOL;

    #[allow(
        unsafe_code,
        reason = "SystemParametersInfoW writes a BOOL through the pointer it is given"
    )]
    pub(super) fn show_animations() -> Option<bool> {
        let mut on = BOOL::from(true);
        // SAFETY: SPI_GETCLIENTAREAANIMATION writes one BOOL through the
        // pointer, which points at a live BOOL for the length of the call.
        let read = unsafe {
            SystemParametersInfoW(
                SPI_GETCLIENTAREAANIMATION,
                0,
                Some(std::ptr::from_mut(&mut on).cast()),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            )
        };
        read.ok().map(|()| on.as_bool())
    }
}

#[cfg(not(windows))]
mod platform {
    pub(super) fn show_animations() -> Option<bool> {
        None
    }
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    #[test]
    fn windows_answers_whether_it_shows_animations() {
        // Either answer is right; what matters is that it answers, since a
        // `None` here would mean every toast always animates.
        assert!(super::show_animations().is_some());
    }
}
