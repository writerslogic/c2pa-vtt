//! WebAssembly bindings, built only for the `wasm32` target and published to
//! npm as `c2pa-vtt`.
//!
//! Cues map to and from JavaScript strings; an embedded Manifest Store is a
//! `Uint8Array`. A file carrying no manifest returns `null` from
//! [`extractManifest`](fn.extract_manifest.html) rather than throwing, because
//! absence of provenance is not an error.

use wasm_bindgen::prelude::*;

use crate::embed::ManifestRef;

fn js_err(e: crate::Error) -> JsError {
    match e.code() {
        Some(code) => JsError::new(&format!("{e} [{code}]")),
        None => JsError::new(&e.to_string()),
    }
}

#[cfg(feature = "hash")]
fn algorithm(alg: &str) -> Result<crate::HashAlg, JsError> {
    match alg {
        "sha256" => Ok(crate::HashAlg::Sha256),
        "sha384" => Ok(crate::HashAlg::Sha384),
        "sha512" => Ok(crate::HashAlg::Sha512),
        other => Err(JsError::new(&format!(
            "unsupported hash algorithm: {other} [algorithm.unsupported]"
        ))),
    }
}

/// Embed a reference to an external C2PA Manifest Store. The preferred form.
#[wasm_bindgen(js_name = embedManifestUrl)]
pub fn embed_manifest_url(vtt: &str, url: &str) -> Result<String, JsError> {
    crate::embed_manifest(vtt, ManifestRef::Url(url)).map_err(js_err)
}

/// Embed a Manifest Store inline as a `data:application/c2pa;base64,...` URI.
#[wasm_bindgen(js_name = embedManifest)]
pub fn embed_manifest(vtt: &str, store: &[u8]) -> Result<String, JsError> {
    crate::embed_manifest(vtt, ManifestRef::Embedded(store)).map_err(js_err)
}

/// Remove the manifest block from a WebVTT file.
#[wasm_bindgen(js_name = removeManifest)]
pub fn remove_manifest(vtt: &str) -> Result<String, JsError> {
    crate::remove_manifest(vtt).map_err(js_err)
}

/// The manifest block, or `null` when the file carries none.
///
/// Returns an object with `reference`, `offset`, and `length`.
#[wasm_bindgen(js_name = extractManifest)]
pub fn extract_manifest(vtt: &str) -> Result<JsValue, JsError> {
    let found = match crate::extract_manifest(vtt) {
        Ok(r) => r,
        Err(crate::Error::NotFound) => return Ok(JsValue::NULL),
        Err(e) => return Err(js_err(e)),
    };
    let out = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&out, &"reference".into(), &found.reference.as_str().into());
    let _ = js_sys::Reflect::set(&out, &"offset".into(), &(found.offset as u32).into());
    let _ = js_sys::Reflect::set(&out, &"length".into(), &(found.length as u32).into());
    Ok(out.into())
}

/// The exclusion range for the `c2pa.hash.data` assertion, as
/// `{ start, length }`.
#[wasm_bindgen(js_name = dataHashExclusion)]
pub fn data_hash_exclusion(vtt: &str) -> Result<JsValue, JsError> {
    let ex = crate::data_hash_exclusion(vtt).map_err(js_err)?;
    let out = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&out, &"start".into(), &(ex.start as u32).into());
    let _ = js_sys::Reflect::set(&out, &"length".into(), &(ex.length as u32).into());
    Ok(out.into())
}

/// Compute the `c2pa.hash.data` value over the file with the manifest block
/// excluded. `alg` is one of `sha256`, `sha384`, `sha512`.
#[cfg(feature = "hash")]
#[wasm_bindgen(js_name = computeDataHash)]
pub fn compute_data_hash(vtt: &str, alg: &str) -> Result<Vec<u8>, JsError> {
    crate::compute_data_hash(vtt, algorithm(alg)?).map_err(js_err)
}

/// Whether the recomputed data hash matches `expected`.
#[cfg(feature = "hash")]
#[wasm_bindgen(js_name = verifyDataHash)]
pub fn verify_data_hash(vtt: &str, expected: &[u8], alg: &str) -> Result<bool, JsError> {
    crate::verify_data_hash(vtt, algorithm(alg)?, expected).map_err(js_err)
}
