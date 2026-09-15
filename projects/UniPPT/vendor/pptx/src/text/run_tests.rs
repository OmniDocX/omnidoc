//! Tests for the `Run` type.

use super::*;
use crate::shapes::action::Hyperlink;
use crate::text::font::RgbColor;
use crate::units::RelationshipId;

#[test]
fn test_run_new_is_empty() {
    let r = Run::new();
    assert_eq!(r.text(), "");
}

#[test]
fn test_run_set_text() {
    let mut r = Run::new();
    r.set_text("Hello");
    assert_eq!(r.text(), "Hello");
}

#[test]
fn test_run_font_mut() {
    let mut r = Run::new();
    r.font_mut().bold = Some(true);
    assert_eq!(r.font().bold, Some(true));
}

#[test]
fn test_run_to_xml_string() {
    let mut r = Run::new();
    r.set_text("Hello");
    let xml = r.to_xml_string();
    assert!(xml.starts_with("<a:r>"));
    assert!(xml.ends_with("</a:r>"));
    assert!(xml.contains("<a:t>Hello</a:t>"));
    assert!(xml.contains("<a:rPr"));
}

#[test]
fn test_run_xml_escaping() {
    let mut r = Run::new();
    r.set_text("A < B & C > D");
    let xml = r.to_xml_string();
    assert!(xml.contains("<a:t>A &lt; B &amp; C &gt; D</a:t>"));
}

#[test]
fn test_xml_escape() {
    assert_eq!(xml_escape("a < b"), "a &lt; b");
    assert_eq!(xml_escape("a & b"), "a &amp; b");
    assert_eq!(xml_escape("a > b"), "a &gt; b");
    assert_eq!(xml_escape(r#"a "b""#), "a &quot;b&quot;");
    assert_eq!(xml_escape("hello"), "hello");
}

#[test]
fn test_run_hyperlink() {
    let mut r = Run::new();
    r.set_text("Click here");
    let mut hlink = Hyperlink::new("https://example.com");
    hlink.r_id = Some(RelationshipId::try_from("rId1").unwrap());
    r.set_hyperlink(hlink);
    let xml = r.to_xml_string();
    assert!(xml.contains("<a:hlinkClick"));
    assert!(xml.contains(r#"r:id="rId1""#));
    assert!(xml.contains("<a:t>Click here</a:t>"));
}

#[test]
fn test_run_hyperlink_with_tooltip() {
    let mut r = Run::new();
    r.set_text("Link");
    let mut hlink = Hyperlink::with_tooltip("https://example.com", "Visit example");
    hlink.r_id = Some(RelationshipId::try_from("rId2").unwrap());
    r.set_hyperlink(hlink);
    let xml = r.to_xml_string();
    assert!(xml.contains(r#"tooltip="Visit example""#));
    assert!(xml.contains(r#"r:id="rId2""#));
}

#[test]
fn test_run_hyperlink_with_font_properties() {
    let mut r = Run::new();
    r.set_text("Bold link");
    r.font_mut().bold = Some(true);
    r.font_mut().color = Some(RgbColor::new(0, 0, 255));
    let mut hlink = Hyperlink::new("https://example.com");
    hlink.r_id = Some(RelationshipId::try_from("rId3").unwrap());
    r.set_hyperlink(hlink);
    let xml = r.to_xml_string();
    assert!(xml.contains(r#"b="1""#));
    assert!(xml.contains("0000FF"));
    assert!(xml.contains("<a:hlinkClick"));
    assert!(xml.contains("</a:rPr>"));
}

#[test]
fn test_run_no_hyperlink() {
    let mut r = Run::new();
    r.set_text("No link");
    let xml = r.to_xml_string();
    assert!(!xml.contains("hlinkClick"));
}

#[test]
fn test_run_is_line_break_default() {
    let r = Run::new();
    assert!(!r.is_line_break);
}

#[test]
fn test_line_break_xml_with_font() {
    let mut r = Run::new();
    r.is_line_break = true;
    r.font_mut().size = Some(18.0);
    let xml = r.to_xml_string();
    assert!(xml.starts_with("<a:br>"));
    assert!(xml.ends_with("</a:br>"));
    assert!(xml.contains(r#"sz="1800""#));
    // Line breaks should NOT have <a:t> text content
    assert!(!xml.contains("<a:t>"));
}

#[test]
fn test_run_set_language() {
    let mut r = Run::new();
    r.set_text("\u{0645}\u{0631}\u{062D}\u{0628}\u{0627}");
    r.set_language("ar-SA");
    let xml = r.to_xml_string();
    assert!(xml.contains(r#"lang="ar-SA""#));
}

#[test]
fn test_run_set_language_hebrew() {
    let mut r = Run::new();
    r.set_text("\u{05E9}\u{05DC}\u{05D5}\u{05DD}");
    r.set_language("he-IL");
    let xml = r.to_xml_string();
    assert!(xml.contains(r#"lang="he-IL""#));
}

#[test]
fn test_run_underline_types() {
    use crate::enums::text::MsoTextUnderlineType;

    let underline_types = [
        (MsoTextUnderlineType::SingleLine, "sng"),
        (MsoTextUnderlineType::DoubleLine, "dbl"),
        (MsoTextUnderlineType::HeavyLine, "heavy"),
        (MsoTextUnderlineType::DottedLine, "dotted"),
        (MsoTextUnderlineType::DottedHeavyLine, "dottedHeavy"),
        (MsoTextUnderlineType::DashLine, "dash"),
        (MsoTextUnderlineType::DashHeavyLine, "dashHeavy"),
        (MsoTextUnderlineType::DashLongLine, "dashLong"),
        (MsoTextUnderlineType::DashLongHeavyLine, "dashLongHeavy"),
        (MsoTextUnderlineType::DotDashLine, "dotDash"),
        (MsoTextUnderlineType::DotDashHeavyLine, "dotDashHeavy"),
        (MsoTextUnderlineType::DotDotDashLine, "dotDotDash"),
        (MsoTextUnderlineType::DotDotDashHeavyLine, "dotDotDashHeavy"),
        (MsoTextUnderlineType::WavyLine, "wavy"),
        (MsoTextUnderlineType::WavyHeavyLine, "wavyHeavy"),
        (MsoTextUnderlineType::WavyDoubleLine, "wavyDbl"),
        (MsoTextUnderlineType::Words, "words"),
        (MsoTextUnderlineType::None, "none"),
    ];

    for (utype, expected_xml) in &underline_types {
        let mut r = Run::new();
        r.set_text("test");
        r.font_mut().underline = Some(*utype);
        let xml = r.to_xml_string();
        assert!(
            xml.contains(&format!(r#"u="{}""#, expected_xml)),
            "Expected u=\"{}\" for {:?}, got: {}",
            expected_xml,
            utype,
            xml
        );
    }
}
