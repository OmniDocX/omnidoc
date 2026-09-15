use std::collections::BTreeMap;
use std::path::PathBuf;

use unippt_core::{export_pptx, import_pptx};

#[test]
#[ignore = "requires the local 9.pptx source corpus"]
fn imports_all_generic_property_animation_behaviors_from_9_pptx() {
    let path = std::env::var_os("UNIPPT_ANIMATION_CORPUS")
        .map(PathBuf::from)
        .expect("Set UNIPPT_ANIMATION_CORPUS to the external corpus fixture path");
    let source = std::fs::read(path).expect("read 9.pptx animation fixture");
    let deck = import_pptx(&source).expect("import 9.pptx");
    let effects = deck.slides.iter().flat_map(|slide| {
        slide
            .inherited_animations
            .iter()
            .chain(slide.animations.iter())
    });

    let mut attributes = BTreeMap::<String, usize>::new();
    let mut behavior_count = 0usize;
    let mut formula_count = 0usize;
    let mut bounce_count = 0usize;
    for effect in effects {
        for behavior in &effect.property_animations {
            behavior_count += 1;
            bounce_count += usize::from(behavior.bounce_end.is_some());
            for attribute in &behavior.attributes {
                *attributes.entry(attribute.clone()).or_default() += 1;
            }
            formula_count += behavior
                .keyframes
                .iter()
                .filter(|keyframe| keyframe.formula.is_some())
                .count();
            assert!(
                effect.target_object_id.is_some(),
                "p:anim spid must bind to a scene object"
            );
        }
    }

    // The package contains two p:timing branches per slide (native Choice and
    // fallback), hence 1,114 raw p:anim elements.  The active loss-aware scene
    // projection intentionally models one branch: 557 behaviors.
    assert_eq!(behavior_count, 557);
    assert_eq!(attributes.get("ppt_h"), Some(&157));
    assert_eq!(attributes.get("ppt_w"), Some(&157));
    assert_eq!(attributes.get("ppt_y"), Some(&119));
    assert_eq!(attributes.get("ppt_x"), Some(&118));
    assert_eq!(attributes.get("style.rotation"), Some(&6));
    assert_eq!(formula_count, 5);
    assert_eq!(bounce_count, 64);

    let output = export_pptx(&deck, Some(&source)).expect("no-op native export");
    assert_eq!(
        output, source,
        "structured p:anim projection must not rewrite untouched XML"
    );
}
