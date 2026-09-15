//! Color struct types: `ThemeColor`, `HslColor`, `SystemColor`, `PresetColor`.

use crate::enums::dml::{MsoThemeColorIndex, PresetColorVal, SystemColorVal};
use crate::error::PptxError;

/// A theme color reference with optional brightness adjustment.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeColor {
    /// Which theme color slot this refers to.
    pub theme_color: MsoThemeColorIndex,
    /// Brightness adjustment from -1.0 (100% darker) to 1.0 (100% lighter).
    /// 0.0 means no adjustment.  `None` means no explicit adjustment.
    pub brightness: Option<f64>,
}

impl ThemeColor {
    /// Create a validated theme color with an optional brightness adjustment.
    ///
    /// `brightness` must be in the range -1.0 to 1.0 (inclusive) when `Some`.
    ///
    /// # Errors
    ///
    /// Returns `Err(PptxError::InvalidValue)` if the brightness is out of range.
    pub fn new(
        theme_color: MsoThemeColorIndex,
        brightness: Option<f64>,
    ) -> Result<Self, PptxError> {
        if let Some(b) = brightness {
            if !(-1.0..=1.0).contains(&b) {
                return Err(PptxError::InvalidValue {
                    field: "ThemeColor.brightness",
                    value: b.to_string(),
                    expected: "-1.0 to 1.0",
                });
            }
        }
        Ok(Self {
            theme_color,
            brightness,
        })
    }
}

/// An HSL (Hue/Saturation/Luminance) color.
///
/// - `hue`: 0.0 to 360.0 degrees
/// - `saturation`: 0.0 to 100.0 percent
/// - `luminance`: 0.0 to 100.0 percent
///
/// OOXML stores hue as value * 60000 (i.e. 60000ths of a degree)
/// and saturation/luminance as value * 1000 (i.e. 1000ths of a percent).
#[derive(Debug, Clone, PartialEq)]
pub struct HslColor {
    pub hue: f64,
    pub saturation: f64,
    pub luminance: f64,
}

impl HslColor {
    /// Create a validated HSL color.
    ///
    /// - `hue`: 0.0 to 360.0 degrees
    /// - `saturation`: 0.0 to 100.0 percent
    /// - `luminance`: 0.0 to 100.0 percent
    ///
    /// # Errors
    ///
    /// Returns `Err(PptxError::InvalidValue)` if any value is out of range.
    pub fn new(hue: f64, saturation: f64, luminance: f64) -> Result<Self, PptxError> {
        if !(0.0..=360.0).contains(&hue) {
            return Err(PptxError::InvalidValue {
                field: "HslColor.hue",
                value: hue.to_string(),
                expected: "0.0 to 360.0",
            });
        }
        if !(0.0..=100.0).contains(&saturation) {
            return Err(PptxError::InvalidValue {
                field: "HslColor.saturation",
                value: saturation.to_string(),
                expected: "0.0 to 100.0",
            });
        }
        if !(0.0..=100.0).contains(&luminance) {
            return Err(PptxError::InvalidValue {
                field: "HslColor.luminance",
                value: luminance.to_string(),
                expected: "0.0 to 100.0",
            });
        }
        Ok(Self {
            hue,
            saturation,
            luminance,
        })
    }
}

/// A system color (OS-defined color, e.g. "windowText", "window").
///
/// The `val` attribute specifies the system color name.
/// The optional `last_color` records the last concrete RGB color used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemColor {
    pub val: SystemColorVal,
    pub last_color: Option<String>,
}

/// A preset (named) color (e.g. "red", "blue", "green").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetColor {
    pub val: PresetColorVal,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hsl_new_valid() {
        let hsl = HslColor::new(180.0, 50.0, 75.0).unwrap(); // EXCEPTION(test-only)
        assert_eq!(hsl.hue, 180.0);
        assert_eq!(hsl.saturation, 50.0);
        assert_eq!(hsl.luminance, 75.0);
    }

    #[test]
    fn test_hsl_new_boundary_values() {
        assert!(HslColor::new(0.0, 0.0, 0.0).is_ok());
        assert!(HslColor::new(360.0, 100.0, 100.0).is_ok());
    }

    #[test]
    fn test_hsl_new_hue_out_of_range() {
        assert!(HslColor::new(-1.0, 50.0, 50.0).is_err());
        assert!(HslColor::new(361.0, 50.0, 50.0).is_err());
    }

    #[test]
    fn test_hsl_new_saturation_out_of_range() {
        assert!(HslColor::new(180.0, -1.0, 50.0).is_err());
        assert!(HslColor::new(180.0, 101.0, 50.0).is_err());
    }

    #[test]
    fn test_hsl_new_luminance_out_of_range() {
        assert!(HslColor::new(180.0, 50.0, -1.0).is_err());
        assert!(HslColor::new(180.0, 50.0, 101.0).is_err());
    }

    #[test]
    fn test_theme_color_new_valid_no_brightness() {
        let tc = ThemeColor::new(MsoThemeColorIndex::Accent1, None).unwrap(); // EXCEPTION(test-only)
        assert_eq!(tc.theme_color, MsoThemeColorIndex::Accent1);
        assert!(tc.brightness.is_none());
    }

    #[test]
    fn test_theme_color_new_valid_with_brightness() {
        let tc = ThemeColor::new(MsoThemeColorIndex::Accent1, Some(0.5)).unwrap(); // EXCEPTION(test-only)
        assert_eq!(tc.brightness, Some(0.5));
    }

    #[test]
    fn test_theme_color_new_boundary_brightness() {
        assert!(ThemeColor::new(MsoThemeColorIndex::Accent1, Some(-1.0)).is_ok());
        assert!(ThemeColor::new(MsoThemeColorIndex::Accent1, Some(1.0)).is_ok());
        assert!(ThemeColor::new(MsoThemeColorIndex::Accent1, Some(0.0)).is_ok());
    }

    #[test]
    fn test_theme_color_new_brightness_out_of_range() {
        assert!(ThemeColor::new(MsoThemeColorIndex::Accent1, Some(-1.1)).is_err());
        assert!(ThemeColor::new(MsoThemeColorIndex::Accent1, Some(1.1)).is_err());
    }
}
