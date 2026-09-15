use super::*;
use crate::dml::color::ColorFormat;
use crate::xml_util::WriteXml;

fn make_basic_picture() -> Picture {
    let mut pic = Picture::new(
        ShapeId(4),
        "Picture 3",
        Emu(0),
        Emu(0),
        Emu(914400),
        Emu(914400),
        "rId2",
    );
    pic.description = Some("Test image".to_string());
    pic
}

#[test]
fn test_basic_picture_xml() {
    let pic = make_basic_picture();
    let xml = pic.to_xml_string();
    assert!(xml.starts_with("<p:pic>"));
    assert!(xml.ends_with("</p:pic>"));
    assert!(xml.contains(r#"id="4""#));
    assert!(xml.contains(r#"name="Picture 3""#));
    assert!(xml.contains(r#"descr="Test image""#));
    assert!(xml.contains(r#"r:embed="rId2""#));
    assert!(xml.contains("noChangeAspect"));
}

#[test]
fn test_picture_flip_xml() {
    let mut pic = make_basic_picture();
    pic.flip_h = true;
    pic.flip_v = true;
    let xml = pic.to_xml_string();
    assert!(xml.contains(r#"flipH="1""#));
    assert!(xml.contains(r#"flipV="1""#));
}

#[test]
fn test_picture_with_crop() {
    let mut pic = make_basic_picture();
    pic.set_crop(0.1, 0.2, 0.15, 0.05).unwrap();
    let xml = pic.to_xml_string();
    assert!(xml.contains("<a:srcRect"));
    assert!(xml.contains(r#"l="10000""#));
    assert!(xml.contains(r#"t="20000""#));
    assert!(xml.contains(r#"r="15000""#));
    assert!(xml.contains(r#"b="5000""#));
}

#[test]
fn test_picture_with_line() {
    let mut pic = make_basic_picture();
    pic.set_line(LineFormat::solid(ColorFormat::rgb(0, 0, 0), Emu(12700)));
    let xml = pic.to_xml_string();
    assert!(xml.contains("<a:ln"));
    assert!(xml.contains("000000"));
}

#[test]
fn test_picture_no_crop_no_src_rect() {
    let pic = make_basic_picture();
    let xml = pic.to_xml_string();
    assert!(!xml.contains("<a:srcRect"));
}

#[test]
fn test_picture_with_hover_action() {
    let mut pic = make_basic_picture();
    pic.set_hover_action(ActionSetting::hyperlink("https://example.com"));
    let xml = pic.to_xml_string();
    assert!(xml.contains("<a:hlinkHover"));
}

#[test]
fn test_picture_with_auto_shape_type() {
    let mut pic = make_basic_picture();
    pic.set_auto_shape_type(MsoAutoShapeType::Oval);
    let xml = pic.to_xml_string();
    assert!(xml.contains(r#"prst="ellipse""#));
    assert!(!xml.contains(r#"prst="rect""#));
}

#[test]
fn test_picture_default_geometry_is_rect() {
    let pic = make_basic_picture();
    let xml = pic.to_xml_string();
    assert!(xml.contains(r#"prst="rect""#));
}

#[test]
fn test_picture_with_rounded_rect_mask() {
    let mut pic = make_basic_picture();
    pic.set_auto_shape_type(MsoAutoShapeType::RoundedRectangle);
    let xml = pic.to_xml_string();
    assert!(xml.contains(r#"prst="roundRect""#));
}

#[test]
fn test_picture_image_proxy_none() {
    let pic = make_basic_picture();
    assert!(pic.image().is_none());
}

#[test]
fn test_picture_image_proxy_with_data() {
    let mut pic = make_basic_picture();
    pic.image_data = Some(vec![1, 2, 3].into());
    pic.image_content_type = Some("image/png".to_string());
    let img = pic.image().unwrap();
    assert_eq!(img.blob(), &[1, 2, 3]);
    assert_eq!(img.content_type(), "image/png");
    assert_eq!(img.ext(), "png");
}

#[test]
fn test_picture_image_proxy_missing_content_type() {
    let mut pic = make_basic_picture();
    pic.image_data = Some(vec![1, 2, 3].into());
    // image_content_type is None
    assert!(pic.image().is_none());
}

#[test]
fn test_picture_with_scene_3d() {
    use crate::dml::effect3d::Scene3D;

    let mut pic = make_basic_picture();
    pic.set_scene_3d(Scene3D::default());
    let xml = pic.to_xml_string();
    assert!(xml.contains("<a:scene3d>"));
    assert!(xml.contains(r#"<a:camera prst="orthographicFront"/>"#));
    assert!(xml.contains(r#"<a:lightRig rig="threePt" dir="t"/>"#));
    // scene3d should be inside spPr
    let sp_pr_start = xml.find("<p:spPr>").unwrap();
    let sp_pr_end = xml.find("</p:spPr>").unwrap();
    let scene_start = xml.find("<a:scene3d>").unwrap();
    assert!(scene_start > sp_pr_start && scene_start < sp_pr_end);
}

#[test]
fn test_picture_with_shape_3d() {
    use crate::dml::effect3d::{Bevel, Shape3D};

    let mut pic = make_basic_picture();
    let mut sp3d = Shape3D::new();
    sp3d.set_bevel_top(Bevel::circle(63500, 25400));
    sp3d.set_material("metal");
    pic.set_shape_3d(sp3d);

    let xml = pic.to_xml_string();
    assert!(xml.contains("<a:sp3d"));
    assert!(xml.contains(r#"prstMaterial="metal""#));
    assert!(xml.contains(r#"<a:bevelT w="63500" h="25400" prst="circle"/>"#));
}

#[test]
fn test_picture_3d_getters() {
    use crate::dml::effect3d::{Bevel, Scene3D, Shape3D};

    let mut pic = make_basic_picture();
    assert!(pic.scene_3d().is_none());
    assert!(pic.shape_3d().is_none());

    pic.set_scene_3d(Scene3D::default());
    pic.set_shape_3d(Shape3D::with_bevel_top(Bevel::circle(63500, 25400)));

    assert!(pic.scene_3d().is_some());
    assert!(pic.shape_3d().is_some());
}

// --- Picture::set_crop validation tests ---

#[test]
fn test_set_crop_boundary_values() {
    let mut pic = make_basic_picture();
    assert!(pic.set_crop(0.0, 0.0, 0.0, 0.0).is_ok());
    assert!(pic.set_crop(1.0, 1.0, 1.0, 1.0).is_ok());
}

#[test]
fn test_set_crop_negative_left() {
    let mut pic = make_basic_picture();
    assert!(pic.set_crop(-0.1, 0.0, 0.0, 0.0).is_err());
}

#[test]
fn test_set_crop_over_one_right() {
    let mut pic = make_basic_picture();
    assert!(pic.set_crop(0.0, 0.0, 1.1, 0.0).is_err());
}

#[test]
fn test_set_crop_negative_top() {
    let mut pic = make_basic_picture();
    assert!(pic.set_crop(0.0, -0.5, 0.0, 0.0).is_err());
}

#[test]
fn test_set_crop_over_one_bottom() {
    let mut pic = make_basic_picture();
    assert!(pic.set_crop(0.0, 0.0, 0.0, 2.0).is_err());
}

// --- Individual crop setter tests ---

#[test]
fn test_set_crop_left_individual() {
    let mut pic = make_basic_picture();
    pic.set_crop_left(0.25).unwrap();
    let xml = pic.to_xml_string();
    assert!(xml.contains(r#"l="25000""#));
    assert!(!xml.contains(r#" t="#));
}

#[test]
fn test_set_crop_top_individual() {
    let mut pic = make_basic_picture();
    pic.set_crop_top(0.3).unwrap();
    let xml = pic.to_xml_string();
    assert!(xml.contains(r#"t="30000""#));
}

#[test]
fn test_set_crop_right_individual() {
    let mut pic = make_basic_picture();
    pic.set_crop_right(0.5).unwrap();
    let xml = pic.to_xml_string();
    assert!(xml.contains(r#"r="50000""#));
}

#[test]
fn test_set_crop_bottom_individual() {
    let mut pic = make_basic_picture();
    pic.set_crop_bottom(0.1).unwrap();
    let xml = pic.to_xml_string();
    assert!(xml.contains(r#"b="10000""#));
}

#[test]
fn test_set_crop_left_invalid() {
    let mut pic = make_basic_picture();
    assert!(pic.set_crop_left(-0.1).is_err());
    assert!(pic.set_crop_left(1.5).is_err());
}

#[test]
fn test_set_crop_top_invalid() {
    let mut pic = make_basic_picture();
    assert!(pic.set_crop_top(-0.01).is_err());
    assert!(pic.set_crop_top(1.01).is_err());
}

#[test]
fn test_set_crop_right_invalid() {
    let mut pic = make_basic_picture();
    assert!(pic.set_crop_right(-1.0).is_err());
    assert!(pic.set_crop_right(2.0).is_err());
}

#[test]
fn test_set_crop_bottom_invalid() {
    let mut pic = make_basic_picture();
    assert!(pic.set_crop_bottom(-0.5).is_err());
    assert!(pic.set_crop_bottom(100.0).is_err());
}

#[test]
fn test_individual_setters_combine_in_xml() {
    let mut pic = make_basic_picture();
    pic.set_crop_left(0.1).unwrap();
    pic.set_crop_top(0.2).unwrap();
    pic.set_crop_right(0.15).unwrap();
    pic.set_crop_bottom(0.05).unwrap();
    let xml = pic.to_xml_string();
    assert!(xml.contains("<a:srcRect"));
    assert!(xml.contains(r#"l="10000""#));
    assert!(xml.contains(r#"t="20000""#));
    assert!(xml.contains(r#"r="15000""#));
    assert!(xml.contains(r#"b="5000""#));
}
