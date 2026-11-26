//! Which Windows subsystem the built exe asks for (BUG-0013).
//!
//! A console-subsystem exe gets a console window from Windows every time it is
//! started, so a tray app built that way shows a black window for as long as it
//! runs. The subsystem is a field in the PE header, set by the linker from
//! `#![windows_subsystem]`; nothing at runtime can observe it from inside, so
//! the test reads it out of the file.
//!
//! A release build is the GUI subsystem. A debug build stays a console program
//! on purpose, so `cargo run` shows the log and the `--help` text.

use std::fs;

const SUBSYSTEM_WINDOWS_GUI: u16 = 2;
const SUBSYSTEM_WINDOWS_CUI: u16 = 3;

/// The `Subsystem` field of the optional header: the file's `e_lfanew` at
/// 0x3C gives the PE header, and the field sits 0x5C bytes after it.
fn subsystem(exe: &[u8]) -> u16 {
    let at = |offset: usize, len: usize| -> &[u8] { &exe[offset..offset + len] };
    let pe = u32::from_le_bytes(at(0x3C, 4).try_into().expect("four bytes")) as usize;
    assert_eq!(at(pe, 4), b"PE\0\0", "not a PE file");
    u16::from_le_bytes(at(pe + 0x5C, 2).try_into().expect("two bytes"))
}

#[cfg(windows)]
#[test]
fn a_release_exe_has_no_console_and_a_debug_exe_keeps_one() {
    let exe = fs::read(env!("CARGO_BIN_EXE_OmniToaster")).expect("the built exe");
    let expected = if cfg!(debug_assertions) {
        SUBSYSTEM_WINDOWS_CUI
    } else {
        SUBSYSTEM_WINDOWS_GUI
    };
    assert_eq!(
        subsystem(&exe),
        expected,
        "2 is the GUI subsystem (no console), 3 the console one"
    );
}
