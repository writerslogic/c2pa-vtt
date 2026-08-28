use crate::error::Error;
use crate::extract::{extract_manifest, is_webvtt, reject_bare_cr};
use c2pa_structured_text::{codec, BEGIN, DATA_URI_PREFIX, END};

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
///
/// # Errors
///
/// [`Error::NotVtt`] if `text` does not begin with the `WEBVTT` signature —
/// including empty input, and content that merely contains the signature
/// further in. A leading byte-order mark is tolerated, since editors add one.
///
/// Input is `&str`, so non-UTF-8 bytes cannot reach this function; the Python
/// and JavaScript bindings reject them at the boundary.
///
/// ```
/// use c2pa_vtt::{embed_manifest, ManifestRef};
///
/// let vtt = "WEBVTT\n\n00:00:00.000 --> 00:00:05.000\nHello\n";
/// let signed = embed_manifest(vtt, ManifestRef::Url("https://example.com/m.c2pa"))?;
///
/// assert!(signed.contains("-----BEGIN C2PA MANIFEST-----"));
/// assert!(signed.starts_with("WEBVTT"));
/// // The cue is untouched.
/// assert!(signed.contains("00:00:00.000 --> 00:00:05.000"));
/// # Ok::<(), c2pa_vtt::Error>(())
/// ```
///
/// Anything that is not WebVTT is refused rather than silently mangled:
///
/// ```
/// use c2pa_vtt::{embed_manifest, Error, ManifestRef};
///
/// let url = "https://example.com/m.c2pa";
/// assert!(matches!(embed_manifest("", ManifestRef::Url(url)), Err(Error::NotVtt)));
/// assert!(matches!(
///     embed_manifest("not a caption file", ManifestRef::Url(url)),
///     Err(Error::NotVtt)
/// ));
/// ```
pub fn embed_manifest(text: &str, manifest: ManifestRef<'_>) -> Result<String, Error> {
    reject_bare_cr(text)?;
    if !is_webvtt(text) {
        return Err(Error::NotVtt);
    }
    match extract_manifest(text) {
        Ok(_) | Err(Error::MultipleManifests) => return Err(Error::AlreadyEmbedded),
        Err(Error::NotFound) => {}
        Err(error) => return Err(error),
    }

    let reference = match manifest {
        ManifestRef::Url(url) => url.to_string(),
        ManifestRef::Embedded(bytes) => {
            format!("{DATA_URI_PREFIX}{}", codec::encode(bytes))
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

/// Remove the C2PA manifest `NOTE` block, restoring the file to its unsigned
/// form.
///
/// # Errors
///
/// [`Error::NotVtt`] if `text` is not WebVTT, [`Error::NotFound`] if it carries
/// no manifest block, and [`Error::MultipleManifests`] if it carries more than
/// one. Removing nothing is an error rather than a silent no-op, so a caller
/// cannot mistake "already clean" for "cleaned".
///
/// ```
/// use c2pa_vtt::{embed_manifest, remove_manifest, ManifestRef};
///
/// let vtt = "WEBVTT\n\n00:00:00.000 --> 00:00:05.000\nHello\n";
/// let signed = embed_manifest(vtt, ManifestRef::Url("https://example.com/m.c2pa"))?;
///
/// // Removal is the exact inverse of embedding.
/// assert_eq!(remove_manifest(&signed)?, vtt);
/// # Ok::<(), c2pa_vtt::Error>(())
/// ```
pub fn remove_manifest(text: &str) -> Result<String, Error> {
    let found = extract_manifest(text)?;
    let before = &text[..found.offset];
    let mut after = &text[found.offset + found.length..];

    // `embed_manifest` writes the block *and* a blank line separating it from
    // the body, but the extraction range covers only the block line — that
    // range is the hard-binding exclusion and must stay exactly the line.
    // Removing the line alone would orphan the separator, so `remove(embed(x))`
    // would gain a blank line instead of returning `x`. Consume it only when
    // the text before the block already ends with a terminator, which is
    // precisely when the leftover would double up.
    if before.ends_with('\n') {
        if let Some(rest) = after.strip_prefix("\r\n") {
            after = rest;
        } else if let Some(rest) = after.strip_prefix('\n') {
            after = rest;
        }
    }

    let mut out = String::with_capacity(before.len() + after.len());
    out.push_str(before);
    out.push_str(after);
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
        assert!(matches!(
            embed_manifest(&once, ManifestRef::Url("urn:b")),
            Err(Error::AlreadyEmbedded)
        ));
    }

    #[test]
    fn embed_rejects_bare_cr() {
        assert!(matches!(
            embed_manifest("WEBVTT\rbody", ManifestRef::Url("urn:x")),
            Err(Error::BareCarriageReturn)
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

#[cfg(test)]
mod round_trip {
    use super::*;

    /// Removing a manifest must restore the file byte for byte. The extraction
    /// range covers only the block line (it is the hard-binding exclusion), but
    /// embedding also writes the blank line separating the block from the body,
    /// so removal has to take that back or signing becomes lossy.
    #[test]
    fn remove_is_the_exact_inverse_of_embed() {
        for original in [
            "WEBVTT\n\n00:00:00.000 --> 00:00:05.000\nHello\n",
            "WEBVTT\r\n\r\n00:00:00.000 --> 00:00:05.000\r\nHello\r\n",
            "WEBVTT - With A Title\n\n00:00:00.000 --> 00:00:01.000\nHi\n",
            "\u{feff}WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nHi\n",
        ] {
            for manifest in [
                ManifestRef::Url("https://example.com/m.c2pa"),
                ManifestRef::Embedded(b"manifest-store-bytes"),
            ] {
                let signed = embed_manifest(original, manifest).unwrap();
                assert_ne!(signed, original, "embedding changed nothing");
                assert_eq!(
                    remove_manifest(&signed).unwrap(),
                    original,
                    "round trip lost or gained bytes for {original:?}"
                );
            }
        }
    }

    /// A hand-written file with no blank line after the block must not lose the
    /// first line of the body.
    #[test]
    fn removal_does_not_eat_the_body_when_there_is_no_separator() {
        let vtt = "WEBVTT\n\nNOTE -----BEGIN C2PA MANIFEST----- urn:x -----END C2PA MANIFEST-----\n00:00:00.000 --> 00:00:01.000\nHi\n";
        assert_eq!(
            remove_manifest(vtt).unwrap(),
            "WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nHi\n"
        );
    }
}
