//! "Start with Windows", and the only registry write OmniToaster makes.
//!
//! A single `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` value, written
//! only when the user turns it on (`docs/specs/tray-and-settings.md` §7). HKCU
//! needs no administrator rights, and per-user is what a portable app wants —
//! ADR-004 allows this one exception to "nothing outside the data folder".
//!
//! Two rules that are easy to get wrong:
//!
//! - **Refresh on every launch.** The value stores an absolute path. Move or
//!   rename the folder and the old path is dead, so a portable app that only
//!   wrote it once would silently stop starting.
//! - **Read the registry, never the settings file.** If the user removes the
//!   value by other means — msconfig, Task Manager's Startup tab, a cleaner —
//!   the toggle must show off. Trusting `settings.toml` would show on and lie.

use std::path::{Path, PathBuf};

/// The `Run` key, relative to HKCU.
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// The value name. Also what a user sees in Task Manager's Startup tab.
pub const VALUE_NAME: &str = "OmniToaster";

/// Why the autostart entry could not be read or changed.
#[derive(Debug, thiserror::Error)]
pub enum AutostartError {
    #[error("the executable's location could not be determined: {0}")]
    ExeLocationUnknown(String),

    #[error("the Run key could not be opened: {0}")]
    KeyUnavailable(String),

    #[error("the autostart value could not be written: {0}")]
    WriteFailed(String),

    #[error("the autostart value could not be removed: {0}")]
    RemoveFailed(String),
}

/// What the registry currently says, which is the only thing the UI should
/// trust.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Autostart {
    /// No value. The toggle shows off.
    Disabled,
    /// A value pointing at this executable. The toggle shows on.
    Enabled,
    /// A value pointing somewhere else — the folder was moved, or another
    /// copy wrote it. The toggle shows on, and the next launch rewrites it.
    EnabledElsewhere { command: String },
}

impl Autostart {
    /// Whether the toggle should read as on.
    pub fn is_on(&self) -> bool {
        !matches!(self, Self::Disabled)
    }
}

/// The command line the `Run` value should hold for `exe`.
///
/// Quoted, because `C:\Program Files\...` without quotes is read by Windows as
/// `C:\Program` with arguments. `--worker` is deliberately absent: what starts
/// at login is the supervisor, which spawns the worker itself
/// (`docs/specs/supervisor.md` §3).
pub fn command_for(exe: &Path) -> String {
    format!("\"{}\"", exe.display())
}

/// This executable's path.
fn current_exe() -> Result<PathBuf, AutostartError> {
    std::env::current_exe().map_err(|e| AutostartError::ExeLocationUnknown(e.to_string()))
}

/// Read the current state from the registry.
pub fn status() -> Result<Autostart, AutostartError> {
    let exe = current_exe()?;
    status_of(&command_for(&exe))
}

/// Read the state, comparing against `expected` rather than the real exe.
/// Split out so tests can drive it without pretending to be another binary.
fn status_of(expected: &str) -> Result<Autostart, AutostartError> {
    match platform::read_value(VALUE_NAME)? {
        None => Ok(Autostart::Disabled),
        Some(found) if paths_match(&found, expected) => Ok(Autostart::Enabled),
        Some(found) => Ok(Autostart::EnabledElsewhere { command: found }),
    }
}

/// Compare two command strings forgivingly.
///
/// Windows paths are case-insensitive, and the value may or may not be quoted
/// depending on who wrote it. Treating `"C:\A\x.exe"` and `C:\a\X.EXE` as
/// different would make every launch rewrite the value for no reason.
fn paths_match(a: &str, b: &str) -> bool {
    fn normalise(s: &str) -> String {
        s.trim().trim_matches('"').replace('/', "\\").to_lowercase()
    }
    normalise(a) == normalise(b)
}

/// Turn autostart on, or refresh the path if it has moved.
///
/// Safe to call on every launch: it writes only when the value is absent or
/// wrong, so a normal start touches nothing.
pub fn enable() -> Result<(), AutostartError> {
    let exe = current_exe()?;
    let want = command_for(&exe);

    if matches!(status_of(&want)?, Autostart::Enabled) {
        return Ok(());
    }
    platform::write_value(VALUE_NAME, &want)
}

/// Turn autostart off by removing the value. Removing one that is not there
/// succeeds: the caller asked for "off", and off is the result.
pub fn disable() -> Result<(), AutostartError> {
    platform::delete_value(VALUE_NAME)
}

/// Refresh the stored path if autostart is on and the exe has moved.
///
/// Called at startup. Does nothing when autostart is off — enabling it is the
/// user's choice, never a side effect of launching.
pub fn refresh_if_enabled() -> Result<bool, AutostartError> {
    let exe = current_exe()?;
    let want = command_for(&exe);

    match status_of(&want)? {
        Autostart::Disabled | Autostart::Enabled => Ok(false),
        Autostart::EnabledElsewhere { .. } => {
            platform::write_value(VALUE_NAME, &want)?;
            Ok(true)
        }
    }
}

#[cfg(windows)]
mod platform {
    use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
    use windows::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ,
        RRF_RT_REG_SZ, RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegGetValueW, RegOpenKeyExW,
        RegSetValueExW,
    };
    use windows::core::HSTRING;

    use super::{AutostartError, RUN_KEY};

    /// Open the Run key for reading, or `None` when the profile has no such
    /// key.
    ///
    /// Not hypothetical: a service account's profile can have
    /// `…\CurrentVersion` with no `Run` subkey, and the CI runner — which
    /// runs as LocalSystem — is exactly such an account. A missing key means
    /// nothing is set to start, which is "off", not a failure.
    #[allow(
        unsafe_code,
        reason = "a Win32 registry call; the contract is argued at the call"
    )]
    fn open_for_read(key_path: &str) -> Result<Option<HKEY>, AutostartError> {
        let path = HSTRING::from(key_path);
        let mut key = HKEY::default();

        // SAFETY: `path` outlives the call; `key` is a live out-parameter the
        // API fills with a handle we close below.
        let status =
            unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, &path, Some(0), KEY_READ, &mut key) };

        if status == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        if status.is_err() {
            return Err(AutostartError::KeyUnavailable(format!("{status:?}")));
        }
        Ok(Some(key))
    }

    /// Open the Run key for writing **without** creating it.
    ///
    /// Deleting a value must not bring the key into existence on a profile
    /// that never had one.
    #[allow(
        unsafe_code,
        reason = "a Win32 registry call; the contract is argued at the call"
    )]
    fn open_for_read_write(key_path: &str) -> Result<Option<HKEY>, AutostartError> {
        let path = HSTRING::from(key_path);
        let mut key = HKEY::default();

        // SAFETY: as open_for_read, with write access added.
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                &path,
                Some(0),
                KEY_READ | KEY_WRITE,
                &mut key,
            )
        };
        if status == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        if status.is_err() {
            return Err(AutostartError::KeyUnavailable(format!("{status:?}")));
        }
        Ok(Some(key))
    }

    /// Open the Run key for writing, creating it if the profile lacks one.
    ///
    /// `RegCreateKeyExW` opens an existing key unchanged and creates it
    /// otherwise, so enabling autostart works on a profile that has never
    /// held a startup entry.
    #[allow(
        unsafe_code,
        reason = "a Win32 registry call; the contract is argued at the call"
    )]
    fn open_for_write(key_path: &str) -> Result<HKEY, AutostartError> {
        let path = HSTRING::from(key_path);
        let mut key = HKEY::default();

        // SAFETY: `path` outlives the call and `key` is a live out-parameter.
        // A null class, the default options and no security attributes are
        // the documented defaults; the disposition is not needed.
        let status = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                &path,
                None,
                None,
                REG_OPTION_NON_VOLATILE,
                KEY_READ | KEY_WRITE,
                None,
                &mut key,
                None,
            )
        };
        if status.is_err() {
            return Err(AutostartError::KeyUnavailable(format!("{status:?}")));
        }
        Ok(key)
    }

    #[allow(unsafe_code, reason = "RegCloseKey is a Win32 call on a handle we own")]
    fn close(key: HKEY) {
        // SAFETY: `key` came from RegOpenKeyExW above and is not used after.
        let _ = unsafe { RegCloseKey(key) };
    }

    #[allow(
        unsafe_code,
        reason = "RegGetValueW is a Win32 call; the buffer protocol is argued at the call"
    )]
    pub(super) fn read_value_in(
        key_path: &str,
        name: &str,
    ) -> Result<Option<String>, AutostartError> {
        let Some(key) = open_for_read(key_path)? else {
            return Ok(None);
        };
        let value = HSTRING::from(name);
        let mut size: u32 = 0;

        // SAFETY: a null data pointer with a live size out-parameter is the
        // documented way to ask how many bytes the value needs.
        let status = unsafe {
            RegGetValueW(
                key,
                None,
                &value,
                RRF_RT_REG_SZ,
                None,
                None,
                Some(&mut size),
            )
        };

        if status == ERROR_FILE_NOT_FOUND {
            close(key);
            return Ok(None);
        }
        if status.is_err() {
            close(key);
            return Err(AutostartError::KeyUnavailable(format!("{status:?}")));
        }

        // size is in bytes, and the value is UTF-16.
        let mut buffer = vec![0u16; (size as usize).div_ceil(2)];
        let mut got = size;

        // SAFETY: `buffer` is live and `got` says how many bytes it holds,
        // which is what bounds the write.
        let status = unsafe {
            RegGetValueW(
                key,
                None,
                &value,
                RRF_RT_REG_SZ,
                None,
                Some(buffer.as_mut_ptr().cast()),
                Some(&mut got),
            )
        };
        close(key);

        if status.is_err() {
            return Err(AutostartError::KeyUnavailable(format!("{status:?}")));
        }

        let text = String::from_utf16_lossy(&buffer);
        Ok(Some(text.trim_end_matches('\0').to_string()))
    }

    #[allow(
        unsafe_code,
        reason = "RegSetValueExW is a Win32 call; the buffer is a live UTF-16 string"
    )]
    pub(super) fn write_value_in(
        key_path: &str,
        name: &str,
        command: &str,
    ) -> Result<(), AutostartError> {
        let key = open_for_write(key_path)?;
        let value = HSTRING::from(name);

        // REG_SZ wants the terminating null included in the byte count.
        let mut wide: Vec<u16> = command.encode_utf16().collect();
        wide.push(0);
        let bytes = unsafe {
            std::slice::from_raw_parts(wide.as_ptr().cast::<u8>(), std::mem::size_of_val(&wide[..]))
        };

        // SAFETY: `bytes` borrows `wide`, which outlives the call, and its
        // length is exactly the buffer's size in bytes.
        let status = unsafe { RegSetValueExW(key, &value, Some(0), REG_SZ, Some(bytes)) };
        close(key);

        if status.is_err() {
            return Err(AutostartError::WriteFailed(format!("{status:?}")));
        }
        Ok(())
    }

    #[allow(
        unsafe_code,
        reason = "RegDeleteValueW is a Win32 call on a key we own"
    )]
    pub(super) fn delete_value_in(key_path: &str, name: &str) -> Result<(), AutostartError> {
        // No key means no value, and the caller asked for "off".
        let Some(key) = open_for_read_write(key_path)? else {
            return Ok(());
        };
        let value = HSTRING::from(name);

        // SAFETY: `value` outlives the call; `key` is open for writing.
        let status = unsafe { RegDeleteValueW(key, &value) };
        close(key);

        // Already absent is success: the caller asked for off.
        if status.is_err() && status != ERROR_FILE_NOT_FOUND {
            return Err(AutostartError::RemoveFailed(format!("{status:?}")));
        }
        Ok(())
    }

    pub(super) fn read_value(name: &str) -> Result<Option<String>, AutostartError> {
        read_value_in(RUN_KEY, name)
    }

    pub(super) fn write_value(name: &str, command: &str) -> Result<(), AutostartError> {
        write_value_in(RUN_KEY, name, command)
    }

    pub(super) fn delete_value(name: &str) -> Result<(), AutostartError> {
        delete_value_in(RUN_KEY, name)
    }
}

#[cfg(not(windows))]
mod platform {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    use super::AutostartError;

    /// An in-memory stand-in so the workspace builds and the logic above can
    /// be tested anywhere. There is no Run key off Windows.
    fn store() -> &'static Mutex<HashMap<String, String>> {
        static STORE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
        STORE.get_or_init(|| Mutex::new(HashMap::new()))
    }

    pub(super) fn read_value(name: &str) -> Result<Option<String>, AutostartError> {
        Ok(store()
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(name)
            .cloned())
    }

    pub(super) fn write_value(name: &str, command: &str) -> Result<(), AutostartError> {
        store()
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(name.to_string(), command.to_string());
        Ok(())
    }

    pub(super) fn delete_value(name: &str) -> Result<(), AutostartError> {
        store()
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(name);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_command_is_quoted() {
        // Unquoted, Windows reads C:\Program Files\... as C:\Program with
        // arguments, and the app silently never starts.
        let cmd = command_for(Path::new(r"C:\Program Files\OmniToaster\OmniToaster.exe"));
        assert!(cmd.starts_with('"') && cmd.ends_with('"'), "{cmd}");
        assert!(cmd.contains("OmniToaster.exe"), "{cmd}");
    }

    #[test]
    fn the_command_does_not_ask_for_the_worker() {
        // What starts at login is the supervisor; it spawns the worker.
        let cmd = command_for(Path::new(r"D:\OmniToaster\OmniToaster.exe"));
        assert!(!cmd.contains("--worker"), "{cmd}");
    }

    #[test]
    fn paths_compare_case_insensitively_and_ignore_quoting() {
        // Otherwise every launch would see a mismatch and rewrite the value.
        assert!(paths_match(
            r#""C:\Apps\OmniToaster\OmniToaster.exe""#,
            r"c:\apps\omnitoaster\omnitoaster.exe"
        ));
        assert!(paths_match(r"C:\A\x.exe", r#"  "C:\A\x.exe"  "#));
        assert!(!paths_match(r"C:\A\x.exe", r"C:\B\x.exe"));
    }

    #[test]
    fn a_moved_exe_reads_as_enabled_elsewhere() {
        // This is the case that makes refresh_if_enabled necessary: the value
        // is present but points at a folder that no longer exists.
        let state = Autostart::EnabledElsewhere {
            command: r#""D:\Old\OmniToaster.exe""#.to_string(),
        };
        assert!(state.is_on(), "a stale path still means the toggle is on");
        assert!(!Autostart::Disabled.is_on());
        assert!(Autostart::Enabled.is_on());
    }

    #[test]
    fn the_value_name_is_the_one_a_user_will_see() {
        // It shows in Task Manager's Startup tab, so it is user-facing.
        assert_eq!(VALUE_NAME, "OmniToaster");
    }

    /// Round-trip a value against the **real** HKCU Run key.
    ///
    /// Everything above tests the logic; this is the only test that proves the
    /// Win32 calls work at all. It uses a run-scoped value name and removes it
    /// in every exit path — never `VALUE_NAME`, because running the suite must
    /// not disable a developer's own autostart.
    #[cfg(windows)]
    #[test]
    fn the_registry_round_trips_a_value_and_leaves_nothing_behind() {
        let name = format!("OmniToaster-test-{}", std::process::id());
        let command = r#""D:\scratch\OmniToaster.exe""#;

        assert_ne!(name, VALUE_NAME, "a test must never touch the real value");

        // Absent to begin with, and absence is not an error.
        assert_eq!(
            super::platform::read_value(&name).expect("read an absent value"),
            None
        );

        super::platform::write_value(&name, command).expect("write");
        let read_back = super::platform::read_value(&name).expect("read");
        assert_eq!(
            read_back.as_deref(),
            Some(command),
            "the value did not survive the round trip intact"
        );

        super::platform::delete_value(&name).expect("delete");
        assert_eq!(
            super::platform::read_value(&name).expect("read after delete"),
            None,
            "the test value was left behind in the user's Run key"
        );

        // Deleting what is already gone is success, not an error.
        super::platform::delete_value(&name).expect("a second delete is a no-op");
    }

    /// A profile whose `Run` key does not exist must read as **off**, not as
    /// an error — and enabling must create the key.
    ///
    /// This is the bug CI caught and a local run could not: the Forgejo
    /// runner is a service running as LocalSystem, whose profile has
    /// `…\CurrentVersion` with no `Run` subkey, so every registry call
    /// failed there while passing on a normal account. Pointing the helpers
    /// at a key that is guaranteed absent reproduces it as any user.
    #[cfg(windows)]
    #[test]
    fn a_missing_key_reads_as_off_and_enabling_creates_it() {
        let absent = format!(r"Software\OmniToaster-test-{}\Run", std::process::id());
        let name = "Probe";

        // Absent key: not an error, just nothing set.
        assert_eq!(
            super::platform::read_value_in(&absent, name).expect("reading a missing key"),
            None,
            "a profile with no Run key should read as off, not fail"
        );

        // And removing from a key that is not there is success, not an error,
        // and must not bring the key into existence.
        super::platform::delete_value_in(&absent, name).expect("deleting from a missing key");
        assert_eq!(
            super::platform::read_value_in(&absent, name).expect("still missing"),
            None,
            "the delete created the key it was meant to leave alone"
        );

        // Writing creates the key, so enabling autostart works on a profile
        // that has never held a startup entry.
        super::platform::write_value_in(&absent, name, "probe").expect("write creates the key");
        assert_eq!(
            super::platform::read_value_in(&absent, name).expect("read back"),
            Some("probe".to_string())
        );

        super::platform::delete_value_in(&absent, name).expect("clean the value");

        // Remove the key this test created, so nothing is left in the hive.
        let parent = format!(r"Software\OmniToaster-test-{}", std::process::id());
        let _ = std::process::Command::new("reg")
            .args(["delete", &format!(r"HKCU\{parent}"), "/f"])
            .output();
    }

    #[test]
    fn the_run_key_is_per_user_and_not_the_machine_wide_one() {
        // HKLM would need administrator rights and would affect every user,
        // which a portable app must not do (ADR-004).
        assert_eq!(
            RUN_KEY, r"Software\Microsoft\Windows\CurrentVersion\Run",
            "the key must be the HKCU Run key"
        );
        assert!(!RUN_KEY.to_lowercase().contains("wow6432"));
    }
}
