//! Validation bridge to the official C2PA SDK ([c2pa-rs]).
//!
//! WebVTT hard-binding validation is owned by this crate (see [`crate::binding`])
//! because c2pa-rs has no stable native handler for `text/vtt`. Everything else
//! — COSE signature, certificate trust, timestamp, and assertion validation of
//! the manifest store — is delegated to c2pa-rs.
//!
//! [`extract_manifest_source`] (zero dependencies) locates the reference and
//! resolves an inline `data:` URI to raw store bytes. With the `c2pa` feature
//! enabled, [`validate`] hands those bytes to a c2pa-rs [`c2pa::Reader`].
//!
//! [c2pa-rs]: https://crates.io/crates/c2pa

use crate::error::Error;
use crate::extract::extract_manifest;
use c2pa_structured_text::{codec, DATA_URI_PREFIX};

/// The origin of a manifest store referenced from a WebVTT file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestSource {
    /// A URI to an external manifest store. The caller is responsible for
    /// fetching the bytes (this crate performs no network I/O).
    Url(String),
    /// A manifest store embedded inline via a `data:` URI, decoded to bytes.
    Embedded(Vec<u8>),
}

/// Extract and resolve the manifest reference from a WebVTT file.
///
/// A `data:application/c2pa;base64,...` reference is decoded to
/// [`ManifestSource::Embedded`]; any other reference is returned verbatim as
/// [`ManifestSource::Url`].
pub fn extract_manifest_source(text: &str) -> Result<ManifestSource, Error> {
    let reference = extract_manifest(text)?.reference;
    match reference.strip_prefix(DATA_URI_PREFIX) {
        Some(b64) => {
            let bytes = codec::decode(b64).map_err(|e| Error::MalformedReference(e.to_string()))?;
            Ok(ManifestSource::Embedded(bytes))
        }
        None => Ok(ManifestSource::Url(reference)),
    }
}

/// Validate a WebVTT asset against a C2PA manifest store using c2pa-rs.
///
/// c2pa-rs performs signature, trust, timestamp, and assertion validation;
/// inspect the returned [`c2pa::Reader`] via
/// [`validation_state`](c2pa::Reader::validation_state) and
/// [`validation_status`](c2pa::Reader::validation_status). Validate the WebVTT
/// hard binding separately with [`crate::binding::verify_data_hash`].
///
/// `manifest_store` is the raw C2PA Manifest Store (JUMBF) bytes, e.g. from
/// [`ManifestSource::Embedded`] or fetched from a [`ManifestSource::Url`].
#[cfg(feature = "c2pa")]
pub fn validate(vtt: &str, manifest_store: &[u8]) -> Result<c2pa::Reader, c2pa::Error> {
    use std::io::Cursor;

    c2pa::Reader::from_context(c2pa::Context::new()).with_manifest_data_and_stream(
        manifest_store,
        "text/vtt",
        Cursor::new(vtt.as_bytes()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::{embed_manifest, ManifestRef};

    #[test]
    fn resolves_url() {
        let vtt =
            embed_manifest("WEBVTT\n", ManifestRef::Url("https://example.com/m.c2pa")).unwrap();
        assert_eq!(
            extract_manifest_source(&vtt).unwrap(),
            ManifestSource::Url("https://example.com/m.c2pa".to_string())
        );
    }

    #[test]
    fn resolves_embedded_data_uri() {
        let store = b"\x00\x01store-bytes\xff";
        let vtt = embed_manifest("WEBVTT\n", ManifestRef::Embedded(store)).unwrap();
        assert_eq!(
            extract_manifest_source(&vtt).unwrap(),
            ManifestSource::Embedded(store.to_vec())
        );
    }

    #[test]
    fn malformed_data_uri_is_rejected() {
        let vtt = "WEBVTT\n\nNOTE -----BEGIN C2PA MANIFEST----- data:application/c2pa;base64,@@@ -----END C2PA MANIFEST-----\n";
        assert!(matches!(
            extract_manifest_source(vtt),
            Err(Error::MalformedReference(_))
        ));
    }
}
