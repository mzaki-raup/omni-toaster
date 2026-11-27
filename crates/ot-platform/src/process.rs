//! Starting another program without a console flashing (FEAT-0079).
//!
//! Every child process OmniToaster starts is spawned with `CREATE_NO_WINDOW`
//! (`docs/specs/supervisor.md` §7), and through this one function, never a bare
//! `Command::new` for an internal command (`.claude/rules/threads-and-scheduling.md`).

use std::ffi::OsStr;
use std::io;
use std::os::windows::process::CommandExt as _;
use std::path::Path;
use std::process::{Child, Command, Stdio};

/// `CREATE_NO_WINDOW`: the child gets no console window of its own.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Start `exe` with `args`, detached from this process's console and with its
/// standard streams closed, and return the child.
pub fn spawn_hidden<I, S>(exe: &Path, args: I) -> io::Result<Child>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new(exe)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_program_is_started_and_its_exit_is_seen() {
        // `cmd /c exit 7` is on every Windows machine.
        let cmd = std::env::var_os("ComSpec").unwrap_or_else(|| "cmd.exe".into());
        let mut child = spawn_hidden(Path::new(&cmd), ["/c", "exit", "7"]).expect("starts");
        assert_eq!(child.wait().expect("waits").code(), Some(7));
    }

    #[test]
    fn a_program_that_does_not_exist_is_an_error_not_a_panic() {
        assert!(spawn_hidden(Path::new(r"D:\no\such\program.exe"), ["x"]).is_err());
    }
}
