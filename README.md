<p align="center">
  <h1 align="center">c2pa-vtt</h1>
  <p align="center">C2PA manifest embedding for WebVTT subtitle and caption files</p>
</p>

<p align="center">
  <a href="https://crates.io/crates/c2pa-vtt"><img src="https://img.shields.io/crates/v/c2pa-vtt.svg" alt="crates.io"></a>
  <a href="https://docs.rs/c2pa-vtt"><img src="https://docs.rs/c2pa-vtt/badge.svg" alt="docs.rs"></a>
  <a href="#license"><img src="https://img.shields.io/crates/l/c2pa-vtt.svg" alt="License"></a>
</p>

## Overview

Embeds and extracts C2PA manifest references in [WebVTT](https://www.w3.org/TR/webvtt1/) files using NOTE blocks with ASCII armour delimiters. The manifest is placed immediately after the `WEBVTT` header, where it survives HLS/DASH segmentation.

```
WEBVTT

NOTE -----BEGIN C2PA MANIFEST----- https://example.com/m.c2pa -----END C2PA MANIFEST-----

00:00:00.000 --> 00:00:05.000
Hello world
```

Zero dependencies.

## Quick Start

```toml
[dependencies]
c2pa-vtt = "0.1"
```

### Embed

```rust
use c2pa_vtt::embed_manifest;

let vtt = "WEBVTT\n\n00:00:00.000 --> 00:00:05.000\nHello\n";
let signed = embed_manifest(vtt, "https://example.com/m.c2pa").unwrap();
```

### Extract

```rust
use c2pa_vtt::extract_manifest;

let result = extract_manifest(signed_vtt).unwrap();
assert_eq!(result.reference, "https://example.com/m.c2pa");
```

### Remove

```rust
use c2pa_vtt::remove_manifest;

let clean = remove_manifest(signed_vtt).unwrap();
```

## Related Crates

| Crate | Description |
|---|---|
| [c2pa-structured-text](https://crates.io/crates/c2pa-structured-text) | Structured text embedding via ASCII armour delimiters |
| [c2pa-text-binding](https://crates.io/crates/c2pa-text-binding) | Soft binding and content fingerprinting for text assets |
| [c2pa-warc](https://github.com/writerslogic/c2pa-warc) | WARC web archive embedding |
| [c2pa-rs](https://crates.io/crates/c2pa) | Official C2PA SDK |

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.

Built by [WritersLogic](https://writerslogic.com)
