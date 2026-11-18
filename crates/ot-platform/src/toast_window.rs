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

/// The shape a toast window is cut to, in the window's own physical pixels.
///
/// The toast's rect where it is drawn this frame — moved by a drawer, scaled
/// by a pop — with its corners' radii, `[nw, ne, sw, se]`, intersected with
/// `clip`, the part of the window that may show: what a slide has inside the
/// work area, what a drawer has opened. Rects are `[left, top, right, bottom]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outline {
    pub rect: [i32; 4],
    pub radii: [i32; 4],
    pub clip: [i32; 4],
}

impl Outline {
    /// The whole of a `width` × `height` window, with these corners.
    pub fn whole([width, height]: [i32; 2], radii: [i32; 4]) -> Self {
        Self {
            rect: [0, 0, width, height],
            radii,
            clip: [0, 0, width, height],
        }
    }
}

/// Cut the toast window titled exactly `title` to `outline` — a plain
/// rectangle filling the window needs no cut at all — and fade the whole
/// window to `opacity` (255 is solid).
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
/// only for a window with no region — so only when the outline is the whole
/// window with every radius zero (CR-0006). Windows would also round such a window's corners on its own;
/// it is told not to.
///
/// `glass` lays Windows' blur behind the window — the accent policy's
/// Acrylic, the one blur Windows draws for a window that is not active
/// (ADR-016). The toast paints its own tint over it, so the blur's own tint is
/// almost nothing. Where Windows will not blur, the toast is meant to be drawn
/// solid: ask [`glass_available`] before painting.
///
/// Called every frame for every toast, so it does nothing unless the window,
/// its size, the outline, the opacity, the shadow or the glass changed since the
/// last call — and when only the opacity changed, as on every frame of a
/// fade, it sets only the window's alpha.
///
/// Returns whether the window exists and carries all of this — false until
/// egui has created it, a frame or more after it was asked for. The window
/// may still be hidden: a toast is kept hidden until this says yes, so it
/// never shows before its alpha is set.
///
/// `mask`, when there is one, is a dissolve's cells or a Genie's bands —
/// rects in the same pixels — and only the part of the outline inside them
/// shows.
pub fn set_shape(
    title: &str,
    outline: Outline,
    mask: Option<&[[i32; 4]]>,
    opacity: u8,
    shadow: bool,
    glass: bool,
) -> bool {
    platform::set_shape(title, outline, mask, opacity, shadow, glass)
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
        DWMWA_TRANSITIONS_FORCEDISABLED, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
        DwmExtendFrameIntoClientArea, DwmSetWindowAttribute,
    };
    use windows::Win32::Graphics::Gdi::{
        CombineRgn, CreateEllipticRgn, CreateRectRgn, DeleteObject, ExtCreateRegion, HRGN,
        OffsetRgn, RDH_RECTANGLES, RGN_AND, RGN_DIFF, RGN_OR, RGNDATA, RGNDATAHEADER, SetWindowRgn,
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

    /// The window, client size, outline, opacity, shadow and glass last
    /// applied.
    type Applied = (
        isize,
        [i32; 2],
        (super::Outline, Option<u64>),
        u8,
        bool,
        bool,
    );

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
    pub(super) fn set_shape(
        title: &str,
        outline: super::Outline,
        mask: Option<&[[i32; 4]]>,
        opacity: u8,
        shadow: bool,
        glass: bool,
    ) -> bool {
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
                    return false;
                };
                window
            }
        };

        let mut client = RECT::default();
        // SAFETY: a live window this process owns, and a RECT the call fills.
        if unsafe { GetClientRect(window, &mut client) }.is_err() {
            return false;
        }
        let size = [client.right - client.left, client.bottom - client.top];
        // A mask is known by a hash of its rects, not their count: a Genie's
        // bands are always the same number, only in new places.
        let shape = (outline, mask.map(mask_key));
        let applied = (window.0 as isize, size, shape, opacity, shadow, glass);
        let before = shapes.get(title).copied();
        if before == Some(applied) {
            return true;
        }
        // Only the opacity moved — every frame of a fade — so the region and
        // the compositor settings stay as they are, and only the alpha is set.
        let reshape = before.is_none_or(|(w, s, r, _, sh, g)| {
            (w, s, r, sh, g) != (window.0 as isize, size, shape, shadow, glass)
        });

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
        if !reshape {
            shapes.insert(title.to_owned(), applied);
            return true;
        }

        let region = region(size, outline, mask);
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
            // No show animation of Windows' own. Windows plays one on a
            // window it has just made visible, and for its length — about
            // 300 ms — what is on screen is its transition, not the toast's
            // frames, so the first third of every entry never showed
            // (BUG-0011). The toast animates itself.
            let no_transitions = BOOL::from(true);
            let _ = DwmSetWindowAttribute(
                window,
                DWMWA_TRANSITIONS_FORCEDISABLED,
                std::ptr::from_ref(&no_transitions).cast(),
                u32::try_from(size_of_val(&no_transitions)).unwrap_or(0),
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
            ?outline,
            opacity,
            shadow,
            glass,
            "a toast window was cut to its shape"
        );
        true
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

    /// A fingerprint of a mask's rects (FNV-1a over their coordinates):
    /// enough to tell one frame's mask from the next without keeping them.
    pub(super) fn mask_key(mask: &[[i32; 4]]) -> u64 {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for value in mask.iter().flatten() {
            for byte in value.to_le_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x0100_0000_01b3);
            }
        }
        hash ^ mask.len() as u64
    }

    /// The outline as a region: its rect with each corner rounded to its
    /// radius, cut to its clip — or `None` for the whole window, square,
    /// which needs no region.
    #[allow(
        unsafe_code,
        reason = "GDI region calls; each one's contract is argued at the call"
    )]
    fn region(
        [client_w, client_h]: [i32; 2],
        outline: super::Outline,
        mask: Option<&[[i32; 4]]>,
    ) -> Option<HRGN> {
        let covers = |r: [i32; 4]| r[0] <= 0 && r[1] <= 0 && r[2] >= client_w && r[3] >= client_h;
        if mask.is_none()
            && outline.radii.iter().all(|r| *r <= 0)
            && covers(outline.rect)
            && covers(outline.clip)
        {
            return None;
        }
        let [x0, y0, x1, y1] = outline.rect;
        let (width, height) = ((x1 - x0).max(0), (y1 - y0).max(0));
        let radii = outline.radii;
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
            // Built at the origin, moved to where the toast is drawn, then
            // cut to the part that may show.
            OffsetRgn(shape, x0, y0);
            let [cx0, cy0, cx1, cy1] = outline.clip;
            let clip = CreateRectRgn(cx0, cy0, cx1.max(cx0), cy1.max(cy0));
            CombineRgn(Some(shape), Some(shape), Some(clip), RGN_AND);
            let _ = DeleteObject(clip.into());
            if let Some(cells) = mask {
                let cells = cells_region(cells);
                CombineRgn(Some(shape), Some(shape), Some(cells), RGN_AND);
                let _ = DeleteObject(cells.into());
            }
            Some(shape)
        }
    }

    /// A dissolve's cells as one region, made in a single call from a list
    /// of rects rather than combined one by one — a toast is over a thousand
    /// cells, and this is redone on every frame of the dissolve.
    #[allow(
        unsafe_code,
        reason = "ExtCreateRegion reads a RGNDATA laid out in a buffer built here"
    )]
    fn cells_region(cells: &[[i32; 4]]) -> HRGN {
        // RGNDATAHEADER, then the rects, as RECTs, in one aligned buffer.
        let header = size_of::<RGNDATAHEADER>();
        let words = (header + cells.len() * size_of::<RECT>()).div_ceil(4);
        let mut buffer = vec![0u32; words];
        let bound = cells
            .iter()
            .fold([i32::MAX, i32::MAX, i32::MIN, i32::MIN], |b, c| {
                [
                    b[0].min(c[0]),
                    b[1].min(c[1]),
                    b[2].max(c[2]),
                    b[3].max(c[3]),
                ]
            });
        let data = buffer.as_mut_ptr().cast::<RGNDATA>();
        // SAFETY: the buffer is u32-aligned, which RGNDATAHEADER and RECT
        // (i32 and u32 fields) need, and long enough for the header and every
        // rect; each write stays inside it. ExtCreateRegion only reads the
        // `nRgnSize` bytes after the header the header says are there.
        unsafe {
            (*data).rdh = RGNDATAHEADER {
                dwSize: u32::try_from(header).unwrap_or(32),
                iType: RDH_RECTANGLES,
                nCount: u32::try_from(cells.len()).unwrap_or(0),
                nRgnSize: u32::try_from(cells.len() * size_of::<RECT>()).unwrap_or(0),
                rcBound: if cells.is_empty() {
                    RECT::default()
                } else {
                    RECT {
                        left: bound[0],
                        top: bound[1],
                        right: bound[2],
                        bottom: bound[3],
                    }
                },
            };
            let rects = buffer.as_mut_ptr().cast::<u8>().add(header).cast::<RECT>();
            for (index, [left, top, right, bottom]) in cells.iter().copied().enumerate() {
                rects.add(index).write(RECT {
                    left,
                    top,
                    right,
                    bottom,
                });
            }
            let total = u32::try_from(header + cells.len() * size_of::<RECT>()).unwrap_or(0);
            let region = ExtCreateRegion(None, total, data);
            if region.is_invalid() {
                // Nothing came back: an empty region shows nothing, which is
                // what a dissolve with no cells yet should show anyway.
                CreateRectRgn(0, 0, 0, 0)
            } else {
                region
            }
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

        // A hidden window counts when looking for one by its exact title: a
        // toast is shaped while still hidden, so it never shows before its
        // alpha is set. The prefix sweep still skips hidden windows.
        // SAFETY: IsWindowVisible only reads the window's state.
        if search.prefix && !unsafe { IsWindowVisible(window) }.as_bool() {
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
        _outline: super::Outline,
        _mask: Option<&[[i32; 4]]>,
        _opacity: u8,
        _shadow: bool,
        _glass: bool,
    ) -> bool {
        false
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
            Outline {
                rect: [0, 0, 100, 50],
                radii: [12, 0, 12, 0],
                clip: [0, 0, 60, 50],
            },
            Some(&[[0, 0, 6, 6], [6, 6, 12, 12]]),
            200,
            true,
            true,
        );
        forget("a window title nothing in this process has");
    }

    #[cfg(windows)]
    #[test]
    fn a_mask_is_known_by_where_its_rects_are_not_how_many() {
        // FEAT-0034: a Genie's 24 bands move every frame and stay 24, and a
        // key made of the count left the window cut to its first frame.
        let here = [[0, 0, 10, 10], [0, 10, 10, 20]];
        let there = [[5, 0, 15, 10], [5, 10, 15, 20]];
        assert_ne!(platform::mask_key(&here), platform::mask_key(&there));
        assert_eq!(platform::mask_key(&here), platform::mask_key(&here));
    }
}
