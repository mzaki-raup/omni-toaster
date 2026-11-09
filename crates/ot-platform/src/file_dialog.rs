//! The Windows "Open" dialog, for choosing an image to import.
//!
//! `IFileOpenDialog` rather than a crate: it is one COM interface in the crate
//! that already owns every Windows call, and it brings no dependency tree
//! with it.
//!
//! **Blocking, and never to be called from the UI thread.** `Show` runs its
//! own modal message loop on the thread that calls it, so on the UI thread it
//! would freeze the settings window, the tray and every toast for as long as
//! the dialog is open. The caller runs it on a worker thread, which also gets
//! its own COM apartment here and gives it back afterwards.
//!
//! The extension filter is a convenience for the person browsing, not a
//! check: the import still decides what a file is from its magic bytes
//! (`docs/06-security-and-privacy.md`).

use std::path::PathBuf;

/// Why the dialog could not be shown. Cancelling is not an error: it is
/// `Ok(None)`.
#[derive(Debug, thiserror::Error)]
pub enum DialogError {
    #[error("the file dialog could not be opened: {0}")]
    Failed(String),
    #[error("this platform has no file dialog")]
    Unsupported,
}

/// Ask the user for one image file.
///
/// `title` and `filter_name` arrive already in the user's language; this
/// crate owns Windows, not words.
pub fn pick_image(title: &str, filter_name: &str) -> Result<Option<PathBuf>, DialogError> {
    platform::pick_image(title, filter_name)
}

#[cfg(windows)]
mod platform {
    use std::path::PathBuf;

    use windows::Win32::Foundation::ERROR_CANCELLED;
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoCreateInstance,
        CoInitializeEx, CoTaskMemFree, CoUninitialize,
    };
    use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
    use windows::Win32::UI::Shell::{
        FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM, FileOpenDialog, IFileOpenDialog, SIGDN_FILESYSPATH,
    };
    use windows::core::{HSTRING, PCWSTR};

    use super::DialogError;

    /// The patterns offered. Only a convenience: the import checks the bytes.
    const PATTERNS: &str = "*.png;*.jpg;*.jpeg;*.webp";

    /// Gives the apartment back when the dialog is done, however it ends.
    struct Apartment;

    impl Drop for Apartment {
        #[allow(
            unsafe_code,
            reason = "CoUninitialize pairs the CoInitializeEx that made this value"
        )]
        fn drop(&mut self) {
            // SAFETY: only constructed after CoInitializeEx succeeded on this
            // same thread, so this is its matching call.
            unsafe { CoUninitialize() };
        }
    }

    #[allow(
        unsafe_code,
        reason = "COM calls; each one's contract is argued at the call"
    )]
    pub(super) fn pick_image(
        title: &str,
        filter_name: &str,
    ) -> Result<Option<PathBuf>, DialogError> {
        let fail = |error: windows::core::Error| DialogError::Failed(error.to_string());

        // SAFETY: a plain initialization on a worker thread that has no
        // apartment yet; the guard below undoes it on every path.
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) }
            .ok()
            .map_err(fail)?;
        let _apartment = Apartment;

        // SAFETY: the class id and interface are the documented pair for the
        // Open dialog, created in-process on this thread's apartment.
        let dialog: IFileOpenDialog =
            unsafe { CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER) }
                .map_err(fail)?;

        let title = HSTRING::from(title);
        let filter_name = HSTRING::from(filter_name);
        let patterns = HSTRING::from(PATTERNS);
        let filters = [COMDLG_FILTERSPEC {
            pszName: PCWSTR(filter_name.as_ptr()),
            pszSpec: PCWSTR(patterns.as_ptr()),
        }];

        // SAFETY: every string outlives these calls, because the HSTRINGs
        // above live until the end of the function and the dialog copies
        // what it is given.
        unsafe {
            dialog.SetTitle(&title).map_err(fail)?;
            dialog.SetFileTypes(&filters).map_err(fail)?;
            let options = dialog.GetOptions().map_err(fail)?;
            dialog
                .SetOptions(options | FOS_FILEMUSTEXIST | FOS_FORCEFILESYSTEM)
                .map_err(fail)?;
        }

        // SAFETY: shown without an owner, which the API allows; it runs its
        // own modal loop on this worker thread and returns when it closes.
        if let Err(error) = unsafe { dialog.Show(None) } {
            // Cancel is how most dialogs end, and it is not a failure.
            if error.code() == ERROR_CANCELLED.to_hresult() {
                return Ok(None);
            }
            return Err(fail(error));
        }

        // SAFETY: Show succeeded, so there is a result; the display name is a
        // CoTaskMemAlloc'd string that this function frees after copying it.
        let path = unsafe {
            let item = dialog.GetResult().map_err(fail)?;
            let name = item.GetDisplayName(SIGDN_FILESYSPATH).map_err(fail)?;
            let text = name.to_string();
            CoTaskMemFree(Some(name.0.cast_const().cast()));
            text.map_err(|error| DialogError::Failed(error.to_string()))?
        };
        Ok(Some(PathBuf::from(path)))
    }
}

#[cfg(not(windows))]
mod platform {
    use std::path::PathBuf;

    use super::DialogError;

    pub(super) fn pick_image(
        _title: &str,
        _filter_name: &str,
    ) -> Result<Option<PathBuf>, DialogError> {
        Err(DialogError::Unsupported)
    }
}
