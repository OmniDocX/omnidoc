use super::*;

fn blank_slide_xml() -> Vec<u8> {
    crate::slide::new_slide_xml()
}

// =========================================================
// Tests for add_movie, title, placeholders
// =========================================================

#[test]
fn test_new_movie_xml() {
    let xml = ShapeTree::new_movie_xml(
        ShapeId(5),
        "Movie 1",
        "rIdVideo",
        "rIdPoster",
        Emu(0),
        Emu(0),
        Emu(914400),
        Emu(914400),
    );
    assert!(xml.contains(r#"id="5""#));
    assert!(xml.contains(r#"name="Movie 1""#));
    assert!(xml.contains(r#"r:link="rIdVideo""#));
    assert!(xml.contains(r#"r:embed="rIdPoster""#));
    assert!(xml.contains("a:videoFile"));
    assert!(xml.contains("noChangeAspect"));
    assert!(xml.contains(r#"prst="rect""#));
}

#[test]
fn test_add_movie() {
    let slide = blank_slide_xml();
    let updated = ShapeTree::add_movie(
        &slide,
        "rIdVideo",
        "rIdPoster",
        Emu(0),
        Emu(0),
        Emu(914400),
        Emu(914400),
    )
    .unwrap();

    let tree = ShapeTree::from_slide_xml(&updated).unwrap();
    assert_eq!(tree.len(), 1);
    let s = &tree.shapes[0];
    assert!(s.name().starts_with("Movie"));
    assert_eq!(s.shape_id(), ShapeId(1));
}

#[test]
fn test_title_returns_title_placeholder() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
<p:grpSpPr/>
<p:sp>
  <p:nvSpPr><p:cNvPr id="2" name="Title 1"/><p:cNvSpPr/><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr>
  <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100" cy="100"/></a:xfrm></p:spPr>
  <p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr/></a:p></p:txBody>
</p:sp>
<p:sp>
  <p:nvSpPr><p:cNvPr id="3" name="Body 2"/><p:cNvSpPr/><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr>
  <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100" cy="100"/></a:xfrm></p:spPr>
  <p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr/></a:p></p:txBody>
</p:sp>
</p:spTree></p:cSld></p:sld>"#;

    let tree = ShapeTree::from_slide_xml(xml).unwrap();
    let title = tree.title();
    assert!(title.is_some());
    assert_eq!(title.unwrap().name(), "Title 1");
}

#[test]
fn test_title_returns_ctr_title() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
<p:grpSpPr/>
<p:sp>
  <p:nvSpPr><p:cNvPr id="2" name="Center Title 1"/><p:cNvSpPr/><p:nvPr><p:ph type="ctrTitle"/></p:nvPr></p:nvSpPr>
  <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100" cy="100"/></a:xfrm></p:spPr>
  <p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr/></a:p></p:txBody>
</p:sp>
</p:spTree></p:cSld></p:sld>"#;

    let tree = ShapeTree::from_slide_xml(xml).unwrap();
    let title = tree.title();
    assert!(title.is_some());
    assert_eq!(title.unwrap().name(), "Center Title 1");
}

#[test]
fn test_title_returns_none_when_no_title() {
    let slide = blank_slide_xml();
    let tree = ShapeTree::from_slide_xml(&slide).unwrap();
    assert!(tree.title().is_none());
}

#[test]
fn test_placeholders() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
<p:grpSpPr/>
<p:sp>
  <p:nvSpPr><p:cNvPr id="2" name="Title 1"/><p:cNvSpPr/><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr>
  <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100" cy="100"/></a:xfrm></p:spPr>
  <p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr/></a:p></p:txBody>
</p:sp>
<p:sp>
  <p:nvSpPr><p:cNvPr id="3" name="Body 2"/><p:cNvSpPr/><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr>
  <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100" cy="100"/></a:xfrm></p:spPr>
  <p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr/></a:p></p:txBody>
</p:sp>
<p:sp>
  <p:nvSpPr><p:cNvPr id="4" name="TextBox 3"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
  <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100" cy="100"/></a:xfrm></p:spPr>
  <p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr/></a:p></p:txBody>
</p:sp>
</p:spTree></p:cSld></p:sld>"#;

    let tree = ShapeTree::from_slide_xml(xml).unwrap();
    assert_eq!(tree.len(), 3);

    let placeholders = tree.placeholders();
    assert_eq!(placeholders.len(), 2);
    assert_eq!(placeholders[0].name(), "Title 1");
    assert_eq!(placeholders[1].name(), "Body 2");
}

#[test]
fn test_placeholders_empty_slide() {
    let slide = blank_slide_xml();
    let tree = ShapeTree::from_slide_xml(&slide).unwrap();
    let placeholders = tree.placeholders();
    assert!(placeholders.is_empty());
}

// =========================================================
// Tests for enhanced parsing (read-modify-write)
// =========================================================

#[test]
fn test_parse_shape_with_text_frame() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
<p:grpSpPr/>
<p:sp>
  <p:nvSpPr><p:cNvPr id="2" name="Title 1"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
  <p:spPr><a:xfrm><a:off x="457200" y="274638"/><a:ext cx="8229600" cy="1143000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill><a:ln w="12700"><a:solidFill><a:srgbClr val="000000"/></a:solidFill></a:ln></p:spPr>
  <p:txBody><a:bodyPr wrap="square" anchor="ctr"/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="2400" b="1"><a:solidFill><a:srgbClr val="FFFFFF"/></a:solidFill><a:latin typeface="Arial"/></a:rPr><a:t>Hello World</a:t></a:r></a:p></p:txBody>
</p:sp>
</p:spTree></p:cSld></p:sld>"#;

    let tree = ShapeTree::from_slide_xml(xml).unwrap();
    assert_eq!(tree.len(), 1);

    let shape = &tree.shapes[0];
    assert_eq!(shape.name(), "Title 1");

    if let Shape::AutoShape(a) = shape {
        assert!(a.text_frame.is_some());
        let tf = a.text_frame.as_ref().unwrap();
        assert!(tf.word_wrap);
        assert_eq!(
            tf.vertical_anchor,
            Some(crate::enums::text::MsoVerticalAnchor::Middle)
        );
        assert_eq!(tf.paragraphs().len(), 1);
        let para = &tf.paragraphs()[0];
        assert_eq!(
            para.alignment,
            Some(crate::enums::text::PpParagraphAlignment::Center)
        );
        assert_eq!(para.runs().len(), 1);
        assert_eq!(para.runs()[0].text(), "Hello World");
        assert_eq!(para.runs()[0].font().bold, Some(true));
        assert_eq!(para.runs()[0].font().size, Some(24.0));
        assert_eq!(para.runs()[0].font().name.as_deref(), Some("Arial"));

        assert!(a.fill.is_some());
        match a.fill.as_ref().unwrap() {
            crate::dml::fill::FillFormat::Solid(sf) => match &sf.color {
                crate::dml::color::ColorFormat::Rgb(rgb) => {
                    assert_eq!(rgb.r, 255);
                    assert_eq!(rgb.g, 0);
                    assert_eq!(rgb.b, 0);
                }
                _ => panic!("Expected RGB color"),
            },
            _ => panic!("Expected Solid fill"),
        }

        assert!(a.line.is_some());
        let line = a.line.as_ref().unwrap();
        assert_eq!(line.width, Some(Emu(12700)));
    } else {
        panic!("Expected AutoShape");
    }
}

#[test]
fn test_shape_tree_preserves_whitespace_only_text_runs() {
    let xml = br#"<p:sld xmlns:p="p" xmlns:a="a"><p:cSld><p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
      <p:sp><p:nvSpPr><p:cNvPr id="2" name="Text 1"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p>
      <a:r><a:rPr/><a:t>Lorem</a:t></a:r><a:r><a:rPr/><a:t xml:space="preserve"> </a:t></a:r><a:r><a:rPr/><a:t>ipsum</a:t></a:r>
      </a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"#;
    let tree = ShapeTree::from_slide_xml(xml).unwrap();
    let Shape::AutoShape(shape) = &tree.shapes[0] else {
        panic!("expected auto shape");
    };
    let runs = shape.text_frame().unwrap().paragraphs()[0].runs();
    assert_eq!(runs[0].text(), "Lorem");
    assert_eq!(runs[1].text(), " ");
    assert_eq!(runs[2].text(), "ipsum");
}

#[test]
fn test_parse_shape_with_no_fill() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
<p:grpSpPr/>
<p:sp>
  <p:nvSpPr><p:cNvPr id="2" name="Shape 1"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
  <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100" cy="100"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/></p:spPr>
</p:sp>
</p:spTree></p:cSld></p:sld>"#;

    let tree = ShapeTree::from_slide_xml(xml).unwrap();
    assert_eq!(tree.len(), 1);

    if let Shape::AutoShape(a) = &tree.shapes[0] {
        assert_eq!(a.fill, Some(crate::dml::fill::FillFormat::NoFill));
    } else {
        panic!("Expected AutoShape");
    }
}

#[test]
fn test_parse_connector_with_line() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
<p:grpSpPr/>
<p:cxnSp>
  <p:nvCxnSpPr><p:cNvPr id="5" name="Connector 1"/><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr>
  <p:spPr><a:xfrm><a:off x="100" y="200"/><a:ext cx="500" cy="600"/></a:xfrm><a:prstGeom prst="line"><a:avLst/></a:prstGeom><a:ln w="25400"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill><a:prstDash val="dash"/></a:ln></p:spPr>
</p:cxnSp>
</p:spTree></p:cSld></p:sld>"#;

    let tree = ShapeTree::from_slide_xml(xml).unwrap();
    assert_eq!(tree.len(), 1);

    if let Shape::Connector(c) = &tree.shapes[0] {
        assert!(c.line.is_some());
        let line = c.line.as_ref().unwrap();
        assert_eq!(line.width, Some(Emu(25400)));
        assert_eq!(
            line.dash_style,
            Some(crate::enums::dml::MsoLineDashStyle::Dash)
        );
    } else {
        panic!("Expected Connector");
    }
}
