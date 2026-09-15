use super::*;
use crate::text::font::RgbColor;

fn sample_theme_xml() -> Vec<u8> {
    br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Office Theme">
<a:themeElements>
<a:clrScheme name="Office">
<a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1>
<a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1>
<a:dk2><a:srgbClr val="1F497D"/></a:dk2>
<a:lt2><a:srgbClr val="EEECE1"/></a:lt2>
<a:accent1><a:srgbClr val="4F81BD"/></a:accent1>
<a:accent2><a:srgbClr val="C0504D"/></a:accent2>
<a:accent3><a:srgbClr val="9BBB59"/></a:accent3>
<a:accent4><a:srgbClr val="8064A2"/></a:accent4>
<a:accent5><a:srgbClr val="4BACC6"/></a:accent5>
<a:accent6><a:srgbClr val="F79646"/></a:accent6>
<a:hlink><a:srgbClr val="0000FF"/></a:hlink>
<a:folHlink><a:srgbClr val="800080"/></a:folHlink>
</a:clrScheme>
</a:themeElements>
</a:theme>"#
        .to_vec()
}

#[test]
fn test_parse_theme_color_scheme() {
    let scheme = parse_theme_color_scheme(&sample_theme_xml())
        .expect("should parse")
        .expect("should have scheme");
    assert_eq!(scheme.dk1, RgbColor::new(0, 0, 0));
    assert_eq!(scheme.lt1, RgbColor::new(255, 255, 255));
    assert_eq!(scheme.dk2, RgbColor::new(31, 73, 125));
    assert_eq!(scheme.lt2, RgbColor::new(238, 236, 225));
    assert_eq!(scheme.accent1, RgbColor::new(79, 129, 189));
    assert_eq!(scheme.accent2, RgbColor::new(192, 80, 77));
    assert_eq!(scheme.accent3, RgbColor::new(155, 187, 89));
    assert_eq!(scheme.accent4, RgbColor::new(128, 100, 162));
    assert_eq!(scheme.accent5, RgbColor::new(75, 172, 198));
    assert_eq!(scheme.accent6, RgbColor::new(247, 150, 70));
    assert_eq!(scheme.hlink, RgbColor::new(0, 0, 255));
    assert_eq!(scheme.fol_hlink, RgbColor::new(128, 0, 128));
}

#[test]
fn test_by_name() {
    let scheme = parse_theme_color_scheme(&sample_theme_xml())
        .expect("should parse")
        .expect("should have scheme");
    assert_eq!(scheme.by_name("dk1"), Some(RgbColor::new(0, 0, 0)));
    assert_eq!(scheme.by_name("tx1"), Some(RgbColor::new(0, 0, 0)));
    assert_eq!(scheme.by_name("lt1"), Some(RgbColor::new(255, 255, 255)));
    assert_eq!(scheme.by_name("bg1"), Some(RgbColor::new(255, 255, 255)));
    assert_eq!(scheme.by_name("accent1"), Some(RgbColor::new(79, 129, 189)));
    assert_eq!(scheme.by_name("hlink"), Some(RgbColor::new(0, 0, 255)));
    assert_eq!(scheme.by_name("folHlink"), Some(RgbColor::new(128, 0, 128)));
    assert_eq!(scheme.by_name("nonexistent"), None);
}

#[test]
fn test_default_scheme() {
    let scheme = ThemeColorScheme::default();
    assert_eq!(scheme.dk1, RgbColor::new(0, 0, 0));
    assert_eq!(scheme.lt1, RgbColor::new(255, 255, 255));
}

#[test]
fn test_parse_empty_xml() {
    let xml = b"<a:theme/>";
    let result = parse_theme_color_scheme(xml);
    // Should return default scheme since no clrScheme found
    assert!(result.unwrap().is_some());
}

#[test]
fn test_parse_invalid_xml() {
    let xml = b"not valid xml<<<";
    let result = parse_theme_color_scheme(xml);
    assert!(result.unwrap().is_none());
}

#[test]
fn test_scheme_clone_eq() {
    let scheme1 = ThemeColorScheme::default();
    let scheme2 = scheme1.clone();
    assert_eq!(scheme1, scheme2);
}

#[test]
fn test_to_xml_string_contains_all_slots() {
    let scheme = ThemeColorScheme::default();
    let xml = scheme.to_xml_string();
    assert!(xml.starts_with(r#"<a:clrScheme name="Office">"#));
    assert!(xml.ends_with("</a:clrScheme>"));
    assert!(xml.contains("<a:dk1>"));
    assert!(xml.contains("<a:dk2>"));
    assert!(xml.contains("<a:lt1>"));
    assert!(xml.contains("<a:lt2>"));
    assert!(xml.contains("<a:accent1>"));
    assert!(xml.contains("<a:accent2>"));
    assert!(xml.contains("<a:accent3>"));
    assert!(xml.contains("<a:accent4>"));
    assert!(xml.contains("<a:accent5>"));
    assert!(xml.contains("<a:accent6>"));
    assert!(xml.contains("<a:hlink>"));
    assert!(xml.contains("<a:folHlink>"));
}

#[test]
fn test_to_xml_string_color_values() {
    let scheme = ThemeColorScheme {
        accent1: RgbColor::new(255, 0, 0),
        ..Default::default()
    };
    let xml = scheme.to_xml_string();
    assert!(xml.contains(r#"<a:accent1><a:srgbClr val="FF0000"/></a:accent1>"#));
}

#[test]
fn test_to_xml_string_roundtrip() {
    let original = ThemeColorScheme::default();
    let xml = original.to_xml_string();
    // Wrap in a minimal theme XML so parser can find it
    let theme_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Office Theme">
<a:themeElements>
{}
</a:themeElements>
</a:theme>"#,
        xml
    );
    let parsed = parse_theme_color_scheme(theme_xml.as_bytes())
        .expect("roundtrip should parse")
        .expect("roundtrip should have scheme");
    assert_eq!(parsed, original);
}

#[test]
fn test_update_theme_color_scheme() {
    let theme_xml = sample_theme_xml();
    let new_scheme = ThemeColorScheme {
        accent1: RgbColor::new(255, 0, 0),
        ..Default::default()
    };

    let updated =
        update_theme_color_scheme(&theme_xml, &new_scheme).expect("update should succeed");
    let updated_str =
        String::from_utf8(updated.clone()).expect("updated XML should be valid UTF-8");

    // The old accent1 value should be gone
    assert!(!updated_str.contains("4F81BD"));
    // The new accent1 value should be present
    assert!(updated_str.contains("FF0000"));

    // Parse back to verify
    let parsed = parse_theme_color_scheme(&updated)
        .expect("should parse updated XML")
        .expect("should have scheme");
    assert_eq!(parsed.accent1, RgbColor::new(255, 0, 0));
}

#[test]
fn test_update_theme_preserves_surrounding_xml() {
    let theme_xml = sample_theme_xml();
    let scheme = ThemeColorScheme::default();

    let updated = update_theme_color_scheme(&theme_xml, &scheme).expect("update should succeed");
    let updated_str = String::from_utf8(updated).expect("updated XML should be valid UTF-8");

    // Should still have the surrounding theme structure
    assert!(updated_str.contains("<a:theme"));
    assert!(updated_str.contains("<a:themeElements>"));
    assert!(updated_str.contains("</a:themeElements>"));
    assert!(updated_str.contains("</a:theme>"));
}

#[test]
fn test_update_theme_no_clr_scheme_errors() {
    let xml = b"<a:theme><a:themeElements></a:themeElements></a:theme>";
    let scheme = ThemeColorScheme::default();
    let result = update_theme_color_scheme(xml, &scheme);
    assert!(result.is_err());
}
