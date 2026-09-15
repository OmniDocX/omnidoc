//! Tests for Font color_type and set_size validation.

use super::*;

#[test]
fn test_font_color_type_none() {
    let f = Font::new();
    assert!(f.color_type().is_none());
}

#[test]
fn test_font_color_type_rgb() {
    let mut f = Font::new();
    f.color = Some(RgbColor::new(255, 0, 0));
    assert_eq!(f.color_type(), Some(MsoColorType::Rgb));
}

#[test]
fn test_font_color_type_from_solid_fill_rgb() {
    use crate::dml::color::ColorFormat;
    let mut f = Font::new();
    f.fill = Some(FillFormat::solid(ColorFormat::rgb(0, 255, 0)));
    assert_eq!(f.color_type(), Some(MsoColorType::Rgb));
}

#[test]
fn test_font_color_type_from_solid_fill_theme() {
    use crate::dml::color::ColorFormat;
    use crate::enums::dml::MsoThemeColorIndex;
    let mut f = Font::new();
    f.fill = Some(FillFormat::solid(ColorFormat::theme(
        MsoThemeColorIndex::Accent1,
    )));
    assert_eq!(f.color_type(), Some(MsoColorType::Scheme));
}

#[test]
fn test_font_color_type_fill_overrides_color() {
    use crate::dml::color::ColorFormat;
    use crate::enums::dml::MsoThemeColorIndex;
    let mut f = Font::new();
    f.color = Some(RgbColor::new(255, 0, 0));
    f.fill = Some(FillFormat::solid(ColorFormat::theme(
        MsoThemeColorIndex::Accent2,
    )));
    // fill takes precedence
    assert_eq!(f.color_type(), Some(MsoColorType::Scheme));
}

#[test]
fn test_font_color_type_no_fill_returns_none() {
    let mut f = Font::new();
    f.fill = Some(FillFormat::no_fill());
    // NoFill is not a solid fill, so color_type is None
    assert!(f.color_type().is_none());
}

// --- Font::set_size validation tests ---

#[test]
fn test_font_set_size_valid() {
    let mut f = Font::new();
    f.set_size(18.0).unwrap();
    assert_eq!(f.size, Some(18.0));
}

#[test]
fn test_font_set_size_zero() {
    let mut f = Font::new();
    assert!(f.set_size(0.0).is_err());
}

#[test]
fn test_font_set_size_negative() {
    let mut f = Font::new();
    assert!(f.set_size(-1.0).is_err());
}

#[test]
fn test_font_set_size_small_positive() {
    let mut f = Font::new();
    f.set_size(0.5).unwrap();
    assert_eq!(f.size, Some(0.5));
}
