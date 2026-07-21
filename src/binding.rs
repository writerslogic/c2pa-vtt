//! Hard binding for WebVTT: the `c2pa.hash.data` data hash.
//!
//! # What "hard binding" means for WebVTT
//!
//! WebVTT is a structured text container, so C2PA binds it with a byte-exact
//! [`c2pa.hash.data`] data hash carrying a **single exclusion range** that
//! covers the manifest `NOTE` block. The hash is computed over the *raw bytes*
//! of the file with that range removed — the `WEBVTT` signature, every cue,
//! `STYLE`/`REGION` block, and author comment is covered; only the manifest
//! block itself is excluded.
//!
//! Unlike the unstructured-text (Unicode Variation Selector) method, structured
//! text hashing applies **no Unicode normalization**: the file is byte-stable
//! on disk and the ASCII delimiters make the excluded range unambiguous.
//! Applying NFC would create false mismatches for legitimate NFD content in cue
//! text. Files must use LF or CRLF line terminators; bare CR is not supported.
//!
//! [`c2pa.hash.data`]: https://spec.c2pa.org/specifications/specifications/2.4/specs/C2PA_Specification.html

use crate::error::Error;
use crate::extract::extract_manifest;

/// A `c2pa.hash.data` exclusion range (`start`/`length`, in bytes), matching the
/// `EXCLUSION_RANGE-map` CDDL rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Exclusion {
    /// Starting byte of the excluded range.
    pub start: usize,
    /// Number of bytes to exclude.
    pub length: usize,
}

/// A C2PA cryptographic hash algorithm usable for the data hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlg {
    Sha256,
    Sha384,
    Sha512,
}

impl HashAlg {
    /// The C2PA hash algorithm identifier (the value of the `alg` field).
    pub fn c2pa_id(self) -> &'static str {
        match self {
            HashAlg::Sha256 => "sha256",
            HashAlg::Sha384 => "sha384",
            HashAlg::Sha512 => "sha512",
        }
    }
}

/// The single exclusion range covering the manifest `NOTE` block, which the
/// generator must place in the `c2pa.hash.data` assertion's `exclusions` field.
///
/// Available with no dependencies so callers using their own hasher can compute
/// the binding themselves.
pub fn data_hash_exclusion(text: &str) -> Result<Exclusion, Error> {
    let found = extract_manifest(text)?;
    Ok(Exclusion {
        start: found.offset,
        length: found.length,
    })
}

/// Compute the `c2pa.hash.data` value over the file with the manifest block
/// excluded. The result is the byte string that goes in the assertion's `hash`
/// field.
#[cfg(feature = "hash")]
pub fn compute_data_hash(text: &str, alg: HashAlg) -> Result<Vec<u8>, Error> {
    let ex = data_hash_exclusion(text)?;
    hash_excluding(text.as_bytes(), ex, alg)
}

/// Verify a `c2pa.hash.data` value against a WebVTT file. Returns `true` when
/// the recomputed hash matches `expected`.
#[cfg(feature = "hash")]
pub fn verify_data_hash(text: &str, alg: HashAlg, expected: &[u8]) -> Result<bool, Error> {
    let got = compute_data_hash(text, alg)?;
    Ok(constant_time_eq(&got, expected))
}

#[cfg(feature = "hash")]
fn hash_excluding(bytes: &[u8], ex: Exclusion, alg: HashAlg) -> Result<Vec<u8>, Error> {
    use c2pa_structured_text::hardbinding::{apply_exclusions, Exclusion as StExclusion};
    use sha2::{Digest, Sha256, Sha384, Sha512};

    let covered = apply_exclusions(
        bytes,
        &[StExclusion {
            start: ex.start,
            length: ex.length,
        }],
    )
    .map_err(|_| Error::ExclusionOutOfRange)?;

    Ok(match alg {
        HashAlg::Sha256 => Sha256::digest(&covered).to_vec(),
        HashAlg::Sha384 => Sha384::digest(&covered).to_vec(),
        HashAlg::Sha512 => Sha512::digest(&covered).to_vec(),
    })
}

#[cfg(feature = "hash")]
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::{embed_manifest, ManifestRef};

    const PLAIN: &str = "WEBVTT\n\n00:00:00.000 --> 00:00:05.000\nHello world\n";

    #[test]
    fn exclusion_covers_note_line() {
        let signed = embed_manifest(PLAIN, ManifestRef::Url("urn:x")).unwrap();
        let ex = data_hash_exclusion(&signed).unwrap();
        let excluded = &signed[ex.start..ex.start + ex.length];
        assert!(excluded.starts_with("NOTE -----BEGIN C2PA MANIFEST-----"));
        assert!(excluded.trim_end().ends_with("-----END C2PA MANIFEST-----"));
    }

    #[test]
    fn alg_identifiers() {
        assert_eq!(HashAlg::Sha256.c2pa_id(), "sha256");
        assert_eq!(HashAlg::Sha384.c2pa_id(), "sha384");
        assert_eq!(HashAlg::Sha512.c2pa_id(), "sha512");
    }

    #[cfg(feature = "hash")]
    #[test]
    fn hash_is_independent_of_reference() {
        // Excluding the NOTE block means two references over identical content
        // must produce the same data hash.
        let a = embed_manifest(PLAIN, ManifestRef::Url("urn:a")).unwrap();
        let b = embed_manifest(
            PLAIN,
            ManifestRef::Url("urn:completely-different-and-longer"),
        )
        .unwrap();
        let ha = compute_data_hash(&a, HashAlg::Sha256).unwrap();
        let hb = compute_data_hash(&b, HashAlg::Sha256).unwrap();
        assert_eq!(ha, hb);
    }

    #[cfg(feature = "hash")]
    #[test]
    fn verify_round_trip_and_tamper() {
        let signed = embed_manifest(PLAIN, ManifestRef::Url("urn:x")).unwrap();
        let hash = compute_data_hash(&signed, HashAlg::Sha256).unwrap();
        assert!(verify_data_hash(&signed, HashAlg::Sha256, &hash).unwrap());

        let tampered = signed.replace("Hello world", "Goodbye world");
        assert!(!verify_data_hash(&tampered, HashAlg::Sha256, &hash).unwrap());
    }

    #[cfg(feature = "hash")]
    #[test]
    fn hash_covers_exactly_the_non_excluded_bytes() {
        // With only the manifest block after the header, the hashed bytes are
        // exactly "WEBVTT\n\n".
        let signed =
            "WEBVTT\n\nNOTE -----BEGIN C2PA MANIFEST----- urn:x -----END C2PA MANIFEST-----\n";
        let h = compute_data_hash(signed, HashAlg::Sha256).unwrap();
        use sha2::{Digest, Sha256};
        let mut d = Sha256::new();
        d.update(b"WEBVTT\n\n");
        assert_eq!(h, d.finalize().to_vec());
    }
}
