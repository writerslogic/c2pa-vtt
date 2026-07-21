use crate::base64;
use crate::error::Error;
use crate::extract::{extract_manifest, is_webvtt, BEGIN, END};

/// A manifest to associate with a WebVTT file.
pub enum ManifestRef<'a> {
    /// A URI to an external C2PA Manifest Store (the preferred form).
    Url(&'a str),
    /// Raw C2PA Manifest Store bytes to embed inline as a
    /// `data:application/c2pa;base64,...` URI.
    Embedded(&'a [u8]),
}

/// Embed a manifest reference as a single-line `NOTE` block placed immediately
/// after the `WEBVTT` signature line, separated from adjacent blocks by blank
/// lines as WebVTT requires.
///
/// The file's existing line-terminator convention (LF or CRLF) is preserved.
/// Fails with [`Error::NotVtt`] if `text` is not a WebVTT file.
pub fn embed_manifest(text: &str, manifest: ManifestRef<'_>) -> Result<String, Error> {
    if !is_webvtt(text) {
        return Err(Error::NotVtt);
    }

    let reference = match manifest {
        ManifestRef::Url(url) => url.to_string(),
        ManifestRef::Embedded(bytes) => {
            format!("data:application/c2pa;base64,{}", base64::encode(bytes))
        }
    };
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let note = format!("NOTE {BEGIN} {reference} {END}");

    let (header, rest) = match text.find('\n') {
        Some(nl) => (&text[..nl], &text[nl + 1..]),
        None => (text, ""),
    };
    let header = header.strip_suffix('\r').unwrap_or(header);
    let body = rest.trim_start_matches(['\r', '\n']);

    let mut out = String::with_capacity(text.len() + note.len() + 4 * newline.len());
    out.push_str(header);
    out.push_str(newline);
    out.push_str(newline);
    out.push_str(&note);
    out.push_str(newline);
    if !body.is_empty() {
        out.push_str(newline);
        out.push_str(body);
    }
    Ok(out)
}

/// Remove the manifest `NOTE` block from a WebVTT file, returning the file with
/// that block's byte range spliced out.
pub fn remove_manifest(text: &str) -> Result<String, Error> {
    let found = extract_manifest(text)?;
    let mut out = String::with_capacity(text.len() - found.length);
    out.push_str(&text[..found.offset]);
    out.push_str(&text[found.offset + found.length..]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embed_after_header_not_before() {
        let plain = "WEBVTT\n\n00:00:00.000 --> 00:00:05.000\nHello\n";
        let signed = embed_manifest(plain, ManifestRef::Url("https://example.com/m.c2pa")).unwrap();
        assert!(signed.starts_with("WEBVTT\n"));
        assert!(signed.contains("NOTE -----BEGIN C2PA MANIFEST-----"));
        // The NOTE block must come after the signature, never before it.
        let sig = signed.find("WEBVTT").unwrap();
        let note = signed.find("NOTE").unwrap();
        assert!(sig < note);
    }

    #[test]
    fn embed_extract_symmetric() {
        let plain = "WEBVTT\n\n00:00:00.000 --> 00:00:05.000\nHello\n";
        let signed = embed_manifest(plain, ManifestRef::Url("urn:uuid:abc")).unwrap();
        assert_eq!(extract_manifest(&signed).unwrap().reference, "urn:uuid:abc");
    }

    #[test]
    fn embed_preserves_crlf() {
        let plain = "WEBVTT\r\n\r\n00:00:00.000 --> 00:00:05.000\r\nHi\r\n";
        let signed = embed_manifest(plain, ManifestRef::Url("urn:x")).unwrap();
        assert!(signed.starts_with("WEBVTT\r\n\r\nNOTE "));
        assert!(!signed.contains('\n') || signed.contains("\r\n"));
        assert_eq!(extract_manifest(&signed).unwrap().reference, "urn:x");
    }

    #[test]
    fn embed_data_uri() {
        let plain = "WEBVTT\n";
        let signed = embed_manifest(plain, ManifestRef::Embedded(b"store-bytes")).unwrap();
        assert!(signed.contains("data:application/c2pa;base64,"));
        assert!(extract_manifest(&signed)
            .unwrap()
            .reference
            .starts_with("data:application/c2pa;base64,"));
    }

    #[test]
    fn remove_restores_no_manifest() {
        let plain = "WEBVTT\n\n00:00:00.000 --> 00:00:05.000\nHello\n";
        let signed = embed_manifest(plain, ManifestRef::Url("urn:x")).unwrap();
        let cleaned = remove_manifest(&signed).unwrap();
        assert!(matches!(extract_manifest(&cleaned), Err(Error::NotFound)));
        assert!(cleaned.contains("00:00:00.000"));
        assert!(cleaned.starts_with("WEBVTT"));
    }

    #[test]
    fn embed_twice_is_rejected() {
        let plain = "WEBVTT\n\n00:00:00.000 --> 00:00:05.000\nHello\n";
        let once = embed_manifest(plain, ManifestRef::Url("urn:a")).unwrap();
        let twice = embed_manifest(&once, ManifestRef::Url("urn:b")).unwrap();
        assert!(matches!(
            extract_manifest(&twice),
            Err(Error::MultipleManifests)
        ));
    }

    #[test]
    fn not_vtt() {
        assert!(matches!(
            embed_manifest("hello", ManifestRef::Url("urn:x")),
            Err(Error::NotVtt)
        ));
    }
}
