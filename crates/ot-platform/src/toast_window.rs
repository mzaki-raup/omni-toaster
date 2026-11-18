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
//! The windows are found by title within **this process**: a toast's title
//! starts with the app's own name, which never changes with the language, and
//! nothing else in the process carries it. A window belonging to another
//! process is never touched.
//!
//! **Corners.** The toast is also cut to its painted shape with a window
//! region (BUG-0007). eframe draws every window through one OpenGL pixel
//! format, and the driver on the reference machine offers none that the
//! desktop compositor treats as transparent — eframe logs "Cannot create
//! transparent window" — so the unpainted corners of a rounded toast came out
//! black. A region makes Windows itself leave those pixels out, on any driver.
//! Its edge is whole pixels, not smoothed: the cost of working everywhere.
//!
//! **Alpha.** "Cannot create transparent window" turned out to mean only that
//! the pixel format lacks an overlay-plane flag. With the frame extended over
//! the whole client area, DWM composites the surface's alpha after all, so a
//! toast can be see-through wherever it paints nothing (ADR-016) — which is
//! what glass and the animations need. The region stays, by the owner's
//! choice, so the corners look as they did.

/// Make every toast window in this process unobtrusive: every window whose
/// title starts with `prefix`.
///
/// Returns how many windows were changed, which is zero on every frame after
/// the first — the call is cheap enough to make each frame and idempotent
/// enough that doing so is not a mistake.
pub fn make_unobtrusive(prefix: &str) -> usize {
    platform::make_unobtrusive(prefix)
}

/// Cut the toast window titled exactly `title` to a rectangle whose corners
/// have these radii, `[nw, ne, sw, se]`, in physical pixels — all zero means
/// a plain rectangle — and fade the whole window to `opacity` (255 is solid).
///
/// The fade is the window's own alpha (`WS_EX_LAYERED`), which Windows
/// composites against the desktop on any driver: the whole toast, text
/// included, as the owner chose (CR-0005).
///
/// Without Windows' shadow, the frame is extended over the whole client area
/// so that what the toast leaves unpainted — alpha 0 — shows the desktop
/// (ADR-016).
///
/// `shadow` asks for Windows' own drop shadow, which the compositor draws
/// only for a window with no region — so only when every radius is zero
/// (CR-0006). Windows would also round such a window's corners on its own;
/// it is told not to.
///
/// `glass` lays Windows' blur behind the window — the accent policy's
/// Acrylic, the one blur Windows draws for a window that is not active
/// (ADR-016). The toast paints its own tint over it, so the blur's own tint is
/// almost nothing. Where Windows will not blur, the toast is meant to be drawn
/// solid: ask [`glass_available`] before painting.
///
/// Called every frame for every toast, so it does nothing unless the window,
/// its size, the radii, the opacity, the shadow or the glass changed since the
/// last call.
pub fn set_shape(title: &str, radii: [i32; 4], opacity: u8, shadow: bool, glass: bool) {
    platform::set_shape(title, radii, opacity, shadow, glass);
}

/// Whether Windows will blur behind a toast: Windows 10 1809 or later, the
/// undocumented call present, and no refusal from it so far. The first
/// failure is logged with its reason, and from then on this says no, so the
/// toast is drawn solid rather than with a glass that is not there
/// (`docs/specs/toast.md` §4.2).
pub fn glass_available() -> bool {
    platform::glass_available()
}

/// Drop what [`set_shape`] remembers about a toast that has gone.
pub fn forget(title: &str) {
    platform::forget(title);
}

#[cfg(windows)]
mod platform {
    use std::collections::HashMap;
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Mutex, OnceLock};

    use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, RECT};
    use windows::Win32::Graphics::Dwm::{
        DWMNCRP_DISABLED, DWMNCRP_ENABLED, DWMWA_NCRENDERING_POLICY,
        DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND, DwmExtendFrameIntoClientArea,
        DwmSetWindowAttribute,
    };
    use windows::Win32::Graphics::Gdi::{
        CombineRgn, CreateEllipticRgn, CreateRectRgn, DeleteObject, HRGN, RGN_AND, RGN_DIFF,
        RGN_OR, SetWindowRgn,
    };
    use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
    use windows::Win32::System::SystemInformation::OSVERSIONINFOW;
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::UI::Controls::MARGINS;
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GWL_EXSTYLE, GetClientRect, GetWindowLongPtrW, GetWindowTextW,
        GetWindowThreadProcessId, IsWindow, IsWindowVisible, LWA_ALPHA, SetLayeredWindowAttributes,
        SetWindowLongPtrW, WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };
    use windows::core::{BOOL, s, w};

    /// What the enumeration is looking for, and what it found.
    struct Search {
        title: Vec<u16>,
        /// Match the title as a prefix rather than exactly.
        prefix: bool,
        process: u32,
        changed: usize,
        found: Option<HWND>,
    }

    /// The window, client size, radii, opacity, shadow and glass last applied.
    type Applied = (isize, [i32; 2], [i32; 4], u8, bool, bool);

    /// Lets `set_shape` skip both the enumeration and the region rebuild on
    /// every frame where nothing changed.
    static SHAPES: Mutex<Option<HashMap<String, Applied>>> = Mutex::new(None);

    #[allow(
        unsafe_code,
        reason = "Win32 calls; each one's contract is argued at the call"
    )]
    pub(super) fn make_unobtrusive(prefix: &str) -> usize {
        let mut search = Search {
            title: prefix.encode_utf16().collect(),
            prefix: true,
            // SAFETY: GetCurrentProcessId takes nothing and returns a value;
            // there is no pointer and no handle to get wrong.
            process: unsafe { GetCurrentProcessId() },
            changed: 0,
            found: None,
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
        reason = "Win32 calls; each one's contract is argued at the call"
    )]
    pub(super) fn set_shape(title: &str, radii: [i32; 4], opacity: u8, shadow: bool, glass: bool) {
        let mut shapes = SHAPES
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let shapes = shapes.get_or_insert_with(HashMap::new);

        // The window this title had last time, if it is still a window.
        let known = shapes
            .get(title)
            .map(|(window, ..)| HWND(*window as *mut _));
        // SAFETY: IsWindow accepts any value and only answers whether it is a
        // live window handle; a stale one is exactly what it is for.
        let window = match known.filter(|w| unsafe { IsWindow(Some(*w)) }.as_bool()) {
            Some(window) => window,
            None => {
                let mut search = Search {
                    title: title.encode_utf16().collect(),
                    prefix: false,
                    // SAFETY: as in make_unobtrusive.
                    process: unsafe { GetCurrentProcessId() },
                    changed: 0,
                    found: None,
                };
                // SAFETY: as in make_unobtrusive: `search` outlives the
                // synchronous enumeration and the callback does not keep it.
                unsafe {
                    let _ = EnumWindows(
                        Some(visit),
                        LPARAM(std::ptr::from_mut(&mut search) as isize),
                    );
                }
                // Not created yet: egui opens the window a frame after it is
                // asked for. The next frame tries again.
                let Some(window) = search.found else {
                    return;
                };
                window
            }
        };

        let mut client = RECT::default();
        // SAFETY: a live window this process owns, and a RECT the call fills.
        if unsafe { GetClientRect(window, &mut client) }.is_err() {
            return;
        }
        let size = [client.right - client.left, client.bottom - client.top];
        let applied = (window.0 as isize, size, radii, opacity, shadow, glass);
        if shapes.get(title) == Some(&applied) {
            return;
        }

        // SAFETY: plain style reads and writes on a window this process
        // owns, and a layered-window attribute call on the same window.
        unsafe {
            let style = GetWindowLongPtrW(window, GWL_EXSTYLE);
            let layered = WS_EX_LAYERED.0 as isize;
            if opacity < u8::MAX {
                if style & layered == 0 {
                    SetWindowLongPtrW(window, GWL_EXSTYLE, style | layered);
                }
                let _ = SetLayeredWindowAttributes(window, COLORREF(0), opacity, LWA_ALPHA);
            } else if style & layered != 0 {
                // Solid: no layering, which costs a composition step for
                // nothing.
                SetWindowLongPtrW(window, GWL_EXSTYLE, style & !layered);
            }
        }

        let region = region(size, radii);
        // SAFETY: on success the system owns the region and frees it when it
        // is replaced or the window goes; on failure it is still ours and is
        // freed here. `None` removes any region.
        unsafe {
            if SetWindowRgn(window, region, true) == 0
                && let Some(region) = region
            {
                let _ = DeleteObject(region.into());
            }
        }
        shapes.insert(title.to_owned(), applied);
        // Windows' shadow, for an uncut window only: the compositor draws
        // none for a window with a region.
        //
        // Without it, the frame is extended over the whole client area —
        // margins of -1 — which is what makes DWM composite the surface's own
        // alpha, so whatever the toast leaves unpainted shows the desktop
        // (ADR-016). The two exclude each other: with non-client rendering on
        // for the shadow, the client area stays opaque.
        let shadow = shadow && region.is_none();
        let (policy, margin) = if shadow {
            (DWMNCRP_ENABLED, 1)
        } else {
            (DWMNCRP_DISABLED, -1)
        };
        let margins = MARGINS {
            cxLeftWidth: margin,
            cxRightWidth: margin,
            cyTopHeight: margin,
            cyBottomHeight: margin,
        };
        // SAFETY: each attribute is passed as a pointer to a live value of
        // the documented type with its size; the margins likewise.
        unsafe {
            let _ = DwmSetWindowAttribute(
                window,
                DWMWA_NCRENDERING_POLICY,
                std::ptr::from_ref(&policy).cast(),
                u32::try_from(size_of_val(&policy)).unwrap_or(0),
            );
            let _ = DwmSetWindowAttribute(
                window,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                std::ptr::from_ref(&DWMWCP_DONOTROUND).cast(),
                u32::try_from(size_of_val(&DWMWCP_DONOTROUND)).unwrap_or(0),
            );
            let _ = DwmExtendFrameIntoClientArea(window, &margins);
        }
        // Glass, or none: the accent is set either way, so a toast whose
        // glass was turned off stops blurring.
        if let Some(set_accent) = accent_call() {
            set_glass(set_accent, window, glass && !shadow);
        }
        tracing::debug!(
            title,
            ?size,
            ?radii,
            opacity,
            shadow,
            glass,
            "a toast window was cut to its shape"
        );
    }

    /// `ACCENT_POLICY`, from the undocumented `SetWindowCompositionAttribute`.
    #[repr(C)]
    struct AccentPolicy {
        state: u32,
        flags: u32,
        /// The blur's own tint, AABBGGRR.
        gradient: u32,
        animation: u32,
    }

    /// `WINDOWCOMPOSITIONATTRIBDATA`.
    #[repr(C)]
    struct CompositionData {
        attribute: u32,
        data: *mut c_void,
        size: usize,
    }

    /// What `GetProcAddress` hands back, before it is given its signature.
    type Export = unsafe extern "system" fn() -> isize;
    type SetAccent = unsafe extern "system" fn(HWND, *mut CompositionData) -> BOOL;
    type GetVersion = unsafe extern "system" fn(*mut OSVERSIONINFOW) -> i32;

    /// `WCA_ACCENT_POLICY`.
    const ACCENT_POLICY: u32 = 19;
    const ACCENT_DISABLED: u32 = 0;
    /// `ACCENT_ENABLE_ACRYLICBLURBEHIND`: the one that blurs an inactive
    /// window. Plain blur-behind (3) draws black on Windows 11 26H2.
    const ACCENT_ACRYLIC: u32 = 4;
    /// Draw the gradient colour as the tint.
    const ACCENT_FLAG_TINT: u32 = 2;
    /// Almost no tint of its own: the toast paints its tint over the blur.
    /// Not black: with a black tint — at alpha 0x01 or 0x10 alike — Windows
    /// 11 26H2 draws the whole window black; a grey one at 0x10 blurs
    /// (measured twice over red and blue stripes, FEAT-0030).
    const BLUR_TINT: u32 = 0x1080_8080;
    /// Windows 10 1809, where the Acrylic accent arrived.
    const FIRST_BUILD: u32 = 17763;

    /// A refusal from Windows, after which every toast is drawn solid.
    static REFUSED: AtomicBool = AtomicBool::new(false);

    pub(super) fn glass_available() -> bool {
        !REFUSED.load(Ordering::Relaxed) && accent_call().is_some()
    }

    /// The accent call, if this Windows has it and is new enough. Looked up
    /// once; the reason it is missing is logged once.
    fn accent_call() -> Option<SetAccent> {
        static CALL: OnceLock<Option<SetAccent>> = OnceLock::new();
        *CALL.get_or_init(|| {
            let build = os_build();
            if build.is_some_and(|build| build < FIRST_BUILD) {
                tracing::warn!(?build, "this Windows is older than 10 1809; toasts are drawn solid instead of glass");
                return None;
            }
            // SAFETY: user32 is loaded in every GUI process, and the name
            // is a NUL-terminated literal. The pointer is transmuted to the
            // signature the call has had since Windows 7, the one every
            // caller of this undocumented function uses.
            #[allow(
                unsafe_code,
                reason = "an undocumented user32 export, loaded by name"
            )]
            let call = unsafe {
                GetModuleHandleW(w!("user32.dll"))
                    .ok()
                    .and_then(|user32| GetProcAddress(user32, s!("SetWindowCompositionAttribute")))
                    .map(|address| std::mem::transmute::<Export, SetAccent>(address))
            };
            if call.is_none() {
                tracing::warn!("Windows has no SetWindowCompositionAttribute; toasts are drawn solid instead of glass");
            }
            call
        })
    }

    /// The Windows build number, from `RtlGetVersion`, which does not lie
    /// about the version to an exe without a compatibility manifest the way
    /// `GetVersionEx` does. `None` if it cannot be asked.
    #[allow(
        unsafe_code,
        reason = "an ntdll export loaded by name, and the struct it fills"
    )]
    fn os_build() -> Option<u32> {
        // SAFETY: ntdll is loaded in every process; the name is a literal;
        // the signature is RtlGetVersion's documented one; the struct is
        // sized as the call requires before it is passed.
        unsafe {
            let ntdll = GetModuleHandleW(w!("ntdll.dll")).ok()?;
            let address = GetProcAddress(ntdll, s!("RtlGetVersion"))?;
            let get_version = std::mem::transmute::<Export, GetVersion>(address);
            let mut info = OSVERSIONINFOW {
                dwOSVersionInfoSize: u32::try_from(size_of::<OSVERSIONINFOW>()).ok()?,
                ..OSVERSIONINFOW::default()
            };
            (get_version(&mut info) == 0).then_some(info.dwBuildNumber)
        }
    }

    /// Turn the blur on or off for one window. A refusal is logged and turns
    /// glass off for every toast from then on.
    #[allow(
        unsafe_code,
        reason = "the undocumented accent call, with structs laid out as it expects"
    )]
    fn set_glass(set_accent: SetAccent, window: HWND, on: bool) {
        let mut policy = AccentPolicy {
            state: if on { ACCENT_ACRYLIC } else { ACCENT_DISABLED },
            flags: if on { ACCENT_FLAG_TINT } else { 0 },
            gradient: if on { BLUR_TINT } else { 0 },
            animation: 0,
        };
        let mut data = CompositionData {
            attribute: ACCENT_POLICY,
            data: std::ptr::from_mut(&mut policy).cast(),
            size: size_of::<AccentPolicy>(),
        };
        // SAFETY: a live window this process owns, and a pointer to `data`,
        // which points at `policy`; both outlive the synchronous call.
        let accepted = unsafe { set_accent(window, &mut data) }.as_bool();
        if on && !accepted && !REFUSED.swap(true, Ordering::Relaxed) {
            tracing::warn!(
                "Windows refused the blur behind a toast; toasts are drawn solid instead of glass"
            );
        }
    }

    pub(super) fn forget(title: &str) {
        let mut shapes = SHAPES
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(shapes) = shapes.as_mut() {
            shapes.remove(title);
        }
    }

    /// A `width` × `height` rectangle with each corner rounded to its radius,
    /// or `None` for a plain rectangle, which needs no region.
    #[allow(
        unsafe_code,
        reason = "GDI region calls; each one's contract is argued at the call"
    )]
    fn region([width, height]: [i32; 2], radii: [i32; 4]) -> Option<HRGN> {
        if radii.iter().all(|r| *r <= 0) {
            return None;
        }
        // SAFETY: GDI region calls on plain integers. Every region made here
        // is either returned (and then owned by the window) or deleted before
        // returning; CombineRgn only writes into a region made here.
        unsafe {
            let shape = CreateRectRgn(0, 0, width, height);
            let corners = [
                (radii[0], false, false),
                (radii[1], true, false),
                (radii[2], false, true),
                (radii[3], true, true),
            ];
            for (r, right, bottom) in corners {
                let r = r.min(width / 2).min(height / 2);
                if r <= 0 {
                    continue;
                }
                // The corner's r × r box, and the circle that rounds it.
                let left = if right { width - r } else { 0 };
                let top = if bottom { height - r } else { 0 };
                let cx = if right { width - r } else { r };
                let cy = if bottom { height - r } else { r };
                let corner_box = CreateRectRgn(left, top, left + r, top + r);
                let circle = CreateEllipticRgn(cx - r, cy - r, cx + r, cy + r);
                let rounded = CreateRectRgn(0, 0, 0, 0);
                CombineRgn(Some(rounded), Some(corner_box), Some(circle), RGN_AND);
                CombineRgn(Some(shape), Some(shape), Some(corner_box), RGN_DIFF);
                CombineRgn(Some(shape), Some(shape), Some(rounded), RGN_OR);
                let _ = DeleteObject(corner_box.into());
                let _ = DeleteObject(circle.into());
                let _ = DeleteObject(rounded.into());
            }
            Some(shape)
        }
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
        let matches = if search.prefix {
            text[..written].starts_with(&search.title)
        } else {
            text[..written] == search.title[..]
        };
        if written == 0 || !matches {
            return true.into();
        }
        if !search.prefix {
            // Looking for one window: this is it.
            search.found = Some(window);
            return false.into();
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
    pub(super) fn make_unobtrusive(_prefix: &str) -> usize {
        0
    }

    pub(super) fn set_shape(
        _title: &str,
        _radii: [i32; 4],
        _opacity: u8,
        _shadow: bool,
        _glass: bool,
    ) {
    }

    pub(super) fn glass_available() -> bool {
        false
    }

    pub(super) fn forget(_title: &str) {}
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

    #[test]
    fn shaping_a_window_that_does_not_exist_is_a_quiet_no_op() {
        // The toast's window opens a frame after it is asked for, so the
        // first calls always find nothing.
        set_shape(
            "a window title nothing in this process has",
            [12, 0, 12, 0],
            200,
            true,
            true,
        );
        forget("a window title nothing in this process has");
    }
}
