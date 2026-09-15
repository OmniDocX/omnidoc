use std::path::PathBuf;

use pptx::dml::fill::FillFormat;
use pptx::shapes::Shape;
use pptx::Presentation;
use unippt_core::{import_pptx, SceneObject};

fn assert_group_fill_inheritance(
    shape: &Shape,
    object: &SceneObject,
    inherited_solid_fill: bool,
) -> usize {
    match shape {
        Shape::AutoShape(shape) if matches!(shape.fill, Some(FillFormat::Background)) => {
            assert!(
                inherited_solid_fill,
                "{} has no solid group fill",
                shape.name
            );
            assert_ne!(
                object.style.fill, "transparent",
                "{} lost grpFill",
                shape.name
            );
            1
        }
        Shape::GroupShape(group) => {
            assert_eq!(group.shapes.len(), object.children.len());
            let group_has_solid_fill = match group.fill.as_ref() {
                Some(FillFormat::Solid(_)) => true,
                Some(FillFormat::Background) | None => inherited_solid_fill,
                _ => false,
            };
            group
                .shapes
                .iter()
                .zip(&object.children)
                .map(|(shape, object)| {
                    assert_group_fill_inheritance(shape, object, group_has_solid_fill)
                })
                .sum()
        }
        _ => 0,
    }
}

#[test]
#[ignore = "requires the local 1.pptx source corpus"]
fn imports_all_525_group_fill_shapes_with_inherited_color() {
    let path = std::env::var_os("UNIPPT_GROUP_FILL_CORPUS")
        .map(PathBuf::from)
        .expect("Set UNIPPT_GROUP_FILL_CORPUS to the external corpus fixture path");
    let source = std::fs::read(path).expect("read 1.pptx group-fill fixture");
    let presentation = Presentation::from_bytes(&source).expect("parse native presentation");
    let slide_refs = presentation.slides().expect("enumerate slides");
    let deck = import_pptx(&source).expect("import 1.pptx");
    assert_eq!(slide_refs.len(), deck.slides.len());

    let mut count = 0;
    for (slide_ref, slide) in slide_refs.iter().zip(&deck.slides) {
        let tree = presentation
            .slide_shape_tree(slide_ref)
            .expect("parse slide shape tree");
        assert_eq!(tree.shapes.len(), slide.objects.len());
        count += tree
            .shapes
            .iter()
            .zip(&slide.objects)
            .map(|(shape, object)| assert_group_fill_inheritance(shape, object, false))
            .sum::<usize>();
    }

    assert_eq!(count, 525);
}
