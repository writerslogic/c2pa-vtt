use std::fmt;

/// Errors returned when embedding, extracting, or hard-binding a manifest.
#[derive(Debug)]
pub enum Error {
    /// The input does not begin with the `WEBVTT` signature line.
    NotVtt,
    /// No manifest `NOTE` block was found in the file.
    NotFound,
    /// More than one manifest `NOTE` block was found. Per the C2PA structured
    /// text embedding rules there shall be at most one; the file is rejected.
    MultipleManifests,
    /// Embedding was requested for a file that already carries a manifest
    /// NOTE block.
    AlreadyEmbedded,
    /// The file contains a bare CR line ending; C2PA structured text accepts
    /// LF and CRLF only.
    BareCarriageReturn,
    /// A manifest `NOTE` block was found but the reference between the
    /// delimiters is empty.
    EmptyReference,
    /// The manifest reference could not be parsed (e.g. a malformed
    /// `data:application/c2pa;base64,...` URI).
    MalformedReference(String),
    /// The hard-binding exclusion range extends beyond the end of the asset.
    ExclusionOutOfRange,
}

impl Error {
    /// The registered C2PA validation status code for this error, or `None`
    /// when the condition carries no status code.
    ///
    /// WebVTT is carried by the structured-text method, for which the
    /// specification defines no format-specific codes — unlike HTML or
    /// unstructured text. Only the data-hash condition maps to a code.
    ///
    /// Every crate in this family exposes this method, so a dispatcher handling
    /// several embedding methods can ask the same question of any of them.
    pub fn code(&self) -> Option<&'static str> {
        Some(match self {
            Self::ExclusionOutOfRange => "assertion.dataHash.malformed",
            Self::NotVtt
            | Self::NotFound
            | Self::MultipleManifests
            | Self::AlreadyEmbedded
            | Self::BareCarriageReturn
            | Self::EmptyReference
            | Self::MalformedReference(_) => return None,
        })
    }

    /// Whether this error means the asset carries no provenance at all, as
    /// opposed to provenance that was found and rejected.
    ///
    /// [`Error::MultipleManifests`] counts: the structured-text rules require
    /// an asset with more than one manifest block to be treated as if no
    /// manifests were located.
    pub fn is_no_manifest_located(&self) -> bool {
        matches!(self, Self::NotFound | Self::MultipleManifests)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotVtt => write!(f, "file does not start with WEBVTT header"),
            Self::NotFound => write!(f, "no manifest NOTE block found"),
            Self::MultipleManifests => write!(f, "multiple manifest NOTE blocks found"),
            Self::AlreadyEmbedded => write!(f, "a manifest NOTE block is already present"),
            Self::BareCarriageReturn => {
                write!(f, "bare CR line endings are not supported; use LF or CRLF")
            }
            Self::EmptyReference => write!(f, "empty manifest reference"),
            Self::MalformedReference(s) => write!(f, "malformed manifest reference: {s}"),
            Self::ExclusionOutOfRange => {
                write!(
                    f,
                    "hard-binding exclusion range extends beyond end of asset"
                )
            }
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    fn all() -> Vec<Error> {
        vec![
            Error::NotVtt,
            Error::NotFound,
            Error::MultipleManifests,
            Error::AlreadyEmbedded,
            Error::BareCarriageReturn,
            Error::EmptyReference,
            Error::MalformedReference("x".into()),
            Error::ExclusionOutOfRange,
        ]
    }

    /// Guards against inventing a code. WebVTT rides the structured-text
    /// method, which has no format-specific codes.
    #[test]
    fn every_code_is_a_registered_identifier() {
        for e in all() {
            if let Some(code) = e.code() {
                assert_eq!(
                    code, "assertion.dataHash.malformed",
                    "{e:?} invented a code"
                );
            }
        }
    }

    #[test]
    fn no_vtt_specific_code_is_emitted() {
        for e in all() {
            if let Some(code) = e.code() {
                assert!(!code.starts_with("manifest."), "{e:?} emits {code}");
            }
        }
    }

    #[test]
    fn locating_outcomes_carry_no_code() {
        for e in [Error::NotFound, Error::MultipleManifests] {
            assert_eq!(e.code(), None, "{e:?} must not report a status code");
            assert!(
                e.is_no_manifest_located(),
                "{e:?} must classify as unsigned"
            );
        }
        // Not a VTT file at all is a different thing from an unsigned one.
        assert!(!Error::NotVtt.is_no_manifest_located());
    }
}
