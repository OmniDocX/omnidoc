use std::path::PathBuf;

use unippt_core::{export_pptx, import_pptx, AnimationKind, AnimationTrigger, SceneObject};

fn flatten(objects: &[SceneObject]) -> Vec<&SceneObject> {
    let mut output = Vec::new();
    for object in objects {
        output.push(object);
        output.extend(flatten(&object.children));
    }
    output
}

#[test]
#[ignore = "requires the local 9.pptx source corpus"]
fn imports_layout_furniture_media_and_exact_native_font() {
    let path = std::env::var_os("UNIPPT_LAYOUT_CORPUS")
        .map(PathBuf::from)
        .expect("Set UNIPPT_LAYOUT_CORPUS to the external corpus fixture path");
    let source = std::fs::read(&path).expect("read 9.pptx layout fixture");
    let deck = import_pptx(&source).expect("import 9.pptx");
    let slide = &deck.slides[0];

    assert_eq!(slide.master_objects.len(), 0);
    assert_eq!(slide.layout_objects.len(), 2);
    assert_eq!(slide.objects.len(), 19);
    assert!(slide.layout_objects[0]
        .media
        .as_ref()
        .is_some_and(|media| media.mime_type.as_deref() == Some("video/x-msvideo")));
    assert!(slide
        .inherited_animations
        .iter()
        .any(|effect| effect.target_object_id == Some(slide.layout_objects[0].id.clone())));

    let subtitle = flatten(&slide.objects)
        .into_iter()
        .find(|object| object.source_shape_id == Some(22))
        .expect("subtitle shape 22");
    let run = &subtitle.text_paragraphs[0].runs[0];
    assert_eq!(run.native_font_family.as_deref(), Some("\u{6977}\u{4f53}"));
    assert!((run.font_size - (28.0 * 96.0 / 72.0)).abs() < 0.01);

    let gradient_title = flatten(&deck.slides[11].objects)
        .into_iter()
        .find(|object| object.source_shape_id == Some(105))
        .expect("slide 12 gradient title shape 105");
    let gradient = gradient_title.text_paragraphs[0].runs[0]
        .gradient
        .as_ref()
        .expect("slide 12 run-level DrawingML gradient");
    assert_eq!(gradient.stops.len(), 2);
    assert_eq!(gradient.stops[0].color, "#FFE09F");
    assert!((gradient.stops[0].opacity - 0.8).abs() < f64::EPSILON);
    assert!((gradient.stops[1].position - 0.93).abs() < f64::EPSILON);

    let translucent_layout_shape = deck
        .slides
        .iter()
        .flat_map(|slide| flatten(&slide.layout_objects))
        .find(|object| object.style.fill.contains("rgba(") && object.style.fill.contains("0.7000"))
        .expect("layout shape with native 70% DrawingML fill alpha");
    assert_eq!(
        translucent_layout_shape.style.opacity, 1.0,
        "fill alpha must not fade the shape's text or stroke"
    );

    let cropped_pictures = deck
        .slides
        .iter()
        .flat_map(|slide| {
            flatten(&slide.master_objects)
                .into_iter()
                .chain(flatten(&slide.layout_objects))
                .chain(flatten(&slide.objects))
        })
        .filter(|object| {
            object.image_crop.left != 0.0
                || object.image_crop.top != 0.0
                || object.image_crop.right != 0.0
                || object.image_crop.bottom != 0.0
        })
        .count();
    assert_eq!(
        cropped_pictures, 14,
        "all native p:pic/a:srcRect crops in 9.pptx must reach the browser scene"
    );

    let slide_2_soft_edge = flatten(&deck.slides[1].objects)
        .into_iter()
        .find(|object| object.source_shape_id == Some(23))
        .expect("slide 2 soft-edge picture 23");
    assert!(
        (slide_2_soft_edge.image_effects.soft_edge_radius - 66.666_667).abs() < 0.001,
        "a:softEdge rad=635000 must project to the scene in browser pixels"
    );

    let slide_12_bird_circle = flatten(&deck.slides[11].objects)
        .into_iter()
        .find(|object| object.source_shape_id == Some(40))
        .expect("slide 12 picture-filled bird circle");
    assert!(slide_12_bird_circle.shape_fill_asset.is_some());
    assert!((slide_12_bird_circle.image_fill_rect.left + 0.15048).abs() < f64::EPSILON);
    assert!((slide_12_bird_circle.image_fill_rect.right + 0.15048).abs() < f64::EPSILON);

    let slide_15_duotone = flatten(&deck.slides[14].objects)
        .into_iter()
        .find(|object| object.source_shape_id == Some(119))
        .expect("slide 15 duotone picture 119");
    let duotone = slide_15_duotone
        .image_effects
        .duotone
        .as_ref()
        .expect("native DrawingML duotone reaches the browser scene");
    assert_eq!(duotone.highlight_color, "#FFFFFF");
    assert_ne!(
        duotone.shadow_color, "#E97798",
        "shade/satMod must be applied"
    );

    assert!(deck.slides.iter().all(|slide| slide
        .inherited_animations
        .iter()
        .chain(&slide.animations)
        .all(|effect| effect.trigger != AnimationTrigger::OnClick)),
        "9.pptx contains only onBegin automatic main sequences; the importer must not invent click batches"
    );
    let animations = deck
        .slides
        .iter()
        .flat_map(|slide| slide.inherited_animations.iter().chain(&slide.animations))
        .collect::<Vec<_>>();
    assert_eq!(
        animations.len(),
        464,
        "all native effects must stay ordered"
    );
    for (kind, expected) in [
        (AnimationKind::RandomBars, 49),
        (AnimationKind::Dissolve, 27),
        (AnimationKind::Wheel, 20),
        (AnimationKind::Circle, 11),
        (AnimationKind::Split, 7),
        (AnimationKind::MotionPath, 7),
    ] {
        assert_eq!(
            animations
                .iter()
                .filter(|effect| effect.effect == kind)
                .count(),
            expected,
            "native preset {kind:?} must not collapse to a fade/custom preview"
        );
    }
    assert!(
        animations
            .iter()
            .all(|effect| effect.effect != AnimationKind::Custom),
        "all 9.pptx preset/filter pairs are now structurally mapped"
    );
    let slide4_gap = deck.slides[3]
        .animations
        .iter()
        .find(|effect| effect.source_timing_id == Some(33))
        .expect("slide 4 native timing node 33");
    assert_eq!(
        slide4_gap.delay_ms, 100,
        "wrapper cTn time slots must preserve the native 100 ms idle gap"
    );

    let expected_transition_kinds = [
        "none",
        "ripple",
        "wind",
        "pageCurlDouble",
        "pageCurlDouble",
        "pageCurlDouble",
        "pageCurlDouble",
        "wind",
        "pageCurlDouble",
        "pageCurlDouble",
        "pageCurlDouble",
        "pageCurlDouble",
        "wind",
        "pageCurlDouble",
        "pageCurlDouble",
        "pageCurlDouble",
        "pageCurlDouble",
        "pageCurlDouble",
        "pageCurlDouble",
        "pageCurlDouble",
        "wind",
        "pageCurlDouble",
        "pageCurlDouble",
        "pageCurlDouble",
        "pageCurlDouble",
        "pageCurlDouble",
        "wind",
    ];
    for (index, (slide, expected_kind)) in deck
        .slides
        .iter()
        .zip(expected_transition_kinds)
        .enumerate()
        .skip(1)
    {
        let transition = slide
            .transition
            .as_ref()
            .unwrap_or_else(|| panic!("slide {} lost its native transition", index + 1));
        assert_eq!(
            transition.kind,
            expected_kind,
            "slide {} must project its native AlternateContent transition instead of the fade fallback",
            index + 1,
        );
    }
    assert!(deck.slides[1]
        .source_transition_xml
        .as_deref()
        .is_some_and(|xml| xml.contains("p14:ripple")));
    assert!(deck.slides[2]
        .source_transition_xml
        .as_deref()
        .is_some_and(|xml| xml.contains("prst=\"wind\"")));
    assert!(deck.slides[3]
        .source_transition_xml
        .as_deref()
        .is_some_and(|xml| xml.contains("prst=\"pageCurlDouble\"")));

    let output = export_pptx(&deck, Some(&source)).expect("no-op native export");
    assert_eq!(
        output, source,
        "inherited projections must not patch slide XML"
    );
}
