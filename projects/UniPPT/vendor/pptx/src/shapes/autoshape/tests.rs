use super::*;
use crate::dml::color::ColorFormat;
use crate::xml_util::WriteXml;

fn make_basic_shape() -> AutoShape {
    AutoShape::with_geometry(
        ShapeId(2),
        "Rectangle 1",
        Emu(914400),
        Emu(914400),
        Emu(1828800),
        Emu(914400),
        PresetGeometry::Rect,
    )
}

#[test]
fn test_basic_autoshape_xml() {
    let shape = make_basic_shape();
    let xml = shape.to_xml_string();
    assert!(xml.starts_with("<p:sp>"));
    assert!(xml.ends_with("</p:sp>"));
    assert!(xml.contains(r#"id="2""#));
    assert!(xml.contains(r#"name="Rectangle 1""#));
    assert!(xml.contains(r#"prst="rect""#));
    assert!(xml.contains(r#"x="914400""#));
    assert!(xml.contains(r#"cx="1828800""#));
}

#[test]
fn test_autoshape_with_fill_and_line() {
    let mut shape = make_basic_shape();
    shape.set_fill(FillFormat::solid(ColorFormat::rgb(255, 0, 0)));
    shape.set_line(LineFormat::solid(ColorFormat::rgb(0, 0, 0), Emu(12700)));

    let xml = shape.to_xml_string();
    assert!(xml.contains("<a:solidFill>"));
    assert!(xml.contains("FF0000"));
    assert!(xml.contains("<a:ln"));
    assert!(xml.contains("000000"));
}

#[test]
fn test_autoshape_textbox() {
    let mut shape = make_basic_shape();
    shape.is_textbox = true;
    let xml = shape.to_xml_string();
    assert!(xml.contains(r#"txBox="1""#));
}

#[test]
fn test_autoshape_with_text_frame() {
    let mut shape = make_basic_shape();
    let mut tf = TextFrame::new();
    tf.set_text("Hello World");
    shape.text_frame = Some(tf);

    let xml = shape.to_xml_string();
    assert!(xml.contains("<p:txBody>"));
    assert!(xml.contains("Hello World"));
}

#[test]
fn test_autoshape_with_rotation() {
    let mut shape = make_basic_shape();
    shape.rotation = 45.0;
    let xml = shape.to_xml_string();
    assert!(xml.contains(r#"rot="2700000""#));
}

#[test]
fn test_autoshape_with_adjustments() {
    let mut shape = make_basic_shape();
    shape.prst_geom = Some(PresetGeometry::RoundRect);
    shape.adjustments = vec![0.25];
    let xml = shape.to_xml_string();
    assert!(xml.contains(r#"name="adj1""#));
    assert!(xml.contains(r#"fmla="val 25000""#));
}

#[test]
fn test_autoshape_with_click_action() {
    let mut shape = make_basic_shape();
    shape.set_click_action(ActionSetting::next_slide());
    let xml = shape.to_xml_string();
    assert!(xml.contains("<a:hlinkClick"));
    assert!(xml.contains("nextslide"));
}

#[test]
fn test_autoshape_with_hover_action() {
    let mut shape = make_basic_shape();
    shape.set_hover_action(ActionSetting::hyperlink("https://example.com"));
    let xml = shape.to_xml_string();
    assert!(xml.contains("<a:hlinkHover"));
}

#[test]
fn test_autoshape_with_click_and_hover() {
    let mut shape = make_basic_shape();
    shape.set_click_action(ActionSetting::next_slide());
    shape.set_hover_action(ActionSetting::hyperlink("https://example.com"));
    let xml = shape.to_xml_string();
    assert!(xml.contains("<a:hlinkClick"));
    assert!(xml.contains("<a:hlinkHover"));
}

#[test]
fn test_autoshape_with_scene_3d() {
    use crate::dml::effect3d::Scene3D;

    let mut shape = make_basic_shape();
    shape.set_scene_3d(Scene3D::default());
    let xml = shape.to_xml_string();
    assert!(xml.contains("<a:scene3d>"));
    assert!(xml.contains(r#"<a:camera prst="orthographicFront"/>"#));
    assert!(xml.contains(r#"<a:lightRig rig="threePt" dir="t"/>"#));
    assert!(xml.contains("</a:scene3d>"));
    // scene3d should be inside spPr
    let sp_pr_start = xml.find("<p:spPr>").unwrap();
    let sp_pr_end = xml.find("</p:spPr>").unwrap();
    let scene_start = xml.find("<a:scene3d>").unwrap();
    assert!(scene_start > sp_pr_start && scene_start < sp_pr_end);
}

#[test]
fn test_autoshape_with_shape_3d() {
    use crate::dml::effect3d::{Bevel, Shape3D};

    let mut shape = make_basic_shape();
    let mut sp3d = Shape3D::new();
    sp3d.set_bevel_top(Bevel::circle(63500, 25400));
    sp3d.extrusion_height = Some(76200);
    sp3d.set_material("warmMatte");
    shape.set_shape_3d(sp3d);

    let xml = shape.to_xml_string();
    assert!(xml.contains("<a:sp3d"));
    assert!(xml.contains(r#"extrusionH="76200""#));
    assert!(xml.contains(r#"prstMaterial="warmMatte""#));
    assert!(xml.contains(r#"<a:bevelT w="63500" h="25400" prst="circle"/>"#));
    // sp3d should be inside spPr
    let sp_pr_start = xml.find("<p:spPr>").unwrap();
    let sp_pr_end = xml.find("</p:spPr>").unwrap();
    let sp3d_start = xml.find("<a:sp3d").unwrap();
    assert!(sp3d_start > sp_pr_start && sp3d_start < sp_pr_end);
}

#[test]
fn test_autoshape_with_full_3d() {
    use crate::dml::effect3d::{Bevel, Camera, LightRig, Rotation3D, Scene3D, Shape3D};

    let mut shape = make_basic_shape();
    shape.set_scene_3d(Scene3D::new(
        Camera::with_rotation("orthographicFront", Rotation3D::new(0, 0, 0)),
        LightRig::new("threePt", "t"),
    ));
    let mut sp3d = Shape3D::new();
    sp3d.extrusion_height = Some(76200);
    sp3d.contour_width = Some(12700);
    sp3d.set_material("warmMatte");
    sp3d.set_bevel_top(Bevel::circle(63500, 25400));
    sp3d.set_bevel_bottom(Bevel::circle(63500, 25400));
    sp3d.extrusion_color = Some(ColorFormat::rgb(255, 0, 0));
    sp3d.contour_color = Some(ColorFormat::rgb(0, 0, 255));
    shape.set_shape_3d(sp3d);

    let xml = shape.to_xml_string();
    assert!(xml.contains("<a:scene3d>"));
    assert!(xml.contains("<a:sp3d"));
    assert!(xml.contains("a:bevelT"));
    assert!(xml.contains("a:bevelB"));
    assert!(xml.contains("a:extrusionClr"));
    assert!(xml.contains("a:contourClr"));
}

#[test]
fn test_autoshape_3d_getters() {
    use crate::dml::effect3d::{Bevel, Scene3D, Shape3D};

    let mut shape = make_basic_shape();
    assert!(shape.scene_3d().is_none());
    assert!(shape.shape_3d().is_none());

    shape.set_scene_3d(Scene3D::default());
    shape.set_shape_3d(Shape3D::with_bevel_top(Bevel::circle(63500, 25400)));

    assert!(shape.scene_3d().is_some());
    assert_eq!(shape.scene_3d().unwrap().camera.preset, "orthographicFront");
    assert!(shape.shape_3d().is_some());
    assert_eq!(
        shape.shape_3d().unwrap().bevel_top.as_ref().unwrap().width,
        63500
    );
}
