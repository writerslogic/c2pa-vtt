// Copyright 2026 WritersLogic. All rights reserved.
// Licensed under the Apache License, Version 2.0 or the MIT license,
// at your option.

//! C2PA manifest embedding for WebVTT subtitle and caption files.
//!
//! WebVTT files use NOTE blocks for comments. The manifest is embedded as a
//! single-line NOTE block immediately after the WEBVTT header, using the
//! C2PA ASCII armour delimiters.

use std::fmt;

const BEGIN: &str = "-----BEGIN C2PA MANIFEST-----";
const END: &str = "-----END C2PA MANIFEST-----";
const HEADER: &str = "WEBVTT";

#[derive(Debug)]
pub enum Error {
    NotFound,
    NotVtt,
    MultipleManifests,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "no manifest NOTE block found"),
            Self::NotVtt => write!(f, "file does not start with WEBVTT header"),
            Self::MultipleManifests => write!(f, "multiple manifest NOTE blocks found"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug)]
pub struct ExtractionResult {
    pub reference: String,
    pub offset: usize,
    pub length: usize,
}

pub fn extract_manifest(text: &str) -> Result<ExtractionResult, Error> {
    if !text.starts_with(HEADER) {
        return Err(Error::NotVtt);
    }

    let note_prefix = "NOTE ";
    let mut found: Option<ExtractionResult> = None;
    let mut pos = 0;

    for line in text.lines() {
        let line_bytes = line.len();
        if line.starts_with(note_prefix) && line.contains(BEGIN) && line.contains(END) {
            if found.is_some() {
                return Err(Error::MultipleManifests);
            }

            let begin_idx = line.find(BEGIN).unwrap() + BEGIN.len();
            let end_idx = line.find(END).unwrap();
            let reference = line[begin_idx..end_idx].trim().to_string();

            let length = line_bytes + if text[pos + line_bytes..].starts_with('\n') { 1 } else if text[pos + line_bytes..].starts_with("\r\n") { 2 } else { 0 };

            found = Some(ExtractionResult {
                reference,
                offset: pos,
                length,
            });
        }
        pos += line_bytes;
        if text[pos..].starts_with("\r\n") {
            pos += 2;
        } else if text[pos..].starts_with('\n') {
            pos += 1;
        }
    }

    found.ok_or(Error::NotFound)
}

pub fn embed_manifest(text: &str, reference: &str) -> Result<String, Error> {
    if !text.starts_with(HEADER) {
        return Err(Error::NotVtt);
    }

    let header_end = text.find('\n').map_or(text.len(), |p| p + 1);
    let header = &text[..header_end];
    let rest = &text[header_end..];

    let note_line = format!("NOTE {BEGIN} {reference} {END}");

    Ok(format!("{header}\n{note_line}\n{rest}"))
}

pub fn remove_manifest(text: &str) -> Result<String, Error> {
    let result = extract_manifest(text)?;
    let mut out = String::with_capacity(text.len());
    out.push_str(&text[..result.offset]);
    out.push_str(&text[result.offset + result.length..]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "WEBVTT\n\nNOTE -----BEGIN C2PA MANIFEST----- https://example.com/m.c2pa -----END C2PA MANIFEST-----\n\n00:00:00.000 --> 00:00:05.000\nHello world\n";

    #[test]
    fn extract_from_vtt() {
        let result = extract_manifest(SAMPLE).unwrap();
        assert_eq!(result.reference, "https://example.com/m.c2pa");
    }

    #[test]
    fn embed_into_vtt() {
        let plain = "WEBVTT\n\n00:00:00.000 --> 00:00:05.000\nHello world\n";
        let result = embed_manifest(plain, "https://example.com/m.c2pa").unwrap();
        assert!(result.contains("NOTE -----BEGIN C2PA MANIFEST-----"));
        assert!(result.starts_with("WEBVTT\n"));
    }

    #[test]
    fn remove_from_vtt() {
        let clean = remove_manifest(SAMPLE).unwrap();
        assert!(!clean.contains(BEGIN));
        assert!(clean.starts_with("WEBVTT"));
        assert!(clean.contains("00:00:00.000"));
    }

    #[test]
    fn not_vtt() {
        assert!(matches!(
            extract_manifest("not a vtt file"),
            Err(Error::NotVtt)
        ));
    }

    #[test]
    fn no_manifest() {
        let plain = "WEBVTT\n\n00:00:00.000 --> 00:00:05.000\nHello\n";
        assert!(matches!(
            extract_manifest(plain),
            Err(Error::NotFound)
        ));
    }
}
