//! Tests for the `Paragraph` type.

use super::*;
use crate::dml::color::ColorFormat;
use crate::enums::text::{PpParagraphAlignment, TextDirection};
use crate::text::font::Font;

#[test]
fn test_paragraph_new_is_empty() {
    let p = Paragraph::new();
    assert_eq!(p.text(), "");
    assert!(p.runs().is_empty());
}

#[test]
fn test_paragraph_add_run() {
    let mut p = Paragraph::new();
    let r = p.add_run();
    r.set_text("Hello");
    assert_eq!(p.text(), "Hello");
    assert_eq!(p.runs().len(), 1);
}

#[test]
fn test_paragraph_multiple_runs() {
    let mut p = Paragraph::new();
    p.add_run().set_text("Hello ");
    p.add_run().set_text("World");
    assert_eq!(p.text(), "Hello World");
    assert_eq!(p.runs().len(), 2);
}

#[test]
fn test_paragraph_alignment() {
    let mut p = Paragraph::new();
    p.set_alignment(PpParagraphAlignment::Center);
    assert_eq!(p.alignment, Some(PpParagraphAlignment::Center));
}

#[test]
fn test_paragraph_clear() {
    let mut p = Paragraph::new();
    p.add_run().set_text("Hello");
    p.clear();
    assert_eq!(p.text(), "");
    assert!(p.runs().is_empty());
}

#[test]
fn test_paragraph_to_xml_string_empty() {
    let p = Paragraph::new();
    let xml = p.to_xml_string();
    assert_eq!(xml, "<a:p></a:p>");
}

#[test]
fn test_paragraph_to_xml_string_with_alignment() {
    let mut p = Paragraph::new();
    p.set_alignment(PpParagraphAlignment::Center);
    p.add_run().set_text("Hello");
    let xml = p.to_xml_string();
    assert!(xml.contains(r#"<a:pPr algn="ctr"/>"#));
}

#[test]
fn test_paragraph_to_xml_string_with_level() {
    let mut p = Paragraph::new();
    p.level = 2;
    let xml = p.to_xml_string();
    assert!(xml.contains(r#"lvl="2""#));
}

#[test]
fn test_paragraph_to_xml_string_with_spacing() {
    let mut p = Paragraph::new();
    p.space_before = Some(12.0);
    p.space_after = Some(6.0);
    let xml = p.to_xml_string();
    assert!(xml.contains(r#"<a:spcBef><a:spcPts val="1200"/></a:spcBef>"#));
    assert!(xml.contains(r#"<a:spcAft><a:spcPts val="600"/></a:spcAft>"#));
}

#[test]
fn test_paragraph_to_xml_string_with_line_spacing() {
    let mut p = Paragraph::new();
    p.line_spacing = Some(1.5);
    let xml = p.to_xml_string();
    assert!(xml.contains(r#"<a:lnSpc><a:spcPct val="150000"/></a:lnSpc>"#));
}

#[test]
fn test_paragraph_default_font() {
    let mut p = Paragraph::new();
    let mut font = Font::new();
    font.size = Some(24.0);
    font.bold = Some(true);
    p.font = Some(font);
    let xml = p.to_xml_string();
    assert!(xml.contains("<a:defRPr"));
    assert!(xml.contains(r#"sz="2400""#));
    assert!(xml.contains(r#"b="1""#));
}

#[test]
fn test_paragraph_default_font_with_name() {
    let mut p = Paragraph::new();
    let mut font = Font::new();
    font.name = Some("Arial".to_string());
    font.size = Some(12.0);
    p.font = Some(font);
    let xml = p.to_xml_string();
    assert!(xml.contains("<a:defRPr"));
    assert!(xml.contains(r#"typeface="Arial""#));
    assert!(xml.contains("</a:defRPr>"));
}

#[test]
fn test_paragraph_no_default_font() {
    let p = Paragraph::new();
    let xml = p.to_xml_string();
    assert!(!xml.contains("defRPr"));
}

#[test]
fn test_bullet_picture() {
    let mut p = Paragraph::new();
    p.set_bullet(BulletFormat::Picture("rId5".to_string()));
    p.add_run().set_text("Picture bullet");
    let xml = p.to_xml_string();
    assert!(xml.contains("<a:buBlip>"));
    assert!(xml.contains(r#"r:embed="rId5""#));
    assert!(xml.contains("</a:buBlip>"));
}

#[test]
fn test_bullet_color() {
    let mut p = Paragraph::new();
    p.bullet_color = Some(ColorFormat::rgb(255, 0, 0));
    p.set_bullet(BulletFormat::Character('\u{2022}'));
    p.add_run().set_text("Red bullet");
    let xml = p.to_xml_string();
    assert!(xml.contains("<a:buClr>"));
    assert!(xml.contains("FF0000"));
    assert!(xml.contains("</a:buClr>"));
}

#[test]
fn test_bullet_font() {
    let mut p = Paragraph::new();
    p.bullet_font = Some("Wingdings".to_string());
    p.set_bullet(BulletFormat::Character('\u{006C}'));
    p.add_run().set_text("Wingdings bullet");
    let xml = p.to_xml_string();
    assert!(xml.contains(r#"<a:buFont typeface="Wingdings"/>"#));
}

#[test]
fn test_bullet_size_pct() {
    let mut p = Paragraph::new();
    p.bullet_size_pct = Some(150.0);
    p.set_bullet(BulletFormat::Character('\u{2022}'));
    p.add_run().set_text("Large bullet");
    let xml = p.to_xml_string();
    assert!(xml.contains(r#"<a:buSzPct val="150000"/>"#));
}

#[test]
fn test_bullet_size_pts() {
    let mut p = Paragraph::new();
    p.bullet_size_pts = Some(14.0);
    p.set_bullet(BulletFormat::Character('\u{2022}'));
    p.add_run().set_text("14pt bullet");
    let xml = p.to_xml_string();
    assert!(xml.contains(r#"<a:buSzPts val="1400"/>"#));
}

#[test]
fn test_bullet_size_pct_takes_precedence() {
    let mut p = Paragraph::new();
    p.bullet_size_pct = Some(120.0);
    p.bullet_size_pts = Some(14.0); // should be ignored when pct is set
    p.set_bullet(BulletFormat::Character('\u{2022}'));
    p.add_run().set_text("Bullet");
    let xml = p.to_xml_string();
    assert!(xml.contains("buSzPct"));
    assert!(!xml.contains("buSzPts"));
}

#[test]
fn test_bullet_autonumbered() {
    let mut p = Paragraph::new();
    p.set_bullet(BulletFormat::AutoNumbered("arabicPeriod".to_string()));
    p.add_run().set_text("Numbered");
    let xml = p.to_xml_string();
    assert!(xml.contains(r#"<a:buAutoNum type="arabicPeriod"/>"#));
}

#[test]
fn test_paragraph_add_line_break() {
    let mut p = Paragraph::new();
    p.add_run().set_text("Before");
    let br = p.add_line_break();
    assert!(br.is_line_break);
    p.add_run().set_text("After");
    assert_eq!(p.runs().len(), 3);
}

#[test]
fn test_paragraph_text_with_line_break() {
    let mut p = Paragraph::new();
    p.add_run().set_text("Line 1");
    p.add_line_break();
    p.add_run().set_text("Line 2");
    assert_eq!(p.text(), "Line 1\nLine 2");
}

#[test]
fn test_line_break_xml() {
    let mut p = Paragraph::new();
    p.add_run().set_text("Before");
    p.add_line_break();
    p.add_run().set_text("After");
    let xml = p.to_xml_string();
    assert!(xml.contains("<a:br>"));
    assert!(xml.contains("</a:br>"));
    assert!(xml.contains("<a:t>Before</a:t>"));
    assert!(xml.contains("<a:t>After</a:t>"));
}

#[test]
fn test_line_break_inherits_paragraph_font() {
    let mut p = Paragraph::new();
    let mut def_font = Font::new();
    def_font.size = Some(24.0);
    p.font = Some(def_font);

    p.add_run().set_text("Text");
    let br = p.add_line_break();
    // The line break should have inherited the paragraph default font
    assert_eq!(br.font().size, Some(24.0));
}

#[test]
fn test_paragraph_with_all_bullet_properties() {
    let mut p = Paragraph::new();
    p.bullet_color = Some(ColorFormat::rgb(0, 128, 0));
    p.bullet_font = Some("Symbol".to_string());
    p.bullet_size_pct = Some(100.0);
    p.set_bullet(BulletFormat::Character('\u{2022}'));
    p.add_run().set_text("Full bullet");
    let xml = p.to_xml_string();
    assert!(xml.contains("<a:buClr>"));
    assert!(xml.contains("008000"));
    assert!(xml.contains(r#"<a:buSzPct val="100000"/>"#));
    assert!(xml.contains(r#"<a:buFont typeface="Symbol"/>"#));
    assert!(xml.contains("<a:buChar"));
}

#[test]
fn test_paragraph_rtl_direction() {
    let mut p = Paragraph::new();
    p.set_text_direction(TextDirection::RightToLeft);
    p.add_run()
        .set_text("\u{0645}\u{0631}\u{062D}\u{0628}\u{0627}");
    let xml = p.to_xml_string();
    assert!(xml.contains(r#"rtl="1""#));
}

#[test]
fn test_paragraph_ltr_direction() {
    let mut p = Paragraph::new();
    p.set_text_direction(TextDirection::LeftToRight);
    p.add_run().set_text("Hello");
    let xml = p.to_xml_string();
    assert!(xml.contains(r#"rtl="0""#));
}

#[test]
fn test_paragraph_no_direction_no_rtl_attr() {
    let mut p = Paragraph::new();
    p.add_run().set_text("Hello");
    let xml = p.to_xml_string();
    assert!(!xml.contains("rtl="));
}

#[test]
fn test_paragraph_rtl_with_alignment() {
    let mut p = Paragraph::new();
    p.set_alignment(PpParagraphAlignment::Right);
    p.set_text_direction(TextDirection::RightToLeft);
    p.add_run().set_text("\u{05E9}\u{05DC}\u{05D5}\u{05DD}");
    let xml = p.to_xml_string();
    assert!(xml.contains(r#"algn="r""#));
    assert!(xml.contains(r#"rtl="1""#));
}

#[test]
fn test_paragraph_rtl_with_language() {
    let mut p = Paragraph::new();
    p.set_text_direction(TextDirection::RightToLeft);
    let run = p.add_run();
    run.set_text("\u{0645}\u{0631}\u{062D}\u{0628}\u{0627}");
    run.set_language("ar-SA");
    let xml = p.to_xml_string();
    assert!(xml.contains(r#"rtl="1""#));
    assert!(xml.contains(r#"lang="ar-SA""#));
}
