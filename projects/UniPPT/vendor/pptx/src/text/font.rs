//! Font (character properties) for text runs.

use std::fmt;

use crate::dml::fill::FillFormat;
use crate::enums::dml::MsoColorType;
use crate::enums::text::MsoTextUnderlineType;
use crate::error::PptxError;
use crate::shapes::action::Hyperlink;
use crate::xml_util::{xml_escape, WriteXml};

/// An RGB color specified as individual red, green, and blue components.
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    /// Create a new RGB color.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Return the 6-digit hex string (e.g. `"FF0000"` for red).
    #[inline]
    #[must_use]
    pub fn to_hex(&self) -> String {
        format!("{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// Parse a 6-digit hex string (e.g. `"FF0000"`).
    ///
    /// # Errors
    ///
    /// Returns `Err(PptxError::InvalidValue)` if the string is not exactly
    /// 6 hex digits.
    pub fn from_hex(hex: &str) -> Result<Self, PptxError> {
        if hex.len() != 6 {
            return Err(PptxError::InvalidValue {
                field: "RgbColor",
                value: hex.to_string(),
                expected: "6-digit hex string (e.g. \"FF0000\")",
            });
        }
        let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| PptxError::InvalidValue {
            field: "RgbColor",
            value: hex.to_string(),
            expected: "6-digit hex string (e.g. \"FF0000\")",
        })?;
        let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| PptxError::InvalidValue {
            field: "RgbColor",
            value: hex.to_string(),
            expected: "6-digit hex string (e.g. \"FF0000\")",
        })?;
        let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| PptxError::InvalidValue {
            field: "RgbColor",
            value: hex.to_string(),
            expected: "6-digit hex string (e.g. \"FF0000\")",
        })?;
        Ok(Self { r, g, b })
    }
}

impl fmt::Display for RgbColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

/// Character formatting properties for a text run.
///
/// Corresponds to the `<a:rPr>` element in OOXML. `None` values indicate
/// that the property should be inherited from the style hierarchy.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Font {
    /// Latin typeface name from `<a:latin>` (e.g. `"Calibri"`).
    pub name: Option<String>,
    /// East Asian typeface name from `<a:ea>` (e.g. `"Microsoft YaHei"`).
    pub east_asia_name: Option<String>,
    /// Complex-script typeface name from `<a:cs>` (e.g. `"Arial"`).
    pub complex_script_name: Option<String>,
    /// Symbol typeface name from `<a:sym>` (e.g. `"Wingdings"`).
    pub symbol_name: Option<String>,
    /// Font size in points (e.g. `18.0` for 18pt).
    pub size: Option<f64>,
    /// Bold flag.
    pub bold: Option<bool>,
    /// Italic flag.
    pub italic: Option<bool>,
    /// Underline type. `None` means inherited; `Some(MsoTextUnderlineType::None)` means
    /// explicitly no underline; `Some(MsoTextUnderlineType::SingleLine)` for single, etc.
    pub underline: Option<MsoTextUnderlineType>,
    /// Font color.
    pub color: Option<RgbColor>,
    /// Strikethrough flag. When `true`, a single strike line is rendered.
    pub strikethrough: Option<bool>,
    /// Subscript flag. When `true`, text is rendered as subscript (baseline -25000).
    pub subscript: Option<bool>,
    /// Superscript flag. When `true`, text is rendered as superscript (baseline +30000).
    pub superscript: Option<bool>,
    /// Exact DrawingML baseline, in thousandths of a percent of font size.
    pub baseline_offset: Option<i64>,
    /// Language identifier (e.g. `"en-US"`, `"ja-JP"`).
    pub language_id: Option<String>,
    /// Fill format for the font (e.g. solid fill for text color via fill).
    pub fill: Option<FillFormat>,
    /// Hyperlink associated with this font (alternative to Run-level hyperlink).
    pub hyperlink: Option<Hyperlink>,
}

impl Font {
    /// Create a new Font with all properties set to `None` (inherited).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the font size in points.
    ///
    /// `size` must be greater than 0.0.
    ///
    /// # Errors
    ///
    /// Returns `Err(PptxError::InvalidValue)` if the size is not positive.
    pub fn set_size(&mut self, size: f64) -> Result<(), PptxError> {
        if size <= 0.0 {
            return Err(PptxError::InvalidValue {
                field: "Font.size",
                value: size.to_string(),
                expected: "greater than 0.0",
            });
        }
        self.size = Some(size);
        Ok(())
    }

    /// Return the color type of this font, if a color or fill with a solid color is set.
    ///
    /// When `fill` is a `SolidFill`, the color type is derived from its `ColorFormat`.
    /// When only a simple `color` (RGB) is set, returns `MsoColorType::Rgb`.
    /// Returns `None` if no color information is present.
    #[must_use]
    pub fn color_type(&self) -> Option<MsoColorType> {
        self.fill.as_ref().map_or_else(
            || {
                if self.color.is_some() {
                    Some(MsoColorType::Rgb)
                } else {
                    None
                }
            },
            |fill| match fill {
                FillFormat::Solid(solid) => Some(solid.color.color_type()),
                _ => None,
            },
        )
    }

    /// Write the `<a:rPr>` XML element into the given writer.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the writer fails.
    pub fn write_xml<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        // Build attributes
        w.write_str("<a:rPr")?;

        // Use custom language_id if set, otherwise default to en-US
        if let Some(ref lang) = self.language_id {
            write!(w, r#" lang="{}""#, xml_escape(lang))?;
        } else {
            w.write_str(r#" lang="en-US""#)?;
        }

        if let Some(size) = self.size {
            #[allow(clippy::cast_possible_truncation)] // intentional f64→i64 for OOXML units
            let sz = (size * 100.0) as i64;
            write!(w, r#" sz="{sz}""#)?;
        }

        if let Some(bold) = self.bold {
            write!(w, r#" b="{}""#, if bold { "1" } else { "0" })?;
        }

        if let Some(italic) = self.italic {
            write!(w, r#" i="{}""#, if italic { "1" } else { "0" })?;
        }

        if let Some(underline) = self.underline {
            write!(w, r#" u="{}""#, underline.to_xml_str())?;
        }

        if let Some(strike) = self.strikethrough {
            write!(
                w,
                r#" strike="{}""#,
                if strike { "sngStrike" } else { "noStrike" }
            )?;
        }

        // Superscript takes precedence over subscript if both are set
        if let Some(value) = self.baseline_offset {
            w.write_str(&format!(" baseline=\"{value}\""))?;
        } else if self.superscript == Some(true) {
            w.write_str(r#" baseline="30000""#)?;
        } else if self.subscript == Some(true) {
            w.write_str(r#" baseline="-25000""#)?;
        }

        w.write_str(r#" dirty="0""#)?;

        // Determine if we have child elements
        let has_children = self.fill.is_some()
            || self.color.is_some()
            || self.name.is_some()
            || self.east_asia_name.is_some()
            || self.complex_script_name.is_some()
            || self.symbol_name.is_some()
            || self.hyperlink.is_some();

        if has_children {
            w.write_char('>')?;

            // Fill takes precedence over simple color when both are set
            if let Some(ref fill) = self.fill {
                fill.write_xml(w)?;
            } else if let Some(ref color) = self.color {
                write!(
                    w,
                    r#"<a:solidFill><a:srgbClr val="{}"/></a:solidFill>"#,
                    color.to_hex()
                )?;
            }

            if let Some(ref name) = self.name {
                write!(w, r#"<a:latin typeface="{}"/>"#, xml_escape(name))?;
            }
            if let Some(ref name) = self.east_asia_name {
                write!(w, r#"<a:ea typeface="{}"/>"#, xml_escape(name))?;
            }
            if let Some(ref name) = self.complex_script_name {
                write!(w, r#"<a:cs typeface="{}"/>"#, xml_escape(name))?;
            }
            if let Some(ref name) = self.symbol_name {
                write!(w, r#"<a:sym typeface="{}"/>"#, xml_escape(name))?;
            }

            if let Some(ref hlink) = self.hyperlink {
                w.write_str("<a:hlinkClick")?;
                if let Some(ref rid) = hlink.r_id {
                    write!(w, r#" r:id="{}""#, xml_escape(rid.as_str()))?;
                }
                if let Some(ref tooltip) = hlink.tooltip {
                    write!(w, r#" tooltip="{}""#, xml_escape(tooltip))?;
                }
                w.write_str("/>")?;
            }

            w.write_str("</a:rPr>")
        } else {
            w.write_str("/>")
        }
    }

    /// Generate the `<a:rPr>` XML element string.
    ///
    /// If no properties are set, produces `<a:rPr lang="en-US" dirty="0"/>`.
    ///
    /// # Panics
    ///
    /// Panics if writing to a `String` fails (should never happen).
    #[must_use]
    pub fn to_xml_string(&self) -> String {
        let mut s = String::new();
        self.write_xml(&mut s)
            .unwrap_or_else(|_| unreachable!("write to String should not fail"));
        s
    }
}

#[cfg(test)]
#[path = "font_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "font_color_tests.rs"]
mod color_tests;
