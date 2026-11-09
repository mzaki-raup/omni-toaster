//! The fonts the app draws with, read from the Windows font folder.
//!
//! OmniToaster bundles no font (ADR-014): egui's `default_fonts` feature ships
//! the Ubuntu font under licences the project does not allow, and a Windows
//! app should be drawing in the Windows UI font anyway. The cost is that
//! **nothing renders until this load succeeds**, so a failure is a startup
//! error and not an empty window.
//!
//! The font folder is found with `GetWindowsDirectoryW` rather than from
//! `%WINDIR%`: an environment variable is the caller's to set, and a font file
//! is parsed, which is not something to point at an arbitrary path
//! (`.claude/rules/security.md`).

use std::path::PathBuf;

/// A font file that was found and read.
#[derive(Debug, Clone)]
pub struct Font {
    /// The file it came from, for the log line and the error message.
    pub path: PathBuf,
    pub bytes: Vec<u8>,
}

/// The candidates for the proportional UI font, best first.
///
/// Segoe UI is the Windows UI font from Vista onwards. Tahoma and Arial are
/// there for a stripped or very old image where it is missing; they look wrong
/// but they are legible, which beats a window of blank rectangles.
const PROPORTIONAL: &[&str] = &["segoeui.ttf", "tahoma.ttf", "arial.ttf"];

/// The candidates for the monospace font. egui's built-in widgets use the
/// monospace family, so an empty one leaves parts of the UI unreadable.
const MONOSPACE: &[&str] = &["consola.ttf", "cour.ttf"];

/// Read the proportional UI font.
pub fn proportional() -> Result<Font, FontError> {
    first_of(PROPORTIONAL)
}

/// Read the monospace font, falling back to the proportional one.
///
/// A proportional monospace family is wrong but harmless — it affects only
/// where egui chooses the monospace family — and it is better than failing to
/// start over a missing Consolas.
pub fn monospace() -> Result<Font, FontError> {
    match first_of(MONOSPACE) {
        Ok(font) => Ok(font),
        Err(_) => proportional(),
    }
}

fn first_of(candidates: &'static [&'static str]) -> Result<Font, FontError> {
    let folder = platform::font_folder().ok_or(FontError::FolderUnknown)?;
    for name in candidates {
        let path = folder.join(name);
        match std::fs::read(&path) {
            Ok(bytes) => return Ok(Font { path, bytes }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(FontError::Unreadable { path, error }),
        }
    }
    Err(FontError::NoneFound {
        folder,
        candidates: candidates.join(", "),
    })
}

/// Why no font could be loaded.
#[derive(Debug, thiserror::Error)]
pub enum FontError {
    #[error("the Windows folder could not be located, so the font folder is unknown")]
    FolderUnknown,
    #[error("none of these fonts are in {folder}: {candidates}")]
    NoneFound { folder: PathBuf, candidates: String },
    #[error("{path} could not be read: {error}")]
    Unreadable {
        path: PathBuf,
        error: std::io::Error,
    },
}

#[cfg(windows)]
mod platform {
    use std::path::PathBuf;

    use windows::Win32::System::SystemInformation::GetWindowsDirectoryW;

    /// `C:\Windows\Fonts`, wherever Windows actually is.
    #[allow(
        unsafe_code,
        reason = "GetWindowsDirectoryW is a Win32 call; the buffer contract is argued at the call"
    )]
    pub(super) fn font_folder() -> Option<PathBuf> {
        // MAX_PATH is the documented bound for this call.
        let mut buffer = [0u16; 260];

        // SAFETY: the buffer is live and `buffer.len()` is its true length in
        // u16s, which is what bounds the write. The call returns the number of
        // characters written, excluding the terminator, and 0 on failure.
        let written = unsafe { GetWindowsDirectoryW(Some(&mut buffer)) } as usize;
        if written == 0 || written > buffer.len() {
            return None;
        }
        Some(PathBuf::from(String::from_utf16_lossy(&buffer[..written])).join("Fonts"))
    }
}

#[cfg(not(windows))]
mod platform {
    use std::path::PathBuf;

    pub(super) fn font_folder() -> Option<PathBuf> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn the_font_folder_is_the_fonts_folder_under_windows() {
        let folder = platform::font_folder().expect("Windows is somewhere");
        assert_eq!(folder.file_name().expect("a last part"), "Fonts");
        assert!(
            folder.is_dir(),
            "{} is not a folder on this machine",
            folder.display()
        );
    }

    #[cfg(windows)]
    #[test]
    fn the_ui_font_loads_and_is_a_real_font_file() {
        let font = proportional().expect("a Windows machine has a UI font");
        assert!(
            font.bytes.len() > 10_000,
            "{} is {} bytes, which is too small to be a font",
            font.path.display(),
            font.bytes.len()
        );
        // TrueType is 0x00010000 and OpenType/CFF is "OTTO"; a TrueType
        // collection is "ttcf". Anything else is not something egui can parse,
        // and the symptom would be a window with no text rather than an error.
        let magic = &font.bytes[..4];
        assert!(
            magic == [0x00, 0x01, 0x00, 0x00] || magic == b"OTTO" || magic == b"ttcf",
            "{} does not start with a font signature: {magic:02x?}",
            font.path.display()
        );
    }

    #[cfg(windows)]
    #[test]
    fn the_monospace_font_falls_back_rather_than_failing() {
        // Whatever this machine has, the call must produce something: egui's
        // own widgets use the monospace family.
        let font = monospace().expect("the fallback chain ends at the UI font");
        assert!(!font.bytes.is_empty());
    }

    #[cfg(not(windows))]
    #[test]
    fn there_is_no_font_folder_off_windows() {
        assert!(matches!(proportional(), Err(FontError::FolderUnknown)));
    }
}
