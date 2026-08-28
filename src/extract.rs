use crate::error::Error;
use c2pa_structured_text::{find_delimiter, BEGIN, END};

pub(crate) const SIGNATURE: &str = "WEBVTT";
pub(crate) const NOTE: &str = "NOTE";

/// A located manifest `NOTE` block.
///
/// `offset` and `length` describe the byte range of the entire manifest block
/// line, including its comment prefix and its trailing line terminator (LF or
/// CRLF). This is exactly the single exclusion range required by the C2PA
/// `c2pa.hash.data` hard binding for structured text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionResult {
    /// The manifest reference found between the delimiters (a URI, or a
    /// `data:application/c2pa;base64,...` URI for an embedded store).
    pub reference: String,
    /// Byte offset of the first byte of the manifest block line.
    pub offset: usize,
    /// Byte length of the manifest block line including its line terminator.
    pub length: usize,
}

/// Returns true if `text` begins with the WebVTT signature (after an optional
/// UTF-8 BOM), followed by end-of-file or a space, tab, or line terminator.
pub(crate) fn is_webvtt(text: &str) -> bool {
    let body = text.strip_prefix('\u{feff}').unwrap_or(text);
    match body.strip_prefix(SIGNATURE) {
        Some(rest) => rest.is_empty() || rest.starts_with([' ', '\t', '\n', '\r']),
        None => false,
    }
}

pub(crate) fn reject_bare_cr(text: &str) -> Result<(), Error> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' {
            if bytes.get(i + 1) != Some(&b'\n') {
                return Err(Error::BareCarriageReturn);
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    Ok(())
}

/// Locate the C2PA manifest `NOTE` block in a WebVTT file.
///
/// Only a single-line `NOTE` comment carrying both delimiters is recognised;
/// occurrences of the delimiters on non-`NOTE` lines (e.g. inside cue text) are
/// ignored.
///
/// # Errors
///
/// [`Error::NotVtt`] if `text` is not WebVTT (including empty input),
/// [`Error::NotFound`] if it carries no manifest block, and
/// [`Error::MultipleManifests`] if more than one qualifying block is present —
/// the structured-text rules allow at most one.
///
/// ```
/// use c2pa_vtt::{embed_manifest, extract_manifest, ManifestRef};
///
/// let vtt = "WEBVTT\n\n00:00:00.000 --> 00:00:05.000\nHello\n";
/// let url = "https://example.com/m.c2pa";
/// let signed = embed_manifest(vtt, ManifestRef::Url(url))?;
///
/// let found = extract_manifest(&signed)?;
/// assert_eq!(found.reference, url);
/// // The block's byte range is what the hard binding excludes.
/// assert!(signed[found.offset..found.offset + found.length].contains("NOTE"));
/// # Ok::<(), c2pa_vtt::Error>(())
/// ```
///
/// A delimiter inside cue text is not a manifest, and empty input is not WebVTT:
///
/// ```
/// use c2pa_vtt::{extract_manifest, Error};
///
/// let vtt = "WEBVTT\n\n00:00:00.000 --> 00:00:05.000\n-----BEGIN C2PA MANIFEST-----\n";
/// assert!(matches!(extract_manifest(vtt), Err(Error::NotFound)));
/// assert!(matches!(extract_manifest(""), Err(Error::NotVtt)));
/// ```
pub fn extract_manifest(text: &str) -> Result<ExtractionResult, Error> {
    reject_bare_cr(text)?;
    if !is_webvtt(text) {
        return Err(Error::NotVtt);
    }

    let bytes = text.as_bytes();
    let mut found: Option<ExtractionResult> = None;
    let mut search = 0;

    while let Some(rel) = find_delimiter(&bytes[search..], BEGIN) {
        let begin = search + rel;
        let after_begin = begin + BEGIN.len();

        let end = match find_delimiter(&bytes[after_begin..], END) {
            Some(r) => after_begin + r,
            None => return Err(Error::NotFound),
        };
        let after_end = end + END.len();

        let line_start = text[..begin].rfind('\n').map_or(0, |p| p + 1);

        // The block is only recognised when the delimiters sit on a WebVTT
        // comment line, i.e. the text before BEGIN on this line is `NOTE`.
        if text[line_start..begin].trim() == NOTE {
            let reference = text[after_begin..end].trim().to_string();
            if reference.is_empty() {
                return Err(Error::EmptyReference);
            }
            if found.is_some() {
                return Err(Error::MultipleManifests);
            }

            let line_end = text[after_end..]
                .find('\n')
                .map_or(text.len(), |p| after_end + p + 1);

            found = Some(ExtractionResult {
                reference,
                offset: line_start,
                length: line_end - line_start,
            });
        }

        search = after_end;
    }

    found.ok_or(Error::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "WEBVTT\n\nNOTE -----BEGIN C2PA MANIFEST----- https://example.com/m.c2pa -----END C2PA MANIFEST-----\n\n00:00:00.000 --> 00:00:05.000\nHello world\n";

    #[test]
    fn extract_reference_and_range() {
        let r = extract_manifest(SAMPLE).unwrap();
        assert_eq!(r.reference, "https://example.com/m.c2pa");
        assert_eq!(r.offset, "WEBVTT\n\n".len());
        // The excluded range is exactly the NOTE line plus its trailing LF.
        let expected_line =
            "NOTE -----BEGIN C2PA MANIFEST----- https://example.com/m.c2pa -----END C2PA MANIFEST-----\n";
        assert_eq!(r.length, expected_line.len());
        assert_eq!(&SAMPLE[r.offset..r.offset + r.length], expected_line);
    }

    #[test]
    fn not_vtt() {
        assert!(matches!(
            extract_manifest("not a vtt file"),
            Err(Error::NotVtt)
        ));
    }

    #[test]
    fn accepts_bom_and_header_text() {
        let vtt = "\u{feff}WEBVTT - Title\n\nNOTE -----BEGIN C2PA MANIFEST----- urn:x -----END C2PA MANIFEST-----\n";
        assert_eq!(extract_manifest(vtt).unwrap().reference, "urn:x");
    }

    #[test]
    fn no_manifest() {
        let plain = "WEBVTT\n\n00:00:00.000 --> 00:00:05.000\nHello\n";
        assert!(matches!(extract_manifest(plain), Err(Error::NotFound)));
    }

    #[test]
    fn empty_reference() {
        let vtt = "WEBVTT\n\nNOTE -----BEGIN C2PA MANIFEST-----  -----END C2PA MANIFEST-----\n";
        assert!(matches!(extract_manifest(vtt), Err(Error::EmptyReference)));
    }

    #[test]
    fn multiple_manifests() {
        let vtt = "WEBVTT\n\nNOTE -----BEGIN C2PA MANIFEST----- https://a -----END C2PA MANIFEST-----\n\nNOTE -----BEGIN C2PA MANIFEST----- https://b -----END C2PA MANIFEST-----\n";
        assert!(matches!(
            extract_manifest(vtt),
            Err(Error::MultipleManifests)
        ));
    }

    #[test]
    fn ignores_delimiter_in_cue_text() {
        let vtt = "WEBVTT\n\n00:00:00.000 --> 00:00:05.000\n-----BEGIN C2PA MANIFEST----- nope -----END C2PA MANIFEST-----\n";
        assert!(matches!(extract_manifest(vtt), Err(Error::NotFound)));
    }

    #[test]
    fn crlf_length_includes_terminator() {
        let vtt = "WEBVTT\r\n\r\nNOTE -----BEGIN C2PA MANIFEST----- urn:x -----END C2PA MANIFEST-----\r\n\r\n";
        let r = extract_manifest(vtt).unwrap();
        assert!(vtt[r.offset..r.offset + r.length].ends_with("\r\n"));
    }

    #[test]
    fn bare_cr_is_rejected() {
        let vtt = "WEBVTT\rNOTE -----BEGIN C2PA MANIFEST----- urn:x -----END C2PA MANIFEST-----";
        assert!(matches!(
            extract_manifest(vtt),
            Err(Error::BareCarriageReturn)
        ));
    }
}

#[cfg(test)]
mod malformed_input {
    use super::*;

    const URL: &str = "https://example.com/m.c2pa";

    /// Every public entry point must refuse non-WebVTT input with a descriptive
    /// error rather than panicking or silently producing nonsense.
    #[test]
    fn malformed_input_is_refused_by_every_entry_point() {
        for bad in [
            "",                      // empty
            "   ",                   // whitespace only
            "not a caption file",    // no signature
            "\u{0}\u{1}\u{2}binary", // control bytes
            "webvtt",                // wrong case: the signature is case-sensitive
            "XWEBVTT",               // signature not at the start
            "WEBVTTX",               // signature not followed by whitespace or EOF
        ] {
            assert!(
                matches!(extract_manifest(bad), Err(Error::NotVtt)),
                "extract accepted {bad:?}"
            );
            assert!(
                matches!(
                    crate::embed_manifest(bad, crate::ManifestRef::Url(URL)),
                    Err(Error::NotVtt)
                ),
                "embed accepted {bad:?}"
            );
            assert!(
                matches!(crate::remove_manifest(bad), Err(Error::NotVtt)),
                "remove accepted {bad:?}"
            );
        }
    }

    /// A byte-order mark is tolerated: editors add one and the file is still
    /// WebVTT.
    #[test]
    fn a_leading_byte_order_mark_is_accepted() {
        let vtt = "\u{feff}WEBVTT\n\n00:00:00.000 --> 00:00:05.000\nHi\n";
        let signed = crate::embed_manifest(vtt, crate::ManifestRef::Url(URL)).unwrap();
        assert_eq!(extract_manifest(&signed).unwrap().reference, URL);
    }

    /// The signature alone, with no cues, is still a valid WebVTT file.
    #[test]
    fn a_signature_only_file_is_valid() {
        for vtt in ["WEBVTT", "WEBVTT\n", "WEBVTT - with a title\n"] {
            assert!(
                crate::embed_manifest(vtt, crate::ManifestRef::Url(URL)).is_ok(),
                "rejected {vtt:?}"
            );
        }
    }

    /// The error message names the problem, so a caller can surface it.
    #[test]
    fn the_error_message_is_descriptive() {
        let msg = Error::NotVtt.to_string();
        assert!(msg.contains("WEBVTT"), "unhelpful message: {msg}");
    }
}
