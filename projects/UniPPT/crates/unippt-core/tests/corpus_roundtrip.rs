use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pptx::{PptxValidator, Presentation, Severity};
use unippt_core::{export_pptx, import_pptx};

#[test]
#[ignore = "requires UNIPPT_CORPUS_DIR with external PPTX fixtures"]
fn noop_round_trip_preserves_every_part_and_animation() {
    let root = PathBuf::from(
        std::env::var("UNIPPT_CORPUS_DIR").expect("UNIPPT_CORPUS_DIR must point to PPTX corpus"),
    );
    let limit = std::env::var("UNIPPT_CORPUS_LIMIT")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(usize::MAX);
    let requested_file = std::env::var_os("UNIPPT_CORPUS_FILE").map(PathBuf::from);
    let files = requested_file
        .as_ref()
        .map_or_else(|| pptx_files(&root), |path| vec![path.clone()]);
    assert!(
        !files.is_empty(),
        "no .pptx files found in {}",
        root.display()
    );

    for path in files.into_iter().take(limit) {
        eprintln!("round-trip {}", path.display());
        let source_bytes = std::fs::read(&path).unwrap();
        let source = Presentation::from_bytes(&source_bytes)
            .unwrap_or_else(|error| panic!("source {} cannot be opened: {error}", path.display()));
        let deck = import_pptx(&source_bytes).unwrap_or_else(|error| {
            panic!("source {} cannot be imported: {error}", path.display())
        });
        let output_bytes = export_pptx(&deck, Some(&source_bytes)).unwrap_or_else(|error| {
            panic!("source {} cannot be exported: {error}", path.display())
        });
        let output = Presentation::from_bytes(&output_bytes).unwrap_or_else(|error| {
            panic!("output for {} cannot be reopened: {error}", path.display())
        });

        assert_eq!(
            source.slide_count().unwrap(),
            output.slide_count().unwrap(),
            "slide count changed for {}",
            path.display()
        );
        assert_parts_equal(&path, &source, &output);

        let source_severe = severe_issue_count(&source);
        let output_severe = severe_issue_count(&output);
        assert!(
            output_severe <= source_severe,
            "new severe validation issues for {}: source={source_severe}, output={output_severe}",
            path.display()
        );
    }
}

#[test]
#[ignore = "requires UNIPPT_CORPUS_DIR with external PPTX fixtures"]
fn edited_slide_preserves_animation_and_transition_xml() {
    let root = PathBuf::from(
        std::env::var("UNIPPT_CORPUS_DIR").expect("UNIPPT_CORPUS_DIR must point to PPTX corpus"),
    );
    let path = pptx_files(&root)
        .into_iter()
        .find(|path| path.file_stem().is_some_and(|stem| stem == "14"))
        .or_else(|| pptx_files(&root).into_iter().next())
        .expect("no .pptx files found in corpus");
    let source_bytes = std::fs::read(&path).unwrap();
    let source = Presentation::from_bytes(&source_bytes).unwrap();
    let mut deck = import_pptx(&source_bytes).unwrap();

    let (slide_index, object_index, part_name) = deck
        .slides
        .iter()
        .enumerate()
        .find_map(|(slide_index, slide)| {
            let part_name = slide.source_part_name.as_deref()?;
            let part = part_blob(&source, part_name)?;
            xml_element(part, b"p:timing")?;
            let object_index = slide.objects.iter().position(|object| {
                object.source_shape_id.is_some()
                    && matches!(
                        object.kind,
                        unippt_core::ObjectKind::Text | unippt_core::ObjectKind::Shape
                    )
                    && !object.text.is_empty()
            })?;
            Some((slide_index, object_index, part_name.to_owned()))
        })
        .expect("corpus has no animated slide with editable source text");

    let source_slide = part_blob(&source, &part_name).unwrap();
    let source_timing = xml_element(source_slide, b"p:timing").unwrap().to_vec();
    let source_transition = xml_element(source_slide, b"p:transition").map(ToOwned::to_owned);
    deck.slides[slide_index].objects[object_index]
        .text
        .push_str(" · UniPPT");

    let output_bytes = export_pptx(&deck, Some(&source_bytes)).unwrap();
    let output = Presentation::from_bytes(&output_bytes).unwrap();
    let output_slide = part_blob(&output, &part_name).unwrap();

    assert_ne!(
        source_slide, output_slide,
        "the edited slide was not patched"
    );
    assert_eq!(
        source_timing,
        xml_element(output_slide, b"p:timing").unwrap(),
        "animation timeline XML changed while editing a shape in {}",
        path.display()
    );
    assert_eq!(
        source_transition.as_deref(),
        xml_element(output_slide, b"p:transition"),
        "slide transition XML changed while editing a shape in {}",
        path.display()
    );
}

#[test]
#[ignore = "requires UNIPPT_CORPUS_DIR with external PPTX fixtures"]
fn structured_animation_edit_writes_native_timing_and_reimports() {
    let root = PathBuf::from(
        std::env::var("UNIPPT_CORPUS_DIR").expect("UNIPPT_CORPUS_DIR must point to PPTX corpus"),
    );
    let path = pptx_files(&root)
        .into_iter()
        .find(|path| path.file_stem().is_some_and(|stem| stem == "14"))
        .or_else(|| pptx_files(&root).into_iter().next())
        .expect("no .pptx files found in corpus");
    let source_bytes = std::fs::read(&path).unwrap();
    let source = Presentation::from_bytes(&source_bytes).expect("source PPTX must reopen");
    let mut deck = import_pptx(&source_bytes).unwrap();
    let (slide_index, animation_index) = deck
        .slides
        .iter()
        .enumerate()
        .find_map(|(slide_index, slide)| {
            slide
                .animations
                .iter()
                .position(|animation| animation.target_shape_id.is_some())
                .map(|animation_index| (slide_index, animation_index))
        })
        .expect("corpus has no structured animation target");
    let original_duration = deck.slides[slide_index].animations[animation_index].duration_ms;
    let edited_duration = original_duration.saturating_add(137).max(638);
    deck.slides[slide_index].animations[animation_index].duration_ms = edited_duration;

    let output_bytes = export_pptx(&deck, Some(&source_bytes)).unwrap();
    let output = Presentation::from_bytes(&output_bytes).expect("edited PPTX must reopen");
    assert!(
        severe_issue_count(&output) <= severe_issue_count(&source),
        "structured animation edit introduced new validation errors in {}",
        path.display()
    );

    let reparsed = import_pptx(&output_bytes).expect("edited animation must reimport");
    let reparsed_animation = reparsed.slides[slide_index]
        .animations
        .iter()
        .find(|candidate| {
            candidate.target_shape_id
                == deck.slides[slide_index].animations[animation_index].target_shape_id
                && candidate.effect == deck.slides[slide_index].animations[animation_index].effect
        })
        .expect("edited animation disappeared after native writeback");
    assert_eq!(reparsed_animation.duration_ms, edited_duration);
    assert_ne!(
        reparsed_animation.duration_ms, original_duration,
        "duration edit was not persisted"
    );
}

fn pptx_files(root: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(root)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("pptx"))
        })
        .collect();
    files.sort_by_key(|path| {
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .and_then(|stem| stem.parse::<u32>().ok())
            .unwrap_or(u32::MAX)
    });
    files
}

fn assert_parts_equal(path: &Path, source: &Presentation, output: &Presentation) {
    let source_parts: BTreeMap<_, _> = source
        .package()
        .parts()
        .map(|part| (part.partname.to_string(), part))
        .collect();
    let output_parts: BTreeMap<_, _> = output
        .package()
        .parts()
        .map(|part| (part.partname.to_string(), part))
        .collect();
    assert_eq!(
        source_parts.keys().collect::<Vec<_>>(),
        output_parts.keys().collect::<Vec<_>>(),
        "part set changed for {}",
        path.display()
    );
    for (part_name, source_part) in source_parts {
        let output_part = output_parts[&part_name];
        assert_eq!(
            source_part.content_type,
            output_part.content_type,
            "content type changed for {part_name} in {}",
            path.display()
        );
        assert_eq!(
            source_part.blob,
            output_part.blob,
            "part bytes changed for {part_name} in {}",
            path.display()
        );
        assert_eq!(
            relationship_fingerprint(source_part),
            relationship_fingerprint(output_part),
            "relationships changed for {part_name} in {}",
            path.display()
        );
    }
}

fn relationship_fingerprint(part: &pptx::opc::Part) -> Vec<(String, String, String, bool)> {
    let mut relationships: Vec<_> = part
        .rels
        .iter()
        .map(|relationship| {
            (
                relationship.r_id.to_string(),
                relationship.rel_type.to_string(),
                relationship.target_ref.clone(),
                relationship.is_external,
            )
        })
        .collect();
    relationships.sort();
    relationships
}

fn severe_issue_count(presentation: &Presentation) -> usize {
    PptxValidator::validate(presentation)
        .into_iter()
        .filter(|issue| matches!(issue.severity, Severity::Critical | Severity::High))
        .count()
}

fn part_blob<'a>(presentation: &'a Presentation, part_name: &str) -> Option<&'a [u8]> {
    presentation
        .package()
        .parts()
        .find(|part| part.partname.to_string() == part_name)
        .map(|part| part.blob.as_slice())
}

fn xml_element<'a>(xml: &'a [u8], qualified_name: &[u8]) -> Option<&'a [u8]> {
    let mut opening = Vec::with_capacity(qualified_name.len() + 1);
    opening.push(b'<');
    opening.extend_from_slice(qualified_name);
    let start = find_bytes(xml, &opening)?;
    let opening_end = start + find_bytes(&xml[start..], b">")? + 1;
    if xml[start..opening_end].ends_with(b"/>") {
        return Some(&xml[start..opening_end]);
    }

    let mut closing = Vec::with_capacity(qualified_name.len() + 3);
    closing.extend_from_slice(b"</");
    closing.extend_from_slice(qualified_name);
    closing.push(b'>');
    let close_start = opening_end + find_bytes(&xml[opening_end..], &closing)?;
    Some(&xml[start..close_start + closing.len()])
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
