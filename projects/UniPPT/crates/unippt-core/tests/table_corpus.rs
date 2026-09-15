use std::path::PathBuf;

use unippt_core::{import_pptx, ObjectKind, SceneObject};

#[test]
#[ignore = "requires UNIPPT_TABLE_FIXTURE with a native p:graphicFrame/a:tbl"]
fn imports_external_native_table_as_structured_scene_data() {
    let path = PathBuf::from(
        std::env::var("UNIPPT_TABLE_FIXTURE")
            .expect("UNIPPT_TABLE_FIXTURE must point to a PPTX containing a native table"),
    );
    let bytes = std::fs::read(&path).expect("table fixture should be readable");
    let deck = import_pptx(&bytes).expect("table fixture should import");
    let tables = deck
        .slides
        .iter()
        .flat_map(|slide| slide.objects.iter())
        .flat_map(walk_objects)
        .filter(|object| object.kind == ObjectKind::Table)
        .collect::<Vec<_>>();

    assert!(
        !tables.is_empty(),
        "{} contains no imported table",
        path.display()
    );
    assert!(tables.iter().all(|object| object.table.is_some()));
    assert!(
        tables
            .iter()
            .all(|object| object.frame.width > 1.0 && object.frame.height > 1.0),
        "native table graphicFrame geometry collapsed in {}: {:?}",
        path.display(),
        tables
            .iter()
            .map(|object| (object.source_shape_id, object.frame))
            .collect::<Vec<_>>()
    );
    assert!(tables.iter().any(|object| {
        object.table.as_ref().is_some_and(|table| {
            !table.columns.is_empty()
                && !table.rows.is_empty()
                && table.rows.iter().any(|row| !row.cells.is_empty())
        })
    }));
    for object in &tables {
        let table = object.table.as_ref().unwrap();
        let grid_width = table.columns.iter().sum::<f64>();
        let grid_height = table.rows.iter().map(|row| row.height).sum::<f64>();
        assert!(
            (object.frame.width - grid_width).abs() <= (object.frame.width * 0.01).max(2.0),
            "table {:?} frame/grid widths differ in {}: {} vs {}",
            object.source_shape_id,
            path.display(),
            object.frame.width,
            grid_width
        );
        assert!(
            (object.frame.height - grid_height).abs() <= (object.frame.height * 0.01).max(2.0),
            "table {:?} frame/grid heights differ in {}: {} vs {}",
            object.source_shape_id,
            path.display(),
            object.frame.height,
            grid_height
        );
    }
    serde_json::to_vec(&deck).expect("structured table scene should serialize");
}

fn walk_objects(object: &SceneObject) -> Vec<&SceneObject> {
    let mut output = vec![object];
    for child in &object.children {
        output.extend(walk_objects(child));
    }
    output
}
