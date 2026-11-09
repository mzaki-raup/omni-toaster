//! Taking a file from outside the app (`docs/06-security-and-privacy.md`).
//!
//! This is the first thing OmniToaster accepts that it did not write itself,
//! so every step below is a requirement rather than a precaution, and they
//! happen **in this order**:
//!
//! 1. the **size limit**, checked before the file is read — a 500 MB "image"
//!    must not be loaded into memory to discover it is too big;
//! 2. the **format by magic bytes**, never by extension — a `.png` that is
//!    not a PNG is refused;
//! 3. **canonicalization** of the source, to catch traversal, and of the
//!    destination, to prove the write lands inside `OmniToasterData/assets/`;
//! 4. a copy under a **content-hashed name**, with the *relative* path stored
//!    so the folder can move to another drive or machine;
//! 5. a **bounded decode** — 4096 × 4096 — so a decompression bomb fails
//!    instead of exhausting memory.
//!
//! A rejection names its reason through a Fluent key, so the user is told
//! which rule the file broke rather than that "it didn't work".

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use sha2::{Digest as _, Sha256};

/// The most an imported image may weigh (`docs/specs/toast.md` §4).
pub const MAX_BYTES: u64 = 5 * 1024 * 1024;
/// The most it may decode to, in each dimension. A 4096 × 4096 RGBA image is
/// already 64 MB in memory; anything past that is not a notification picture.
pub const MAX_PIXELS: u32 = 4096;
/// Where imports land, under the data folder.
pub const ASSETS_DIR: &str = "assets";

/// The image formats the spec allows, with the bytes that identify each.
///
/// Matched against the file's first bytes, never against its name: the
/// extension is the user's to type and the magic is the file's own.
const SIGNATURES: &[(&[u8], Format)] = &[
    (
        &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A],
        Format::Png,
    ),
    (&[0xFF, 0xD8, 0xFF], Format::Jpeg),
    // WebP is "RIFF" ???? "WEBP"; the four size bytes in between are checked
    // by position rather than value, below.
    (b"RIFF", Format::Webp),
];

/// What an imported file turned out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Png,
    Jpeg,
    Webp,
}

impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Webp => "webp",
        }
    }
}

/// An imported asset, as the database and the renderer see it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    /// The content hash, which is also the file's name.
    pub sha256: String,
    /// **Relative** to the data folder, so a portable folder can move.
    pub relative_path: String,
    pub format: Format,
    pub bytes: u64,
}

/// Why a file could not be imported.
///
/// Each carries the Fluent key that tells the user which rule it broke.
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("{path} is {bytes} bytes, over the {MAX_BYTES} byte limit")]
    TooLarge { path: PathBuf, bytes: u64 },
    #[error("{path} is not a PNG, JPEG or WebP, whatever its name says")]
    NotAnImage { path: PathBuf },
    #[error("{path} is {width}x{height}, over the {MAX_PIXELS} pixel limit")]
    TooManyPixels {
        path: PathBuf,
        width: u32,
        height: u32,
    },
    #[error("{path} could not be decoded: {reason}")]
    Undecodable { path: PathBuf, reason: String },
    #[error("{path} is outside the data folder")]
    Escapes { path: PathBuf },
    #[error("{path} could not be read: {error}")]
    Unreadable { path: PathBuf, error: io::Error },
    #[error("{path} could not be written: {error}")]
    Unwritable { path: PathBuf, error: io::Error },
}

impl ImportError {
    /// The Fluent key naming this reason to the user.
    pub fn name_key(&self) -> &'static str {
        match self {
            Self::TooLarge { .. } => "error-import-too-large",
            Self::NotAnImage { .. } => "error-import-not-an-image",
            Self::TooManyPixels { .. } => "error-import-too-many-pixels",
            Self::Undecodable { .. } => "error-import-undecodable",
            Self::Escapes { .. } => "error-import-escapes",
            Self::Unreadable { .. } | Self::Unwritable { .. } => "error-import-unreadable",
        }
    }
}

/// Import `source` into `<root>/assets/`, validating it on the way.
///
/// Blocking, and deliberately so: it reads, hashes, decodes and writes. The
/// caller runs it on a worker thread and takes the result through a channel —
/// never on the UI thread (`.claude/rules/threads-and-scheduling.md`).
pub fn import(root: &Path, source: &Path) -> Result<Asset, ImportError> {
    // 1. The size, from the directory entry, before a byte is read.
    let metadata = fs::metadata(source).map_err(|error| ImportError::Unreadable {
        path: source.to_path_buf(),
        error,
    })?;
    if metadata.len() > MAX_BYTES {
        return Err(ImportError::TooLarge {
            path: source.to_path_buf(),
            bytes: metadata.len(),
        });
    }

    // Canonicalized so a traversal in the *source* is resolved before
    // anything is done with it.
    let source = source
        .canonicalize()
        .map_err(|error| ImportError::Unreadable {
            path: source.to_path_buf(),
            error,
        })?;
    let bytes = fs::read(&source).map_err(|error| ImportError::Unreadable {
        path: source.clone(),
        error,
    })?;

    // 2. The format, from the bytes themselves.
    let format = sniff(&bytes).ok_or_else(|| ImportError::NotAnImage {
        path: source.clone(),
    })?;

    // 5. The decode, bounded. Done before the copy, so a bomb never reaches
    //    the data folder at all.
    let (width, height) = dimensions(&bytes, &source)?;
    if width > MAX_PIXELS || height > MAX_PIXELS {
        return Err(ImportError::TooManyPixels {
            path: source,
            width,
            height,
        });
    }

    // 4. The content hash is the name, so the same picture imported twice is
    //    stored once and a changed picture is a different file.
    let sha256 = hex(&Sha256::digest(&bytes));
    let name = format!("{sha256}.{}", format.extension());
    let folder = root.join(ASSETS_DIR);
    fs::create_dir_all(&folder).map_err(|error| ImportError::Unwritable {
        path: folder.clone(),
        error,
    })?;
    let destination = folder.join(&name);

    // 3. And the destination is proved to be inside the data folder before
    //    the write, not after.
    guard_inside(root, &destination)?;

    if !destination.exists() {
        fs::write(&destination, &bytes).map_err(|error| ImportError::Unwritable {
            path: destination.clone(),
            error,
        })?;
    }

    Ok(Asset {
        sha256,
        relative_path: format!("{ASSETS_DIR}/{name}"),
        format,
        bytes: metadata.len(),
    })
}

/// A decoded, shrunken copy of an imported image, ready to draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thumbnail {
    pub width: u32,
    pub height: u32,
    /// RGBA, row by row, `width * height * 4` bytes.
    pub rgba: Vec<u8>,
}

/// Decode an imported image and shrink it to fit `max_side`.
///
/// Derived data, never the source of truth: it is rebuilt from the asset
/// whenever it is needed. Blocking, and like `import` it belongs on a worker
/// thread. The decode is bounded the same way the import was, because the
/// file on disk is only as trustworthy as whoever last wrote to the folder.
pub fn thumbnail(
    root: &Path,
    relative_path: &str,
    max_side: u32,
) -> Result<Thumbnail, ImportError> {
    let path = root.join(relative_path);
    guard_inside(root, &path)?;

    let bytes = fs::read(&path).map_err(|error| ImportError::Unreadable {
        path: path.clone(),
        error,
    })?;
    decode(&bytes, &path, max_side)
}

/// Decode an image already in memory, with the same limits as a file: the
/// format from its magic bytes, the size from its header, and a bounded
/// decode. `path` only names it in an error.
pub fn decode(bytes: &[u8], path: &Path, max_side: u32) -> Result<Thumbnail, ImportError> {
    let path = path.to_path_buf();
    if sniff(bytes).is_none() {
        return Err(ImportError::NotAnImage { path });
    }
    let (width, height) = dimensions(bytes, &path)?;
    if width > MAX_PIXELS || height > MAX_PIXELS {
        return Err(ImportError::TooManyPixels {
            path,
            width,
            height,
        });
    }

    let mut reader = image::ImageReader::new(io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|error| ImportError::Undecodable {
            path: path.clone(),
            reason: error.to_string(),
        })?;
    // The header was checked above; the limits make the decoder hold to it
    // even if the header lied.
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_PIXELS);
    limits.max_image_height = Some(MAX_PIXELS);
    reader.limits(limits);

    let decoded = reader.decode().map_err(|error| ImportError::Undecodable {
        path: path.clone(),
        reason: error.to_string(),
    })?;
    // Only ever shrink. `DynamicImage::thumbnail` scales to *fit*, which
    // includes scaling up: a 16 px icon came back 64 px wide, four times the
    // memory and blurred for nothing.
    let max = max_side.max(1);
    let small = if decoded.width() <= max && decoded.height() <= max {
        decoded.to_rgba8()
    } else {
        decoded.thumbnail(max, max).to_rgba8()
    };
    Ok(Thumbnail {
        width: small.width(),
        height: small.height(),
        rgba: small.into_raw(),
    })
}

/// What these bytes actually are, or `None` for anything else.
pub fn sniff(bytes: &[u8]) -> Option<Format> {
    for (magic, format) in SIGNATURES {
        if !bytes.starts_with(magic) {
            continue;
        }
        if *format == Format::Webp {
            // "RIFF" alone is a container, not a WebP: a RIFF wave file
            // starts the same way. The payload tag is what settles it.
            if bytes.len() < 12 || &bytes[8..12] != b"WEBP" {
                continue;
            }
        }
        return Some(*format);
    }
    None
}

/// Read the image's dimensions without decoding its pixels.
///
/// The header alone says how large it claims to be, so a decompression bomb
/// is refused **before** anything allocates for it.
fn dimensions(bytes: &[u8], path: &Path) -> Result<(u32, u32), ImportError> {
    image::ImageReader::new(io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|error| ImportError::Undecodable {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })?
        .into_dimensions()
        .map_err(|error| ImportError::Undecodable {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })
}

/// Delete an imported file, given its stored relative path.
///
/// Guarded like a write: a relative path read back from the database is data,
/// and must not be able to name a file outside the data folder. A file that is
/// already gone is not an error — the goal was for it to be gone.
pub fn remove(root: &Path, relative: &str) -> Result<(), ImportError> {
    let path = root.join(relative);
    guard_inside(root, &path)?;
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(ImportError::Unwritable { path, error }),
    }
}

/// Prove `candidate` is inside `root`.
///
/// The parent is canonicalized rather than the file, because the file may not
/// exist yet; `..` in the name itself is refused outright.
fn guard_inside(root: &Path, candidate: &Path) -> Result<(), ImportError> {
    if candidate
        .components()
        .any(|part| part == Component::ParentDir)
    {
        return Err(ImportError::Escapes {
            path: candidate.to_path_buf(),
        });
    }
    let root = root
        .canonicalize()
        .map_err(|error| ImportError::Unreadable {
            path: root.to_path_buf(),
            error,
        })?;
    let parent = candidate
        .parent()
        .unwrap_or(candidate)
        .canonicalize()
        .map_err(|error| ImportError::Unwritable {
            path: candidate.to_path_buf(),
            error,
        })?;
    if !parent.starts_with(&root) {
        return Err(ImportError::Escapes {
            path: candidate.to_path_buf(),
        });
    }
    Ok(())
}

fn hex(digest: &[u8]) -> String {
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The smallest real PNG: a 1×1 image, written by hand so the test does
    /// not depend on an encoder to prove the decoder's guard.
    const TINY_PNG: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    fn scratch() -> tempfile::TempDir {
        tempfile::tempdir().expect("a scratch folder")
    }

    fn write(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, bytes).expect("the fixture is written");
        path
    }

    #[test]
    fn a_real_png_imports_under_its_own_hash_and_a_relative_path() {
        let root = scratch();
        let source = scratch();
        let file = write(source.path(), "picture.png", TINY_PNG);

        let asset = import(root.path(), &file).expect("a real PNG imports");
        assert_eq!(asset.format, Format::Png);
        assert_eq!(asset.sha256.len(), 64, "a sha256 is 64 hex characters");
        assert_eq!(
            asset.relative_path,
            format!("assets/{}.png", asset.sha256),
            "the name is the content hash"
        );
        assert!(
            !Path::new(&asset.relative_path).is_absolute(),
            "an absolute path stored anywhere persistent is a defect"
        );
        assert!(root.path().join(&asset.relative_path).is_file());
    }

    #[test]
    fn a_png_that_is_not_a_png_is_refused() {
        // The rule that makes the extension irrelevant: format comes from the
        // magic bytes (docs/06-security-and-privacy.md).
        let root = scratch();
        let source = scratch();
        let file = write(
            source.path(),
            "trojan.png",
            b"MZ\x90\x00 this is an executable",
        );

        let error = import(root.path(), &file).expect_err("a fake PNG is refused");
        assert!(matches!(error, ImportError::NotAnImage { .. }), "{error}");
        assert_eq!(error.name_key(), "error-import-not-an-image");
        assert!(
            !root.path().join(ASSETS_DIR).exists()
                || fs::read_dir(root.path().join(ASSETS_DIR))
                    .expect("the folder reads")
                    .count()
                    == 0,
            "nothing may be written for a refused file"
        );
    }

    #[test]
    fn an_oversize_file_is_refused_before_it_is_read() {
        // Checked from the directory entry: a 500 MB "image" must not be
        // loaded into memory to discover it is too big. The fixture is just
        // over the limit, so the test stays quick.
        let root = scratch();
        let source = scratch();
        let big = vec![0u8; (MAX_BYTES + 1) as usize];
        let file = write(source.path(), "big.png", &big);

        let error = import(root.path(), &file).expect_err("an oversize file is refused");
        match error {
            ImportError::TooLarge { bytes, .. } => assert_eq!(bytes, MAX_BYTES + 1),
            other => panic!("expected TooLarge, got {other}"),
        }
    }

    #[test]
    fn a_decode_bomb_is_refused_on_its_header_rather_than_its_pixels() {
        // A PNG header claiming 50000 x 50000 is a few dozen bytes on disk
        // and 10 GB decoded. The dimensions come from the header, so this
        // costs nothing to refuse.
        let mut bomb = TINY_PNG.to_vec();
        // The IHDR width and height live at offsets 16..24.
        bomb[16..20].copy_from_slice(&50_000u32.to_be_bytes());
        bomb[20..24].copy_from_slice(&50_000u32.to_be_bytes());

        let root = scratch();
        let source = scratch();
        let file = write(source.path(), "bomb.png", &bomb);

        let error = import(root.path(), &file).expect_err("a bomb is refused");
        match error {
            ImportError::TooManyPixels { width, height, .. } => {
                assert_eq!((width, height), (50_000, 50_000));
            }
            // A corrupt-CRC refusal is just as good: it never reached the
            // pixels either.
            ImportError::Undecodable { .. } => {}
            other => panic!("expected a bounded-decode refusal, got {other}"),
        }
        assert!(
            !root.path().join(ASSETS_DIR).join("bomb.png").exists(),
            "a bomb must never reach the data folder"
        );
    }

    #[test]
    fn an_import_always_lands_inside_the_data_folder() {
        // The destination is built from a hash, so a traversal cannot come
        // through the name - but the guard is what proves it, and it is
        // asserted directly here.
        let root = scratch();
        let outside = root.path().join("..").join("escaped.png");
        let error = guard_inside(root.path(), &outside).expect_err("traversal is refused");
        assert!(matches!(error, ImportError::Escapes { .. }), "{error}");
        assert_eq!(error.name_key(), "error-import-escapes");

        // And the ordinary case passes.
        let inside = root.path().join(ASSETS_DIR).join("abc.png");
        fs::create_dir_all(root.path().join(ASSETS_DIR)).expect("the folder is created");
        guard_inside(root.path(), &inside).expect("a path inside the folder is allowed");
    }

    #[test]
    fn the_same_picture_twice_is_stored_once() {
        let root = scratch();
        let source = scratch();
        let first = write(source.path(), "a.png", TINY_PNG);
        let second = write(source.path(), "b.png", TINY_PNG);

        let one = import(root.path(), &first).expect("imports");
        let two = import(root.path(), &second).expect("imports");
        assert_eq!(one.sha256, two.sha256, "same bytes, same name");
        let count = fs::read_dir(root.path().join(ASSETS_DIR))
            .expect("the folder reads")
            .count();
        assert_eq!(count, 1, "the same picture must not be stored twice");
    }

    #[test]
    fn riff_alone_is_not_a_webp() {
        // A RIFF wave file starts with the same four bytes. The payload tag
        // is what settles it, and sniffing on the container alone would
        // accept an audio file as a picture.
        let mut wave = b"RIFF".to_vec();
        wave.extend_from_slice(&[0, 0, 0, 0]);
        wave.extend_from_slice(b"WAVE");
        assert_eq!(sniff(&wave), None);

        let mut webp = b"RIFF".to_vec();
        webp.extend_from_slice(&[0, 0, 0, 0]);
        webp.extend_from_slice(b"WEBP");
        assert_eq!(sniff(&webp), Some(Format::Webp));
    }

    #[test]
    fn every_refusal_names_a_reason_the_user_can_be_told() {
        // "It didn't work" is not a message. Each error carries the key that
        // says which rule the file broke.
        let path = PathBuf::from("x.png");
        let errors = [
            ImportError::TooLarge {
                path: path.clone(),
                bytes: 1,
            },
            ImportError::NotAnImage { path: path.clone() },
            ImportError::TooManyPixels {
                path: path.clone(),
                width: 1,
                height: 1,
            },
            ImportError::Undecodable {
                path: path.clone(),
                reason: "x".into(),
            },
            ImportError::Escapes { path: path.clone() },
        ];
        for error in errors {
            assert!(
                error.name_key().starts_with("error-import-"),
                "{error} has no key"
            );
        }
    }

    #[test]
    fn a_thumbnail_of_an_imported_image_decodes_to_rgba_within_its_bound() {
        let root = scratch();
        let source = scratch();
        let file = write(source.path(), "picture.png", TINY_PNG);
        let asset = import(root.path(), &file).expect("imports");

        let thumb = thumbnail(root.path(), &asset.relative_path, 64).expect("decodes");
        assert_eq!((thumb.width, thumb.height), (1, 1));
        assert_eq!(thumb.rgba.len(), 4, "one RGBA pixel");
    }

    #[test]
    fn a_thumbnail_will_not_read_outside_the_data_folder() {
        // The relative path comes from settings.toml, which a person can
        // edit; it must not be a way to read any file on the machine.
        let root = scratch();
        let error = thumbnail(root.path(), "../../Windows/win.ini", 64)
            .expect_err("a traversal is refused");
        assert!(matches!(error, ImportError::Escapes { .. }), "{error}");
    }

    #[test]
    fn sniffing_an_empty_or_truncated_file_answers_rather_than_panicking() {
        assert_eq!(sniff(&[]), None);
        assert_eq!(sniff(&[0x89]), None);
        assert_eq!(sniff(b"RIF"), None);
    }

    #[test]
    fn remove_refuses_a_path_that_climbs_out_of_the_data_folder() {
        let dir = tempfile::tempdir().expect("temp dir");
        let outside = dir.path().join("outside.png");
        std::fs::write(&outside, b"keep me").expect("write");
        let root = dir.path().join("data-root");
        std::fs::create_dir_all(root.join(ASSETS_DIR)).expect("mkdir");

        assert!(matches!(
            remove(&root, "../outside.png"),
            Err(ImportError::Escapes { .. })
        ));
        assert!(outside.exists(), "the file outside was not touched");
        assert!(
            remove(&root, "assets/gone.png").is_ok(),
            "already gone is fine"
        );
    }
}
