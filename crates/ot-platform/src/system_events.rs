//! The Windows events that invalidate the scheduler's queue: the clock or the
//! time zone was changed, and the machine resumed from sleep.
//!
//! Windows broadcasts both to **top-level** windows only — a message-only
//! window (`HWND_MESSAGE`) never sees a broadcast — so this owns one hidden,
//! zero-sized top-level window on a thread of its own, with its own message
//! loop. `WM_TIMECHANGE` covers a clock change and a time-zone change alike;
//! `WM_POWERBROADCAST` with `PBT_APMRESUMEAUTOMATIC` is the resume, sent on
//! every wake whether or not a user is there.
//!
//! The scheduler does not rely on these alone: it also notices a clock jump
//! itself and re-reads the zone (`docs/specs/reminders.md` §7). They make the
//! recompute immediate rather than at its next bounded wake.

/// What happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemEvent {
    /// The clock or the time zone was changed.
    TimeChanged,
    /// The machine woke from sleep or hibernation.
    Resumed,
}

/// Keeps the watcher alive; dropping it closes the hidden window and ends its
/// thread.
#[derive(Debug)]
pub struct SystemEvents {
    window: platform::Window,
}

impl Drop for SystemEvents {
    fn drop(&mut self) {
        platform::close(&self.window);
    }
}

/// Watch for the events, calling `on_event` on the watcher's own thread for
/// each one. The callback must be quick — a channel send, not work.
pub fn watch(
    on_event: impl Fn(SystemEvent) + Send + 'static,
) -> Result<SystemEvents, std::io::Error> {
    let window = platform::start(Box::new(on_event))?;
    Ok(SystemEvents { window })
}

#[cfg(windows)]
mod platform {
    use super::SystemEvent;
    use std::cell::RefCell;
    use std::sync::mpsc;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, MSG,
        PBT_APMRESUMEAUTOMATIC, PostMessageW, PostQuitMessage, RegisterClassW, WM_CLOSE,
        WM_DESTROY, WM_POWERBROADCAST, WM_TIMECHANGE, WNDCLASSW, WS_EX_TOOLWINDOW, WS_POPUP,
    };
    use windows::core::w;

    type Handler = Box<dyn Fn(SystemEvent) + Send>;

    thread_local! {
        /// The callback, owned by the watcher thread that runs the window
        /// procedure, so the procedure needs no pointer smuggled through the
        /// window's user data.
        static HANDLER: RefCell<Option<Handler>> = const { RefCell::new(None) };
    }

    /// The hidden window, as an integer: an `HWND` is not `Send`, and this is
    /// only ever used to post to it, which any thread may do.
    #[derive(Debug)]
    pub(super) struct Window(isize);

    pub(super) fn start(handler: Handler) -> Result<Window, std::io::Error> {
        let (ready, answer) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("system-events".into())
            .spawn(move || run(handler, &ready))?;
        answer
            .recv()
            .map_err(|_| std::io::Error::other("the system-events thread ended before starting"))?
            .map(Window)
            .map_err(std::io::Error::other)
    }

    #[allow(
        unsafe_code,
        reason = "registering a window class and running its message loop are Win32 calls"
    )]
    fn run(handler: Handler, ready: &mpsc::SyncSender<Result<isize, String>>) {
        HANDLER.with(|slot| *slot.borrow_mut() = Some(handler));

        // SAFETY: the module handle of this exe; never freed.
        let instance = match unsafe { GetModuleHandleW(None) } {
            Ok(module) => module.into(),
            Err(error) => {
                let _ = ready.send(Err(error.to_string()));
                return;
            }
        };
        let class = WNDCLASSW {
            lpfnWndProc: Some(procedure),
            hInstance: instance,
            lpszClassName: w!("OmniToasterSystemEvents"),
            ..Default::default()
        };
        // SAFETY: `class` is fully initialised and its name is a static
        // string. A second registration in the same process fails with
        // "class already exists", which CreateWindowExW below still accepts.
        unsafe { RegisterClassW(&class) };

        // SAFETY: the class was registered above; a zero-sized, never-shown
        // popup with no parent, menu or creation data.
        let window = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("OmniToasterSystemEvents"),
                w!("OmniToaster system events"),
                WS_POPUP,
                0,
                0,
                0,
                0,
                None,
                None,
                Some(instance),
                None,
            )
        };
        let window = match window {
            Ok(window) => window,
            Err(error) => {
                let _ = ready.send(Err(error.to_string()));
                return;
            }
        };
        let _ = ready.send(Ok(window.0 as isize));

        let mut message = MSG::default();
        // SAFETY: `message` is a live MSG for the duration of each call. The
        // loop ends on WM_QUIT (0) or an error (-1).
        while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
            // SAFETY: a message GetMessageW just filled in.
            unsafe { DispatchMessageW(&message) };
        }
    }

    #[allow(unsafe_code, reason = "a window procedure is called by Windows")]
    extern "system" fn procedure(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        let event = match message {
            WM_TIMECHANGE => Some(SystemEvent::TimeChanged),
            WM_POWERBROADCAST if wparam.0 == PBT_APMRESUMEAUTOMATIC as usize => {
                Some(SystemEvent::Resumed)
            }
            WM_DESTROY => {
                // SAFETY: ends this thread's message loop.
                unsafe { PostQuitMessage(0) };
                None
            }
            _ => None,
        };
        if let Some(event) = event {
            HANDLER.with(|slot| {
                if let Some(handler) = slot.borrow().as_ref() {
                    handler(event);
                }
            });
        }
        // SAFETY: the default handling for every message, including
        // WM_CLOSE, which destroys the window.
        unsafe { DefWindowProcW(window, message, wparam, lparam) }
    }

    #[allow(unsafe_code, reason = "posting to a window is a Win32 call")]
    pub(super) fn close(window: &Window) {
        // SAFETY: posting is safe from any thread; a window already gone
        // makes the call fail, which is ignored.
        let _ = unsafe {
            PostMessageW(
                Some(HWND(window.0 as *mut _)),
                WM_CLOSE,
                WPARAM(0),
                LPARAM(0),
            )
        };
    }

    /// Post a message to the watcher's window, as Windows would send it.
    #[cfg(test)]
    #[allow(unsafe_code, reason = "posting to a window is a Win32 call")]
    pub(super) fn post(window: &Window, message: u32, wparam: usize) {
        // SAFETY: as in `close`.
        let _ = unsafe {
            PostMessageW(
                Some(HWND(window.0 as *mut _)),
                message,
                WPARAM(wparam),
                LPARAM(0),
            )
        };
    }
}

#[cfg(not(windows))]
mod platform {
    use super::SystemEvent;

    /// Nothing to watch off Windows; the scheduler's own clock checks remain.
    #[derive(Debug)]
    pub(super) struct Window;

    pub(super) fn start(
        _handler: Box<dyn Fn(SystemEvent) + Send>,
    ) -> Result<Window, std::io::Error> {
        Ok(Window)
    }

    pub(super) fn close(_window: &Window) {}
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;
    use windows::Win32::UI::WindowsAndMessaging::{
        PBT_APMRESUMEAUTOMATIC, PBT_APMSUSPEND, WM_POWERBROADCAST, WM_TIMECHANGE,
    };

    #[test]
    fn the_two_messages_arrive_as_their_events_and_others_do_not() {
        // The messages are posted to the watcher's own window rather than
        // caused, so the test changes neither the clock nor the power state.
        let (sender, events) = mpsc::channel();
        let watcher = watch(move |event| {
            let _ = sender.send(event);
        })
        .expect("the watcher starts");

        platform::post(&watcher.window, WM_POWERBROADCAST, PBT_APMSUSPEND as usize);
        platform::post(&watcher.window, WM_TIMECHANGE, 0);
        platform::post(
            &watcher.window,
            WM_POWERBROADCAST,
            PBT_APMRESUMEAUTOMATIC as usize,
        );

        let wait = Duration::from_secs(5);
        assert_eq!(events.recv_timeout(wait), Ok(SystemEvent::TimeChanged));
        assert_eq!(events.recv_timeout(wait), Ok(SystemEvent::Resumed));
        assert!(
            events.recv_timeout(Duration::from_millis(200)).is_err(),
            "a suspend is not a resume"
        );
    }
}
