use crate::presentation::Presentation;

#[test]
fn test_notes_slide() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let slide_ref = prs.add_slide(&layouts[0]).unwrap();

    // Create notes slide
    let notes1 = prs.notes_slide_or_create(&slide_ref).unwrap();
    assert!(notes1.partname.as_str().contains("notesSlides"));

    // Getting again should return same one
    let notes2 = prs.notes_slide_or_create(&slide_ref).unwrap();
    assert_eq!(notes1.partname.as_str(), notes2.partname.as_str());
}

#[test]
fn test_notes_master_none_by_default() {
    let prs = Presentation::new().unwrap();
    let nm = prs.notes_master().unwrap();
    // The default template may or may not have a notes master.
    // If it doesn't have one, this is None.
    // We just verify the call doesn't error.
    let _ = nm;
}

#[test]
fn test_notes_master_or_create() {
    let mut prs = Presentation::new().unwrap();
    let nm = prs.notes_master_or_create().unwrap();
    assert!(nm.partname.as_str().contains("notesMasters"));

    // Getting again should return the same one
    let nm2 = prs.notes_master_or_create().unwrap();
    assert_eq!(nm.partname.as_str(), nm2.partname.as_str());

    // Verify the XML
    let xml = prs.notes_master_xml(&nm).unwrap();
    let xml_str = String::from_utf8_lossy(xml);
    assert!(xml_str.contains("<p:notesMaster"));
}

#[test]
fn test_notes_master_round_trip() {
    let mut prs = Presentation::new().unwrap();
    let nm = prs.notes_master_or_create().unwrap();

    // Save and reopen
    let bytes = prs.to_bytes().unwrap();
    let prs2 = Presentation::from_bytes(&bytes).unwrap();
    let nm2 = prs2.notes_master().unwrap();
    assert!(nm2.is_some());
    assert_eq!(nm2.unwrap().partname.as_str(), nm.partname.as_str());
}

#[test]
fn test_notes_slide_text_none() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let slide_ref = prs.add_slide(&layouts[0]).unwrap();

    // No notes slide yet
    let text = prs.notes_slide_text(&slide_ref).unwrap();
    assert_eq!(text, None);
}

#[test]
fn test_notes_slide_text_empty() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let slide_ref = prs.add_slide(&layouts[0]).unwrap();

    // Create notes slide (default has empty text)
    prs.notes_slide_or_create(&slide_ref).unwrap();
    let text = prs.notes_slide_text(&slide_ref).unwrap();
    assert!(text.is_some());
    // The default notes slide has an empty body paragraph
    assert_eq!(text.unwrap(), "");
}

#[test]
fn test_notes_slide_none_when_no_notes() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let slide_ref = prs.add_slide(&layouts[0]).unwrap();

    let ns = prs.notes_slide(&slide_ref).unwrap();
    assert!(ns.is_none());
}

#[test]
fn test_notes_slide_parses_shapes() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let slide_ref = prs.add_slide(&layouts[0]).unwrap();

    prs.notes_slide_or_create(&slide_ref).unwrap();

    let ns = prs.notes_slide(&slide_ref).unwrap();
    assert!(ns.is_some());
    let ns = ns.unwrap();

    // Default notes slide has 2 shapes: slide image placeholder and notes body
    assert_eq!(ns.shapes().len(), 2);
}

#[test]
fn test_notes_slide_placeholders() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let slide_ref = prs.add_slide(&layouts[0]).unwrap();

    prs.notes_slide_or_create(&slide_ref).unwrap();

    let ns = prs.notes_slide(&slide_ref).unwrap().unwrap();
    let placeholders = ns.placeholders();

    // Both shapes in the default notes slide are placeholders
    assert_eq!(placeholders.len(), 2);
}

#[test]
fn test_notes_slide_notes_placeholder() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let slide_ref = prs.add_slide(&layouts[0]).unwrap();

    prs.notes_slide_or_create(&slide_ref).unwrap();

    let ns = prs.notes_slide(&slide_ref).unwrap().unwrap();
    let notes_ph = ns.notes_placeholder();
    assert!(notes_ph.is_some());

    let ph = notes_ph.unwrap();
    assert!(ph.is_placeholder());
    let ph_fmt = ph.placeholder().unwrap();
    assert_eq!(
        ph_fmt.ph_type,
        Some(crate::enums::shapes::PpPlaceholderType::Body)
    );
    assert_eq!(ph_fmt.idx, crate::units::PlaceholderIndex(1));
}

#[test]
fn test_notes_slide_notes_text_empty() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let slide_ref = prs.add_slide(&layouts[0]).unwrap();

    prs.notes_slide_or_create(&slide_ref).unwrap();

    let ns = prs.notes_slide(&slide_ref).unwrap().unwrap();
    assert_eq!(ns.notes_text(), "");
}

#[test]
fn test_notes_slide_notes_placeholder_has_text_body() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let slide_ref = prs.add_slide(&layouts[0]).unwrap();

    prs.notes_slide_or_create(&slide_ref).unwrap();

    let ns = prs.notes_slide(&slide_ref).unwrap().unwrap();
    // The notes placeholder is an AutoShape with a text body
    let ph = ns.notes_placeholder().unwrap();
    assert!(ph.has_text_frame());
    // Use notes_text() for actual text content
    assert_eq!(ns.notes_text(), "");
}

#[test]
fn test_notes_slide_name_none() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let slide_ref = prs.add_slide(&layouts[0]).unwrap();

    // No notes slide -> None
    let name = prs.notes_slide_name(&slide_ref).unwrap();
    assert!(name.is_none());
}

#[test]
fn test_notes_slide_name_default() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let slide_ref = prs.add_slide(&layouts[0]).unwrap();

    prs.notes_slide_or_create(&slide_ref).unwrap();

    // Default notes slide XML doesn't have a name attribute
    let name = prs.notes_slide_name(&slide_ref).unwrap();
    assert!(name.is_none());
}

#[test]
fn test_set_notes_slide_background_solid() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let slide_ref = prs.add_slide(&layouts[0]).unwrap();

    let notes_ref = prs.notes_slide_or_create(&slide_ref).unwrap();

    prs.set_notes_slide_background_solid(&slide_ref, "0000FF")
        .unwrap();

    // Verify the notes slide XML contains the background
    let notes_part = prs.package().part(&notes_ref.partname).unwrap();
    let xml_str = String::from_utf8_lossy(&notes_part.blob);
    assert!(xml_str.contains("<p:bg>"));
    assert!(xml_str.contains(r#"val="0000FF""#));
}

#[test]
fn test_set_notes_slide_background_solid_no_notes_fails() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let slide_ref = prs.add_slide(&layouts[0]).unwrap();

    // Should fail because no notes slide exists
    let result = prs.set_notes_slide_background_solid(&slide_ref, "FF0000");
    assert!(result.is_err());
}
