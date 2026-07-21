//! Runtime proof of the c2pa-rs validation bridge over a real signed manifest.
//!
//! Builds a genuinely signed C2PA Manifest Store whose `c2pa.hash.data`
//! assertion carries the exclusion range and hash this crate computes for a
//! WebVTT file, then validates it back through [`c2pa_vtt::bridge::validate`].
//! Confirms that c2pa-rs accepts `format = "text/vtt"` (no `UnsupportedType`),
//! runs COSE signature validation, and validates the WebVTT hard binding via
//! its format-independent data-hash path — matching what this crate computes.
#![cfg(feature = "c2pa")]

use c2pa::assertions::DataHash;
use c2pa::{create_signer, Builder, HashRange, SigningAlg};
use c2pa_vtt::{
    bridge, compute_data_hash, data_hash_exclusion, embed_manifest, HashAlg, ManifestRef,
};

const CERT: &[u8] = include_bytes!("fixtures/certs/es256.pub");
const KEY: &[u8] = include_bytes!("fixtures/certs/es256.pem");

const MANIFEST_JSON: &str = r#"{
    "claim_generator_info": [{ "name": "c2pa-vtt-test", "version": "0.0.0" }],
    "title": "captions.vtt",
    "assertions": [
        { "label": "c2pa.actions", "data": { "actions": [{ "action": "c2pa.created" }] } }
    ]
}"#;

/// Sign a standalone manifest store carrying this crate's WebVTT data hash.
fn sign_store_for(signed_vtt: &str) -> Vec<u8> {
    let ex = data_hash_exclusion(signed_vtt).unwrap();
    let hash = compute_data_hash(signed_vtt, HashAlg::Sha256).unwrap();

    let signer = create_signer::from_keys(CERT, KEY, SigningAlg::Es256, None).unwrap();
    let mut builder = Builder::from_json(MANIFEST_JSON).unwrap();
    builder
        .data_hashed_placeholder(signer.reserve_size(), "application/c2pa")
        .unwrap();

    let mut dh = DataHash::new("webvtt", HashAlg::Sha256.c2pa_id());
    dh.exclusions = Some(vec![HashRange::new(ex.start as u64, ex.length as u64)]);
    dh.set_hash(hash);

    builder
        .sign_data_hashed_embeddable(signer.as_ref(), &dh, "application/c2pa")
        .unwrap()
}

#[test]
fn bridge_validates_real_signed_hard_binding() {
    let clean = "WEBVTT\n\n00:00:00.000 --> 00:00:05.000\nHello world\n";
    let signed_vtt = embed_manifest(clean, ManifestRef::Url("urn:uuid:test")).unwrap();
    let store = sign_store_for(&signed_vtt);

    // Delegation runs: text/vtt is accepted, signature + hard binding validate.
    let reader = bridge::validate(&signed_vtt, &store).expect("c2pa reader");
    let results = reader.validation_results().expect("validation results");

    let failures: Vec<&str> = results.failure().iter().map(|s| s.code()).collect();
    // No hard-binding failure: our exclusion + hash matched c2pa-rs's own check.
    assert!(
        !failures.iter().any(|c| c.contains("dataHash")),
        "unexpected data-hash failure: {failures:?}"
    );
    // Positively: c2pa-rs reports the data hash matched, and the signature is valid.
    let successes: Vec<&str> = results.success().iter().map(|s| s.code()).collect();
    assert!(
        successes.contains(&"assertion.dataHash.match"),
        "expected assertion.dataHash.match; successes={successes:?}"
    );
    assert!(
        successes.iter().any(|c| c.contains("claimSignature")),
        "expected a claimSignature success; successes={successes:?}"
    );
}

#[test]
fn bridge_detects_tampered_cue() {
    let clean = "WEBVTT\n\n00:00:00.000 --> 00:00:05.000\nHello world\n";
    let signed_vtt = embed_manifest(clean, ManifestRef::Url("urn:uuid:test")).unwrap();
    let store = sign_store_for(&signed_vtt);

    // Alter a cue outside the manifest block: c2pa-rs must fail the hard binding.
    let tampered = signed_vtt.replace("Hello world", "Goodbye world");
    let reader = bridge::validate(&tampered, &store).expect("c2pa reader");
    let failures: Vec<&str> = reader
        .validation_results()
        .expect("validation results")
        .failure()
        .iter()
        .map(|s| s.code())
        .collect();
    assert!(
        failures.contains(&"assertion.dataHash.mismatch"),
        "tamper not detected; failures={failures:?}"
    );
}
