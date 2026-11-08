//! Embeds the application icon and the version metadata into
//! `OmniToaster.exe`.
//!
//! Without this the exe carries the default Rust icon, which is what Explorer,
//! the taskbar, Alt-Tab and the shortcut a user pins would all show. The
//! VERSIONINFO block is what Properties ▸ Details reads, and what SmartScreen
//! quotes back at someone deciding whether to trust an unsigned download — so
//! it is worth filling in properly rather than leaving blank
//! (`docs/06-security-and-privacy.md`).
//!
//! Windows only. On any other target this is a no-op, so `cargo clippy` still
//! passes on Linux and the eventual cross-platform work is not blocked
//! (`docs/01-vision-and-scope.md`).

fn main() {
    // The icon is generated from one master by `scripts/make-icons.ps1`;
    // rebuild when either the script's output or this file changes.
    println!("cargo:rerun-if-changed=../../assets/icon/omnitoaster.ico");
    println!("cargo:rerun-if-changed=build.rs");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let icon = "../../assets/icon/omnitoaster.ico";
    if !std::path::Path::new(icon).exists() {
        // Fail loudly. A missing icon that silently produced a default-looking
        // exe would reach a release before anyone noticed.
        panic!("{icon} is missing — run `pwsh -File scripts/make-icons.ps1`");
    }

    let mut res = winresource::WindowsResource::new();
    res.set_icon(icon)
        .set("ProductName", "OmniToaster")
        .set(
            "FileDescription",
            "OmniToaster — portable toast notifications",
        )
        .set("CompanyName", "OmniToaster")
        .set("LegalCopyright", "MIT licensed. See LICENSE.")
        .set("OriginalFilename", "OmniToaster.exe")
        .set("InternalName", "OmniToaster");

    if let Err(error) = res.compile() {
        // The resource compiler comes with the Windows SDK. Say so, because
        // "failed to compile resources" on its own sends people hunting
        // through Rust rather than installing a toolchain component.
        panic!(
            "embedding the icon failed: {error}\n\
             This needs the Windows SDK's rc.exe (installed with the MSVC build tools)."
        );
    }
}
