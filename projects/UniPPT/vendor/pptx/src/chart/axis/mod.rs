//! Chart axis types.

pub(crate) mod category;
mod category_tests;
pub(crate) mod date;
mod date_tests;
pub(crate) mod value;
mod value_ext;
mod value_tests;

pub use category::CategoryAxis;
pub use date::DateAxis;
pub use value::ValueAxis;

use crate::text::font::Font;
use crate::text::TextFrame;

use super::chart::ChartFormat;

/// Represents the title of a chart axis, with optional rich text and formatting.
#[derive(Debug, Clone)]
pub struct AxisTitle {
    /// Rich text content for the axis title.
    pub text_frame: Option<TextFrame>,
    /// Whether the title has a text frame.
    has_text_frame: bool,
    /// Visual formatting (fill + line) for the title area.
    pub format: Option<ChartFormat>,
}

impl AxisTitle {
    /// Create a new empty axis title.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            text_frame: None,
            has_text_frame: false,
            format: None,
        }
    }

    /// Create an axis title from a plain text string.
    #[must_use]
    pub fn from_text(text: &str) -> Self {
        let mut tf = TextFrame::new();
        tf.set_text(text);
        Self {
            text_frame: Some(tf),
            has_text_frame: true,
            format: None,
        }
    }

    /// Whether the title has a text frame.
    #[must_use]
    pub const fn has_text_frame(&self) -> bool {
        self.has_text_frame
    }

    /// Get the text frame, if present.
    #[must_use]
    pub const fn text_frame(&self) -> Option<&TextFrame> {
        self.text_frame.as_ref()
    }

    /// Get or create a mutable text frame.
    pub fn text_frame_mut(&mut self) -> &mut TextFrame {
        self.has_text_frame = true;
        self.text_frame.get_or_insert_with(TextFrame::new)
    }

    /// Set the text frame.
    pub fn set_text_frame(&mut self, text_frame: TextFrame) {
        self.has_text_frame = true;
        self.text_frame = Some(text_frame);
    }

    /// Get the plain-text representation of the title, if any.
    #[must_use]
    pub fn text(&self) -> Option<String> {
        self.text_frame.as_ref().map(TextFrame::text)
    }
}

/// Creates an empty axis title with no text frame.
impl Default for AxisTitle {
    fn default() -> Self {
        Self::new()
    }
}

/// Formatting properties for axis tick labels.
#[derive(Debug, Clone)]
pub struct TickLabels {
    /// Font for the tick labels.
    font: Option<Font>,
    /// Number format string for the tick labels.
    pub number_format: Option<String>,
    /// Whether the number format is linked to the source data.
    pub number_format_is_linked: bool,
    /// Label offset from the axis (percentage, 0-1000).
    pub offset: Option<u32>,
}

impl TickLabels {
    /// Create new tick labels with default settings.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            font: None,
            number_format: None,
            number_format_is_linked: true,
            offset: None,
        }
    }

    /// Get the font, if explicitly set.
    #[must_use]
    pub const fn font(&self) -> Option<&Font> {
        self.font.as_ref()
    }

    /// Get or create a mutable font for the tick labels.
    pub fn font_mut(&mut self) -> &mut Font {
        self.font.get_or_insert_with(Font::new)
    }
}

/// Creates empty tick labels with no font or number format.
impl Default for TickLabels {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // AxisTitle tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_axis_title_new() {
        let at = AxisTitle::new();
        assert!(!at.has_text_frame());
        assert!(at.text_frame().is_none());
        assert!(at.text().is_none());
        assert!(at.format.is_none());
    }

    #[test]
    fn test_axis_title_from_text() {
        let at = AxisTitle::from_text("Revenue");
        assert!(at.has_text_frame());
        assert_eq!(at.text(), Some("Revenue".to_string()));
    }

    #[test]
    fn test_axis_title_text_frame_mut() {
        let mut at = AxisTitle::new();
        assert!(!at.has_text_frame());
        at.text_frame_mut().set_text("New Title");
        assert!(at.has_text_frame());
        assert_eq!(at.text(), Some("New Title".to_string()));
    }

    #[test]
    fn test_axis_title_set_text_frame() {
        let mut at = AxisTitle::new();
        let mut tf = TextFrame::new();
        tf.set_text("Custom Title");
        at.set_text_frame(tf);
        assert!(at.has_text_frame());
        assert_eq!(at.text(), Some("Custom Title".to_string()));
    }

    // -----------------------------------------------------------------------
    // TickLabels tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_tick_labels_new() {
        let tl = TickLabels::new();
        assert!(tl.font().is_none());
        assert!(tl.number_format.is_none());
        assert!(tl.number_format_is_linked);
        assert!(tl.offset.is_none());
    }

    #[test]
    fn test_tick_labels_font_mut() {
        let mut tl = TickLabels::new();
        tl.font_mut().bold = Some(true);
        assert!(tl.font().is_some());
        assert_eq!(tl.font().unwrap().bold, Some(true));
    }

    #[test]
    fn test_tick_labels_properties() {
        let mut tl = TickLabels::new();
        tl.number_format = Some("#,##0".to_string());
        tl.number_format_is_linked = false;
        tl.offset = Some(100);
        assert_eq!(tl.number_format.as_deref(), Some("#,##0"));
        assert!(!tl.number_format_is_linked);
        assert_eq!(tl.offset, Some(100));
    }
}
