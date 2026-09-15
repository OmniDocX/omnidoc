//! Print settings and handout layout configuration for presentations.

use std::fmt;

use crate::enums::misc::{HandoutLayout, PrintColorMode, PrintOrientation, PrintWhat};
use crate::error::PptxResult;
use crate::xml_util::WriteXml;

/// Print settings for a presentation.
///
/// Maps to the `<p:prnPr>` element in presentation.xml.
///
/// # Examples
///
/// ```
/// use pptx::print_settings::PrintSettings;
/// use pptx::enums::misc::{PrintColorMode, PrintWhat, HandoutLayout, PrintOrientation};
///
/// let settings = PrintSettings::new()
///     .with_color_mode(PrintColorMode::Grayscale)
///     .with_print_what(PrintWhat::Handouts)
///     .with_handout_layout(HandoutLayout::Six)
///     .with_orientation(PrintOrientation::Landscape)
///     .with_frame_slides(true);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintSettings {
    /// How the presentation should be printed with respect to color.
    pub color_mode: PrintColorMode,
    /// What content to print.
    pub print_what: PrintWhat,
    /// The handout layout (number of slides per page), used when `print_what` is `Handouts`.
    pub handout_layout: Option<HandoutLayout>,
    /// The print orientation.
    pub orientation: PrintOrientation,
    /// Whether to frame slides when printing.
    pub frame_slides: bool,
}

impl PrintSettings {
    /// Create new print settings with sensible defaults.
    ///
    /// Defaults: color mode = Color, print what = Slides, no handout layout,
    /// orientation = Portrait, frame slides = false.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            color_mode: PrintColorMode::Color,
            print_what: PrintWhat::Slides,
            handout_layout: None,
            orientation: PrintOrientation::Portrait,
            frame_slides: false,
        }
    }

    /// Set the color mode.
    #[must_use]
    pub const fn with_color_mode(mut self, mode: PrintColorMode) -> Self {
        self.color_mode = mode;
        self
    }

    /// Set what content to print.
    #[must_use]
    pub const fn with_print_what(mut self, what: PrintWhat) -> Self {
        self.print_what = what;
        self
    }

    /// Set the handout layout (slides per page).
    #[must_use]
    pub const fn with_handout_layout(mut self, layout: HandoutLayout) -> Self {
        self.handout_layout = Some(layout);
        self
    }

    /// Set the print orientation.
    #[must_use]
    pub const fn with_orientation(mut self, orientation: PrintOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Set whether to frame slides.
    #[must_use]
    pub const fn with_frame_slides(mut self, frame: bool) -> Self {
        self.frame_slides = frame;
        self
    }

    /// Parse print settings from a `<p:prnPr>` XML element string.
    ///
    /// Expects the attributes of a `<p:prnPr ... />` or `<p:prnPr ...>` tag.
    ///
    /// # Errors
    ///
    /// Returns an error if any attribute in the element is malformed.
    pub(crate) fn from_xml_element(attrs: &quick_xml::events::BytesStart<'_>) -> PptxResult<Self> {
        let mut settings = Self::new();

        if let Some(val) = crate::xml_util::attr_value(attrs, b"clrMode")? {
            if let Some(mode) = PrintColorMode::from_xml_str(&val) {
                settings.color_mode = mode;
            }
        }

        if let Some(val) = crate::xml_util::attr_value(attrs, b"prnWhat")? {
            if let Some(what) = PrintWhat::from_xml_str(&val) {
                settings.print_what = what;
            }
        }

        // The handout layout is specified via the prnWhat attribute when it
        // contains a handouts value like "handouts4".
        if let Some(val) = crate::xml_util::attr_value(attrs, b"prnWhat")? {
            if let Some(layout) = HandoutLayout::from_xml_str(&val) {
                settings.handout_layout = Some(layout);
                settings.print_what = PrintWhat::Handouts;
            }
        }

        if let Some(val) = crate::xml_util::attr_value(attrs, b"orient")? {
            if let Some(orient) = PrintOrientation::from_xml_str(&val) {
                settings.orientation = orient;
            }
        }

        if let Some(val) = crate::xml_util::attr_value(attrs, b"frameSlides")? {
            settings.frame_slides = val.as_ref() == "1" || val.as_ref() == "true";
        }

        Ok(settings)
    }
}

impl Default for PrintSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl WriteXml for PrintSettings {
    fn write_xml<W: fmt::Write>(&self, w: &mut W) -> fmt::Result {
        w.write_str("<p:prnPr")?;

        // prnWhat: if handout layout is specified, use the handout value;
        // otherwise use the print_what value (only emit if not the default "slides").
        if let Some(layout) = self.handout_layout {
            write!(w, r#" prnWhat="{}""#, layout.to_xml_str())?;
        } else if self.print_what != PrintWhat::Slides {
            write!(w, r#" prnWhat="{}""#, self.print_what.to_xml_str())?;
        }

        if self.color_mode != PrintColorMode::Color {
            write!(w, r#" clrMode="{}""#, self.color_mode.to_xml_str())?;
        }

        if self.orientation != PrintOrientation::Portrait {
            write!(w, r#" orient="{}""#, self.orientation.to_xml_str())?;
        }

        if self.frame_slides {
            w.write_str(r#" frameSlides="1""#)?;
        }

        w.write_str("/>")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_print_settings() {
        let settings = PrintSettings::new();
        assert_eq!(settings.color_mode, PrintColorMode::Color);
        assert_eq!(settings.print_what, PrintWhat::Slides);
        assert_eq!(settings.handout_layout, None);
        assert_eq!(settings.orientation, PrintOrientation::Portrait);
        assert!(!settings.frame_slides);
    }

    #[test]
    fn test_default_xml_minimal() {
        let settings = PrintSettings::new();
        let xml = settings.to_xml_string();
        assert_eq!(xml, "<p:prnPr/>");
    }

    #[test]
    fn test_grayscale_xml() {
        let settings = PrintSettings::new().with_color_mode(PrintColorMode::Grayscale);
        let xml = settings.to_xml_string();
        assert!(xml.contains(r#"clrMode="gray""#));
    }

    #[test]
    fn test_handout_layout_xml() {
        let settings = PrintSettings::new()
            .with_print_what(PrintWhat::Handouts)
            .with_handout_layout(HandoutLayout::Six);
        let xml = settings.to_xml_string();
        assert!(xml.contains(r#"prnWhat="handouts6""#));
    }

    #[test]
    fn test_notes_xml() {
        let settings = PrintSettings::new().with_print_what(PrintWhat::Notes);
        let xml = settings.to_xml_string();
        assert!(xml.contains(r#"prnWhat="notes""#));
    }

    #[test]
    fn test_landscape_xml() {
        let settings = PrintSettings::new().with_orientation(PrintOrientation::Landscape);
        let xml = settings.to_xml_string();
        assert!(xml.contains(r#"orient="landscape""#));
    }

    #[test]
    fn test_frame_slides_xml() {
        let settings = PrintSettings::new().with_frame_slides(true);
        let xml = settings.to_xml_string();
        assert!(xml.contains(r#"frameSlides="1""#));
    }

    #[test]
    fn test_all_options_xml() {
        let settings = PrintSettings::new()
            .with_color_mode(PrintColorMode::PureBlackWhite)
            .with_print_what(PrintWhat::Handouts)
            .with_handout_layout(HandoutLayout::Four)
            .with_orientation(PrintOrientation::Landscape)
            .with_frame_slides(true);
        let xml = settings.to_xml_string();
        assert!(xml.contains(r#"prnWhat="handouts4""#));
        assert!(xml.contains(r#"clrMode="pureBlkWht""#));
        assert!(xml.contains(r#"orient="landscape""#));
        assert!(xml.contains(r#"frameSlides="1""#));
    }

    #[test]
    fn test_builder_pattern() {
        let settings = PrintSettings::new()
            .with_color_mode(PrintColorMode::Grayscale)
            .with_print_what(PrintWhat::Outline)
            .with_orientation(PrintOrientation::Landscape);
        assert_eq!(settings.color_mode, PrintColorMode::Grayscale);
        assert_eq!(settings.print_what, PrintWhat::Outline);
        assert_eq!(settings.orientation, PrintOrientation::Landscape);
    }

    #[test]
    fn test_parse_from_xml_element() {
        let xml =
            br#"<p:prnPr prnWhat="handouts6" clrMode="gray" orient="landscape" frameSlides="1"/>"#;
        let mut reader = quick_xml::Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(quick_xml::events::Event::Empty(ref e))
                | Ok(quick_xml::events::Event::Start(ref e)) => {
                    let settings = PrintSettings::from_xml_element(e).unwrap();
                    assert_eq!(settings.color_mode, PrintColorMode::Grayscale);
                    assert_eq!(settings.print_what, PrintWhat::Handouts);
                    assert_eq!(settings.handout_layout, Some(HandoutLayout::Six));
                    assert_eq!(settings.orientation, PrintOrientation::Landscape);
                    assert!(settings.frame_slides);
                    break;
                }
                Ok(quick_xml::events::Event::Eof) => break,
                _ => {}
            }
        }
    }

    #[test]
    fn test_parse_default_xml_element() {
        let xml = br#"<p:prnPr/>"#;
        let mut reader = quick_xml::Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(quick_xml::events::Event::Empty(ref e))
                | Ok(quick_xml::events::Event::Start(ref e)) => {
                    let settings = PrintSettings::from_xml_element(e).unwrap();
                    assert_eq!(settings.color_mode, PrintColorMode::Color);
                    assert_eq!(settings.print_what, PrintWhat::Slides);
                    assert_eq!(settings.handout_layout, None);
                    assert_eq!(settings.orientation, PrintOrientation::Portrait);
                    assert!(!settings.frame_slides);
                    break;
                }
                Ok(quick_xml::events::Event::Eof) => break,
                _ => {}
            }
        }
    }

    #[test]
    fn test_roundtrip_xml() {
        let original = PrintSettings::new()
            .with_color_mode(PrintColorMode::Grayscale)
            .with_print_what(PrintWhat::Handouts)
            .with_handout_layout(HandoutLayout::Four)
            .with_orientation(PrintOrientation::Landscape)
            .with_frame_slides(true);
        let xml_str = original.to_xml_string();

        // Parse the XML back
        let xml_bytes = xml_str.as_bytes();
        let mut reader = quick_xml::Reader::from_reader(xml_bytes);
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(quick_xml::events::Event::Empty(ref e))
                | Ok(quick_xml::events::Event::Start(ref e)) => {
                    let parsed = PrintSettings::from_xml_element(e).unwrap();
                    assert_eq!(parsed.color_mode, original.color_mode);
                    assert_eq!(parsed.handout_layout, original.handout_layout);
                    assert_eq!(parsed.orientation, original.orientation);
                    assert_eq!(parsed.frame_slides, original.frame_slides);
                    // print_what should be Handouts since handout_layout is set
                    assert_eq!(parsed.print_what, PrintWhat::Handouts);
                    break;
                }
                Ok(quick_xml::events::Event::Eof) => break,
                _ => {}
            }
        }
    }
}
