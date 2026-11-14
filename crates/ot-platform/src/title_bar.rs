//! The settings window's title bar, which Windows draws and egui does not
//! (CR-0002).
//!
//! egui paints everything inside the window in the app's own palette, but the
//! title bar is Windows' own chrome, and Windows draws it in whatever mode it
//! was last told — so the window was light with a dark title bar. Windows 10
//! (20H1) and later draw it dark when the window asks with
//! `DWMWA_USE_IMMERSIVE_DARK_MODE`, which is all this does.
//!
//! The window is handed in by its handle rather than found by title: there is
//! exactly one window this applies to, and its owner has the handle.

use raw_window_handle::RawWindowHandle;

/// Draw `window`'s title bar dark or light, to match the app's appearance.
///
/// Returns whether Windows took the setting. A handle that is not a Win32
/// window, or a Windows too old for the attribute, answers `false` and
/// changes nothing; the caller may simply ask again later.
pub fn follow_appearance(window: RawWindowHandle, dark: bool) -> bool {
    platform::follow_appearance(window, dark)
}

#[cfg(windows)]
mod platform {
    use raw_window_handle::RawWindowHandle;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute};
    use windows::core::BOOL;

    #[allow(
        unsafe_code,
        reason = "a DWM attribute call; its contract is argued at the call"
    )]
    pub(super) fn follow_appearance(window: RawWindowHandle, dark: bool) -> bool {
        let RawWindowHandle::Win32(handle) = window else {
            return false;
        };
        let window = HWND(handle.hwnd.get() as *mut _);
        let value = BOOL::from(dark);
        // SAFETY: the handle is the live window eframe just handed over, and
        // the attribute is passed as a pointer to a BOOL with its size, as the
        // documentation for DWMWA_USE_IMMERSIVE_DARK_MODE requires.
        unsafe {
            DwmSetWindowAttribute(
                window,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                std::ptr::from_ref(&value).cast(),
                u32::try_from(size_of::<BOOL>()).unwrap_or(0),
            )
        }
        .is_ok()
    }
}

#[cfg(not(windows))]
mod platform {
    use raw_window_handle::RawWindowHandle;

    pub(super) fn follow_appearance(_window: RawWindowHandle, _dark: bool) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_handle_that_is_not_a_win32_window_is_left_alone() {
        // The only path that can run without a real window: it must answer
        // rather than reach for Win32 with something that is not a window.
        let web = RawWindowHandle::Web(raw_window_handle::WebWindowHandle::new(1));
        assert!(!follow_appearance(web, true));
        assert!(!follow_appearance(web, false));
    }
}
