use unippt_core::{import_pptx, import_pptx_compact, ASSET_REFERENCE_PREFIX};

#[test]
fn imports_real_powerpoint_fixture_into_scene() {
    let bytes = include_bytes!("fixtures/test1.pptx");
    let deck = import_pptx(bytes).expect("real PPTX fixture should import");

    assert_eq!(deck.format, "unippt");
    assert!(!deck.slides.is_empty());
    assert!(deck.width > 0.0 && deck.height > 0.0);
    assert!(deck.slides.iter().any(|slide| !slide.objects.is_empty()));
    serde_json::to_vec(&deck).expect("imported scene should serialize");
}

#[test]
fn compact_import_keeps_unique_assets_out_of_scene_json() {
    let bytes = include_bytes!("fixtures/test1.pptx");
    let compact = import_pptx_compact(bytes).expect("compact PPTX import should succeed");
    let compact_json = serde_json::to_vec(&compact.deck).expect("compact scene should serialize");

    assert!(compact
        .assets
        .iter()
        .all(|asset| asset.id.len() == 64 && !asset.bytes.is_empty()));
    for asset in &compact.assets {
        let reference = format!("{ASSET_REFERENCE_PREFIX}{}", asset.id);
        assert!(compact_json
            .windows(reference.len())
            .any(|window| window == reference.as_bytes()));
        assert!(!compact_json
            .windows(asset.data_uri_prefix.len())
            .any(|window| window == asset.data_uri_prefix.as_bytes()));
    }

    let inline = compact.into_inline_deck();
    let legacy = import_pptx(bytes).expect("legacy inline import should succeed");
    assert_eq!(inline, legacy);
}
