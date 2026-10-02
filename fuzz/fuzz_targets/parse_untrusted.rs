#![no_main]

use c2pa_vtt::{extract_manifest, extract_manifest_source};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    let _ = extract_manifest(data);
    let _ = extract_manifest_source(data);
});
