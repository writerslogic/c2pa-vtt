<!-- repo-header:start -->
<img src="https://github.com/writerslogic.png?size=160" alt="c2pa-vtt logo" width="120" align="left">

<h1>c2pa-vtt</h1>

<p><strong>C2PA manifest embedding for WebVTT subtitle and caption files</strong></p>

<br clear="left">

[![CI](https://img.shields.io/github/actions/workflow/status/writerslogic/c2pa-vtt/ci.yml?style=flat-square&labelColor=20232a&branch=main&label=CI)](https://github.com/writerslogic/c2pa-vtt/actions/workflows/ci.yml) [![OpenSSF Scorecard](https://img.shields.io/ossf-scorecard/github.com/writerslogic/c2pa-vtt?style=flat-square&labelColor=20232a&label=OpenSSF)](https://securityscorecards.dev/viewer/?uri=github.com/writerslogic/c2pa-vtt) [![OpenSSF Best Practices](https://www.bestpractices.dev/projects/14417/badge)](https://www.bestpractices.dev/projects/14417) [![License](https://img.shields.io/github/license/writerslogic/c2pa-vtt?style=flat-square&labelColor=20232a&color=007ec6&label=license)](https://github.com/writerslogic/c2pa-vtt/blob/main/LICENSE-APACHE) [![Code of Conduct](https://img.shields.io/badge/code%20of%20conduct-Contributor%20Covenant%202.1-6a4c93?style=flat-square&labelColor=20232a)](https://github.com/writerslogic/c2pa-vtt/blob/main/CODE_OF_CONDUCT.md) [![C2PA](https://img.shields.io/badge/standard-C2PA%20related-6a4c93?style=flat-square&labelColor=20232a)](https://c2pa.org/) [![GitHub Sponsors](https://img.shields.io/badge/GitHub%20Sponsors-Sponsor-EA4AAA?style=flat-square&labelColor=20232a)](https://github.com/sponsors/dcondrey) <a href="https://crates.io/crates/c2pa-vtt"><img src="https://img.shields.io/crates/v/c2pa-vtt.svg?style=flat-square&labelColor=20232a&color=007ec6" alt="crates.io"></a> <a href="https://docs.rs/c2pa-vtt"><img src="https://img.shields.io/docsrs/c2pa-vtt?style=flat-square&labelColor=20232a&color=007ec6" alt="docs.rs"></a>
<!-- repo-header:end -->

## Overview

Embeds, extracts, and hard-binds a C2PA Manifest Store reference in [WebVTT](https://www.w3.org/TR/webvtt1/) files. The manifest is carried in a single-line `NOTE` comment using the fixed ASCII armour delimiters, placed immediately after the `WEBVTT` signature (where it survives HLS/DASH segmentation), per the C2PA [structured text embedding](https://spec.c2pa.org/specifications/specifications/2.4/specs/C2PA_Specification.html) rules.

> [!NOTE]
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

The same crate is published for JavaScript/WebAssembly and Python, built from this source:

```bash
npm install c2pa-vtt   # wasm-bindgen build
pip install c2pa-vtt   # PyO3 abi3 wheel, CPython 3.9+
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

Part of a family of single-purpose crates, one per C2PA embedding method. Each
is standalone and independently versioned.

| Crate | Description |
|---|---|
| [c2pa-structured-text](https://crates.io/crates/c2pa-structured-text) | Structured text: ASCII-armoured manifest in a comment or front matter |
| [c2pa-unstructured-text](https://crates.io/crates/c2pa-unstructured-text) | Unstructured text: invisible Unicode variation-selector run |
| [c2pa-html](https://crates.io/crates/c2pa-html) | HTML: `script` and `link` elements in the document head |
| [c2pa-http](https://crates.io/crates/c2pa-http) | HTTP: the `c2pa-manifest` `Link` header, with a Tower middleware |
| [c2pa-text-binding](https://crates.io/crates/c2pa-text-binding) | Soft binding and content fingerprinting for text assets |
| [c2pa-zip](https://crates.io/crates/c2pa-zip) | ZIP-based documents: EPUB, DOCX, ODT, OXPS |
| [c2pa-warc](https://crates.io/crates/c2pa-warc) | WARC web archive embedding (ISO 28500) |
| [c2pa-fonts](https://crates.io/crates/c2pa-fonts) | OpenType/TrueType (SFNT) font embedding |
| [c2pa-ml](https://crates.io/crates/c2pa-ml) | ML model containers: GGUF, SafeTensors, ONNX |
| [c2pa](https://crates.io/crates/c2pa) | Official C2PA SDK |

## Security

Found a vulnerability? Please report it privately — see [SECURITY.md](./SECURITY.md).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.

Built by [WritersLogic](https://writerslogic.com)
