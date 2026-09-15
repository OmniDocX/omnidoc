//! Tests for the `TextFrame` type.

use super::*;
use crate::dml::color::ColorFormat;
use crate::enums::text::{MsoAutoSize, MsoTextUnderlineType};
use crate::shapes::action::Hyperlink;
use crate::text::bullet::BulletFormat;
use crate::text::font::{Font, RgbColor};
use crate::units::RelationshipId;

#[test]
fn test_textframe_new_has_one_paragraph() {
    let tf = TextFrame::new();
    assert_eq!(tf.paragraphs().len(), 1);
    assert_eq!(tf.text(), "");
}

#[test]
fn test_textframe_set_text_simple() {
    let mut tf = TextFrame::new();
    tf.set_text("Hello World");
    assert_eq!(tf.text(), "Hello World");
    assert_eq!(tf.paragraphs().len(), 1);
}

#[test]
fn test_textframe_set_text_multiline() {
    let mut tf = TextFrame::new();
    tf.set_text("Line 1\nLine 2\nLine 3");
    assert_eq!(tf.text(), "Line 1\nLine 2\nLine 3");
    assert_eq!(tf.paragraphs().len(), 3);
}

#[test]
fn test_textframe_add_paragraph() {
    let mut tf = TextFrame::new();
    tf.set_text("First");
    let p = tf.add_paragraph();
    p.add_run().set_text("Second");
    assert_eq!(tf.paragraphs().len(), 2);
    assert_eq!(tf.text(), "First\nSecond");
}

#[test]
fn test_textframe_clear() {
    let mut tf = TextFrame::new();
    tf.set_text("Line 1\nLine 2");
    tf.clear();
    assert_eq!(tf.paragraphs().len(), 1);
    assert_eq!(tf.text(), "");
}

#[test]
fn test_textframe_paragraphs_mut() {
    use crate::enums::text::PpParagraphAlignment;
    let mut tf = TextFrame::new();
    tf.set_text("Hello");
    tf.paragraphs_mut()[0].set_alignment(PpParagraphAlignment::Right);
    assert_eq!(
        tf.paragraphs()[0].alignment,
        Some(PpParagraphAlignment::Right)
    );
}

#[test]
fn test_textframe_default_margins() {
    let tf = TextFrame::new();
    assert_eq!(tf.margin_left, Some(Emu(91440)));
    assert_eq!(tf.margin_right, Some(Emu(91440)));
    assert_eq!(tf.margin_top, Some(Emu(45720)));
    assert_eq!(tf.margin_bottom, Some(Emu(45720)));
}

#[test]
fn test_textframe_to_xml_string() {
    let mut tf = TextFrame::new();
    tf.set_text("Hello World");
    let xml = tf.to_xml_string();

    assert!(xml.starts_with("<p:txBody>"));
    assert!(xml.ends_with("</p:txBody>"));
    assert!(xml.contains("<a:bodyPr"));
    assert!(xml.contains(r#"wrap="square""#));
    assert!(xml.contains(r#"lIns="91440""#));
    assert!(xml.contains(r#"tIns="45720""#));
    assert!(xml.contains(r#"rIns="91440""#));
    assert!(xml.contains(r#"bIns="45720""#));
    assert!(xml.contains("<a:lstStyle/>"));
    assert!(xml.contains("<a:p>"));
    assert!(xml.contains("<a:t>Hello World</a:t>"));
}

#[test]
fn test_textframe_to_xml_no_wrap() {
    let mut tf = TextFrame::new();
    tf.word_wrap = false;
    let xml = tf.to_xml_string();
    assert!(xml.contains(r#"wrap="none""#));
}

#[test]
fn test_textframe_to_xml_auto_size() {
    let mut tf = TextFrame::new();
    tf.auto_size = MsoAutoSize::TextToFitShape;
    let xml = tf.to_xml_string();
    assert!(xml.contains("<a:normAutofit/>"));
}

#[test]
fn test_textframe_to_xml_auto_size_shape_to_fit() {
    let mut tf = TextFrame::new();
    tf.auto_size = MsoAutoSize::ShapeToFitText;
    let xml = tf.to_xml_string();
    assert!(xml.contains("<a:spAutoFit/>"));
}

#[test]
fn test_full_xml_generation() {
    use crate::enums::text::PpParagraphAlignment;

    let mut tf = TextFrame::new();

    // First paragraph: centered, bold, 18pt
    {
        let p = &mut tf.paragraphs_mut()[0];
        p.set_alignment(PpParagraphAlignment::Center);
        let r = p.add_run();
        r.set_text("Title");
        r.font_mut().bold = Some(true);
        r.font_mut().size = Some(18.0);
    }

    // Second paragraph: left-aligned, red text
    {
        let p = tf.add_paragraph();
        p.set_alignment(PpParagraphAlignment::Left);
        let r = p.add_run();
        r.set_text("Body text");
        r.font_mut().color = Some(RgbColor::new(255, 0, 0));
    }

    let xml = tf.to_xml_string();
    assert!(xml.contains(r#"algn="ctr""#));
    assert!(xml.contains(r#"algn="l""#));
    assert!(xml.contains(r#"b="1""#));
    assert!(xml.contains(r#"sz="1800""#));
    assert!(xml.contains(r#"val="FF0000""#));
    assert!(xml.contains("<a:t>Title</a:t>"));
    assert!(xml.contains("<a:t>Body text</a:t>"));
}

#[test]
fn test_set_text_empty() {
    let mut tf = TextFrame::new();
    tf.set_text("");
    assert_eq!(tf.paragraphs().len(), 1);
    assert_eq!(tf.text(), "");
}

#[test]
fn test_set_text_replaces_existing() {
    let mut tf = TextFrame::new();
    tf.set_text("Old text");
    tf.set_text("New text");
    assert_eq!(tf.text(), "New text");
    assert_eq!(tf.paragraphs().len(), 1);
}

#[test]
fn test_fit_text_no_scale() {
    let mut tf = TextFrame::new();
    tf.set_text("Fit me");
    tf.fit_text(None);
    assert_eq!(tf.auto_size, MsoAutoSize::TextToFitShape);
    assert!(tf.font_scale.is_none());
    let xml = tf.to_xml_string();
    assert!(xml.contains("<a:normAutofit/>"));
}

#[test]
fn test_fit_text_with_scale() {
    let mut tf = TextFrame::new();
    tf.set_text("Fit me smaller");
    tf.fit_text(Some(80.0));
    assert_eq!(tf.auto_size, MsoAutoSize::TextToFitShape);
    assert_eq!(tf.font_scale, Some(80.0));
    let xml = tf.to_xml_string();
    assert!(xml.contains(r#"<a:normAutofit fontScale="80000"/>"#));
}

#[test]
fn test_fit_text_100_percent() {
    let mut tf = TextFrame::new();
    tf.fit_text(Some(100.0));
    let xml = tf.to_xml_string();
    assert!(xml.contains(r#"fontScale="100000""#));
}

#[test]
fn test_textframe_rotation() {
    let mut tf = TextFrame::new();
    tf.rotation = Some(90.0);
    let xml = tf.to_xml_string();
    assert!(xml.contains(r#"rot="5400000""#));
}

#[test]
fn test_textframe_rotation_negative() {
    let mut tf = TextFrame::new();
    tf.rotation = Some(-45.0);
    let xml = tf.to_xml_string();
    assert!(xml.contains(r#"rot="-2700000""#));
}

#[test]
fn test_textframe_no_rotation() {
    let tf = TextFrame::new();
    let xml = tf.to_xml_string();
    assert!(!xml.contains("rot="));
}

#[test]
fn test_full_textframe_with_new_features() {
    use crate::enums::text::PpParagraphAlignment;

    let mut tf = TextFrame::new();
    tf.rotation = Some(0.0);
    tf.fit_text(Some(90.0));

    {
        let p = &mut tf.paragraphs_mut()[0];
        p.set_alignment(PpParagraphAlignment::Left);
        p.bullet_color = Some(ColorFormat::rgb(128, 0, 0));
        p.set_bullet(BulletFormat::Character('\u{2022}'));

        let mut def_font = Font::new();
        def_font.size = Some(14.0);
        p.font = Some(def_font);

        let r = p.add_run();
        r.set_text("First item");
        r.font_mut().bold = Some(true);
        r.font_mut().underline = Some(MsoTextUnderlineType::SingleLine);

        let mut hlink = Hyperlink::new("https://example.com");
        hlink.r_id = Some(RelationshipId::try_from("rId1").unwrap());
        r.set_hyperlink(hlink);
    }

    let xml = tf.to_xml_string();
    assert!(xml.contains("<a:normAutofit"));
    assert!(xml.contains(r#"fontScale="90000""#));
    assert!(xml.contains(r#"rot="0""#));
    assert!(xml.contains("<a:buClr>"));
    assert!(xml.contains("<a:defRPr"));
    assert!(xml.contains(r#"u="sng""#));
    assert!(xml.contains("<a:hlinkClick"));
    assert!(xml.contains(r#"r:id="rId1""#));
}
