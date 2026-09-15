use std::path::PathBuf;

use pptx::Presentation;

#[test]
#[ignore = "requires UNIPPT_TABLE_FIXTURE with a native table graphicFrame"]
fn real_table_graphic_frames_keep_direct_transform_geometry() {
    let path = PathBuf::from(
        std::env::var("UNIPPT_TABLE_FIXTURE")
            .expect("UNIPPT_TABLE_FIXTURE must point to a PPTX containing a table"),
    );
    let bytes = std::fs::read(&path).expect("fixture should be readable");
    let presentation = Presentation::from_bytes(&bytes).expect("fixture should open");
    let slides = presentation.slides().expect("slides should resolve");
    let mut tables = Vec::new();
    for slide in &slides {
        let tree = presentation
            .slide_shape_tree(slide)
            .expect("slide shape tree should parse");
        for shape in &tree.shapes {
            if let Some(frame) = shape.as_graphic_frame().filter(|frame| frame.has_table) {
                tables.push((slide.partname.to_string(), frame.clone()));
            }
        }
    }

    assert!(!tables.is_empty(), "{} contains no table", path.display());
    for (slide, frame) in tables {
        assert!(
            frame.width.0 > 0 && frame.height.0 > 0,
            "table {} on {slide} collapsed to {}x{} EMU",
            frame.shape_id.0,
            frame.width.0,
            frame.height.0
        );
    }
}
