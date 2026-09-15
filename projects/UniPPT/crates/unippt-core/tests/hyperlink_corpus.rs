use std::path::PathBuf;

use unippt_core::{export_pptx, import_pptx, SceneObject};

#[test]
#[ignore = "requires UNIPPT_HYPERLINK_FIXTURE with external run hyperlinks"]
fn imports_external_run_hyperlinks_without_promoting_them_to_shapes() {
    let path = PathBuf::from(
        std::env::var("UNIPPT_HYPERLINK_FIXTURE")
            .expect("UNIPPT_HYPERLINK_FIXTURE must point to a PPTX with run hyperlinks"),
    );
    let source = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    let deck = import_pptx(&source)
        .unwrap_or_else(|error| panic!("cannot import {}: {error}", path.display()));

    let mut run_links = Vec::new();
    let mut shape_links = Vec::new();
    for slide in &deck.slides {
        for object in slide.objects.iter().flat_map(walk_objects) {
            if let Some(link) = object.hyperlinks.click.as_ref() {
                if link
                    .target
                    .as_deref()
                    .is_some_and(|target| target.starts_with("http"))
                {
                    shape_links.push((slide.source_part_name.clone(), link.target.clone()));
                }
            }
            for paragraph in &object.text_paragraphs {
                for run in &paragraph.runs {
                    if let Some(link) = run.hyperlinks.click.as_ref() {
                        if link
                            .target
                            .as_deref()
                            .is_some_and(|target| target.starts_with("http"))
                        {
                            run_links.push((
                                slide.source_part_name.clone(),
                                object.source_shape_id,
                                paragraph.source_index,
                                run.source_index,
                                link.target.clone(),
                            ));
                        }
                    }
                }
            }
        }
    }

    assert!(
        run_links.len() >= 3,
        "{} should expose its three known external run hyperlinks; got {run_links:?}",
        path.display()
    );
    for expected in ["slide5.xml", "slide6.xml", "slide19.xml"] {
        assert!(
            run_links.iter().any(|(slide, ..)| slide
                .as_deref()
                .is_some_and(|part| part.ends_with(expected))),
            "missing run hyperlink on {expected}: {run_links:?}"
        );
    }
    assert!(
        shape_links.is_empty(),
        "run hyperlinks were incorrectly promoted to shapes: {shape_links:?}"
    );

    let output = export_pptx(&deck, Some(&source)).expect("no-op hyperlink export should succeed");
    assert_eq!(
        output, source,
        "no-op export changed the hyperlink fixture package"
    );
}

fn walk_objects(object: &SceneObject) -> Vec<&SceneObject> {
    let mut output = vec![object];
    for child in &object.children {
        output.extend(walk_objects(child));
    }
    output
}
