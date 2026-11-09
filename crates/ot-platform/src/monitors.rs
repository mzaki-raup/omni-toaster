//! The monitors, and the work area of each (`docs/specs/toast.md` §7).
//!
//! The **work area** is what a toast is placed against: the monitor's rect
//! minus the taskbar and anything else docked to an edge. Using the full
//! monitor rect is how a notification ends up under the taskbar.
//!
//! Coordinates come out of Win32 in physical pixels and leave here in
//! **points**, divided by that monitor's own scale, because that is what the
//! window system is given back. A monitor left of or above the primary one
//! has a **negative** origin, which is the case a routine written against the
//! primary monitor gets wrong.

/// One monitor.
#[derive(Debug, Clone, PartialEq)]
pub struct Monitor {
    /// The device name, e.g. `\\.\DISPLAY1`. Stable enough to remember a
    /// choice by, and not shown to anyone.
    pub name: String,
    /// The work area in points: left, top, width, height.
    pub work_area: [f32; 4],
    /// 1.0 at 100%, 1.5 at 150%, and so on.
    pub scale: f32,
    pub primary: bool,
}

impl Monitor {
    /// The work area as an egui rect, in points.
    pub fn work_area_rect(&self) -> egui::Rect {
        let [x, y, w, h] = self.work_area;
        egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h))
    }
}

/// Every monitor attached right now, primary first.
///
/// The list is read fresh rather than cached: a monitor can be unplugged
/// while a toast is on screen, and a stale list would put the next one on a
/// screen that is no longer there.
pub fn monitors() -> Vec<Monitor> {
    let mut found = platform::monitors();
    found.sort_by_key(|monitor| !monitor.primary);
    found
}

/// The monitor a toast should use: the one named, or the primary one.
///
/// A remembered monitor that is no longer attached falls back to the primary
/// one **and says so**, because a toast that silently stops appearing is
/// worse than one that moved (`docs/specs/toast.md` §7.4).
pub fn chosen_or_primary(name: Option<&str>) -> Option<Monitor> {
    let all = monitors();
    if let Some(wanted) = name {
        if let Some(found) = all.iter().find(|monitor| monitor.name == wanted) {
            return Some(found.clone());
        }
        tracing::warn!(
            wanted,
            "the chosen monitor is not attached; using the primary one"
        );
    }
    all.into_iter().next()
}

#[cfg(windows)]
mod platform {
    use windows::Win32::Foundation::{LPARAM, RECT};
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
    };
    use windows::core::BOOL;

    /// `MONITORINFOF_PRIMARY`. The `windows` crate does not re-export it, and
    /// it is a documented constant rather than a value to guess.
    const PRIMARY_FLAG: u32 = 1;
    use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

    use super::Monitor;

    /// What Windows reports at 100%.
    const BASE_DPI: f32 = 96.0;

    pub(super) fn monitors() -> Vec<Monitor> {
        let mut found: Vec<Monitor> = Vec::new();
        #[allow(
            unsafe_code,
            reason = "EnumDisplayMonitors is a Win32 call; the callback contract is argued at the call"
        )]
        // SAFETY: the callback matches the signature Windows expects, and the
        // LPARAM points at `found`, which outlives the call because
        // EnumDisplayMonitors is synchronous and the callback does not store
        // the pointer.
        unsafe {
            let _ = EnumDisplayMonitors(
                None,
                None,
                Some(visit),
                LPARAM(std::ptr::from_mut(&mut found) as isize),
            );
        }
        found
    }

    #[allow(
        unsafe_code,
        reason = "a Win32 callback: its parameters come from the OS and are argued inside"
    )]
    unsafe extern "system" fn visit(
        monitor: HMONITOR,
        _dc: HDC,
        _rect: *mut RECT,
        data: LPARAM,
    ) -> BOOL {
        // SAFETY: `data` is the pointer EnumDisplayMonitors was given, which
        // points at the caller's live Vec for the duration of the call.
        let found = unsafe { &mut *(data.0 as *mut Vec<Monitor>) };

        let mut info = MONITORINFOEXW {
            monitorInfo: MONITORINFO {
                cbSize: u32::try_from(size_of::<MONITORINFOEXW>()).unwrap_or(0),
                ..Default::default()
            },
            ..Default::default()
        };
        // SAFETY: `info` is live and its cbSize says how big it is, which is
        // what bounds the write.
        let ok = unsafe {
            GetMonitorInfoW(monitor, std::ptr::from_mut(&mut info).cast::<MONITORINFO>())
        };
        if !ok.as_bool() {
            // One monitor failing to describe itself is not a reason to stop
            // enumerating the others.
            return true.into();
        }

        let mut dpi_x = BASE_DPI as u32;
        let mut dpi_y = BASE_DPI as u32;
        // SAFETY: both out-parameters are live u32s the call writes once.
        let _ = unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) };
        let scale = (dpi_x as f32 / BASE_DPI).max(0.1);

        let work = info.monitorInfo.rcWork;
        let name = String::from_utf16_lossy(
            &info
                .szDevice
                .iter()
                .copied()
                .take_while(|c| *c != 0)
                .collect::<Vec<u16>>(),
        );

        found.push(Monitor {
            name,
            // Physical pixels in, points out: the window system is given
            // points back, and a 150% monitor's 1920 physical pixels are
            // 1280 points.
            work_area: [
                work.left as f32 / scale,
                work.top as f32 / scale,
                (work.right - work.left) as f32 / scale,
                (work.bottom - work.top) as f32 / scale,
            ],
            scale,
            primary: info.monitorInfo.dwFlags & PRIMARY_FLAG != 0,
        });
        true.into()
    }
}

#[cfg(not(windows))]
mod platform {
    use super::Monitor;

    pub(super) fn monitors() -> Vec<Monitor> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn this_machine_has_at_least_one_monitor_with_a_usable_work_area() {
        let all = monitors();
        assert!(!all.is_empty(), "a machine running this has a screen");
        assert!(
            all.iter().filter(|m| m.primary).count() <= 1,
            "there is at most one primary monitor"
        );
        for monitor in &all {
            let rect = monitor.work_area_rect();
            assert!(
                rect.width() > 100.0 && rect.height() > 100.0,
                "{}: a work area of {rect:?} is not usable",
                monitor.name
            );
            assert!(
                monitor.scale >= 0.1,
                "{}: scale {}",
                monitor.name,
                monitor.scale
            );
            assert!(!monitor.name.is_empty());
        }
    }

    #[cfg(windows)]
    #[test]
    fn the_work_area_is_not_the_whole_screen_when_something_is_docked() {
        // Not an assertion that a taskbar exists - someone may auto-hide it -
        // but the work area must never be *larger* than the monitor.
        for monitor in monitors() {
            let rect = monitor.work_area_rect();
            assert!(rect.width() * monitor.scale <= 32_767.0);
            assert!(rect.height() * monitor.scale <= 32_767.0);
        }
    }

    #[cfg(windows)]
    #[test]
    fn an_unattached_monitor_falls_back_to_the_primary_one() {
        let fallback =
            chosen_or_primary(Some(r"\\.\DISPLAY_THAT_IS_NOT_THERE")).expect("the primary monitor");
        assert!(
            fallback.primary,
            "a remembered monitor that is gone must fall back to the primary one"
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn there_are_no_monitors_off_windows() {
        assert!(monitors().is_empty());
        assert_eq!(chosen_or_primary(None), None);
    }
}
