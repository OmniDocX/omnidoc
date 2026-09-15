use super::*;
use crate::enums::shapes::PresetGeometry;

fn blank_slide_xml() -> Vec<u8> {
    crate::slide::new_slide_xml()
}

#[test]
fn test_add_shape_rectangle() {
    let slide = blank_slide_xml();
    let updated = ShapeTree::add_shape(
        &slide,
        MsoAutoShapeType::Rectangle,
        Emu(914400),
        Emu(914400),
        Emu(1828800),
        Emu(914400),
    )
    .unwrap();

    let tree = ShapeTree::from_slide_xml(&updated).unwrap();
    assert_eq!(tree.len(), 1);
    let s = &tree.shapes[0];
    assert_eq!(s.shape_id(), ShapeId(1));
    assert!(s.name().starts_with("Rectangle"));
    assert_eq!(s.left(), Emu(914400));
    assert_eq!(s.width(), Emu(1828800));
}

#[test]
fn test_add_shape_oval() {
    let slide = blank_slide_xml();
    let updated = ShapeTree::add_shape(
        &slide,
        MsoAutoShapeType::Oval,
        Emu(0),
        Emu(0),
        Emu(914400),
        Emu(914400),
    )
    .unwrap();

    let tree = ShapeTree::from_slide_xml(&updated).unwrap();
    assert_eq!(tree.len(), 1);
    let s = &tree.shapes[0];
    assert!(s.name().starts_with("Oval"));
    if let Shape::AutoShape(a) = s {
        assert_eq!(a.prst_geom, Some(PresetGeometry::Ellipse));
    } else {
        panic!("Expected AutoShape");
    }
}

#[test]
fn test_add_multiple_shapes_increments_ids() {
    let slide = blank_slide_xml();
    let updated = ShapeTree::add_shape(
        &slide,
        MsoAutoShapeType::Rectangle,
        Emu(0),
        Emu(0),
        Emu(100),
        Emu(100),
    )
    .unwrap();
    let updated = ShapeTree::add_shape(
        &updated,
        MsoAutoShapeType::Rectangle,
        Emu(200),
        Emu(200),
        Emu(100),
        Emu(100),
    )
    .unwrap();

    let tree = ShapeTree::from_slide_xml(&updated).unwrap();
    assert_eq!(tree.len(), 2);
    assert_eq!(tree.shapes[0].shape_id(), ShapeId(1));
    assert_eq!(tree.shapes[1].shape_id(), ShapeId(2));
    assert_ne!(tree.shapes[0].name(), tree.shapes[1].name());
}

#[test]
fn test_add_textbox() {
    let slide = blank_slide_xml();
    let updated =
        ShapeTree::add_textbox(&slide, Emu(100000), Emu(200000), Emu(2743200), Emu(457200))
            .unwrap();

    let tree = ShapeTree::from_slide_xml(&updated).unwrap();
    assert_eq!(tree.len(), 1);
    let s = &tree.shapes[0];
    assert!(s.name().starts_with("TextBox"));
    assert!(s.has_text_frame());
    if let Shape::AutoShape(a) = s {
        assert!(a.is_textbox);
    } else {
        panic!("Expected AutoShape (textbox)");
    }
}

#[test]
fn test_add_picture() {
    let slide = blank_slide_xml();
    let updated =
        ShapeTree::add_picture(&slide, "rId2", Emu(0), Emu(0), Emu(914400), Emu(914400)).unwrap();

    let tree = ShapeTree::from_slide_xml(&updated).unwrap();
    assert_eq!(tree.len(), 1);
    let s = &tree.shapes[0];
    assert!(s.name().starts_with("Picture"));
    if let Shape::Picture(p) = s {
        assert_eq!(p.image_r_id.as_deref(), Some("rId2"));
    } else {
        panic!("Expected Picture shape");
    }
}

#[test]
fn test_add_table() {
    let slide = blank_slide_xml();
    let updated = ShapeTree::add_table(
        &slide,
        3,
        4,
        Emu(914400),
        Emu(914400),
        Emu(4572000),
        Emu(1371600),
    )
    .unwrap();

    let tree = ShapeTree::from_slide_xml(&updated).unwrap();
    assert_eq!(tree.len(), 1);
    let s = &tree.shapes[0];
    assert!(s.name().starts_with("Table"));
    assert!(s.has_table());
}

#[test]
fn test_add_connector_straight() {
    let slide = blank_slide_xml();
    let updated = ShapeTree::add_connector(
        &slide,
        MsoConnectorType::Straight,
        Emu(100),
        Emu(200),
        Emu(500),
        Emu(600),
    )
    .unwrap();

    let tree = ShapeTree::from_slide_xml(&updated).unwrap();
    assert_eq!(tree.len(), 1);
    let s = &tree.shapes[0];
    assert!(s.name().starts_with("Connector"));
    if let Shape::Connector(c) = s {
        assert_eq!(c.prst_geom, Some(PresetGeometry::Line));
        assert!(!c.flip_h);
        assert!(!c.flip_v);
        assert_eq!(c.left, Emu(100));
        assert_eq!(c.top, Emu(200));
        assert_eq!(c.width, Emu(400));
        assert_eq!(c.height, Emu(400));
    } else {
        panic!("Expected Connector shape");
    }
}

#[test]
fn test_add_connector_with_flip() {
    let slide = blank_slide_xml();
    let updated = ShapeTree::add_connector(
        &slide,
        MsoConnectorType::Straight,
        Emu(500),
        Emu(600),
        Emu(100),
        Emu(200),
    )
    .unwrap();

    let tree = ShapeTree::from_slide_xml(&updated).unwrap();
    assert_eq!(tree.len(), 1);
    if let Shape::Connector(c) = &tree.shapes[0] {
        assert!(c.flip_h);
        assert!(c.flip_v);
        assert_eq!(c.left, Emu(100));
        assert_eq!(c.top, Emu(200));
    } else {
        panic!("Expected Connector shape");
    }
}

#[test]
fn test_add_connector_elbow() {
    let slide = blank_slide_xml();
    let updated = ShapeTree::add_connector(
        &slide,
        MsoConnectorType::Elbow,
        Emu(0),
        Emu(0),
        Emu(1000),
        Emu(1000),
    )
    .unwrap();

    let tree = ShapeTree::from_slide_xml(&updated).unwrap();
    if let Shape::Connector(c) = &tree.shapes[0] {
        assert_eq!(c.prst_geom, Some(PresetGeometry::BentConnector3));
    } else {
        panic!("Expected Connector");
    }
}

#[test]
fn test_add_group_shape() {
    let slide = blank_slide_xml();
    let updated =
        ShapeTree::add_group_shape(&slide, Emu(0), Emu(0), Emu(914400), Emu(914400)).unwrap();

    let tree = ShapeTree::from_slide_xml(&updated).unwrap();
    assert_eq!(tree.len(), 1);
    let s = &tree.shapes[0];
    assert!(s.name().starts_with("Group"));
    if let Shape::GroupShape(g) = s {
        assert!(g.shapes.is_empty());
    } else {
        panic!("Expected GroupShape");
    }
}

#[test]
fn test_add_mixed_shapes() {
    let slide = blank_slide_xml();
    let updated = ShapeTree::add_shape(
        &slide,
        MsoAutoShapeType::Rectangle,
        Emu(0),
        Emu(0),
        Emu(100),
        Emu(100),
    )
    .unwrap();
    let updated = ShapeTree::add_textbox(&updated, Emu(200), Emu(200), Emu(300), Emu(100)).unwrap();
    let updated =
        ShapeTree::add_picture(&updated, "rId3", Emu(400), Emu(400), Emu(200), Emu(200)).unwrap();

    let tree = ShapeTree::from_slide_xml(&updated).unwrap();
    assert_eq!(tree.len(), 3);
    assert_eq!(tree.shapes[0].shape_id(), ShapeId(1));
    assert_eq!(tree.shapes[1].shape_id(), ShapeId(2));
    assert_eq!(tree.shapes[2].shape_id(), ShapeId(3));
    assert!(matches!(&tree.shapes[0], Shape::AutoShape(_)));
    assert!(matches!(&tree.shapes[1], Shape::AutoShape(_)));
    assert!(matches!(&tree.shapes[2], Shape::Picture(_)));
}
