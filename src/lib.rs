// Copyright 2026 WritersLogic. All rights reserved.
// Licensed under the Apache License, Version 2.0 or the MIT license,
// at your option.

//! C2PA manifest embedding and hard binding for WebVTT subtitle and caption
//! files.
//!
//! WebVTT is a structured text format, so a C2PA Manifest Store is associated
//! with it using the fixed ASCII armour delimiters
//! (`-----BEGIN C2PA MANIFEST-----` / `-----END C2PA MANIFEST-----`) inside a
//! single-line `NOTE` comment placed immediately after the `WEBVTT` signature,
//! as specified by the C2PA structured text embedding section. Placement after
//! the signature (rather than at the start of the file) is what makes this
//! WebVTT-specific: the `WEBVTT` line is a reserved header that must come first.
//!
//! ```text
//! WEBVTT
//!
//! NOTE -----BEGIN C2PA MANIFEST----- https://example.com/m.c2pa -----END C2PA MANIFEST-----
//!
//! 00:00:00.000 --> 00:00:05.000
//! Hello world
//! ```
//!
//! # Scope relative to `c2pa-structured-text`
//!
//! `c2pa-structured-text` implements the general structured text embedding
//! method and lists WebVTT as one comment style. This crate is the **canonical
//! WebVTT implementation**: the general method's "prepend a comment line" rule
//! would place the block before the `WEBVTT` signature and produce an invalid
//! file, and the hard binding needs WebVTT-aware placement. `c2pa-structured-text`
//! documents the WebVTT delimiter but defers correct placement and hard binding
//! here.
//!
//! # Hard binding
//!
//! See [`binding`] for the byte-exact `c2pa.hash.data` data hash and the single
//! exclusion range covering the manifest block.
//!
//! # Validation
//!
//! See [`bridge`] for extracting the manifest store and delegating
//! signature/trust/assertion validation to the official C2PA SDK.
//!
//! # Features
//!
//! - `hash` (default): `sha2`-backed [`binding::compute_data_hash`] /
//!   [`binding::verify_data_hash`]. Disable with `default-features = false` for
//!   a zero-dependency embed/extract build.
//! - `c2pa` (off by default): the [`bridge::validate`] delegation to c2pa-rs.

mod base64;
pub mod binding;
pub mod bridge;
mod embed;
mod error;
mod extract;

pub use binding::{data_hash_exclusion, Exclusion, HashAlg};
pub use bridge::{extract_manifest_source, ManifestSource};
pub use embed::{embed_manifest, remove_manifest, ManifestRef};
pub use error::Error;
pub use extract::{extract_manifest, ExtractionResult};

#[cfg(feature = "hash")]
pub use binding::{compute_data_hash, verify_data_hash};
