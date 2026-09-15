//! Chart-level format and title types.

use crate::dml::fill::FillFormat;
use crate::dml::line::LineFormat;
use crate::text::TextFrame;

/// Chart-level format (fill + line for the chart area).
#[derive(Debug, Clone)]
pub struct ChartFormat {
    /// Fill for the chart area.
    pub fill: Option<FillFormat>,
    /// Line (border) for the chart area.
    pub line: Option<LineFormat>,
}

impl ChartFormat {
    /// Create an empty format (inherited).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            fill: None,
            line: None,
        }
    }
}

/// Creates an empty chart format with no fill or line.
impl Default for ChartFormat {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents the title of a chart, with optional rich text and formatting.
#[derive(Debug, Clone)]
pub struct ChartTitle {
    /// Rich text content for the chart title.
    pub text_frame: Option<TextFrame>,
    /// Whether the title has a text frame.
    has_text_frame: bool,
    /// Visual formatting (fill + line) for the title area.
    pub format: Option<ChartFormat>,
}

impl ChartTitle {
    /// Create a new empty chart title.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            text_frame: None,
            has_text_frame: false,
            format: None,
        }
    }

    /// Create a chart title from a plain text string.
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
    pub fn set_text_frame(&mut self, tf: TextFrame) {
        self.has_text_frame = true;
        self.text_frame = Some(tf);
    }

    /// Get the plain text of the title, if a text frame is present.
    #[must_use]
    pub fn text(&self) -> Option<String> {
        self.text_frame.as_ref().map(TextFrame::text)
    }
}

/// Creates an empty chart title with no text content.
impl Default for ChartTitle {
    fn default() -> Self {
        Self::new()
    }
}
