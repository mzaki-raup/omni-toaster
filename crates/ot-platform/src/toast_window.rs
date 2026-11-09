//! Keeping a toast out of the taskbar and out of Alt+Tab.
//!
//! `egui::ViewportBuilder` gets most of the way there — borderless,
//! transparent, always on top, never activated — but not all the way, and the
//! gap is invisible until someone presses Alt+Tab. Measured on the built
//! window (FEAT-0018):
//!
//! | Flag | After `ViewportBuilder` | Wanted |
//! |---|---|---|
//! | `WS_EX_TOPMOST` | set | set |
//! | focus on show | not taken | not taken |
//! | `WS_EX_TOOLWINDOW` | **clear** | set — this is what keeps it out of Alt+Tab |
//! | `WS_EX_APPWINDOW` | **set** | clear — this forces a taskbar button |
//!
//! `with_taskbar(false)` removes the taskbar *button* through the shell's
//! taskbar list, which leaves the window in Alt+Tab and leaves
//! `WS_EX_APPWINDOW` on it. Alt+Tab membership is decided by the extended
//! style, so the style is what has to change.
//!
//! The windows are found by title within **this process**: a toast's title is
//! the app's own name and never changes with the language, and nothing else
//! in the process carries it. A window belonging to another process is never
//! touched.

/// Make every toast window in this process unobtrusive.
///
/// Returns how many windows were changed, which is zero on every frame after
/// the first — the call is cheap enough to make each frame and idempotent
/// enough that doing so is not a mistake.
pub fn make_unobtrusive(title: &str) -> usize {
    platform::make_unobtrusive(title)
}

#[cfg(windows)]
mod platform {
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GWL_EXSTYLE, GetWindowLongPtrW, GetWindowTextW, GetWindowThreadProcessId,
        IsWindowVisible, SetWindowLongPtrW, WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };

    /// What the enumeration is looking for, and what it found.
    struct Search {
        title: Vec<u16>,
        process: u32,
        changed: usize,
    }

    #[allow(
        unsafe_code,
        reason = "Win32 calls; each one's contract is argued at the call"
    )]
    pub(super) fn make_unobtrusive(title: &str) -> usize {
        let mut search = Search {
            title: title.encode_utf16().collect(),
            // SAFETY: GetCurrentProcessId takes nothing and returns a value;
            // there is no pointer and no handle to get wrong.
            process: unsafe { GetCurrentProcessId() },
            changed: 0,
        };

        // SAFETY: the callback matches the EnumWindows signature, and the
        // LPARAM is a pointer to `search`, which outlives the call because
        // EnumWindows is synchronous. The callback does not store it.
        unsafe {
            let _ = EnumWindows(
                Some(visit),
                LPARAM(std::ptr::from_mut(&mut search) as isize),
            );
        }
        search.changed
    }

    #[allow(
        unsafe_code,
        reason = "a Win32 callback: its parameters come from the OS and are argued inside"
    )]
    unsafe extern "system" fn visit(window: HWND, data: LPARAM) -> windows::core::BOOL {
        // SAFETY: `data` is the pointer EnumWindows was given, which points at
        // the caller's live `Search` for the duration of the enumeration.
        let search = unsafe { &mut *(data.0 as *mut Search) };

        let mut owner = 0u32;
        // SAFETY: `window` comes from the enumeration and `owner` is a live
        // u32 the call writes once.
        unsafe { GetWindowThreadProcessId(window, Some(&mut owner)) };
        if owner != search.process {
            return true.into();
        }

        // SAFETY: the buffer is live and its length bounds the write.
        if !unsafe { IsWindowVisible(window) }.as_bool() {
            return true.into();
        }

        let mut text = [0u16; 256];
        // SAFETY: as above; the call returns how many characters it wrote.
        let written = unsafe { GetWindowTextW(window, &mut text) } as usize;
        if written == 0 || text[..written] != search.title[..] {
            return true.into();
        }

        // SAFETY: a plain style read on a window this process owns.
        let style = unsafe { GetWindowLongPtrW(window, GWL_EXSTYLE) };
        let wanted = (style | WS_EX_TOOLWINDOW.0 as isize | WS_EX_NOACTIVATE.0 as isize)
            & !(WS_EX_APPWINDOW.0 as isize);
        if wanted != style {
            // SAFETY: a plain style write on a window this process owns. The
            // flags are the documented extended-style bits.
            unsafe { SetWindowLongPtrW(window, GWL_EXSTYLE, wanted) };
            search.changed += 1;
            tracing::debug!("a toast window was taken out of the taskbar and Alt+Tab");
        }
        true.into()
    }
}

#[cfg(not(windows))]
mod platform {
    pub(super) fn make_unobtrusive(_title: &str) -> usize {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_title_nothing_carries_changes_nothing() {
        // Also proves the enumeration is safe to run with no match: it is
        // called every frame.
        assert_eq!(
            make_unobtrusive("a window title nothing in this process has"),
            0
        );
    }
}
