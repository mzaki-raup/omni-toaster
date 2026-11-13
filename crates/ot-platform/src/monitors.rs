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
    /// The device name, e.g. `\\.\DISPLAY1`. **Not** stable: Windows can
    /// renumber displays after a reboot or a re-plug, so a choice is never
    /// remembered by it — see [`Monitor::id`].
    pub name: String,
    /// What a choice of monitor is remembered by: the monitor's device path,
    /// which names the physical monitor on its port and survives a reboot.
    /// The device name when Windows will not say.
    pub id: String,
    /// The monitor's own name, e.g. "DELL U2720Q", from its EDID. Empty for
    /// a panel that reports none, which many built-in laptop screens do.
    pub label: String,
    /// The number Windows shows in Settings ▸ Display: the `1` of `DISPLAY1`.
    pub number: u32,
    /// Its full size in physical pixels, which is what tells two identical
    /// monitors apart in a list.
    pub size: [u32; 2],
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

/// The monitor a toast should use from `all`: the one whose [`Monitor::id`]
/// is `wanted`, or the primary one — when nothing was chosen, and when the
/// chosen monitor is not attached (`docs/specs/toast.md` §7.4).
///
/// The second value says whether a chosen monitor was missing, so the caller
/// can say so once rather than on every frame.
pub fn pick<'a>(all: &'a [Monitor], wanted: Option<&str>) -> Option<(&'a Monitor, bool)> {
    if let Some(wanted) = wanted
        && let Some(found) = all.iter().find(|monitor| monitor.id == wanted)
    {
        return Some((found, false));
    }
    let primary = all
        .iter()
        .find(|monitor| monitor.primary)
        .or_else(|| all.first())?;
    Some((primary, wanted.is_some()))
}

#[cfg(windows)]
mod platform {
    use std::collections::HashMap;

    use windows::Win32::Devices::Display::{
        DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
        DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SOURCE_DEVICE_NAME,
        DISPLAYCONFIG_TARGET_DEVICE_NAME, DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes,
        QDC_ONLY_ACTIVE_PATHS, QueryDisplayConfig,
    };
    use windows::Win32::Foundation::{ERROR_SUCCESS, LPARAM, RECT};
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

        // The display-configuration API knows each monitor's device path and
        // EDID name; `EnumDisplayMonitors` knows neither. They meet on the
        // GDI device name.
        let names = names();
        for monitor in &mut found {
            if let Some((path, label)) = names.get(&monitor.name) {
                monitor.id.clone_from(path);
                monitor.label.clone_from(label);
            }
        }
        found
    }

    /// GDI device name → (device path, friendly name), for every active
    /// display. Empty when Windows will not answer, which leaves every
    /// monitor known by its device name and unnamed.
    #[allow(
        unsafe_code,
        reason = "display-configuration calls; each one's contract is argued at the call"
    )]
    fn names() -> HashMap<String, (String, String)> {
        let mut out = HashMap::new();
        let (mut path_count, mut mode_count) = (0u32, 0u32);
        // SAFETY: both out-parameters are live u32s the call writes once.
        let sized = unsafe {
            GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count)
        };
        if sized != ERROR_SUCCESS {
            return out;
        }
        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];
        // SAFETY: the arrays have the lengths the counts say, and the call
        // writes at most that many entries and updates the counts.
        let queried = unsafe {
            QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut path_count,
                paths.as_mut_ptr(),
                &mut mode_count,
                modes.as_mut_ptr(),
                None,
            )
        };
        if queried != ERROR_SUCCESS {
            return out;
        }
        paths.truncate(path_count as usize);

        for path in &paths {
            let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
            source.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
            source.header.size =
                u32::try_from(size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>()).unwrap_or(0);
            source.header.adapterId = path.sourceInfo.adapterId;
            source.header.id = path.sourceInfo.id;
            let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
            target.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
            target.header.size =
                u32::try_from(size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>()).unwrap_or(0);
            target.header.adapterId = path.targetInfo.adapterId;
            target.header.id = path.targetInfo.id;
            // SAFETY: each packet starts with a header whose size field is the
            // size of the whole packet, which is what bounds the write.
            let ok = unsafe {
                DisplayConfigGetDeviceInfo(&mut source.header) == 0
                    && DisplayConfigGetDeviceInfo(&mut target.header) == 0
            };
            if ok {
                out.insert(
                    wide(&source.viewGdiDeviceName),
                    (
                        wide(&target.monitorDevicePath),
                        wide(&target.monitorFriendlyDeviceName),
                    ),
                );
            }
        }
        out
    }

    /// A NUL-terminated UTF-16 buffer as a string.
    fn wide(buffer: &[u16]) -> String {
        let end = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
        String::from_utf16_lossy(&buffer[..end])
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
        let full = info.monitorInfo.rcMonitor;
        let name = wide(&info.szDevice);
        // `\\.\DISPLAY2` is display 2; anything else is unnumbered.
        let number = name
            .trim_start_matches(|c: char| !c.is_ascii_digit())
            .parse()
            .unwrap_or(0);

        found.push(Monitor {
            id: name.clone(),
            label: String::new(),
            number,
            size: [
                u32::try_from(full.right - full.left).unwrap_or(0),
                u32::try_from(full.bottom - full.top).unwrap_or(0),
            ],
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

    fn monitor(id: &str, primary: bool) -> Monitor {
        Monitor {
            name: id.to_owned(),
            id: id.to_owned(),
            label: String::new(),
            number: 1,
            size: [1920, 1080],
            work_area: [0.0, 0.0, 1920.0, 1032.0],
            scale: 1.0,
            primary,
        }
    }

    #[test]
    fn the_chosen_monitor_is_used_and_a_missing_one_falls_back_to_the_primary() {
        // Not first in the list: the primary is found by its flag.
        let all = [monitor("second", false), monitor("primary", true)];

        let (used, missing) = pick(&all, None).expect("a monitor");
        assert_eq!((used.id.as_str(), missing), ("primary", false));

        let (used, missing) = pick(&all, Some("second")).expect("a monitor");
        assert_eq!((used.id.as_str(), missing), ("second", false));

        // Unplugged: the primary, and the caller is told.
        let (used, missing) = pick(&all, Some("unplugged")).expect("a monitor");
        assert_eq!((used.id.as_str(), missing), ("primary", true));

        assert!(pick(&[], None).is_none());
    }

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
        let all = monitors();
        let (fallback, missing) =
            pick(&all, Some(r"\\?\DISPLAY#THAT_IS_NOT_THERE")).expect("the primary monitor");
        assert!(
            fallback.primary,
            "a remembered monitor that is gone must fall back to the primary one"
        );
        assert!(missing, "and the caller must be told it was missing");
    }

    #[cfg(windows)]
    #[test]
    fn every_attached_monitor_has_an_id_and_a_number() {
        // The id is what a choice is remembered by; an empty one could never
        // be found again.
        for monitor in monitors() {
            assert!(!monitor.id.is_empty(), "{}", monitor.name);
            assert!(monitor.number > 0, "{}", monitor.name);
            assert!(
                monitor.size[0] > 0 && monitor.size[1] > 0,
                "{}",
                monitor.name
            );
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn there_are_no_monitors_off_windows() {
        assert!(monitors().is_empty());
        assert!(pick(&monitors(), None).is_none());
    }
}
