#![cfg(windows)]

use std::path::PathBuf;

use unippt_core::{import_pptx, SceneObject};

#[test]
#[ignore = "requires the local 1.pptx font/theme corpus"]
fn resolves_real_drawingml_font_slots_without_leaking_theme_tokens() {
    let path = std::env::var_os("UNIPPT_FONT_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.tmp/zindex-audit/1.pptx")
        });
    let bytes = std::fs::read(path).expect("read font/theme PPTX fixture");
    let deck = import_pptx(&bytes).expect("import font/theme PPTX fixture");
    let mut families = Vec::new();
    for slide in &deck.slides {
        collect_families(&slide.objects, &mut families);
    }

    assert!(
        !families.is_empty(),
        "fixture should expose text font families"
    );
    assert!(
        families
            .iter()
            .all(|family| !family.contains("+mj-") && !family.contains("+mn-")),
        "unresolved theme font token reached scene model: {:?}",
        families
            .iter()
            .filter(|family| family.contains("+mj-") || family.contains("+mn-"))
            .collect::<Vec<_>>()
    );
    assert!(
        families.iter().any(|family| family.contains(',')),
        "resolved browser families should retain a fallback stack"
    );
}

fn collect_families(objects: &[SceneObject], families: &mut Vec<String>) {
    for object in objects {
        if !object.text_style.font_family.trim().is_empty() {
            families.push(object.text_style.font_family.clone());
        }
        families.extend(
            object
                .text_paragraphs
                .iter()
                .flat_map(|paragraph| paragraph.runs.iter())
                .map(|run| run.font_family.clone()),
        );
        if let Some(table) = &object.table {
            for cell in table.rows.iter().flat_map(|row| row.cells.iter()) {
                families.push(cell.text_style.font_family.clone());
                families.extend(
                    cell.text_paragraphs
                        .iter()
                        .flat_map(|paragraph| paragraph.runs.iter())
                        .map(|run| run.font_family.clone()),
                );
            }
        }
        collect_families(&object.children, families);
    }
}
