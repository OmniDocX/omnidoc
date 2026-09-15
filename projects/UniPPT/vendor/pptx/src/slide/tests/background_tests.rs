use crate::slide::*;

#[test]
fn test_set_slide_background_solid() {
    let mut xml = new_slide_xml();
    set_slide_background_solid(&mut xml, "FF0000").unwrap();
    let s = String::from_utf8(xml).unwrap();
    assert!(s.contains("<p:bg>"));
    assert!(s.contains(r#"val="FF0000""#));
}

#[test]
fn test_set_slide_background_gradient() {
    use crate::dml::color::ColorFormat;
    use crate::dml::fill::{GradientFill, GradientStop};

    let mut xml = new_slide_xml();
    let gradient = GradientFill {
        stops: vec![
            GradientStop {
                position: 0.0,
                color: ColorFormat::rgb(255, 0, 0),
                opacity: 1.0,
            },
            GradientStop {
                position: 1.0,
                color: ColorFormat::rgb(0, 0, 255),
                opacity: 1.0,
            },
        ],
        angle: Some(90.0),
    };
    set_slide_background_gradient(&mut xml, &gradient).unwrap();
    let s = String::from_utf8(xml).unwrap();
    assert!(s.contains("<p:bg>"));
    assert!(s.contains("<a:gradFill>"));
    assert!(s.contains("FF0000"));
    assert!(s.contains("0000FF"));
    assert!(s.contains("<a:lin"));
}

#[test]
fn test_set_slide_background_image() {
    let mut xml = new_slide_xml();
    set_slide_background_image(&mut xml, "rId5").unwrap();
    let s = String::from_utf8(xml).unwrap();
    assert!(s.contains("<p:bg>"));
    assert!(s.contains("<a:blipFill>"));
    assert!(s.contains(r#"r:embed="rId5""#));
    assert!(s.contains("<a:stretch>"));
}

#[test]
fn test_set_follow_master_background_remove() {
    let mut xml = new_slide_xml();
    set_slide_background_solid(&mut xml, "FF0000").unwrap();
    let s = std::str::from_utf8(&xml).unwrap();
    assert!(s.contains("<p:bg>"));

    set_follow_master_background(&mut xml, true).unwrap();
    let s = String::from_utf8(xml).unwrap();
    assert!(!s.contains("<p:bg>"));
}

#[test]
fn test_set_follow_master_background_false_noop() {
    let mut xml = new_slide_xml();
    let original = xml.clone();
    set_follow_master_background(&mut xml, false).unwrap();
    assert_eq!(xml, original);
}

#[test]
fn test_gradient_background_replaces_existing() {
    use crate::dml::color::ColorFormat;
    use crate::dml::fill::{GradientFill, GradientStop};

    let mut xml = new_slide_xml();
    set_slide_background_solid(&mut xml, "FF0000").unwrap();

    let gradient = GradientFill {
        stops: vec![
            GradientStop {
                position: 0.0,
                color: ColorFormat::rgb(0, 255, 0),
                opacity: 1.0,
            },
            GradientStop {
                position: 1.0,
                color: ColorFormat::rgb(0, 0, 255),
                opacity: 1.0,
            },
        ],
        angle: Some(45.0),
    };
    set_slide_background_gradient(&mut xml, &gradient).unwrap();
    let s = String::from_utf8(xml).unwrap();
    assert!(s.contains("<a:gradFill>"));
    assert!(!s.contains("FF0000"));
    assert_eq!(s.matches("<p:bg>").count(), 1);
}

#[test]
fn test_image_background_replaces_existing() {
    let mut xml = new_slide_xml();
    set_slide_background_solid(&mut xml, "FF0000").unwrap();
    set_slide_background_image(&mut xml, "rId10").unwrap();
    let s = String::from_utf8(xml).unwrap();
    assert!(s.contains("<a:blipFill>"));
    assert!(!s.contains("solidFill"));
    assert_eq!(s.matches("<p:bg>").count(), 1);
}

#[test]
fn test_remove_bg_element() {
    let xml = r#"<p:cSld><p:bg><p:bgPr><a:solidFill/></p:bgPr></p:bg><p:spTree/></p:cSld>"#;
    let result = background::remove_bg_element(xml);
    assert!(!result.contains("<p:bg>"));
    assert!(result.contains("<p:spTree/>"));
}

#[test]
fn test_remove_bg_element_no_bg() {
    let xml = r#"<p:cSld><p:spTree/></p:cSld>"#;
    let result = background::remove_bg_element(xml);
    assert_eq!(result, xml);
}
