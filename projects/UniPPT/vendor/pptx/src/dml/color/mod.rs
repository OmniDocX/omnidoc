//! `DrawingML` color types.

mod types;
mod xml;

pub use types::{HslColor, PresetColor, SystemColor, ThemeColor};

use crate::enums::dml::{MsoColorType, MsoThemeColorIndex, PresetColorVal, SystemColorVal};
use crate::text::font::RgbColor;

/// A color specification, either RGB or a theme color reference.
///
/// In `DrawingML`, colors can be specified in several ways.  We support the
/// two most common: direct sRGB values (`<a:srgbClr>`) and scheme/theme
/// color references (`<a:schemeClr>`), plus HSL, system, and preset colors.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum ColorFormat {
    /// An explicit RGB color (e.g. `<a:srgbClr val="FF0000"/>`).
    Rgb(RgbColor),
    /// A theme color reference (e.g. `<a:schemeClr val="accent1"/>`).
    Theme(ThemeColor),
    /// An HSL color (e.g. `<a:hslClr hue="0" sat="100000" lum="50000"/>`).
    Hsl(HslColor),
    /// A system color (e.g. `<a:sysClr val="windowText"/>`).
    System(SystemColor),
    /// A preset (named) color (e.g. `<a:prstClr val="red"/>`).
    Preset(PresetColor),
}

impl ColorFormat {
    /// Create an RGB color from individual components.
    #[must_use]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::Rgb(RgbColor::new(r, g, b))
    }

    /// Create a theme color with no brightness adjustment.
    #[must_use]
    pub const fn theme(theme_color: MsoThemeColorIndex) -> Self {
        Self::Theme(ThemeColor {
            theme_color,
            brightness: None,
        })
    }

    /// Create a theme color with a brightness adjustment.
    #[must_use]
    pub const fn theme_with_brightness(theme_color: MsoThemeColorIndex, brightness: f64) -> Self {
        Self::Theme(ThemeColor {
            theme_color,
            brightness: Some(brightness),
        })
    }

    /// Create an HSL color.
    ///
    /// - `hue`: 0.0 to 360.0 degrees
    /// - `saturation`: 0.0 to 100.0 percent
    /// - `luminance`: 0.0 to 100.0 percent
    #[must_use]
    pub const fn hsl(hue: f64, saturation: f64, luminance: f64) -> Self {
        Self::Hsl(HslColor {
            hue,
            saturation,
            luminance,
        })
    }

    /// Create a system color.
    #[must_use]
    pub fn system(val: &str) -> Self {
        Self::System(SystemColor {
            val: SystemColorVal::from_xml_str(val),
            last_color: None,
        })
    }

    /// Create a system color with a last-used RGB value.
    #[must_use]
    pub fn system_with_last_color(val: &str, last_color: &str) -> Self {
        Self::System(SystemColor {
            val: SystemColorVal::from_xml_str(val),
            last_color: Some(last_color.to_string()),
        })
    }

    /// Return the color type classification for this color.
    #[must_use]
    pub const fn color_type(&self) -> MsoColorType {
        match self {
            Self::Rgb(_) => MsoColorType::Rgb,
            Self::Theme(_) => MsoColorType::Scheme,
            Self::Hsl(_) => MsoColorType::Hsl,
            Self::System(_) => MsoColorType::System,
            Self::Preset(_) => MsoColorType::Preset,
        }
    }

    /// Create a preset (named) color.
    #[must_use]
    pub fn preset(val: &str) -> Self {
        Self::Preset(PresetColor {
            val: PresetColorVal::from_xml_str(val),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_convenience() {
        let c = ColorFormat::rgb(0, 128, 255);
        match &c {
            ColorFormat::Rgb(rgb) => {
                assert_eq!(rgb.r, 0);
                assert_eq!(rgb.g, 128);
                assert_eq!(rgb.b, 255);
            }
            _ => panic!("expected Rgb variant"), // EXCEPTION(test-only)
        }
    }
}
