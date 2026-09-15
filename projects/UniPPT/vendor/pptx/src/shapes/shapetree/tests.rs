use super::*;
use crate::dml::fill::FillFormat;
use crate::enums::shapes::PresetGeometry;
use crate::units::Inches;
use crate::xml_util::WriteXml;

#[test]
fn test_parse_empty_slide() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
<p:grpSpPr/>
</p:spTree></p:cSld></p:sld>"#;

    let tree = ShapeTree::from_slide_xml(xml).unwrap();
    assert_eq!(tree.len(), 0);
}

#[test]
fn test_parse_slide_with_shapes() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
<p:grpSpPr/>
<p:sp>
  <p:nvSpPr><p:cNvPr id="2" name="Title 1"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr>
  <p:spPr><a:xfrm><a:off x="457200" y="274638"/><a:ext cx="8229600" cy="1143000"/></a:xfrm></p:spPr>
  <p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr/></a:p></p:txBody>
</p:sp>
<p:sp>
  <p:nvSpPr><p:cNvPr id="3" name="TextBox 2"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
  <p:spPr><a:xfrm><a:off x="100000" y="200000"/><a:ext cx="300000" cy="400000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr>
  <p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr/></a:p></p:txBody>
</p:sp>
</p:spTree></p:cSld></p:sld>"#;

    let tree = ShapeTree::from_slide_xml(xml).unwrap();
    assert_eq!(tree.len(), 2);

    let s0 = &tree.shapes[0];
    assert_eq!(s0.name(), "Title 1");
    assert_eq!(s0.shape_id(), ShapeId(2));
    assert!(s0.is_placeholder());
    assert_eq!(s0.left(), Emu(457200));
    assert_eq!(s0.width(), Emu(8229600));

    let s1 = &tree.shapes[1];
    assert_eq!(s1.name(), "TextBox 2");
    assert_eq!(s1.shape_id(), ShapeId(3));
    assert!(!s1.is_placeholder());
    assert!(s1.has_text_frame());
    if let Shape::AutoShape(a) = s1 {
        assert!(a.is_textbox);
        assert_eq!(a.prst_geom, Some(PresetGeometry::Rect));
    }
}

#[test]
fn test_parse_group_keeps_children_and_child_coordinate_space() {
    let xml = br#"<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
<p:grpSp><p:nvGrpSpPr><p:cNvPr id="4" name="Group 3"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
<p:grpSpPr><a:xfrm><a:off x="1000" y="2000"/><a:ext cx="8000" cy="4000"/><a:chOff x="100" y="200"/><a:chExt cx="4000" cy="2000"/></a:xfrm></p:grpSpPr>
<p:sp><p:nvSpPr><p:cNvPr id="5" name="Child A"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="100" y="200"/><a:ext cx="1000" cy="500"/></a:xfrm></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>A</a:t></a:r></a:p></p:txBody></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="6" name="Child B"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="2100" y="1200"/><a:ext cx="1000" cy="500"/></a:xfrm></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>B</a:t></a:r></a:p></p:txBody></p:sp>
</p:grpSp>
<p:sp><p:nvSpPr><p:cNvPr id="7" name="Top Layer"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1" cy="1"/></a:xfrm></p:spPr></p:sp>
</p:spTree></p:cSld></p:sld>"#;

    let tree = ShapeTree::from_slide_xml(xml).unwrap();
    assert_eq!(tree.len(), 2);
    let Shape::GroupShape(group) = &tree.shapes[0] else {
        panic!("expected recursive group");
    };
    assert_eq!(group.shapes.len(), 2);
    assert_eq!(group.child_left, Emu(100));
    assert_eq!(group.child_top, Emu(200));
    assert_eq!(group.child_width, Emu(4000));
    assert_eq!(group.child_height, Emu(2000));
    assert_eq!(group.shapes[0].name(), "Child A");
    assert_eq!(group.shapes[1].name(), "Child B");
    assert_eq!(tree.shapes[1].name(), "Top Layer");
}

#[test]
fn test_parse_group_retains_fill_line_and_group_fill_child() {
    let xml = br#"<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
<p:grpSp><p:nvGrpSpPr><p:cNvPr id="4" name="Filled Group"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
<p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1000" cy="1000"/><a:chOff x="0" y="0"/><a:chExt cx="1000" cy="1000"/></a:xfrm><a:solidFill><a:srgbClr val="112233"/></a:solidFill><a:ln w="12700"><a:solidFill><a:srgbClr val="445566"/></a:solidFill></a:ln></p:grpSpPr>
<p:sp><p:nvSpPr><p:cNvPr id="5" name="Inherited Child"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1000" cy="1000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:grpFill/></p:spPr></p:sp>
</p:grpSp></p:spTree></p:cSld></p:sld>"#;

    let tree = ShapeTree::from_slide_xml(xml).unwrap();
    let Shape::GroupShape(group) = &tree.shapes[0] else {
        panic!("expected group shape");
    };
    assert!(matches!(group.fill, Some(FillFormat::Solid(_))));
    assert!(group.line.is_some());
    let Shape::AutoShape(child) = &group.shapes[0] else {
        panic!("expected child auto shape");
    };
    assert_eq!(child.fill, Some(FillFormat::Background));

    let round_trip = group.to_xml_string();
    assert!(round_trip.contains(r#"<a:srgbClr val="112233"/>"#));
    assert!(round_trip.contains(r#"<a:ln w="12700">"#));
}

#[test]
fn test_parse_picture() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
<p:grpSpPr/>
<p:pic>
  <p:nvPicPr><p:cNvPr id="4" name="Picture 3" descr="Test image"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr>
  <p:blipFill><a:blip r:embed="rId2"/><a:stretch><a:fillRect/></a:stretch></p:blipFill>
  <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm></p:spPr>
</p:pic>
</p:spTree></p:cSld></p:sld>"#;

    let tree = ShapeTree::from_slide_xml(xml).unwrap();
    assert_eq!(tree.len(), 1);
    let s = &tree.shapes[0];
    assert_eq!(s.name(), "Picture 3");
    if let Shape::Picture(p) = s {
        assert_eq!(p.image_r_id.as_deref(), Some("rId2"));
        assert_eq!(p.description.as_deref(), Some("Test image"));
        assert_eq!(p.width, Emu(914400));
    } else {
        panic!("Expected Picture shape");
    }
}

#[test]
fn test_parse_picture_retains_source_crop() {
    let xml = br#"<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
<p:pic><p:nvPicPr><p:cNvPr id="4" name="Cropped picture"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr>
<p:blipFill><a:blip r:embed="rId2"/><a:srcRect l="18887" t="9703" r="16572" b="34374"/><a:stretch><a:fillRect/></a:stretch></p:blipFill>
<p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr></p:pic>
</p:spTree></p:cSld></p:sld>"#;

    let tree = ShapeTree::from_slide_xml(xml).unwrap();
    let Shape::Picture(picture) = &tree.shapes[0] else {
        panic!("expected picture");
    };
    assert!((picture.crop_left - 0.18887).abs() < f64::EPSILON);
    assert!((picture.crop_top - 0.09703).abs() < f64::EPSILON);
    assert!((picture.crop_right - 0.16572).abs() < f64::EPSILON);
    assert!((picture.crop_bottom - 0.34374).abs() < f64::EPSILON);
}

#[test]
fn test_parse_picture_retains_duotone_and_color_transforms() {
    use crate::dml::effect::DuotoneColorTransform;

    let xml = br#"<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:pic><p:nvPicPr><p:cNvPr id="119" name="Picture 118"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="rId7"><a:duotone><a:srgbClr val="E97798"><a:shade val="45000"/><a:satMod val="135000"/></a:srgbClr><a:prstClr val="white"/></a:duotone></a:blip><a:stretch/></p:blipFill><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm></p:spPr></p:pic></p:spTree></p:cSld></p:sld>"#;
    let tree = ShapeTree::from_slide_xml(xml).unwrap();
    let Shape::Picture(picture) = &tree.shapes[0] else {
        panic!("expected picture")
    };
    let effect = picture.duotone.as_ref().expect("duotone");
    assert_eq!(effect.colors.len(), 2);
    assert_eq!(
        effect.colors[0].transforms,
        vec![
            DuotoneColorTransform::Shade(0.45),
            DuotoneColorTransform::SaturationModulation(1.35)
        ]
    );
    let round_trip = picture.to_xml_string();
    assert!(round_trip.contains(r#"<a:duotone><a:srgbClr val="E97798"><a:shade val="45000"/><a:satMod val="135000"/></a:srgbClr><a:prstClr val="white"/></a:duotone>"#));
}

#[test]
fn test_parse_graphic_frame_direct_transform_geometry() {
    let xml = br#"<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="31" name="Table 30"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
<p:xfrm rot="60000"><a:off x="123400" y="567800"/><a:ext cx="3456000" cy="2345000"/></p:xfrm>
<a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr/><a:tblGrid><a:gridCol w="3456000"><a:extLst><a:ext uri="{9D8B030D-6E8A-4147-A177-3AD203B41FA5}"><a:colId val="20000"/></a:ext></a:extLst></a:gridCol></a:tblGrid><a:tr h="2345000"><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p/></a:txBody><a:tcPr/></a:tc><a:extLst><a:ext uri="{0D108BD9-81ED-4DB2-BD59-A6C34878D82A}"><a:rowId val="10000"/></a:ext></a:extLst></a:tr></a:tbl></a:graphicData></a:graphic>
</p:graphicFrame></p:spTree></p:cSld></p:sld>"#;

    let tree = ShapeTree::from_slide_xml(xml).unwrap();
    let frame = tree.shapes[0]
        .as_graphic_frame()
        .expect("expected graphic frame");
    assert_eq!(frame.left, Emu(123400));
    assert_eq!(frame.top, Emu(567800));
    assert_eq!(frame.width, Emu(3456000));
    assert_eq!(frame.height, Emu(2345000));
    assert_eq!(frame.rotation, 1.0);
    assert!(frame.has_table);
}

#[test]
fn test_parse_connector() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
<p:grpSpPr/>
<p:cxnSp>
  <p:nvCxnSpPr><p:cNvPr id="5" name="Connector 4"/><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr>
  <p:spPr><a:xfrm flipH="1"><a:off x="100" y="200"/><a:ext cx="500" cy="600"/></a:xfrm><a:prstGeom prst="line"><a:avLst/></a:prstGeom></p:spPr>
</p:cxnSp>
</p:spTree></p:cSld></p:sld>"#;

    let tree = ShapeTree::from_slide_xml(xml).unwrap();
    assert_eq!(tree.len(), 1);
    if let Shape::Connector(c) = &tree.shapes[0] {
        assert_eq!(c.name, "Connector 4");
        assert!(c.flip_h);
        assert!(!c.flip_v);
        assert_eq!(c.prst_geom, Some(PresetGeometry::Line));
    } else {
        panic!("Expected Connector shape");
    }
}

#[test]
fn test_new_textbox_xml() {
    let xml = ShapeTree::new_textbox_xml(
        ShapeId(5),
        "TextBox 4",
        Emu(914400),
        Emu(914400),
        Emu(2743200),
        Emu(457200),
    );
    assert!(xml.contains(r#"id="5""#));
    assert!(xml.contains(r#"name="TextBox 4""#));
    assert!(xml.contains(r#"txBox="1""#));
    assert!(xml.contains(r#"x="914400""#));
}

#[test]
fn test_new_autoshape_xml() {
    let xml = ShapeTree::new_autoshape_xml(
        ShapeId(6),
        "Oval 5",
        Emu(100000),
        Emu(200000),
        Emu(300000),
        Emu(400000),
        "ellipse",
    );
    assert!(xml.contains(r#"prst="ellipse""#));
    assert!(xml.contains(r#"id="6""#));
}

#[test]
fn test_new_picture_xml() {
    let xml = ShapeTree::new_picture_xml(
        ShapeId(7),
        "Picture 6",
        "A photo",
        "rId2",
        Emu(0),
        Emu(0),
        Emu(914400),
        Emu(914400),
    );
    assert!(xml.contains(r#"r:embed="rId2""#));
    assert!(xml.contains(r#"descr="A photo""#));
}

#[test]
fn test_new_table_xml() {
    let xml = ShapeTree::new_table_xml(
        ShapeId(8),
        "Table 7",
        2,
        3,
        Emu(914400),
        Emu(914400),
        Emu(2743200),
        Emu(914400),
    );
    assert!(xml.contains("a:tbl"));
    assert!(xml.contains("a:tr"));
    assert!(xml.contains("a:tc"));
}

#[test]
fn test_units_in_shapes() {
    let left: Emu = Inches(1.0).into();
    let top: Emu = Inches(2.0).into();
    assert_eq!(left, Emu(914400));
    assert_eq!(top, Emu(1828800));
}

#[test]
fn test_shape_name_for_prst_common() {
    assert_eq!(shape_name_for_prst("rect"), "Rectangle");
    assert_eq!(shape_name_for_prst("ellipse"), "Oval");
    assert_eq!(shape_name_for_prst("roundRect"), "Rounded Rectangle");
    assert_eq!(shape_name_for_prst("diamond"), "Diamond");
    assert_eq!(shape_name_for_prst("cloud"), "Cloud");
    assert_eq!(shape_name_for_prst("heart"), "Heart");
}

#[test]
fn test_shape_name_for_prst_unknown() {
    assert_eq!(shape_name_for_prst("unknownShape"), "Freeform");
}

#[test]
fn test_turbo_add_flag() {
    let mut tree = ShapeTree::default();
    assert!(!tree.turbo_add_enabled());
    tree.set_turbo_add_enabled(true);
    assert!(tree.turbo_add_enabled());
    tree.set_turbo_add_enabled(false);
    assert!(!tree.turbo_add_enabled());
}

#[test]
fn test_new_connector_xml_with_flip() {
    let xml = ShapeTree::new_connector_xml_with_flip(
        ShapeId(5),
        "Connector 1",
        Emu(100),
        Emu(200),
        Emu(300),
        Emu(400),
        "line",
        true,
        false,
    );
    assert!(xml.contains(r#"flipH="1""#));
    assert!(!xml.contains(r#"flipV="1""#));
    assert!(xml.contains(r#"prst="line""#));
}

#[test]
fn test_new_connector_xml_with_flip_both() {
    let xml = ShapeTree::new_connector_xml_with_flip(
        ShapeId(5),
        "Connector 1",
        Emu(100),
        Emu(200),
        Emu(300),
        Emu(400),
        "line",
        true,
        true,
    );
    assert!(xml.contains(r#"flipH="1""#));
    assert!(xml.contains(r#"flipV="1""#));
}

#[test]
fn test_new_connector_xml_with_no_flip() {
    let xml = ShapeTree::new_connector_xml_with_flip(
        ShapeId(5),
        "Connector 1",
        Emu(100),
        Emu(200),
        Emu(300),
        Emu(400),
        "line",
        false,
        false,
    );
    assert!(!xml.contains("flipH"));
    assert!(!xml.contains("flipV"));
}

#[test]
fn test_new_group_shape_xml() {
    let xml = ShapeTree::new_group_shape_xml(
        ShapeId(10),
        "Group 1",
        Emu(0),
        Emu(0),
        Emu(914400),
        Emu(914400),
    );
    assert!(xml.contains(r#"id="10""#));
    assert!(xml.contains(r#"name="Group 1""#));
    assert!(xml.contains("<p:grpSp"));
    assert!(xml.contains("<a:chOff"));
    assert!(xml.contains("<a:chExt"));
}
