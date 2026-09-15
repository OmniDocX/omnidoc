use crate::opc::pack_uri::PackURI;
use crate::slide::*;
use crate::units::{RelationshipId, SlideId};

#[test]
fn test_parse_slide_ids_empty() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId1"/></p:sldMasterIdLst>
</p:presentation>"#;
    let ids = parse_slide_ids(xml).unwrap();
    assert!(ids.is_empty());
}

#[test]
fn test_parse_slide_master_ids() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId1"/></p:sldMasterIdLst>
</p:presentation>"#;
    let ids = parse_slide_master_ids(xml).unwrap();
    assert_eq!(ids.len(), 1);
    assert_eq!(ids[0].0, "rId1");
    assert_eq!(ids[0].1, SlideId(2147483648));
}

#[test]
fn test_parse_layout_name() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" type="title" preserve="1"><p:cSld name="Title Slide"><p:spTree/></p:cSld></p:sldLayout>"#;
    // EXCEPTION(unwrap): test-only code with known-valid input
    let name = parse_layout_name(xml).unwrap();
    assert_eq!(name, "Title Slide");
}

#[test]
fn test_parse_slide_size() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:sldSz cx="9144000" cy="6858000" type="screen4x3"/>
</p:presentation>"#;
    let size = parse_slide_size(xml).unwrap();
    assert_eq!(size, Some((9144000, 6858000)));
}

#[test]
fn test_new_slide_xml() {
    let xml = new_slide_xml();
    assert!(!xml.is_empty());
    let s = String::from_utf8(xml).unwrap();
    assert!(s.contains("<p:sld"));
    assert!(s.contains("<p:spTree>"));
}

#[test]
fn test_add_slide_id_to_presentation_xml() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId1"/></p:sldMasterIdLst><p:sldSz cx="9144000" cy="6858000"/></p:presentation>"#;

    let result = add_slide_id_to_presentation_xml(xml, "rId7", SlideId(256)).unwrap();
    let result_str = String::from_utf8(result).unwrap();
    assert!(result_str.contains("<p:sldIdLst>"));
    assert!(result_str.contains(r#"<p:sldId id="256" r:id="rId7"/>"#));
}

#[test]
fn test_next_slide_id() {
    assert_eq!(next_slide_id(&[]), SlideId(256));
    assert_eq!(
        next_slide_id(&[
            ("rId1".to_string(), SlideId(256)),
            ("rId2".to_string(), SlideId(257))
        ]),
        SlideId(258)
    );
}

#[test]
fn test_remove_slide_id_from_presentation_xml() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId1"/></p:sldMasterIdLst><p:sldIdLst><p:sldId id="256" r:id="rId7"/><p:sldId id="257" r:id="rId8"/></p:sldIdLst></p:presentation>"#;

    let result = remove_slide_id_from_presentation_xml(xml, "rId7").unwrap();
    let result_str = String::from_utf8(result).unwrap();
    assert!(!result_str.contains("rId7"));
    assert!(result_str.contains("rId8"));
    assert!(result_str.contains(r#"<p:sldId id="257" r:id="rId8"/>"#));
}

#[test]
fn test_remove_last_slide_id() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldIdLst><p:sldId id="256" r:id="rId7"/></p:sldIdLst></p:presentation>"#;

    let result = remove_slide_id_from_presentation_xml(xml, "rId7").unwrap();
    let result_str = String::from_utf8(result).unwrap();
    assert!(!result_str.contains("rId7"));
    assert!(result_str.contains("<p:sldIdLst/>"));
}

#[test]
fn test_set_slide_size_in_xml() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:sldSz cx="9144000" cy="6858000" type="screen4x3"/>
</p:presentation>"#;

    let result = set_slide_size_in_xml(xml, 12192000, 6858000).unwrap();
    let result_str = String::from_utf8(result).unwrap();
    assert!(result_str.contains("12192000"));
    assert!(result_str.contains("6858000"));
    assert!(result_str.contains("screen4x3"));
}

#[test]
fn test_get_layout_by_name() {
    // EXCEPTION(unwrap): test-only code with known-valid input
    let layouts = vec![
        SlideLayoutRef {
            r_id: RelationshipId::try_from("rId1").unwrap(),
            partname: PackURI::new("/ppt/slideLayouts/slideLayout1.xml").unwrap(),
            name: "Title Slide".to_string(),
            slide_master_part_name: None,
        },
        SlideLayoutRef {
            r_id: RelationshipId::try_from("rId2").unwrap(),
            partname: PackURI::new("/ppt/slideLayouts/slideLayout2.xml").unwrap(),
            name: "Title and Content".to_string(),
            slide_master_part_name: None,
        },
    ];

    let found = get_layout_by_name(&layouts, "Title Slide");
    assert!(found.is_some());
    assert_eq!(found.unwrap().r_id.as_str(), "rId1");

    let not_found = get_layout_by_name(&layouts, "Nonexistent");
    assert!(not_found.is_none());
}

#[test]
fn test_placeholder_shapes_from_layout() {
    let layout_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld name="Title Slide"><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id="2" name="Title 1"/><p:cNvSpPr/><p:nvPr><p:ph type="ctrTitle"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr/></a:p></p:txBody></p:sp><p:sp><p:nvSpPr><p:cNvPr id="3" name="Subtitle 2"/><p:cNvSpPr/><p:nvPr><p:ph type="subTitle" idx="1"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr/></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sldLayout>"#;

    let placeholders = placeholder_shapes_from_layout(layout_xml).unwrap();
    assert_eq!(placeholders.len(), 2);
    assert!(placeholders[0].contains("ctrTitle"));
    assert!(placeholders[1].contains("subTitle"));
}

#[test]
fn test_placeholder_shapes_from_layout_no_placeholders() {
    let layout_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld name="Blank"><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld></p:sldLayout>"#;

    let placeholders = placeholder_shapes_from_layout(layout_xml).unwrap();
    assert!(placeholders.is_empty());
}

#[test]
fn test_remove_layout_from_master_xml() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld/><p:sldLayoutIdLst><p:sldLayoutId id="2147483649" r:id="rId1"/><p:sldLayoutId id="2147483650" r:id="rId2"/><p:sldLayoutId id="2147483651" r:id="rId3"/></p:sldLayoutIdLst></p:sldMaster>"#;

    let result = remove_layout_from_master_xml(xml, "rId2").unwrap();
    let result_str = String::from_utf8(result).unwrap();
    assert!(!result_str.contains("rId2"));
    assert!(result_str.contains("rId1"));
    assert!(result_str.contains("rId3"));
}

#[test]
fn test_remove_layout_from_master_xml_not_found() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldLayoutIdLst><p:sldLayoutId id="2147483649" r:id="rId1"/></p:sldLayoutIdLst></p:sldMaster>"#;

    let result = remove_layout_from_master_xml(xml, "rId99").unwrap();
    let result_str = String::from_utf8(result).unwrap();
    assert!(result_str.contains("rId1"));
}

#[test]
fn test_slide_properties_struct() {
    let props = SlideProperties {
        slide_id: SlideId(256),
        name: "Test Slide".to_string(),
        has_notes_slide: true,
    };
    assert_eq!(props.slide_id, SlideId(256));
    assert_eq!(props.name, "Test Slide");
    assert!(props.has_notes_slide);
}

// =========================================================
// Tests for reorder_slide_in_presentation_xml
// =========================================================

#[test]
fn test_reorder_slide_move_first_to_last() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldIdLst><p:sldId id="256" r:id="rId7"/><p:sldId id="257" r:id="rId8"/><p:sldId id="258" r:id="rId9"/></p:sldIdLst></p:presentation>"#;

    let result = reorder_slide_in_presentation_xml(xml, 0, 2).unwrap();
    let result_str = String::from_utf8(result).unwrap();
    let pos7 = result_str.find("rId7").unwrap();
    let pos8 = result_str.find("rId8").unwrap();
    let pos9 = result_str.find("rId9").unwrap();
    assert!(pos8 < pos9);
    assert!(pos9 < pos7);
}

#[test]
fn test_reorder_slide_move_last_to_first() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldIdLst><p:sldId id="256" r:id="rId7"/><p:sldId id="257" r:id="rId8"/><p:sldId id="258" r:id="rId9"/></p:sldIdLst></p:presentation>"#;

    let result = reorder_slide_in_presentation_xml(xml, 2, 0).unwrap();
    let result_str = String::from_utf8(result).unwrap();
    let pos7 = result_str.find("rId7").unwrap();
    let pos8 = result_str.find("rId8").unwrap();
    let pos9 = result_str.find("rId9").unwrap();
    assert!(pos9 < pos7);
    assert!(pos7 < pos8);
}

#[test]
fn test_reorder_slide_out_of_range() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldIdLst><p:sldId id="256" r:id="rId7"/></p:sldIdLst></p:presentation>"#;

    let result = reorder_slide_in_presentation_xml(xml, 0, 5);
    assert!(result.is_err());
}

#[test]
fn test_reorder_slide_no_sld_id_lst() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldMasterIdLst/></p:presentation>"#;

    let result = reorder_slide_in_presentation_xml(xml, 0, 1);
    assert!(result.is_err());
}

// =========================================================
// Tests for Slide property APIs
// =========================================================

#[test]
fn test_parse_slide_name_present() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld name="My Slide"><p:spTree/></p:cSld></p:sld>"#;
    let name = parse_slide_name(xml).unwrap();
    assert_eq!(name, Some("My Slide".to_string()));
}

#[test]
fn test_parse_slide_name_absent() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree/></p:cSld></p:sld>"#;
    let name = parse_slide_name(xml).unwrap();
    assert_eq!(name, None);
}

#[test]
fn test_parse_slide_name_empty() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld name=""><p:spTree/></p:cSld></p:sld>"#;
    let name = parse_slide_name(xml).unwrap();
    assert_eq!(name, None);
}
