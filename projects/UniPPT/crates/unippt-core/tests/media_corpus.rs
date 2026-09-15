use std::path::PathBuf;

use unippt_core::{export_pptx, import_pptx, MediaKind};

#[test]
#[ignore = "requires UNIPPT_CORPUS_DIR with external PPTX fixtures"]
fn imports_native_audio_and_keeps_media_packages_byte_identical() {
    let root = PathBuf::from(
        std::env::var("UNIPPT_CORPUS_DIR").expect("UNIPPT_CORPUS_DIR must point to PPTX corpus"),
    );
    for stem in ["1", "9", "17", "33"] {
        let path = root.join(format!("{stem}.pptx"));
        let source = std::fs::read(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let deck = import_pptx(&source)
            .unwrap_or_else(|error| panic!("cannot import {}: {error}", path.display()));
        let media_objects: Vec<_> = deck
            .slides
            .iter()
            .flat_map(|slide| slide.objects.iter())
            .filter(|object| object.media.is_some())
            .collect();
        assert!(
            media_objects.iter().any(|object| {
                let item = object.media.as_ref().unwrap();
                item.kind == MediaKind::Audio
                    && item
                        .asset
                        .as_deref()
                        .is_some_and(|asset| asset.starts_with("data:audio/"))
            }),
            "{} did not expose its native audio as a playable scene asset",
            path.display()
        );
        assert!(
            deck.slides.iter().any(|slide| {
                slide.animations.iter().any(|animation| {
                    animation.effect == unippt_core::AnimationKind::Media
                        && animation.media_action.as_deref() == Some("play")
                        && animation.target_object_id.as_deref().is_some_and(|target| {
                            media_objects.iter().any(|object| object.id == target)
                        })
                })
            }),
            "{} did not bind its native mediacall timing to the audio poster object",
            path.display()
        );

        let output = export_pptx(&deck, Some(&source))
            .unwrap_or_else(|error| panic!("cannot export {}: {error}", path.display()));
        assert_eq!(
            source,
            output,
            "no-op export changed media parts, relationships, timing, or package bytes for {}",
            path.display()
        );
    }
}
