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
}

#[cfg(not(windows))]
mod platform {
    /// Omni Toaster is Windows-only for now. The stub exists so every other
    /// crate still compiles and `cargo clippy` passes on Linux, which is what
    /// keeps the eventual macOS and Linux work possible
    /// (`docs/01-vision-and-scope.md`).
    pub(super) fn user_locale() -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::user_locale;

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
