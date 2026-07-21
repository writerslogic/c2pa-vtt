//! End-to-end generate/verify over real WebVTT files with multiple cues,
//! author comments (single- and multi-line), STYLE and REGION blocks.

use c2pa_vtt::{
    data_hash_exclusion, embed_manifest, extract_manifest, extract_manifest_source,
    remove_manifest, Error, ManifestRef, ManifestSource,
};

const CAPTIONS: &str = include_str!("fixtures/captions.vtt");
const MINIMAL: &str = include_str!("fixtures/minimal.vtt");

#[test]
fn generate_then_verify_reference_symmetric() {
    for original in [CAPTIONS, MINIMAL] {
        // No manifest to begin with.
        assert!(matches!(extract_manifest(original), Err(Error::NotFound)));

        let reference = "https://cdn.example.com/manifests/abc123.c2pa";
        let signed = embed_manifest(original, ManifestRef::Url(reference)).unwrap();

        // Still a valid WebVTT file: signature first, NOTE after it.
        assert!(signed.starts_with("WEBVTT"));
        assert!(signed.find("WEBVTT").unwrap() < signed.find("NOTE -----BEGIN").unwrap());

        // Symmetric: what we embedded is what we extract.
        let extracted = extract_manifest(&signed).unwrap();
        assert_eq!(extracted.reference, reference);
        assert_eq!(
            extract_manifest_source(&signed).unwrap(),
            ManifestSource::Url(reference.to_string())
        );

        // The exclusion range is precisely the NOTE block, and everything else
        // (cues, STYLE, REGION, author comments) is preserved.
        let ex = data_hash_exclusion(&signed).unwrap();
        let excluded = &signed[ex.start..ex.start + ex.length];
        // The excluded range is the whole NOTE line including its terminator.
        assert!(excluded.ends_with('\n'));
        assert_eq!(
            excluded.trim(),
            "NOTE -----BEGIN C2PA MANIFEST----- https://cdn.example.com/manifests/abc123.c2pa -----END C2PA MANIFEST-----"
        );

        // Removing the block yields a manifest-free file with content intact.
        let cleaned = remove_manifest(&signed).unwrap();
        assert!(matches!(extract_manifest(&cleaned), Err(Error::NotFound)));
        assert!(!cleaned.contains("BEGIN C2PA MANIFEST"));
    }
}

#[test]
fn styling_and_comments_survive_round_trip() {
    let signed = embed_manifest(CAPTIONS, ManifestRef::Url("urn:uuid:1")).unwrap();
    // Author NOTE comments must not be confused with the manifest block.
    assert!(signed.contains("This is an ordinary multi-line author comment."));
    assert!(signed.contains("NOTE A short single-line author comment."));
    // STYLE / REGION blocks and cue payloads are untouched.
    assert!(signed.contains("STYLE"));
    assert!(signed.contains("REGION"));
    assert!(signed.contains("id:speaker"));
    assert!(signed.contains("<b>Welcome</b> to the show."));
    assert!(signed.contains("00:00:08.000 --> 00:00:12.000"));
}

#[test]
fn embedded_data_uri_round_trip() {
    let store = b"\x00JUMBF-manifest-store-bytes\xff\xfe";
    let signed = embed_manifest(MINIMAL, ManifestRef::Embedded(store)).unwrap();
    match extract_manifest_source(&signed).unwrap() {
        ManifestSource::Embedded(bytes) => assert_eq!(bytes, store),
        other => panic!("expected embedded store, got {other:?}"),
    }
}

#[cfg(feature = "hash")]
#[test]
fn hard_binding_detects_tampering() {
    use c2pa_vtt::{compute_data_hash, verify_data_hash, HashAlg};

    let signed = embed_manifest(CAPTIONS, ManifestRef::Url("urn:uuid:2")).unwrap();

    for alg in [HashAlg::Sha256, HashAlg::Sha384, HashAlg::Sha512] {
        let hash = compute_data_hash(&signed, alg).unwrap();
        assert!(verify_data_hash(&signed, alg, &hash).unwrap());

        // Tamper a cue: hard binding must fail.
        let tampered = signed.replace("The final caption.", "The altered caption.");
        assert!(!verify_data_hash(&tampered, alg, &hash).unwrap());

        // Changing only the manifest reference must NOT break the binding,
        // because the whole NOTE block is excluded.
        let re_referenced = remove_manifest(&signed).unwrap();
        let re_signed =
            embed_manifest(&re_referenced, ManifestRef::Url("urn:uuid:different")).unwrap();
        assert!(verify_data_hash(&re_signed, alg, &hash).unwrap());
    }
}

#[test]
fn non_vtt_input_is_rejected() {
    assert!(matches!(
        embed_manifest("<html></html>", ManifestRef::Url("urn:x")),
        Err(Error::NotVtt)
    ));
    assert!(matches!(extract_manifest("plain text"), Err(Error::NotVtt)));
}
