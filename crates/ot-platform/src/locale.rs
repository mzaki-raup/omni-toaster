//! The Windows display language.
//!
//! `GetUserDefaultLocaleName` is the native answer and is reliable on Windows
//! (ADR-006 chose it over the `sys-locale` crate for that reason). Reading it
//! is platform code, which is why it lives here and not in `ot-i18n`
//! (`docs/03-architecture.md`).

/// The user's display locale as a BCP-47 name, for example `en-GB` or `ms-MY`.
///
/// Returns `None` when the locale cannot be determined, which the caller
/// should treat as "use the documented default", not as an error worth showing
/// anyone.
pub fn user_locale() -> Option<String> {
    platform::user_locale()
}

/// What Windows' regional settings say about writing a date and a time
/// (`docs/10-i18n.md`: English dates follow them). Each part is `None` when
/// Windows cannot say, and the caller uses its documented default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Regional {
    /// The short date puts the month before the day (`m/d/yyyy`).
    pub month_first: Option<bool>,
    /// The time format is 24-hour.
    pub twenty_four_hour: Option<bool>,
}

/// Read the user's regional date order and clock.
pub fn regional() -> Regional {
    Regional {
        month_first: platform::short_date_pattern()
            .as_deref()
            .and_then(month_first),
        twenty_four_hour: platform::time_pattern()
            .as_deref()
            .and_then(twenty_four_hour),
    }
}

/// Whether a Windows short-date pattern (`M/d/yyyy`, `dd/MM/yyyy`) puts the
/// month first. Quoted literals are skipped; a year-first pattern counts as
/// not month-first.
pub fn month_first(pattern: &str) -> Option<bool> {
    first_field(pattern, &['d', 'M', 'y']).map(|field| field == 'M')
}

/// Whether a Windows time pattern (`h:mm tt`, `HH:mm`) is 24-hour.
pub fn twenty_four_hour(pattern: &str) -> Option<bool> {
    first_field(pattern, &['h', 'H']).map(|field| field == 'H')
}

fn first_field(pattern: &str, fields: &[char]) -> Option<char> {
    let mut quoted = false;
    pattern.chars().find(|c| {
        if *c == '\'' {
            quoted = !quoted;
            return false;
        }
        !quoted && fields.contains(c)
    })
}

#[cfg(windows)]
mod platform {
    use windows::Win32::Globalization::GetUserDefaultLocaleName;

    /// `LOCALE_NAME_MAX_LENGTH` from `winnls.h`. The `windows` crate does not
    /// re-export it, and it has been 85 since Vista; the buffer is sized from
    /// this rather than guessed.
    const LOCALE_NAME_MAX_LENGTH: usize = 85;

    // The one place in the workspace that calls a Win32 function, which is why
    // `ot-platform` exists at all. The workspace lints make `unsafe` an error
    // everywhere, so this is scoped to the single function that needs it
    // rather than allowed crate-wide (.claude/rules/rust.md).
    #[allow(
        unsafe_code,
        reason = "GetUserDefaultLocaleName is a Win32 call; the safety contract is argued below"
    )]
    pub(super) fn user_locale() -> Option<String> {
        // The API writes a null-terminated UTF-16 name and returns its length
        // including the terminator, or 0 on failure.
        let mut buffer = [0u16; LOCALE_NAME_MAX_LENGTH];

        // SAFETY: `buffer` is a live, writable array and the slice passed
        // carries its true length, which is what the API uses to bound the
        // write. It writes at most that many UTF-16 units and returns how many
        // it wrote, so nothing is read beyond what was initialised.
        let written = unsafe { GetUserDefaultLocaleName(&mut buffer) };

        if written <= 0 {
            tracing::warn!("GetUserDefaultLocaleName returned {written}; falling back");
            return None;
        }

        // Drop the trailing null before decoding.
        let units = (written as usize).saturating_sub(1).min(buffer.len());
        let name = String::from_utf16_lossy(&buffer[..units]);
        let name = name.trim_end_matches('\0').trim().to_string();

        if name.is_empty() { None } else { Some(name) }
    }

    pub(super) fn short_date_pattern() -> Option<String> {
        info(windows::Win32::Globalization::LOCALE_SSHORTDATE)
    }

    pub(super) fn time_pattern() -> Option<String> {
        info(windows::Win32::Globalization::LOCALE_STIMEFORMAT)
    }

    #[allow(
        unsafe_code,
        reason = "GetLocaleInfoEx is a Win32 call; the safety contract is argued below"
    )]
    fn info(kind: u32) -> Option<String> {
        let mut buffer = [0u16; 80];
        // SAFETY: the null name means the user's default locale; `buffer` is
        // a live, writable array and the slice carries its true length, which
        // bounds the write. The return is how many units were written,
        // including the terminator, or 0 on failure.
        let written = unsafe {
            windows::Win32::Globalization::GetLocaleInfoEx(
                windows::core::PCWSTR::null(),
                kind,
                Some(&mut buffer),
            )
        };
        if written <= 0 {
            return None;
        }
        let units = (written as usize).saturating_sub(1).min(buffer.len());
        let text = String::from_utf16_lossy(&buffer[..units]);
        (!text.is_empty()).then_some(text)
    }
}

#[cfg(not(windows))]
mod platform {
    /// OmniToaster is Windows-only for now. The stub exists so every other
    /// crate still compiles and `cargo clippy` passes on Linux, which is what
    /// keeps the eventual macOS and Linux work possible
    /// (`docs/01-vision-and-scope.md`).
    pub(super) fn user_locale() -> Option<String> {
        None
    }

    pub(super) fn short_date_pattern() -> Option<String> {
        None
    }

    pub(super) fn time_pattern() -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{month_first, regional, twenty_four_hour, user_locale};

    #[test]
    fn the_date_order_and_the_clock_are_read_from_the_patterns() {
        assert_eq!(month_first("M/d/yyyy"), Some(true));
        assert_eq!(month_first("dd/MM/yyyy"), Some(false));
        assert_eq!(month_first("d.M.yyyy"), Some(false));
        assert_eq!(
            month_first("yyyy-MM-dd"),
            Some(false),
            "year first is not month first"
        );
        assert_eq!(
            month_first("'Day' d 'of' MMMM"),
            Some(false),
            "quoted text is skipped"
        );
        assert_eq!(month_first(""), None);
        assert_eq!(twenty_four_hour("h:mm tt"), Some(false));
        assert_eq!(twenty_four_hour("HH:mm:ss"), Some(true));
        assert_eq!(twenty_four_hour("tt hh:mm"), Some(false));
        assert_eq!(twenty_four_hour("mm"), None);
    }

    #[test]
    fn this_machine_answers_both_on_windows() {
        let regional = regional();
        if cfg!(windows) {
            assert!(regional.month_first.is_some(), "{regional:?}");
            assert!(regional.twenty_four_hour.is_some(), "{regional:?}");
        }
    }

    #[test]
    fn the_locale_is_either_absent_or_a_plausible_bcp47_name() {
        // What the locale *is* depends on the machine, so this asserts the
        // shape rather than a value: a non-empty name, no interior null, and a
        // primary subtag of ASCII letters. On a non-Windows build it is None,
        // which is also acceptable.
        match user_locale() {
            None => {}
            Some(name) => {
                assert!(!name.is_empty(), "an empty name should have been None");
                assert!(!name.contains('\0'), "a null survived decoding: {name:?}");
                let primary = name.split(['-', '_']).next().unwrap_or_default();
                assert!(
                    !primary.is_empty() && primary.chars().all(|c| c.is_ascii_alphabetic()),
                    "{name:?} has no plausible primary subtag"
                );
            }
        }
    }
}
