//! `DrawingML` line (border/outline) formatting.

use crate::dml::color::ColorFormat;
use crate::dml::fill::FillFormat;
use crate::enums::dml::MsoLineDashStyle;
use crate::units::Emu;
use crate::xml_util::WriteXml;

/// Line cap style.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineCap {
    Flat,
    Round,
    Square,
}

impl LineCap {
    /// Return the XML attribute value.
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::Flat => "flat",
            Self::Round => "rnd",
            Self::Square => "sq",
        }
    }
}

/// Line join style.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineJoin {
    Round,
    Bevel,
    Miter,
}

/// Line (outline/border) formatting for a shape.
///
/// Corresponds to the `<a:ln>` element.  Controls the color, width,
/// and dash style of shape outlines and connector lines.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LineFormat {
    /// Line color.  If `None`, the color is inherited from the style hierarchy.
    pub color: Option<ColorFormat>,
    /// Line width.  If `None`, the width is inherited.
    pub width: Option<Emu>,
    /// Dash style for the line.  If `None`, solid or inherited.
    pub dash_style: Option<MsoLineDashStyle>,
    /// Fill for the line itself (usually solid or no-fill).
    pub fill: Option<FillFormat>,
    /// Line end cap style.
    pub cap: Option<LineCap>,
    /// Line join style.
    pub join: Option<LineJoin>,
}

impl LineFormat {
    /// Create a new `LineFormat` with all properties inherited (None).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a solid line with the given color and width.
    #[must_use]
    pub fn solid(color: ColorFormat, width: Emu) -> Self {
        let fill = FillFormat::solid(color.clone());
        Self {
            color: Some(color),
            width: Some(width),
            dash_style: None,
            fill: Some(fill),
            cap: None,
            join: None,
        }
    }

    /// Write the `<a:ln>` XML element into the given writer.
    ///
    /// Returns `Ok(true)` if XML was written, `Ok(false)` if there are no
    /// explicit properties (all inherited) and nothing was written.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the writer fails.
    pub fn write_xml<W: std::fmt::Write>(&self, w: &mut W) -> Result<bool, std::fmt::Error> {
        let has_any = self.color.is_some()
            || self.width.is_some()
            || self.dash_style.is_some()
            || self.fill.is_some()
            || self.cap.is_some()
            || self.join.is_some();

        if !has_any {
            return Ok(false);
        }

        w.write_str("<a:ln")?;

        if let Some(w_val) = self.width {
            write!(w, r#" w="{}""#, w_val.0)?;
        }

        if let Some(cap) = self.cap {
            write!(w, r#" cap="{}""#, cap.to_xml_str())?;
        }

        w.write_char('>')?;

        // Fill (solidFill/noFill/etc.) for the line itself
        if let Some(ref fill) = self.fill {
            fill.write_xml(w)?;
        } else if let Some(ref color) = self.color {
            // Shorthand: if color is set but fill is not, emit a solidFill
            w.write_str("<a:solidFill>")?;
            color.write_xml(w)?;
            w.write_str("</a:solidFill>")?;
        }

        if let Some(dash) = self.dash_style {
            write!(w, r#"<a:prstDash val="{}"/>"#, dash.to_xml_str())?;
        }

        match self.join {
            Some(LineJoin::Round) => w.write_str("<a:round/>")?,
            Some(LineJoin::Bevel) => w.write_str("<a:bevel/>")?,
            Some(LineJoin::Miter) => w.write_str("<a:miter/>")?,
            None => {}
        }

        w.write_str("</a:ln>")?;
        Ok(true)
    }

    /// Generate the `<a:ln>` XML element string.
    ///
    /// Returns `None` if there are no explicit properties to emit (all inherited).
    #[must_use]
    pub fn to_xml_string(&self) -> Option<String> {
        let mut s = String::new();
        match self.write_xml(&mut s) {
            Ok(true) => Some(s),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_line_no_xml() {
        let l = LineFormat::new();
        assert!(l.to_xml_string().is_none());
    }

    #[test]
    fn test_solid_line_xml() {
        let l = LineFormat::solid(ColorFormat::rgb(0, 0, 0), Emu(12700));
        let xml = l.to_xml_string().unwrap();
        assert!(xml.starts_with("<a:ln"));
        assert!(xml.contains(r#"w="12700""#));
        assert!(xml.contains("<a:solidFill>"));
        assert!(xml.contains(r#"val="000000""#));
        assert!(xml.ends_with("</a:ln>"));
    }

    #[test]
    fn test_line_with_dash_style() {
        let l = LineFormat {
            color: Some(ColorFormat::rgb(255, 0, 0)),
            width: Some(Emu(25400)),
            dash_style: Some(MsoLineDashStyle::Dash),
            fill: None,
            cap: None,
            join: None,
        };
        let xml = l.to_xml_string().unwrap();
        assert!(xml.contains(r#"w="25400""#));
        assert!(xml.contains(r#"<a:prstDash val="dash"/>"#));
        // color should be emitted as solidFill since fill is None
        assert!(xml.contains("<a:solidFill>"));
        assert!(xml.contains("FF0000"));
    }

    #[test]
    fn test_line_width_only() {
        let l = LineFormat {
            width: Some(Emu(9525)),
            ..LineFormat::new()
        };
        let xml = l.to_xml_string().unwrap();
        assert!(xml.contains(r#"w="9525""#));
    }

    #[test]
    fn test_line_no_fill() {
        let l = LineFormat {
            fill: Some(FillFormat::no_fill()),
            ..LineFormat::new()
        };
        let xml = l.to_xml_string().unwrap();
        assert!(xml.contains("<a:noFill/>"));
    }
}
