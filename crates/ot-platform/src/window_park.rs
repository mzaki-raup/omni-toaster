//! Putting the settings window away without hiding it (BUG-0008).
//!
//! Closing the settings window must leave the app running behind its tray
//! icon. Hiding the window did that — and stopped the app dead: Windows sends
//! no paint messages to a hidden window, eframe draws only when painted, and
//! everything the app does happens in a frame. Measured: 65 frames in three
//! seconds visible, **none** in five hidden. No frame drained the tray's
//! clicks, so no menu item did anything, and no toast could have fired.
//!
//! So the window is **parked** instead: still visible as far as Windows is
//! concerned, which keeps the frames coming, but moved off every screen and
//! taken out of the taskbar and Alt+Tab, which is everything a user means by
//! "closed". Bringing it back restores where it was.

use raw_window_handle::RawWindowHandle;

/// Where a parked window was, so it can be put back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parked {
    /// Left, top, width, height, in physical pixels.
    rect: [i32; 4],
    /// The window's extended style before it was parked.
    style: isize,
}

/// Park `window`: off every screen, out of the taskbar and Alt+Tab, still
/// receiving frames. `None` when it is not a Win32 window or Windows refused,
/// in which case the caller should fall back to hiding it.
pub fn park(window: RawWindowHandle) -> Option<Parked> {
    platform::park(window)
}

/// Bring a parked window back to where it was, in front, with the focus.
pub fn unpark(window: RawWindowHandle, parked: Parked) {
    platform::unpark(window, parked);
}

#[cfg(windows)]
mod platform {
    use raw_window_handle::RawWindowHandle;
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GetWindowLongPtrW, GetWindowRect, IsIconic, SW_HIDE, SW_RESTORE, SW_SHOW,
        SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SetForegroundWindow,
        SetWindowLongPtrW, SetWindowPos, ShowWindow, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
    };

    use super::Parked;

    /// Where Windows itself puts a minimized window: far outside any monitor
    /// arrangement.
    const OFF_SCREEN: i32 = -32000;

    fn hwnd(window: RawWindowHandle) -> Option<HWND> {
        match window {
            RawWindowHandle::Win32(handle) => Some(HWND(handle.hwnd.get() as *mut _)),
            _ => None,
        }
    }

    #[allow(
        unsafe_code,
        reason = "window calls on the app's own window; argued at the block"
    )]
    pub(super) fn park(window: RawWindowHandle) -> Option<Parked> {
        let window = hwnd(window)?;
        // SAFETY: every call takes the live handle eframe handed over and
        // plain values; the RECT is a live out-parameter written once.
        unsafe {
            // A minimized window's rect is already the off-screen one, which
            // would be a useless place to restore to.
            if IsIconic(window).as_bool() {
                let _ = ShowWindow(window, SW_RESTORE);
            }
            let mut rect = RECT::default();
            GetWindowRect(window, &mut rect).ok()?;
            let style = GetWindowLongPtrW(window, GWL_EXSTYLE);

            // The taskbar re-reads a window's style only when it is shown, so
            // the style changes between a hide and a show.
            let _ = ShowWindow(window, SW_HIDE);
            SetWindowLongPtrW(
                window,
                GWL_EXSTYLE,
                (style | WS_EX_TOOLWINDOW.0 as isize) & !(WS_EX_APPWINDOW.0 as isize),
            );
            let _ = SetWindowPos(
                window,
                None,
                OFF_SCREEN,
                OFF_SCREEN,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
            let _ = ShowWindow(window, SW_SHOWNOACTIVATE);

            Some(Parked {
                rect: [
                    rect.left,
                    rect.top,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                ],
                style,
            })
        }
    }

    #[allow(
        unsafe_code,
        reason = "window calls on the app's own window; argued at the block"
    )]
    pub(super) fn unpark(window: RawWindowHandle, parked: Parked) {
        let Some(window) = hwnd(window) else {
            return;
        };
        let [x, y, width, height] = parked.rect;
        // SAFETY: as in `park`: the app's own live window and plain values.
        unsafe {
            let _ = ShowWindow(window, SW_HIDE);
            SetWindowLongPtrW(window, GWL_EXSTYLE, parked.style);
            let _ = SetWindowPos(window, None, x, y, width, height, SWP_NOZORDER);
            let _ = ShowWindow(window, SW_SHOW);
            let _ = SetForegroundWindow(window);
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use raw_window_handle::RawWindowHandle;

    use super::Parked;

    pub(super) fn park(_window: RawWindowHandle) -> Option<Parked> {
        None
    }

    pub(super) fn unpark(_window: RawWindowHandle, _parked: Parked) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_handle_that_is_not_a_win32_window_is_not_parked() {
        // The caller falls back to hiding, so this must answer, not guess.
        let web = RawWindowHandle::Web(raw_window_handle::WebWindowHandle::new(1));
        assert_eq!(park(web), None);
    }
}
