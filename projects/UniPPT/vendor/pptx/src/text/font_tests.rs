//! Tests for the `Font` and `RgbColor` types.

use super::*;
use crate::enums::text::MsoTextUnderlineType;
use crate::units::RelationshipId;

#[test]
fn test_rgb_color_new() {
    let c = RgbColor::new(255, 0, 128);
    assert_eq!(c.r, 255);
    assert_eq!(c.g, 0);
    assert_eq!(c.b, 128);
}

#[test]
fn test_rgb_color_to_hex() {
    assert_eq!(RgbColor::new(255, 0, 0).to_hex(), "FF0000");
    assert_eq!(RgbColor::new(0, 255, 0).to_hex(), "00FF00");
    assert_eq!(RgbColor::new(0, 0, 255).to_hex(), "0000FF");
    assert_eq!(RgbColor::new(18, 52, 86).to_hex(), "123456");
}

#[test]
fn test_rgb_color_from_hex() {
    assert_eq!(
        RgbColor::from_hex("FF0000").unwrap(),
        RgbColor::new(255, 0, 0)
    );
    assert_eq!(
        RgbColor::from_hex("00FF00").unwrap(),
        RgbColor::new(0, 255, 0)
    );
    assert_eq!(
        RgbColor::from_hex("0000FF").unwrap(),
        RgbColor::new(0, 0, 255)
    );
}

#[test]
fn test_rgb_color_from_hex_invalid() {
    assert!(RgbColor::from_hex("FF00").is_err());
    assert!(RgbColor::from_hex("GGGGGG").is_err());
    assert!(RgbColor::from_hex("").is_err());
}

#[test]
fn test_rgb_color_display() {
    let c = RgbColor::new(255, 0, 128);
    assert_eq!(format!("{}", c), "#FF0080");
}

#[test]
fn test_font_default() {
    let f = Font::new();
    assert!(f.name.is_none());
    assert!(f.east_asia_name.is_none());
    assert!(f.complex_script_name.is_none());
    assert!(f.symbol_name.is_none());
    assert!(f.size.is_none());
    assert!(f.bold.is_none());
    assert!(f.italic.is_none());
    assert!(f.underline.is_none());
    assert!(f.color.is_none());
}

#[test]
fn test_font_to_xml_string_default() {
    let f = Font::new();
    let xml = f.to_xml_string();
    assert!(xml.contains("lang=\"en-US\""));
    assert!(xml.contains("dirty=\"0\""));
    assert!(!xml.contains("sz="));
    assert!(!xml.contains("b="));
}

#[test]
fn test_font_to_xml_string_with_size() {
    let mut f = Font::new();
    f.size = Some(18.0);
    let xml = f.to_xml_string();
    assert!(xml.contains(r#"sz="1800""#));
}

#[test]
fn test_font_to_xml_string_bold() {
    let mut f = Font::new();
    f.bold = Some(true);
    let xml = f.to_xml_string();
    assert!(xml.contains(r#"b="1""#));
}

#[test]
fn test_font_to_xml_string_italic() {
    let mut f = Font::new();
    f.italic = Some(true);
    let xml = f.to_xml_string();
    assert!(xml.contains(r#"i="1""#));
}

#[test]
fn test_font_to_xml_string_underline_single() {
    let mut f = Font::new();
    f.underline = Some(MsoTextUnderlineType::SingleLine);
    let xml = f.to_xml_string();
    assert!(xml.contains(r#"u="sng""#));
}

#[test]
fn test_font_to_xml_string_no_underline() {
    let mut f = Font::new();
    f.underline = Some(MsoTextUnderlineType::None);
    let xml = f.to_xml_string();
    assert!(xml.contains(r#"u="none""#));
}

#[test]
fn test_font_to_xml_string_underline_double() {
    let mut f = Font::new();
    f.underline = Some(MsoTextUnderlineType::DoubleLine);
    let xml = f.to_xml_string();
    assert!(xml.contains(r#"u="dbl""#));
}

#[test]
fn test_font_to_xml_string_underline_wavy() {
    let mut f = Font::new();
    f.underline = Some(MsoTextUnderlineType::WavyLine);
    let xml = f.to_xml_string();
    assert!(xml.contains(r#"u="wavy""#));
}

#[test]
fn test_font_to_xml_string_underline_dotted() {
    let mut f = Font::new();
    f.underline = Some(MsoTextUnderlineType::DottedLine);
    let xml = f.to_xml_string();
    assert!(xml.contains(r#"u="dotted""#));
}

#[test]
fn test_font_to_xml_string_underline_heavy() {
    let mut f = Font::new();
    f.underline = Some(MsoTextUnderlineType::HeavyLine);
    let xml = f.to_xml_string();
    assert!(xml.contains(r#"u="heavy""#));
}

#[test]
fn test_font_to_xml_string_underline_words() {
    let mut f = Font::new();
    f.underline = Some(MsoTextUnderlineType::Words);
    let xml = f.to_xml_string();
    assert!(xml.contains(r#"u="words""#));
}

#[test]
fn test_font_to_xml_string_with_color() {
    let mut f = Font::new();
    f.color = Some(RgbColor::new(255, 0, 0));
    let xml = f.to_xml_string();
    assert!(xml.contains(r#"<a:solidFill><a:srgbClr val="FF0000"/></a:solidFill>"#));
}

#[test]
fn test_font_to_xml_string_with_name() {
    let mut f = Font::new();
    f.name = Some("Calibri".to_string());
    let xml = f.to_xml_string();
    assert!(xml.contains(r#"<a:latin typeface="Calibri"/>"#));
}

#[test]
fn test_font_to_xml_string_with_script_typefaces() {
    let mut f = Font::new();
    f.name = Some("Calibri".to_string());
    f.east_asia_name = Some("Microsoft YaHei".to_string());
    f.complex_script_name = Some("Arial".to_string());
    f.symbol_name = Some("Wingdings".to_string());
    let xml = f.to_xml_string();
    assert!(xml.contains(r#"<a:latin typeface="Calibri"/>"#));
    assert!(xml.contains(r#"<a:ea typeface="Microsoft YaHei"/>"#));
    assert!(xml.contains(r#"<a:cs typeface="Arial"/>"#));
    assert!(xml.contains(r#"<a:sym typeface="Wingdings"/>"#));
}

#[test]
fn test_font_to_xml_string_full() {
    let mut f = Font::new();
    f.name = Some("Arial".to_string());
    f.size = Some(24.0);
    f.bold = Some(true);
    f.italic = Some(false);
    f.underline = Some(MsoTextUnderlineType::SingleLine);
    f.color = Some(RgbColor::new(0, 0, 255));

    let xml = f.to_xml_string();
    assert!(xml.contains(r#"sz="2400""#));
    assert!(xml.contains(r#"b="1""#));
    assert!(xml.contains(r#"i="0""#));
    assert!(xml.contains(r#"u="sng""#));
    assert!(xml.contains(r#"val="0000FF""#));
    assert!(xml.contains(r#"typeface="Arial""#));
    // Should be a non-self-closing tag since it has children
    assert!(xml.contains("</a:rPr>"));
}

#[test]
fn test_font_fill_solid() {
    use crate::dml::color::ColorFormat;
    let mut f = Font::new();
    f.fill = Some(FillFormat::solid(ColorFormat::rgb(255, 128, 0)));
    let xml = f.to_xml_string();
    assert!(xml.contains("<a:solidFill>"));
    assert!(xml.contains("FF8000"));
    assert!(xml.contains("</a:rPr>"));
}

#[test]
fn test_font_fill_overrides_color() {
    use crate::dml::color::ColorFormat;
    let mut f = Font::new();
    f.color = Some(RgbColor::new(255, 0, 0));
    f.fill = Some(FillFormat::solid(ColorFormat::rgb(0, 255, 0)));
    let xml = f.to_xml_string();
    // fill should be used, not the simple color
    assert!(xml.contains("00FF00"));
    assert!(!xml.contains("FF0000"));
}

#[test]
fn test_font_fill_no_fill() {
    let mut f = Font::new();
    f.fill = Some(FillFormat::no_fill());
    let xml = f.to_xml_string();
    assert!(xml.contains("<a:noFill/>"));
}

#[test]
fn test_font_hyperlink() {
    let mut f = Font::new();
    let mut hlink = Hyperlink::new("https://example.com");
    hlink.r_id = Some(RelationshipId::try_from("rId1").unwrap());
    f.hyperlink = Some(hlink);
    let xml = f.to_xml_string();
    assert!(xml.contains("<a:hlinkClick"));
    assert!(xml.contains(r#"r:id="rId1""#));
}

#[test]
fn test_font_hyperlink_with_tooltip() {
    let mut f = Font::new();
    let mut hlink = Hyperlink::with_tooltip("https://example.com", "Click here");
    hlink.r_id = Some(RelationshipId::try_from("rId2").unwrap());
    f.hyperlink = Some(hlink);
    let xml = f.to_xml_string();
    assert!(xml.contains(r#"tooltip="Click here""#));
    assert!(xml.contains(r#"r:id="rId2""#));
}
