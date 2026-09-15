use crate::enums::shapes::PresetGeometry;
use crate::shapes::graphfrm;
use crate::shapes::Shape;
use crate::units::{Emu, ShapeId};
use crate::xml_util::WriteXml;

use super::tests::make_empty_group;

// -----------------------------------------------------------------------
// add_textbox tests
// -----------------------------------------------------------------------

#[test]
fn test_add_textbox() {
    let mut group = make_empty_group();
    let shape = group.add_textbox(Emu(100), Emu(200), Emu(300), Emu(400));

    assert_eq!(shape.shape_id(), ShapeId(11));
    assert_eq!(shape.name(), "TextBox 1");
    assert_eq!(shape.left(), Emu(100));
    assert_eq!(shape.top(), Emu(200));
    assert_eq!(shape.width(), Emu(300));
    assert_eq!(shape.height(), Emu(400));

    if let Shape::AutoShape(a) = shape {
        assert!(a.is_textbox);
        assert_eq!(a.prst_geom, Some(PresetGeometry::Rect));
    } else {
        panic!("Expected AutoShape");
    }
}

#[test]
fn test_add_textbox_increments_name() {
    let mut group = make_empty_group();
    group.add_textbox(Emu(0), Emu(0), Emu(100), Emu(100));
    let second = group.add_textbox(Emu(0), Emu(0), Emu(100), Emu(100));
    assert_eq!(second.name(), "TextBox 2");
}

#[test]
fn test_add_textbox_xml() {
    let mut group = make_empty_group();
    group.add_textbox(Emu(100), Emu(200), Emu(300), Emu(400));
    let xml = group.to_xml_string();
    assert!(xml.contains("<p:sp>"));
    assert!(xml.contains(r#"txBox="1""#));
    assert!(xml.contains(r#"id="11""#));
}

// -----------------------------------------------------------------------
// add_autoshape tests
// -----------------------------------------------------------------------

#[test]
fn test_add_autoshape_rect() {
    let mut group = make_empty_group();
    let shape = group.add_autoshape("rect", Emu(0), Emu(0), Emu(500), Emu(500));

    assert_eq!(shape.shape_id(), ShapeId(11));
    assert_eq!(shape.name(), "Rectangle 1");
    if let Shape::AutoShape(a) = shape {
        assert!(!a.is_textbox);
        assert_eq!(a.prst_geom, Some(PresetGeometry::Rect));
    } else {
        panic!("Expected AutoShape");
    }
}

#[test]
fn test_add_autoshape_ellipse() {
    let mut group = make_empty_group();
    let shape = group.add_autoshape("ellipse", Emu(0), Emu(0), Emu(500), Emu(500));

    assert_eq!(shape.name(), "Oval 1");
    if let Shape::AutoShape(a) = shape {
        assert_eq!(a.prst_geom, Some(PresetGeometry::Ellipse));
    } else {
        panic!("Expected AutoShape");
    }
}

#[test]
fn test_add_autoshape_increments_name() {
    let mut group = make_empty_group();
    group.add_autoshape("rect", Emu(0), Emu(0), Emu(100), Emu(100));
    let second = group.add_autoshape("rect", Emu(0), Emu(0), Emu(100), Emu(100));
    assert_eq!(second.name(), "Rectangle 2");
}

#[test]
fn test_add_autoshape_xml() {
    let mut group = make_empty_group();
    group.add_autoshape("ellipse", Emu(100), Emu(200), Emu(300), Emu(400));
    let xml = group.to_xml_string();
    assert!(xml.contains(r#"prst="ellipse""#));
    assert!(xml.contains(r#"id="11""#));
}

// -----------------------------------------------------------------------
// add_picture tests
// -----------------------------------------------------------------------

#[test]
fn test_add_picture() {
    let mut group = make_empty_group();
    let shape = group.add_picture("rId2", Emu(0), Emu(0), Emu(914400), Emu(914400));

    assert_eq!(shape.shape_id(), ShapeId(11));
    assert_eq!(shape.name(), "Picture 1");
    if let Shape::Picture(p) = shape {
        assert_eq!(p.image_r_id.as_deref(), Some("rId2"));
    } else {
        panic!("Expected Picture");
    }
}

#[test]
fn test_add_picture_increments_name() {
    let mut group = make_empty_group();
    group.add_picture("rId1", Emu(0), Emu(0), Emu(100), Emu(100));
    let second = group.add_picture("rId2", Emu(0), Emu(0), Emu(100), Emu(100));
    assert_eq!(second.name(), "Picture 2");
}

#[test]
fn test_add_picture_xml() {
    let mut group = make_empty_group();
    group.add_picture("rId5", Emu(0), Emu(0), Emu(914400), Emu(914400));
    let xml = group.to_xml_string();
    assert!(xml.contains("<p:pic>"));
    assert!(xml.contains(r#"r:embed="rId5""#));
}

// -----------------------------------------------------------------------
// add_connector tests
// -----------------------------------------------------------------------

#[test]
fn test_add_connector_straight() {
    let mut group = make_empty_group();
    let shape = group.add_connector("line", Emu(100), Emu(200), Emu(500), Emu(600));

    assert_eq!(shape.shape_id(), ShapeId(11));
    assert_eq!(shape.name(), "Connector 1");
    if let Shape::Connector(c) = shape {
        assert_eq!(c.prst_geom, Some(PresetGeometry::Line));
        assert_eq!(c.left, Emu(100));
        assert_eq!(c.top, Emu(200));
        assert_eq!(c.width, Emu(400)); // 500 - 100
        assert_eq!(c.height, Emu(400)); // 600 - 200
        assert!(!c.flip_h);
        assert!(!c.flip_v);
    } else {
        panic!("Expected Connector");
    }
}

#[test]
fn test_add_connector_with_flip() {
    let mut group = make_empty_group();
    // end < begin => flipped
    let shape = group.add_connector("line", Emu(500), Emu(600), Emu(100), Emu(200));

    if let Shape::Connector(c) = shape {
        assert!(c.flip_h);
        assert!(c.flip_v);
        assert_eq!(c.left, Emu(100));
        assert_eq!(c.top, Emu(200));
    } else {
        panic!("Expected Connector");
    }
}

#[test]
fn test_add_connector_xml() {
    let mut group = make_empty_group();
    group.add_connector("line", Emu(0), Emu(0), Emu(1000), Emu(1000));
    let xml = group.to_xml_string();
    assert!(xml.contains("<p:cxnSp>"));
    assert!(xml.contains(r#"prst="line""#));
}

// -----------------------------------------------------------------------
// add_table tests
// -----------------------------------------------------------------------

#[test]
fn test_add_table() {
    let mut group = make_empty_group();
    let shape = group.add_table(3, 4, Emu(914400), Emu(914400), Emu(6096000), Emu(370840));

    assert_eq!(shape.shape_id(), ShapeId(11));
    assert_eq!(shape.name(), "Table 1");
    if let Shape::GraphicFrame(g) = shape {
        assert!(g.has_table);
        assert!(!g.has_chart);
        assert_eq!(
            g.graphic_data_uri.as_deref(),
            Some(graphfrm::graphic_data_uri::TABLE)
        );
        assert_eq!(g.height, Emu(370840 * 3));
    } else {
        panic!("Expected GraphicFrame");
    }
}

#[test]
fn test_add_table_increments_name() {
    let mut group = make_empty_group();
    group.add_table(1, 1, Emu(0), Emu(0), Emu(1000), Emu(300));
    let second = group.add_table(2, 2, Emu(0), Emu(0), Emu(1000), Emu(300));
    assert_eq!(second.name(), "Table 2");
}

// -----------------------------------------------------------------------
// add_group_shape tests
// -----------------------------------------------------------------------

#[test]
fn test_add_group_shape() {
    let mut group = make_empty_group();
    let shape = group.add_group_shape();

    assert_eq!(shape.shape_id(), ShapeId(11));
    assert_eq!(shape.name(), "Group 1");
    if let Shape::GroupShape(g) = shape {
        assert!(g.shapes.is_empty());
        assert_eq!(g.left, Emu(0));
        assert_eq!(g.width, Emu(0));
    } else {
        panic!("Expected GroupShape");
    }
}

#[test]
fn test_add_group_shape_increments_name() {
    let mut group = make_empty_group();
    group.add_group_shape();
    let second = group.add_group_shape();
    assert_eq!(second.name(), "Group 2");
}

#[test]
fn test_add_group_shape_xml() {
    let mut group = make_empty_group();
    group.add_group_shape();
    let xml = group.to_xml_string();
    // Should contain nested group shape XML
    let count = xml.matches("<p:grpSp>").count();
    assert_eq!(count, 2); // outer + inner
}
