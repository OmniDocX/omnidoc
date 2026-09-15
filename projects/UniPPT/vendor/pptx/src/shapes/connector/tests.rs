use super::*;
use crate::dml::color::ColorFormat;

fn make_basic_connector() -> Connector {
    Connector::line(
        ShapeId(5),
        "Connector 4",
        Emu(100),
        Emu(200),
        Emu(500),
        Emu(600),
    )
}

#[test]
fn test_basic_connector_xml() {
    let conn = make_basic_connector();
    let xml = conn.to_xml_string();
    assert!(xml.starts_with("<p:cxnSp>"));
    assert!(xml.ends_with("</p:cxnSp>"));
    assert!(xml.contains(r#"id="5""#));
    assert!(xml.contains(r#"name="Connector 4""#));
    assert!(xml.contains(r#"prst="line""#));
}

#[test]
fn test_connector_with_connections() {
    let mut conn = make_basic_connector();
    conn.set_begin_connection(ShapeId(2), ConnectionPointIndex(0));
    conn.set_end_connection(ShapeId(3), ConnectionPointIndex(2));
    let xml = conn.to_xml_string();
    assert!(xml.contains(r#"<a:stCxn id="2" idx="0"/>"#));
    assert!(xml.contains(r#"<a:endCxn id="3" idx="2"/>"#));
}

#[test]
fn test_connector_with_line() {
    let mut conn = make_basic_connector();
    conn.set_line(LineFormat::solid(ColorFormat::rgb(0, 0, 255), Emu(25400)));
    let xml = conn.to_xml_string();
    assert!(xml.contains("<a:ln"));
    assert!(xml.contains("0000FF"));
    assert!(xml.contains(r#"w="25400""#));
}

#[test]
fn test_connector_flip() {
    let mut conn = make_basic_connector();
    conn.flip_h = true;
    let xml = conn.to_xml_string();
    assert!(xml.contains(r#"flipH="1""#));
}

// Tests for begin_x/begin_y/end_x/end_y convenience methods

#[test]
fn test_connector_begin_end_no_flip() {
    let conn = make_basic_connector();
    // No flip: begin is at (left, top), end is at (left+width, top+height)
    assert_eq!(conn.begin_x(), Emu(100));
    assert_eq!(conn.begin_y(), Emu(200));
    assert_eq!(conn.end_x(), Emu(600)); // 100 + 500
    assert_eq!(conn.end_y(), Emu(800)); // 200 + 600
}

#[test]
fn test_connector_begin_end_flip_h() {
    let mut conn = make_basic_connector();
    conn.flip_h = true;
    // flip_h: begin is at (left+width, top), end is at (left, top+height)
    assert_eq!(conn.begin_x(), Emu(600)); // 100 + 500
    assert_eq!(conn.begin_y(), Emu(200));
    assert_eq!(conn.end_x(), Emu(100));
    assert_eq!(conn.end_y(), Emu(800)); // 200 + 600
}

#[test]
fn test_connector_begin_end_flip_v() {
    let mut conn = make_basic_connector();
    conn.flip_v = true;
    // flip_v: begin is at (left, top+height), end is at (left+width, top)
    assert_eq!(conn.begin_x(), Emu(100));
    assert_eq!(conn.begin_y(), Emu(800)); // 200 + 600
    assert_eq!(conn.end_x(), Emu(600)); // 100 + 500
    assert_eq!(conn.end_y(), Emu(200));
}

#[test]
fn test_connector_begin_end_flip_both() {
    let mut conn = make_basic_connector();
    conn.flip_h = true;
    conn.flip_v = true;
    // Both flips: begin is at (left+width, top+height), end is at (left, top)
    assert_eq!(conn.begin_x(), Emu(600)); // 100 + 500
    assert_eq!(conn.begin_y(), Emu(800)); // 200 + 600
    assert_eq!(conn.end_x(), Emu(100));
    assert_eq!(conn.end_y(), Emu(200));
}
