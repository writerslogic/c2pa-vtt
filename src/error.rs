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
    /// A manifest `NOTE` block was found but the reference between the
    /// delimiters is empty.
    EmptyReference,
    /// The manifest reference could not be parsed (e.g. a malformed
    /// `data:application/c2pa;base64,...` URI).
    MalformedReference(String),
    /// The hard-binding exclusion range extends beyond the end of the asset.
    ExclusionOutOfRange,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotVtt => write!(f, "file does not start with WEBVTT header"),
            Self::NotFound => write!(f, "no manifest NOTE block found"),
            Self::MultipleManifests => write!(f, "multiple manifest NOTE blocks found"),
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
