use super::*;
use crate::enums::shapes::PresetGeometry;
use crate::shapes::autoshape::AutoShape;
use crate::xml_util::WriteXml;

pub(super) fn make_empty_group() -> GroupShape {
    GroupShape {
        shape_id: ShapeId(10),
        name: "Group 1".to_string(),
        left: Emu(0),
        top: Emu(0),
        width: Emu(914400),
        height: Emu(914400),
        rotation: 0.0,
        child_left: Emu(0),
        child_top: Emu(0),
        child_width: Emu(914400),
        child_height: Emu(914400),
        fill: None,
        line: None,
        shapes: Vec::new(),
    }
}

#[test]
fn test_empty_group_xml() {
    let group = make_empty_group();
    let xml = group.to_xml_string();
    assert!(xml.starts_with("<p:grpSp>"));
    assert!(xml.ends_with("</p:grpSp>"));
    assert!(xml.contains(r#"id="10""#));
    assert!(xml.contains("<a:chOff"));
    assert!(xml.contains("<a:chExt"));
}

#[test]
fn test_group_with_child() {
    let mut group = make_empty_group();

    let child = AutoShape {
        shape_id: ShapeId(11),
        name: "Rect 1".to_string(),
        left: Emu(0),
        top: Emu(0),
        width: Emu(457200),
        height: Emu(457200),
        rotation: 0.0,
        prst_geom: Some(PresetGeometry::Rect),
        is_textbox: false,
        placeholder: None,
        tx_body_xml: None,
        fill: None,
        line: None,
        text_frame: None,
        click_action: None,
        hover_action: None,
        adjustments: Vec::new(),
        shadow: None,
        custom_geometry: None,
        scene_3d: None,
        shape_3d: None,
    };
    group.add_shape(Shape::AutoShape(Box::new(child)));

    assert_eq!(group.len(), 1);
    let xml = group.to_xml_string();
    assert!(xml.contains("<p:sp>"));
    assert!(xml.contains(r#"id="11""#));
}

// -----------------------------------------------------------------------
// max_shape_id tests
// -----------------------------------------------------------------------

#[test]
fn test_max_shape_id_empty() {
    let group = make_empty_group();
    assert_eq!(group.max_shape_id(), ShapeId(0));
}

#[test]
fn test_max_shape_id_with_children() {
    let mut group = make_empty_group();
    group.add_textbox(Emu(0), Emu(0), Emu(100), Emu(100));
    group.add_textbox(Emu(0), Emu(0), Emu(100), Emu(100));
    // shape_id of group is 10, children should be 11 and 12
    assert_eq!(group.max_shape_id(), ShapeId(12));
}

#[test]
fn test_max_shape_id_nested_group() {
    let mut group = make_empty_group();
    // Add a child and then a nested group with its own child
    group.add_textbox(Emu(0), Emu(0), Emu(100), Emu(100)); // id=11
    let nested = group.add_group_shape(); // id=12
    if let Shape::GroupShape(ref mut g) = nested {
        g.add_textbox(Emu(0), Emu(0), Emu(50), Emu(50)); // id=13
    }
    assert_eq!(group.max_shape_id(), ShapeId(13));
}

// -----------------------------------------------------------------------
// Mixed add_* + ID increment tests
// -----------------------------------------------------------------------

#[test]
fn test_mixed_shapes_increment_ids() {
    let mut group = make_empty_group();
    group.add_textbox(Emu(0), Emu(0), Emu(100), Emu(100)); // id=11
    group.add_autoshape("rect", Emu(0), Emu(0), Emu(100), Emu(100)); // id=12
    group.add_picture("rId1", Emu(0), Emu(0), Emu(100), Emu(100)); // id=13
    group.add_connector("line", Emu(0), Emu(0), Emu(100), Emu(100)); // id=14
    group.add_table(1, 1, Emu(0), Emu(0), Emu(100), Emu(100)); // id=15
    group.add_group_shape(); // id=16

    assert_eq!(group.len(), 6);
    assert_eq!(group.shapes[0].shape_id(), ShapeId(11));
    assert_eq!(group.shapes[1].shape_id(), ShapeId(12));
    assert_eq!(group.shapes[2].shape_id(), ShapeId(13));
    assert_eq!(group.shapes[3].shape_id(), ShapeId(14));
    assert_eq!(group.shapes[4].shape_id(), ShapeId(15));
    assert_eq!(group.shapes[5].shape_id(), ShapeId(16));
}

#[test]
fn test_group_xml_with_mixed_children() {
    let mut group = make_empty_group();
    group.add_textbox(Emu(0), Emu(0), Emu(100), Emu(100));
    group.add_autoshape("ellipse", Emu(100), Emu(100), Emu(200), Emu(200));
    group.add_picture("rId1", Emu(200), Emu(200), Emu(300), Emu(300));

    let xml = group.to_xml_string();
    assert!(xml.starts_with("<p:grpSp>"));
    assert!(xml.ends_with("</p:grpSp>"));
    // Contains both autoshapes and picture
    assert!(xml.contains("<p:sp>"));
    assert!(xml.contains("<p:pic>"));
    assert!(xml.contains(r#"txBox="1""#));
    assert!(xml.contains(r#"prst="ellipse""#));
}
