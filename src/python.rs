//! Python bindings, built with [maturin]/[PyO3] behind the `python` feature and
//! published to PyPI as `c2pa-vtt`.
//!
//! WebVTT is text, so cues map to and from Python `str`; an embedded Manifest
//! Store is `bytes`. A file carrying no manifest returns `None` from
//! [`extract_manifest`](fn.extract_manifest.html) rather than raising, because
//! absence of provenance is not an error.
//!
//! [maturin]: https://www.maturin.rs/
//! [PyO3]: https://pyo3.rs/

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};

use crate::embed::ManifestRef;

fn map_err(e: crate::Error) -> PyErr {
    match e.code() {
        Some(code) => PyValueError::new_err(format!("{e} [{code}]")),
        None => PyValueError::new_err(e.to_string()),
    }
}

#[cfg(feature = "hash")]
fn algorithm(alg: &str) -> PyResult<crate::HashAlg> {
    match alg {
        "sha256" => Ok(crate::HashAlg::Sha256),
        "sha384" => Ok(crate::HashAlg::Sha384),
        "sha512" => Ok(crate::HashAlg::Sha512),
        other => Err(PyValueError::new_err(format!(
            "unsupported hash algorithm: {other} [algorithm.unsupported]"
        ))),
    }
}

/// Embed a reference to an external C2PA Manifest Store. The preferred form.
#[pyfunction]
fn embed_manifest_url(vtt: &str, url: &str) -> PyResult<String> {
    crate::embed_manifest(vtt, ManifestRef::Url(url)).map_err(map_err)
}

/// Embed a Manifest Store inline as a `data:application/c2pa;base64,...` URI.
#[pyfunction]
fn embed_manifest(vtt: &str, store: &[u8]) -> PyResult<String> {
    crate::embed_manifest(vtt, ManifestRef::Embedded(store)).map_err(map_err)
}

/// Remove the manifest block from a WebVTT file.
#[pyfunction]
fn remove_manifest(vtt: &str) -> PyResult<String> {
    crate::remove_manifest(vtt).map_err(map_err)
}

/// The manifest block, or `None` when the file carries none.
///
/// Returns a dict with `reference`, `offset`, and `length`.
#[pyfunction]
fn extract_manifest<'py>(py: Python<'py>, vtt: &str) -> PyResult<Option<Bound<'py, PyDict>>> {
    let found = match crate::extract_manifest(vtt) {
        Ok(r) => r,
        // Carrying no manifest is not a failure; more than one is.
        Err(crate::Error::NotFound) => return Ok(None),
        Err(e) => return Err(map_err(e)),
    };
    let out = PyDict::new(py);
    out.set_item("reference", found.reference.as_str())?;
    out.set_item("offset", found.offset)?;
    out.set_item("length", found.length)?;
    Ok(Some(out))
}

/// The `(start, length)` exclusion range for the `c2pa.hash.data` assertion.
#[pyfunction]
fn data_hash_exclusion(vtt: &str) -> PyResult<(usize, usize)> {
    let ex = crate::data_hash_exclusion(vtt).map_err(map_err)?;
    Ok((ex.start, ex.length))
}

/// Compute the `c2pa.hash.data` value over the file with the manifest block
/// excluded. `alg` is one of `sha256`, `sha384`, `sha512`.
#[cfg(feature = "hash")]
#[pyfunction]
#[pyo3(signature = (vtt, alg = "sha256"))]
fn compute_data_hash<'py>(py: Python<'py>, vtt: &str, alg: &str) -> PyResult<Bound<'py, PyBytes>> {
    let digest = crate::compute_data_hash(vtt, algorithm(alg)?).map_err(map_err)?;
    Ok(PyBytes::new(py, &digest))
}

/// Whether the recomputed data hash matches `expected`.
#[cfg(feature = "hash")]
#[pyfunction]
#[pyo3(signature = (vtt, expected, alg = "sha256"))]
fn verify_data_hash(vtt: &str, expected: &[u8], alg: &str) -> PyResult<bool> {
    crate::verify_data_hash(vtt, algorithm(alg)?, expected).map_err(map_err)
}

#[pymodule]
fn c2pa_vtt(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(embed_manifest, m)?)?;
    m.add_function(wrap_pyfunction!(embed_manifest_url, m)?)?;
    m.add_function(wrap_pyfunction!(remove_manifest, m)?)?;
    m.add_function(wrap_pyfunction!(extract_manifest, m)?)?;
    m.add_function(wrap_pyfunction!(data_hash_exclusion, m)?)?;
    #[cfg(feature = "hash")]
    {
        m.add_function(wrap_pyfunction!(compute_data_hash, m)?)?;
        m.add_function(wrap_pyfunction!(verify_data_hash, m)?)?;
    }
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
