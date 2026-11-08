//! One OmniToaster at a time, per role.
//!
//! A second launch must not produce a second tray icon and a second scheduler
//! firing the same reminders twice (`docs/specs/tray-and-settings.md` §5). The
//! guard is a named mutex: held for the process's lifetime, released by the
//! kernel when the process ends — including when it is killed, which a
//! lock file could not promise.
//!
//! The supervisor and its worker are both `OmniToaster.exe` and both take a
//! guard, so they must not collide on the name. [`Role`] is what keeps them
//! apart.

/// Which process is asking. The supervisor and the worker run from the same
/// executable and must not block each other (`docs/specs/supervisor.md` §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Supervisor,
    Worker,
}

impl Role {
    /// The mutex name for this role.
    ///
    /// `Local\` scopes it to the session, so two users logged into the same
    /// machine each get their own instance — a `Global\` name would let one
    /// user's running copy lock another out.
    pub fn mutex_name(self) -> &'static str {
        match self {
            Self::Supervisor => r"Local\OmniToaster.Supervisor",
            Self::Worker => r"Local\OmniToaster.Worker",
        }
    }
}

/// Why the guard could not be taken.
#[derive(Debug, thiserror::Error)]
pub enum InstanceError {
    /// Another process in this session already holds it. The caller should
    /// signal the running instance and exit 0, not treat this as a crash.
    #[error("another OmniToaster is already running in this session")]
    AlreadyRunning,

    #[error("the single-instance mutex could not be created: {0}")]
    Unavailable(String),
}

/// Proof that this process is the only one of its role.
///
/// Held for as long as the process should stay single. Dropping it releases
/// the claim, so it is kept alive in `main` rather than bound to `_`.
#[derive(Debug)]
pub struct InstanceGuard {
    role: Role,
    inner: platform::Handle,
}

impl InstanceGuard {
    /// Claim the single-instance slot for `role`.
    pub fn acquire(role: Role) -> Result<Self, InstanceError> {
        platform::acquire(role).map(|inner| Self { role, inner })
    }

    /// The role this guard holds.
    pub fn role(&self) -> Role {
        self.role
    }
}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        platform::release(&mut self.inner);
    }
}

#[cfg(windows)]
mod platform {
    use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::core::HSTRING;

    use super::{InstanceError, Role};

    #[derive(Debug)]
    pub(super) struct Handle(HANDLE);

    #[allow(
        unsafe_code,
        reason = "CreateMutexW, GetLastError and CloseHandle are Win32 calls; each contract is argued at the call"
    )]
    pub(super) fn acquire(role: Role) -> Result<Handle, InstanceError> {
        let name = HSTRING::from(role.mutex_name());

        // SAFETY: `name` outlives the call and is a valid null-terminated
        // wide string. A null security descriptor gives the default, and
        // `false` means we do not need initial ownership — creating it is
        // enough to claim the name.
        let handle = unsafe { CreateMutexW(None, false, &name) }
            .map_err(|e| InstanceError::Unavailable(e.to_string()))?;

        // CreateMutexW succeeds and returns the existing handle when the name
        // is taken, so the error code is the only way to tell the two apart.
        // SAFETY: called immediately after the API that set it, on this thread.
        let already = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;

        if already {
            // Close our handle to the existing mutex: holding it would keep
            // the name alive after the first instance exits.
            // SAFETY: `handle` was just returned by CreateMutexW and is not
            // used again.
            let _ = unsafe { CloseHandle(handle) };
            return Err(InstanceError::AlreadyRunning);
        }

        Ok(Handle(handle))
    }

    #[allow(
        unsafe_code,
        reason = "CloseHandle is a Win32 call; the handle is owned and closed once"
    )]
    pub(super) fn release(handle: &mut Handle) {
        if handle.0.is_invalid() {
            return;
        }
        // SAFETY: the handle came from CreateMutexW in `acquire`, is owned by
        // this guard, and is invalidated below so a second drop is a no-op.
        let _ = unsafe { CloseHandle(handle.0) };
        handle.0 = HANDLE::default();
    }
}

#[cfg(not(windows))]
mod platform {
    use std::collections::HashSet;
    use std::sync::{Mutex, OnceLock};

    use super::{InstanceError, Role};

    /// A process-local stand-in so the rest of the workspace compiles and
    /// tests elsewhere. It cannot see other processes, which is exactly why
    /// OmniToaster is Windows-only for now
    /// (`docs/01-vision-and-scope.md`).
    fn held() -> &'static Mutex<HashSet<&'static str>> {
        static HELD: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();
        HELD.get_or_init(|| Mutex::new(HashSet::new()))
    }

    #[derive(Debug)]
    pub(super) struct Handle(&'static str);

    pub(super) fn acquire(role: Role) -> Result<Handle, InstanceError> {
        let name = role.mutex_name();
        let mut set = held().lock().unwrap_or_else(|p| p.into_inner());
        if !set.insert(name) {
            return Err(InstanceError::AlreadyRunning);
        }
        Ok(Handle(name))
    }

    pub(super) fn release(handle: &mut Handle) {
        let mut set = held().lock().unwrap_or_else(|p| p.into_inner());
        set.remove(handle.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_roles_use_different_names() {
        // If they collided, the supervisor would lock its own worker out and
        // the app could never start (docs/specs/supervisor.md §3).
        assert_ne!(
            Role::Supervisor.mutex_name(),
            Role::Worker.mutex_name(),
            "the supervisor and the worker would block each other"
        );
    }

    #[test]
    fn the_names_are_session_scoped() {
        // Global\ would let one logged-in user's copy lock another user out.
        for role in [Role::Supervisor, Role::Worker] {
            let name = role.mutex_name();
            assert!(
                name.starts_with(r"Local\"),
                "{name} is not session-scoped; a Global name locks out other users"
            );
            assert!(name.contains("OmniToaster"), "{name} is not identifiable");
        }
    }

    #[test]
    fn a_second_claim_of_the_same_role_is_refused_and_a_release_frees_it() {
        let first = InstanceGuard::acquire(Role::Supervisor).expect("the first claim succeeds");
        assert_eq!(first.role(), Role::Supervisor);

        let second = InstanceGuard::acquire(Role::Supervisor);
        assert!(
            matches!(second, Err(InstanceError::AlreadyRunning)),
            "a second claim should be refused, got {second:?}"
        );

        // Dropping must free the name, or a restart after a clean exit would
        // be locked out of its own slot.
        drop(first);
        let third = InstanceGuard::acquire(Role::Supervisor);
        assert!(
            third.is_ok(),
            "the name should be free after the guard is dropped, got {third:?}"
        );
    }

    #[test]
    fn the_worker_is_not_blocked_by_the_supervisor() {
        // The real arrangement: both run at once, from the same executable.
        let supervisor = InstanceGuard::acquire(Role::Worker).expect("worker claim");
        let other = InstanceGuard::acquire(Role::Supervisor);
        assert!(
            other.is_ok(),
            "the supervisor must be able to run alongside its worker, got {other:?}"
        );
        drop(supervisor);
    }

    #[test]
    fn already_running_reads_as_advice_rather_than_a_crash() {
        // The caller is expected to exit 0 on this, so the message must not
        // sound like a failure.
        let rendered = InstanceError::AlreadyRunning.to_string();
        assert!(rendered.contains("already running"), "{rendered}");
    }
}
