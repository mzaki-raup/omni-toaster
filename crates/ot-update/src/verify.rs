//! The signature check (FEAT-0078, ADR-010, ADR-024).
//!
//! Every release's `manifest.json` is signed with the project's minisign key, and
//! the public half is **embedded in the exe** (`keys/minisign.pub`): a file beside
//! the exe would be attacker-writable in a portable folder. The signature is
//! checked **before** the manifest is read as anything but bytes, and a failure
//! fails closed — nothing is downloaded, and nothing falls back to installing
//! anyway. Because of this, a release served by a hostile forge could not make a
//! copy install anything.

use crate::manifest::{self, Manifest, ManifestError};

/// The project's public key, as committed in `keys/minisign.pub`.
pub const PUBLIC_KEY: &str = include_str!("../../../keys/minisign.pub");

/// Why a manifest was not accepted.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VerifyError {
    #[error("the public key is not usable: {0}")]
    BadKey(String),
    #[error("the signature file is not a minisign signature: {0}")]
    BadSignature(String),
    /// The signature does not match the manifest and key. The only safe reading is
    /// that the manifest was altered, or was signed by someone else.
    #[error("the manifest's signature does not verify")]
    Rejected,
    #[error(transparent)]
    Manifest(#[from] ManifestError),
}

/// Check `signature` over `manifest_bytes` against `public_key` (the embedded key
/// in production), and only then read the manifest.
pub fn verify_manifest(
    manifest_bytes: &[u8],
    signature: &str,
    public_key: &str,
) -> Result<Manifest, VerifyError> {
    verify_manifest_with(manifest_bytes, signature, public_key, true)
}

/// [`verify_manifest`], with HTTPS optionally not required of the manifest's URLs
/// (a test's local server only).
pub(crate) fn verify_manifest_with(
    manifest_bytes: &[u8],
    signature: &str,
    public_key: &str,
    require_https: bool,
) -> Result<Manifest, VerifyError> {
    if manifest_bytes.len() > manifest::MAX_MANIFEST {
        return Err(ManifestError::TooLarge.into());
    }
    let key = minisign_verify::PublicKey::decode(public_key)
        .map_err(|error| VerifyError::BadKey(error.to_string()))?;
    let signature = minisign_verify::Signature::decode(signature)
        .map_err(|error| VerifyError::BadSignature(error.to_string()))?;
    // `false`: only the prehashed signatures rsign2 makes by default, not the
    // legacy un-hashed kind, which a release never carries.
    key.verify(manifest_bytes, &signature, false)
        .map_err(|_| VerifyError::Rejected)?;
    Ok(manifest::parse_with(manifest_bytes, require_https)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real(name: &str) -> Vec<u8> {
        std::fs::read(format!(
            "{}/tests/fixtures/real-0.18.0/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .expect("the fixture")
    }

    #[test]
    fn the_real_release_verifies_against_the_key_embedded_in_the_exe() {
        let signature = String::from_utf8(real("manifest.json.minisig")).expect("utf8");
        let manifest = verify_manifest(&real("manifest.json"), &signature, PUBLIC_KEY)
            .expect("a release published by this project verifies");
        assert_eq!(manifest.version.to_string(), "0.18.0");
    }

    #[test]
    fn a_manifest_altered_by_one_byte_is_rejected_before_it_is_read() {
        let signature = String::from_utf8(real("manifest.json.minisig")).expect("utf8");
        let mut bytes = real("manifest.json");
        // Change the digest of the exe: exactly what an attacker would want.
        let at = bytes
            .windows(8)
            .position(|w| w == b"9c55fa26")
            .expect("the digest");
        bytes[at] = b'0';
        assert_eq!(
            verify_manifest(&bytes, &signature, PUBLIC_KEY),
            Err(VerifyError::Rejected)
        );
    }

    #[test]
    fn a_signature_from_another_key_is_rejected() {
        use minisign::KeyPair;
        let other = KeyPair::generate_unencrypted_keypair().expect("a key");
        let bytes = real("manifest.json");
        let signed = minisign::sign(
            Some(&other.pk),
            &other.sk,
            std::io::Cursor::new(&bytes),
            Some("someone else"),
            None,
        )
        .expect("signs")
        .to_string();
        assert_eq!(
            verify_manifest(&bytes, &signed, PUBLIC_KEY),
            Err(VerifyError::Rejected),
            "well-formed, valid for its own key, and not ours"
        );
    }

    #[test]
    fn garbage_keys_and_signatures_fail_closed() {
        let bytes = real("manifest.json");
        let signature = String::from_utf8(real("manifest.json.minisig")).expect("utf8");
        assert!(matches!(
            verify_manifest(&bytes, &signature, "not a key"),
            Err(VerifyError::BadKey(_))
        ));
        assert!(matches!(
            verify_manifest(&bytes, "not a signature", PUBLIC_KEY),
            Err(VerifyError::BadSignature(_))
        ));
        assert!(matches!(
            verify_manifest(&bytes, "", PUBLIC_KEY),
            Err(VerifyError::BadSignature(_))
        ));
    }

    #[test]
    fn a_correctly_signed_manifest_that_is_not_a_manifest_is_still_refused() {
        use minisign::KeyPair;
        let pair = KeyPair::generate_unencrypted_keypair().expect("a key");
        let bytes = br#"{"schema":2}"#;
        let signed = minisign::sign(
            Some(&pair.pk),
            &pair.sk,
            std::io::Cursor::new(&bytes[..]),
            None,
            None,
        )
        .expect("signs")
        .to_string();
        let public = pair.pk.to_box().expect("a box").to_string();
        assert!(matches!(
            verify_manifest(bytes, &signed, &public),
            Err(VerifyError::Manifest(_))
        ));
    }

    #[test]
    fn the_embedded_key_is_the_one_committed_for_manual_verification() {
        assert!(PUBLIC_KEY.contains("minisign public key"), "{PUBLIC_KEY}");
        assert!(PUBLIC_KEY.lines().any(|line| line.starts_with("RW")));
    }
}
