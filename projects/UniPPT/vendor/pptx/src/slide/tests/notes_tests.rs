use crate::slide::*;

#[test]
fn test_new_notes_slide_xml() {
    let xml = new_notes_slide_xml();
    let s = String::from_utf8(xml).unwrap();
    assert!(s.contains("<p:notes"));
    assert!(s.contains("sldImg"));
    assert!(s.contains("Notes Placeholder"));
}

#[test]
fn test_parse_notes_slide_text_with_content() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:notes xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id="2" name="Slide Image Placeholder 1"/><p:cNvSpPr><a:spLocks noGrp="1" noRot="1" noChangeAspect="1"/></p:cNvSpPr><p:nvPr><p:ph type="sldImg"/></p:nvPr></p:nvSpPr><p:spPr/></p:sp><p:sp><p:nvSpPr><p:cNvPr id="3" name="Notes Placeholder 2"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Hello notes</a:t></a:r></a:p><a:p><a:r><a:t>Second paragraph</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:notes>"#;
    let text = parse_notes_slide_text(xml).unwrap();
    assert_eq!(text, "Hello notes\nSecond paragraph");
}

#[test]
fn test_parse_notes_slide_text_empty() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:notes xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id="3" name="Notes Placeholder 2"/><p:cNvSpPr/><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr lang="en-US"/></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:notes>"#;
    let text = parse_notes_slide_text(xml).unwrap();
    assert_eq!(text, "");
}

#[test]
fn test_parse_notes_slide_text_no_body_placeholder() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:notes xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld></p:notes>"#;
    let text = parse_notes_slide_text(xml).unwrap();
    assert_eq!(text, "");
}

#[test]
fn test_new_notes_master_xml() {
    let xml = new_notes_master_xml();
    let s = String::from_utf8(xml).unwrap();
    assert!(s.contains("<p:notesMaster"));
    assert!(s.contains("<p:spTree>"));
    assert!(s.contains("<p:clrMap"));
}

// =========================================================
// Tests for NotesSlide / parse_notes_slide
// =========================================================

#[test]
fn test_parse_notes_slide_default_xml() {
    let xml = new_notes_slide_xml();
    let ns = parse_notes_slide(&xml).unwrap();
    assert!(ns.name().is_none());
    assert_eq!(ns.shapes().len(), 2);
    assert_eq!(ns.notes_text(), "");
}

#[test]
fn test_parse_notes_slide_placeholders() {
    let xml = new_notes_slide_xml();
    let ns = parse_notes_slide(&xml).unwrap();

    let placeholders = ns.placeholders();
    assert_eq!(placeholders.len(), 2);

    let notes_ph = ns.notes_placeholder();
    assert!(notes_ph.is_some());

    let ph = notes_ph.unwrap().placeholder().unwrap();
    assert_eq!(
        ph.ph_type,
        Some(crate::enums::shapes::PpPlaceholderType::Body)
    );
    assert_eq!(ph.idx, crate::units::PlaceholderIndex(1));
}

#[test]
fn test_parse_notes_slide_with_text() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:notes xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr><p:sp><p:nvSpPr><p:cNvPr id="2" name="Slide Image Placeholder 1"/><p:cNvSpPr><a:spLocks noGrp="1" noRot="1" noChangeAspect="1"/></p:cNvSpPr><p:nvPr><p:ph type="sldImg"/></p:nvPr></p:nvSpPr><p:spPr/></p:sp><p:sp><p:nvSpPr><p:cNvPr id="3" name="Notes Placeholder 2"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Hello notes</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:notes>"#;

    let ns = parse_notes_slide(xml).unwrap();
    assert_eq!(ns.notes_text(), "Hello notes");
}

#[test]
fn test_parse_notes_slide_with_name() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:notes xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld name="My Notes"><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr><p:sp><p:nvSpPr><p:cNvPr id="2" name="Slide Image Placeholder 1"/><p:cNvSpPr/><p:nvPr><p:ph type="sldImg"/></p:nvPr></p:nvSpPr><p:spPr/></p:sp><p:sp><p:nvSpPr><p:cNvPr id="3" name="Notes Placeholder 2"/><p:cNvSpPr/><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr/></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:notes>"#;

    let ns = parse_notes_slide(xml).unwrap();
    assert_eq!(ns.name(), Some("My Notes"));
}

#[test]
fn test_parse_notes_slide_notes_placeholder_has_text() {
    let xml = new_notes_slide_xml();
    let ns = parse_notes_slide(&xml).unwrap();

    let ph = ns.notes_placeholder();
    assert!(ph.is_some());
    assert!(ph.unwrap().has_text_frame());
    assert_eq!(ns.notes_text(), "");
}

#[test]
fn test_notes_slide_multiline_text() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:notes xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr><p:sp><p:nvSpPr><p:cNvPr id="2" name="Slide Image Placeholder 1"/><p:cNvSpPr/><p:nvPr><p:ph type="sldImg"/></p:nvPr></p:nvSpPr><p:spPr/></p:sp><p:sp><p:nvSpPr><p:cNvPr id="3" name="Notes Placeholder 2"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Line one</a:t></a:r></a:p><a:p><a:r><a:t>Line two</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:notes>"#;

    let ns = parse_notes_slide(xml).unwrap();
    assert_eq!(ns.notes_text(), "Line one\nLine two");
}
