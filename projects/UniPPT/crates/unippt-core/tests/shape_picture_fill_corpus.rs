use std::path::PathBuf;

use unippt_core::{export_pptx, import_pptx, SceneObject};

fn picture_fill_objects(objects: &[SceneObject]) -> Vec<&SceneObject> {
    let mut output = Vec::new();
    for object in objects {
        if object.shape_fill_asset.is_some() {
            output.push(object);
        }
        output.extend(picture_fill_objects(&object.children));
    }
    output
}

#[test]
#[ignore = "requires the local 1.pptx source corpus"]
fn imports_all_native_shape_picture_fills_and_keeps_package_byte_exact() {
    let path = std::env::var_os("UNIPPT_SHAPE_FILL_CORPUS")
        .map(PathBuf::from)
        .expect("Set UNIPPT_SHAPE_FILL_CORPUS to the external corpus fixture path");
    let source = std::fs::read(&path).expect("read 1.pptx picture-fill fixture");
    let deck = import_pptx(&source).expect("import 1.pptx");

    let expected = [(5usize, 3usize), (8, 2), (13, 4), (16, 5)];
    let mut total = 0;
    for (slide_number, expected_count) in expected {
        let objects = picture_fill_objects(&deck.slides[slide_number - 1].objects);
        assert_eq!(
            objects.len(),
            expected_count,
            "unexpected picture-fill count on slide {slide_number}"
        );
        for object in objects {
            let asset = object.shape_fill_asset.as_deref().unwrap();
            assert!(asset.starts_with("data:image/"));
            assert_eq!(object.asset.as_deref(), Some(asset));
            assert!(object.style.fill.contains(asset));
            assert!(object.style.fill.starts_with("url(\"data:image/"));
        }
        total += expected_count;
    }
    assert_eq!(
        deck.slides
            .iter()
            .map(|slide| picture_fill_objects(&slide.objects).len())
            .sum::<usize>(),
        total
    );

    let output = export_pptx(&deck, Some(&source)).expect("no-op native export");
    assert_eq!(
        output, source,
        "untouched picture fills must preserve the source package byte exactly"
    );
}
