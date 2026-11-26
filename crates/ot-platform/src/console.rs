//! Output for an exe that has no console of its own (BUG-0013).
//!
//! A release `OmniToaster.exe` is a GUI-subsystem program, so Windows gives it
//! no console and double-clicking it shows no black window. Two things follow:
//!
//! - **From a terminal** `--help`, `--version` and the other flags must still
//!   print, so [`attach`] joins the parent's console when the process has no
//!   standard output of its own. A handle that is already there — a pipe or a
//!   file, as the packaging scripts use to read `--version` — is never
//!   replaced.
//! - **A message the user must see** — the data folder is not writable, a copy
//!   is already running — would vanish with the console, and the app would
//!   just not start. [`report`] shows it in a message box when there is
//!   nowhere else for it to go.
//!
//! A debug build is a console program (`cargo run` shows the log), where both
//! functions only print.

use std::sync::atomic::{AtomicBool, Ordering};

/// Whether standard error leads somewhere a person can read it. True until
/// [`attach`] finds it does not.
static HAS_OUTPUT: AtomicBool = AtomicBool::new(true);

/// How serious a [`report`] is: the icon of its message box, and whether it
/// goes to standard error or standard output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The app cannot do what was asked.
    Error,
    /// Not a failure: the app is telling the user something.
    Notice,
}

/// Join the parent's console when this process has no standard output. Call it
/// first thing in `main`, before anything prints.
pub fn attach() {
    HAS_OUTPUT.store(platform::attach(), Ordering::Relaxed);
}

/// Tell the user something they must see: to the console when there is one,
/// and in a message box when there is not.
pub fn report(kind: Kind, text: &str) {
    match kind {
        Kind::Error => eprintln!("{text}"),
        Kind::Notice => println!("{text}"),
    }
    if !HAS_OUTPUT.load(Ordering::Relaxed) {
        platform::message_box(kind, text);
    }
}

#[cfg(windows)]
mod platform {
    use std::fs::OpenOptions;
    use std::os::windows::io::IntoRawHandle;

    use windows::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
    use windows::Win32::System::Console::{
        ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_ERROR_HANDLE, STD_HANDLE,
        STD_OUTPUT_HANDLE, SetStdHandle,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        MB_ICONERROR, MB_ICONINFORMATION, MB_OK, MB_SETFOREGROUND, MESSAGEBOX_STYLE, MessageBoxW,
    };
    use windows::core::HSTRING;

    use super::Kind;

    /// Whether the process has this standard handle: Windows hands a GUI
    /// program a null handle when nothing was redirected to it.
    fn has(which: STD_HANDLE) -> bool {
        // SAFETY: GetStdHandle takes no pointer and only reads process state.
        #[allow(unsafe_code, reason = "GetStdHandle is a plain Win32 query")]
        let handle = unsafe { GetStdHandle(which) };
        matches!(handle, Ok(h) if !h.is_invalid() && h != INVALID_HANDLE_VALUE)
    }

    /// Point a missing standard handle at the console just attached, and
    /// leave a present one alone. Returns whether the handle ends up valid.
    fn fill(which: STD_HANDLE) -> bool {
        if has(which) {
            return true;
        }
        let Ok(console) = OpenOptions::new().write(true).open("CONOUT$") else {
            return false;
        };
        // The handle is kept for the life of the process: it is the new
        // standard stream, so it is deliberately never closed.
        let raw = HANDLE(console.into_raw_handle());
        // SAFETY: `raw` is an open handle this function owns and leaks on
        // purpose, so it stays valid for as long as the process uses it.
        #[allow(
            unsafe_code,
            reason = "SetStdHandle installs a handle opened just above"
        )]
        let set = unsafe { SetStdHandle(which, raw) };
        set.is_ok()
    }

    pub(super) fn attach() -> bool {
        if has(STD_OUTPUT_HANDLE) && has(STD_ERROR_HANDLE) {
            return true;
        }
        // SAFETY: AttachConsole takes a process id constant and no pointer.
        #[allow(unsafe_code, reason = "AttachConsole is a plain Win32 call")]
        let attached = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
        if attached.is_err() {
            // Started by double-click, a shortcut or Run: no parent console.
            return false;
        }
        let out = fill(STD_OUTPUT_HANDLE);
        let err = fill(STD_ERROR_HANDLE);
        out && err
    }

    pub(super) fn message_box(kind: Kind, text: &str) {
        let icon: MESSAGEBOX_STYLE = match kind {
            Kind::Error => MB_ICONERROR,
            Kind::Notice => MB_ICONINFORMATION,
        };
        let text = HSTRING::from(text);
        let title = HSTRING::from("OmniToaster");
        // SAFETY: both strings outlive the call, and a null owner is allowed.
        #[allow(unsafe_code, reason = "MessageBoxW is a plain Win32 call")]
        unsafe {
            MessageBoxW(None, &text, &title, MB_OK | icon | MB_SETFOREGROUND);
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use super::Kind;

    pub(super) fn attach() -> bool {
        true
    }

    pub(super) fn message_box(_kind: Kind, _text: &str) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_test_run_has_output_to_report_to() {
        // `cargo test` captures the standard streams but they exist, so
        // attaching must neither fail nor decide a message box is needed:
        // one would block the run waiting for someone to click it.
        attach();
        assert!(HAS_OUTPUT.load(Ordering::Relaxed));
        report(Kind::Notice, "console::report in a test");
    }
}
