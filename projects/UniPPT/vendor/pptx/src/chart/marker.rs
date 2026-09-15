//! Chart marker types.

use crate::dml::fill::FillFormat;
use crate::dml::line::LineFormat;
use crate::enums::chart::XlMarkerStyle;

/// Format properties for a marker.
#[derive(Debug, Clone)]
pub struct MarkerFormat {
    /// Fill for the marker.
    pub fill: Option<FillFormat>,
    /// Line (outline) for the marker.
    pub line: Option<LineFormat>,
}

impl MarkerFormat {
    /// Create an empty marker format.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            fill: None,
            line: None,
        }
    }
}

/// Creates an empty marker format with no fill or line.
impl Default for MarkerFormat {
    fn default() -> Self {
        Self::new()
    }
}

/// A marker on a chart data point or series (used in line, scatter, radar charts).
#[derive(Debug, Clone)]
pub struct Marker {
    style: XlMarkerStyle,
    size: Option<u32>,
    format: Option<MarkerFormat>,
}

impl Marker {
    /// Create a new marker with the given style.
    #[must_use]
    pub const fn new(style: XlMarkerStyle) -> Self {
        Self {
            style,
            size: None,
            format: None,
        }
    }

    /// Create a marker with style and size.
    #[must_use]
    pub const fn with_size(style: XlMarkerStyle, size: u32) -> Self {
        Self {
            style,
            size: Some(size),
            format: None,
        }
    }

    /// The marker style.
    #[must_use]
    pub const fn style(&self) -> XlMarkerStyle {
        self.style
    }

    /// Set the marker style.
    pub fn set_style(&mut self, style: XlMarkerStyle) {
        self.style = style;
    }

    /// The marker size (2-72), or `None` for default.
    #[must_use]
    pub const fn size(&self) -> Option<u32> {
        self.size
    }

    /// Set the marker size.
    pub fn set_size(&mut self, size: Option<u32>) {
        self.size = size;
    }

    /// The marker format (fill + line).
    #[must_use]
    pub const fn format(&self) -> Option<&MarkerFormat> {
        self.format.as_ref()
    }

    /// Mutable access to the marker format. Creates a default if `None`.
    pub fn format_mut(&mut self) -> &mut MarkerFormat {
        self.format.get_or_insert_with(MarkerFormat::new)
    }

    /// Set the marker format.
    pub fn set_format(&mut self, format: MarkerFormat) {
        self.format = Some(format);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dml::color::ColorFormat;

    #[test]
    fn test_marker_format() {
        let mut marker = Marker::new(XlMarkerStyle::Circle);
        assert!(marker.format().is_none());

        let fmt = marker.format_mut();
        fmt.fill = Some(FillFormat::solid(ColorFormat::rgb(255, 0, 0)));
        assert!(marker.format().is_some());
    }

    #[test]
    fn test_marker_with_line_format() {
        let mut marker = Marker::with_size(XlMarkerStyle::Diamond, 8);
        let fmt = marker.format_mut();
        fmt.line = Some(LineFormat {
            color: Some(ColorFormat::rgb(0, 0, 0)),
            width: Some(crate::units::Emu(12700)),
            ..LineFormat::default()
        });
        assert!(marker.format().unwrap().line.is_some());
    }
}
