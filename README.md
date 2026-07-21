<p align="center">
  <h1 align="center">c2pa-vtt</h1>
  <p align="center">C2PA manifest embedding and hard binding for WebVTT subtitle and caption files</p>
</p>

<p align="center">
  <a href="https://crates.io/crates/c2pa-vtt"><img src="https://img.shields.io/crates/v/c2pa-vtt.svg" alt="crates.io"></a>
  <a href="https://docs.rs/c2pa-vtt"><img src="https://docs.rs/c2pa-vtt/badge.svg" alt="docs.rs"></a>
  <a href="#license"><img src="https://img.shields.io/crates/l/c2pa-vtt.svg" alt="License"></a>
</p>

## Overview

Embeds, extracts, and hard-binds a C2PA Manifest Store reference in [WebVTT](https://www.w3.org/TR/webvtt1/) files. The manifest is carried in a single-line `NOTE` comment using the fixed ASCII armour delimiters, placed immediately after the `WEBVTT` signature (where it survives HLS/DASH segmentation), per the C2PA [structured text embedding](https://spec.c2pa.org/specifications/specifications/2.4/specs/C2PA_Specification.html) rules.

> **Canonical owner of WebVTT.** WebVTT is structured text per the specification, so the general [c2pa-structured-text](https://github.com/writerslogic/c2pa-structured-text) crate could embed into it via the `NOTE` comment style — but this crate owns `.vtt`. It targets the streaming-safe placement after the `WEBVTT` signature and validates the header. Use this crate for WebVTT; use `c2pa-structured-text` for other structured text.

```
WEBVTT

NOTE -----BEGIN C2PA MANIFEST----- https://example.com/m.c2pa -----END C2PA MANIFEST-----

00:00:00.000 --> 00:00:05.000
Hello world
```

## Scope: this crate vs. `c2pa-structured-text`

The C2PA structured text embedding section covers WebVTT as one comment style (`NOTE`), and [`c2pa-structured-text`](https://crates.io/crates/c2pa-structured-text) implements that general method. **This crate is the canonical WebVTT implementation; `c2pa-structured-text` documents the WebVTT delimiter but defers placement and hard binding here.** WebVTT needs format awareness that the general method lacks:

- **Placement.** The general "prepend a comment line" rule would put the block *before* the `WEBVTT` signature and produce an invalid file. WebVTT reserves its first line, so the block goes immediately after it, with blank-line separation.
- **Hard binding.** Computing and validating the `c2pa.hash.data` exclusion range requires locating the `NOTE` block within valid WebVTT structure (cues, `STYLE`/`REGION` blocks, author comments).

There is no silently-overlapping second implementation: `c2pa-structured-text` points to this crate for WebVTT.

## Hard binding

WebVTT is a structured text container, so its hard binding is a **byte-exact** [`c2pa.hash.data`](https://spec.c2pa.org/specifications/specifications/2.4/specs/C2PA_Specification.html) data hash carrying a **single exclusion range** that covers the manifest `NOTE` block. The hash is computed over the raw bytes of the file with that one range removed: the `WEBVTT` signature, every cue, `STYLE`/`REGION` block, and author comment is bound; only the manifest block itself is excluded.

- **No normalization.** Unlike the unstructured-text (Unicode Variation Selector) method, structured text hashing applies no Unicode normalization. The file is byte-stable on disk and the ASCII delimiters make the excluded range unambiguous; applying NFC would create false mismatches for legitimate NFD content in cue text.
- **Line terminators.** LF or CRLF only; the file's convention is preserved and bare CR is not supported (per the spec).
- **Exclusion range.** `[offset, offset + length)` of the `NOTE` line including its trailing terminator — exactly the value returned by [`data_hash_exclusion`] and the `offset`/`length` on [`extract_manifest`].

A byte-exact hard binding is therefore feasible and implemented here. It is fragile under re-encoding that changes bytes outside the manifest block (line-ending conversion, BOM insertion, trailing-whitespace edits) — inherent to any hard binding over a text container. For robustness against such transformations, pair it with a soft binding from [`c2pa-text-binding`](https://crates.io/crates/c2pa-text-binding).

## Quick Start

```toml
[dependencies]
c2pa-vtt = "0.2"
```

### Generate: embed a reference and compute the hard binding

```rust
use c2pa_vtt::{embed_manifest, compute_data_hash, ManifestRef, HashAlg};

let vtt = "WEBVTT\n\n00:00:00.000 --> 00:00:05.000\nHello\n";
let signed = embed_manifest(vtt, ManifestRef::Url("https://example.com/m.c2pa")).unwrap();

// The value for the c2pa.hash.data assertion's `hash` field (alg = HashAlg::c2pa_id).
let hash = compute_data_hash(&signed, HashAlg::Sha256).unwrap();
```

A manifest store may instead be embedded inline with `ManifestRef::Embedded(&store_bytes)`, encoded as a `data:application/c2pa;base64,...` reference.

### Verify: extract and check the hard binding

```rust
use c2pa_vtt::{extract_manifest, verify_data_hash, HashAlg};

let result = extract_manifest(&signed).unwrap();
assert_eq!(result.reference, "https://example.com/m.c2pa");

let ok = verify_data_hash(&signed, HashAlg::Sha256, &hash).unwrap();
```

### Remove

```rust
use c2pa_vtt::remove_manifest;

let clean = remove_manifest(&signed).unwrap();
```

## Validation bridge

Hard-binding validation is owned by this crate (c2pa-rs has no stable native `text/vtt` handler). Everything else — COSE signature, certificate trust, timestamp, and assertion validation of the manifest store — is delegated to [c2pa-rs](https://crates.io/crates/c2pa).

`extract_manifest_source` (zero dependencies) resolves the reference, decoding an inline `data:` URI to raw store bytes:

```rust
use c2pa_vtt::{extract_manifest_source, ManifestSource};

match extract_manifest_source(&signed).unwrap() {
    ManifestSource::Url(url) => { /* fetch the store; this crate performs no network I/O */ }
    ManifestSource::Embedded(bytes) => { /* raw C2PA Manifest Store */ }
}
```

With the `c2pa` feature enabled, `bridge::validate(vtt, &store_bytes)` hands the store to a c2pa-rs `Reader`; inspect `validation_state()` / `validation_status()`, and validate the WebVTT hard binding with `verify_data_hash`.

## Features

| Feature | Default | Adds |
|---|---|---|
| `hash` | ✅ | `sha2`-backed `compute_data_hash` / `verify_data_hash` |
| `c2pa` | | `bridge::validate` delegation to c2pa-rs (pulls in the `c2pa` crate) |

Build with `default-features = false` for a zero-dependency embed/extract build.

## Conformance

This crate implements the structured text embedding and data hash for WebVTT as specified, and delegates signature/trust validation to c2pa-rs. It makes **no conformance or certification claim**; validate against the [C2PA specification](https://spec.c2pa.org/) and a reference C2PA tool for interop.

## Related Crates

| Crate | Description |
|---|---|
| [c2pa-structured-text](https://crates.io/crates/c2pa-structured-text) | General structured text embedding (defers WebVTT to this crate) |
| [c2pa-text-binding](https://crates.io/crates/c2pa-text-binding) | Soft binding and content fingerprinting for text assets |
| [c2pa-warc](https://github.com/writerslogic/c2pa-warc) | WARC web archive embedding |
| [c2pa-rs](https://crates.io/crates/c2pa) | Official C2PA SDK |

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.

Built by [WritersLogic](https://writerslogic.com)
