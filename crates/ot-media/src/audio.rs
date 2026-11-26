//! Checking an audio file the user supplies (FEAT-0060, ADR-022,
//! `docs/06-security-and-privacy.md`).
//!
//! The same rules as an imported picture, in the same order: the **size** before
//! a byte is read, the **format from the bytes** and never the name, and a
//! **bounded decode** — the whole clip is decoded, up to [`MAX_SECONDS`], so a
//! corrupt file or one that never ends is refused with a message instead of
//! hanging a worker or filling memory. The copy under a hashed name is the
//! store's job (`ot_store::assets::store_audio`); this crate never writes.

use std::io::Cursor;
use std::time::Duration;

use rodio::Source as _;

/// The most an imported clip may weigh. Ten minutes of 192 kbps MP3 is 14 MB.
pub const MAX_BYTES: u64 = 20 * 1024 * 1024;

/// The longest a clip may run: also the ten-minute cap a persistent
/// notification has (`docs/specs/toast.md` §6).
pub const MAX_SECONDS: u64 = 600;

/// The audio formats an import may be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Wav,
    Mp3,
    Ogg,
}

impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Wav => "wav",
            Self::Mp3 => "mp3",
            Self::Ogg => "ogg",
        }
    }
}

/// Why a file was refused. Each names the rule it broke.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AudioError {
    #[error("the file is {0} bytes, over the {MAX_BYTES} byte limit")]
    TooLarge(u64),
    #[error("the file is not a WAV, MP3 or Ogg Vorbis, whatever its name says")]
    NotAudio,
    #[error("the file could not be decoded: {0}")]
    Undecodable(String),
    #[error("the clip runs longer than {MAX_SECONDS} seconds")]
    TooLong,
}

impl AudioError {
    /// The Fluent key that names the reason to the user.
    pub fn name_key(&self) -> &'static str {
        match self {
            Self::TooLarge(_) => "error-import-audio-too-large",
            Self::NotAudio => "error-import-not-audio",
            Self::Undecodable(_) => "error-import-undecodable",
            Self::TooLong => "error-import-audio-too-long",
        }
    }
}

/// What a good file turned out to be.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Checked {
    pub format: Format,
    pub duration: Duration,
}

/// The format of `bytes` from their first bytes, or `None`.
pub fn sniff(bytes: &[u8]) -> Option<Format> {
    match bytes {
        [
            b'R',
            b'I',
            b'F',
            b'F',
            _,
            _,
            _,
            _,
            b'W',
            b'A',
            b'V',
            b'E',
            ..,
        ] => Some(Format::Wav),
        [b'O', b'g', b'g', b'S', ..] => Some(Format::Ogg),
        // An ID3 tag, or an MPEG audio frame: eleven set bits of sync.
        [b'I', b'D', b'3', ..] => Some(Format::Mp3),
        [0xFF, second, ..] if second & 0xE0 == 0xE0 => Some(Format::Mp3),
        _ => None,
    }
}

/// The size rule, from a length already known, before the file is read.
pub fn check_size(bytes: u64) -> Result<(), AudioError> {
    if bytes > MAX_BYTES {
        return Err(AudioError::TooLarge(bytes));
    }
    Ok(())
}

/// Check a clip: its size, its format from the bytes, and that all of it decodes
/// within [`MAX_SECONDS`]. Blocking, up to a few seconds for a long clip, so it
/// runs on a worker thread.
pub fn check(bytes: &[u8]) -> Result<Checked, AudioError> {
    check_size(bytes.len() as u64)?;
    let format = sniff(bytes).ok_or(AudioError::NotAudio)?;
    let decoder = rodio::Decoder::new(Cursor::new(bytes.to_vec()))
        .map_err(|error| AudioError::Undecodable(error.to_string()))?;
    let channels = u64::from(decoder.channels().get());
    let rate = u64::from(decoder.sample_rate().get());
    let cap = MAX_SECONDS * rate * channels;
    // Every sample, counted up to the cap: a header that lies about its length,
    // or a stream with no end, stops here.
    let mut samples = 0u64;
    for _ in decoder {
        samples += 1;
        if samples > cap {
            return Err(AudioError::TooLong);
        }
    }
    if samples == 0 {
        return Err(AudioError::Undecodable(
            "there is no audio in it".to_owned(),
        ));
    }
    Ok(Checked {
        format,
        duration: Duration::from_secs_f64(samples as f64 / (rate * channels) as f64),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mono 16-bit PCM WAV of `seconds` of a quiet tone at 8 kHz.
    pub(crate) fn wav(seconds: u32) -> Vec<u8> {
        let rate = 8000u32;
        let data_len = rate * 2 * seconds;
        let mut out = Vec::with_capacity(44 + data_len as usize);
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data_len).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes()); // PCM
        out.extend_from_slice(&1u16.to_le_bytes()); // mono
        out.extend_from_slice(&rate.to_le_bytes());
        out.extend_from_slice(&(rate * 2).to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());
        for n in 0..(rate * seconds) {
            let wave = if (n / 20) % 2 == 0 { 800i16 } else { -800i16 };
            out.extend_from_slice(&wave.to_le_bytes());
        }
        out
    }

    #[test]
    fn the_format_comes_from_the_bytes_and_never_the_name() {
        assert_eq!(sniff(&wav(1)), Some(Format::Wav));
        assert_eq!(sniff(b"OggS\0\x02rest"), Some(Format::Ogg));
        assert_eq!(sniff(b"ID3\x04\0\0\0\0\0\0"), Some(Format::Mp3));
        assert_eq!(
            sniff(&[0xFF, 0xFB, 0x90, 0x00]),
            Some(Format::Mp3),
            "a frame sync"
        );
        // Things that are not audio, whatever they are called.
        assert_eq!(sniff(b"\x89PNG\r\n\x1a\n"), None);
        assert_eq!(sniff(b"RIFF\0\0\0\0WEBPVP8 "), None, "a WebP is also RIFF");
        assert_eq!(sniff(b"plain text, named clip.mp3"), None);
        assert_eq!(sniff(&[0xFF, 0x00]), None, "no sync bits");
        assert_eq!(sniff(&[]), None);
    }

    #[test]
    fn a_good_clip_is_checked_and_its_length_measured() {
        let checked = check(&wav(2)).expect("a good WAV");
        assert_eq!(checked.format, Format::Wav);
        assert!(
            (checked.duration.as_secs_f64() - 2.0).abs() < 0.05,
            "{:?}",
            checked.duration
        );
    }

    #[test]
    fn the_size_limit_is_checked_before_anything_is_read() {
        assert_eq!(check_size(MAX_BYTES), Ok(()));
        assert_eq!(
            check_size(MAX_BYTES + 1),
            Err(AudioError::TooLarge(MAX_BYTES + 1))
        );
    }

    #[test]
    fn what_is_not_audio_or_will_not_decode_is_refused_with_its_reason() {
        assert_eq!(check(b"\x89PNG\r\n\x1a\nrest"), Err(AudioError::NotAudio));
        assert_eq!(check(&[]), Err(AudioError::NotAudio));

        // A WAV header with the audio cut off and replaced by noise.
        let mut corrupt = wav(1);
        corrupt.truncate(30);
        corrupt.extend_from_slice(&[0xAB; 64]);
        assert!(
            matches!(check(&corrupt), Err(AudioError::Undecodable(_))),
            "{:?}",
            check(&corrupt)
        );

        // Each refusal names a Fluent key.
        for error in [
            AudioError::TooLarge(1),
            AudioError::NotAudio,
            AudioError::Undecodable(String::new()),
            AudioError::TooLong,
        ] {
            assert!(error.name_key().starts_with("error-import-"));
        }
    }

    #[test]
    fn a_clip_longer_than_the_cap_is_refused_however_small_the_file() {
        let long = wav(u32::try_from(MAX_SECONDS).expect("fits") + 2);
        assert!(
            long.len() as u64 <= MAX_BYTES,
            "small enough to pass the size rule"
        );
        assert_eq!(check(&long), Err(AudioError::TooLong));
    }
}
