//! Data labels for chart series and data points.

use crate::enums::chart::XlDataLabelPosition;
use crate::text::font::Font;
use crate::text::TextFrame;

/// Formatting properties for the data labels on a series or data point.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct DataLabels {
    show_value: bool,
    show_category_name: bool,
    show_series_name: bool,
    show_percent: bool,
    show_legend_key: bool,
    show_bubble_size: bool,
    show_leader_lines: bool,
    number_format: Option<String>,
    number_format_is_linked: bool,
    position: Option<XlDataLabelPosition>,
    font: Option<Font>,
}

impl DataLabels {
    /// Create data labels with default settings (all hidden).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            show_value: false,
            show_category_name: false,
            show_series_name: false,
            show_percent: false,
            show_legend_key: false,
            show_bubble_size: false,
            show_leader_lines: false,
            number_format: None,
            number_format_is_linked: true,
            position: None,
            font: None,
        }
    }

    /// Whether the value is shown on the data label.
    #[must_use]
    pub const fn show_value(&self) -> bool {
        self.show_value
    }

    /// Set whether the value is shown.
    pub fn set_show_value(&mut self, value: bool) {
        self.show_value = value;
    }

    /// Whether the category name is shown on the data label.
    #[must_use]
    pub const fn show_category_name(&self) -> bool {
        self.show_category_name
    }

    /// Set whether the category name is shown.
    pub fn set_show_category_name(&mut self, value: bool) {
        self.show_category_name = value;
    }

    /// Whether the series name is shown on the data label.
    #[must_use]
    pub const fn show_series_name(&self) -> bool {
        self.show_series_name
    }

    /// Set whether the series name is shown.
    pub fn set_show_series_name(&mut self, value: bool) {
        self.show_series_name = value;
    }

    /// Whether the percentage is shown on the data label.
    #[must_use]
    pub const fn show_percent(&self) -> bool {
        self.show_percent
    }

    /// Set whether the percentage is shown.
    pub fn set_show_percent(&mut self, value: bool) {
        self.show_percent = value;
    }

    /// Whether the legend key is shown on the data label.
    #[must_use]
    pub const fn show_legend_key(&self) -> bool {
        self.show_legend_key
    }

    /// Set whether the legend key is shown.
    pub fn set_show_legend_key(&mut self, value: bool) {
        self.show_legend_key = value;
    }

    /// Whether the bubble size is shown on the data label.
    #[must_use]
    pub const fn show_bubble_size(&self) -> bool {
        self.show_bubble_size
    }

    /// Set whether the bubble size is shown.
    pub fn set_show_bubble_size(&mut self, value: bool) {
        self.show_bubble_size = value;
    }

    /// Whether leader lines are shown.
    #[must_use]
    pub const fn show_leader_lines(&self) -> bool {
        self.show_leader_lines
    }

    /// Set whether leader lines are shown.
    pub fn set_show_leader_lines(&mut self, value: bool) {
        self.show_leader_lines = value;
    }

    /// The number format for data label values, or `None` for default.
    #[must_use]
    pub fn number_format(&self) -> Option<&str> {
        self.number_format.as_deref()
    }

    /// Set the number format.
    pub fn set_number_format(&mut self, format: Option<&str>) {
        self.number_format = format.map(ToString::to_string);
    }

    /// Whether the number format is linked to the source data.
    #[must_use]
    pub const fn number_format_is_linked(&self) -> bool {
        self.number_format_is_linked
    }

    /// Set whether the number format is linked.
    pub fn set_number_format_is_linked(&mut self, value: bool) {
        self.number_format_is_linked = value;
    }

    /// The data label position, or `None` for the default position.
    #[must_use]
    pub const fn position(&self) -> Option<XlDataLabelPosition> {
        self.position
    }

    /// Set the data label position.
    pub fn set_position(&mut self, position: Option<XlDataLabelPosition>) {
        self.position = position;
    }

    /// The font for data label text, if set.
    #[must_use]
    pub const fn font(&self) -> Option<&Font> {
        self.font.as_ref()
    }

    /// Mutable access to the font. Creates a default if `None`.
    pub fn font_mut(&mut self) -> &mut Font {
        self.font.get_or_insert_with(Font::new)
    }

    /// Set the font for data labels.
    pub fn set_font(&mut self, font: Font) {
        self.font = Some(font);
    }
}

/// Creates empty data labels with no visibility flags or formatting.
impl Default for DataLabels {
    fn default() -> Self {
        Self::new()
    }
}

/// Formatting properties for a single data label on a data point.
#[derive(Debug, Clone)]
pub struct DataLabel {
    show_value: Option<bool>,
    show_category_name: Option<bool>,
    show_series_name: Option<bool>,
    number_format: Option<String>,
    position: Option<XlDataLabelPosition>,
    font: Option<Font>,
    text_frame: Option<TextFrame>,
    has_text_frame: bool,
}

impl DataLabel {
    /// Create a data label with no explicit settings (inherits from series).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            show_value: None,
            show_category_name: None,
            show_series_name: None,
            number_format: None,
            position: None,
            font: None,
            text_frame: None,
            has_text_frame: false,
        }
    }

    /// Whether the value is shown, or `None` to inherit.
    #[must_use]
    pub const fn show_value(&self) -> Option<bool> {
        self.show_value
    }

    /// Set whether the value is shown.
    pub fn set_show_value(&mut self, value: Option<bool>) {
        self.show_value = value;
    }

    /// Whether the category name is shown, or `None` to inherit.
    #[must_use]
    pub const fn show_category_name(&self) -> Option<bool> {
        self.show_category_name
    }

    /// Set whether the category name is shown.
    pub fn set_show_category_name(&mut self, value: Option<bool>) {
        self.show_category_name = value;
    }

    /// Whether the series name is shown, or `None` to inherit.
    #[must_use]
    pub const fn show_series_name(&self) -> Option<bool> {
        self.show_series_name
    }

    /// Set whether the series name is shown.
    pub fn set_show_series_name(&mut self, value: Option<bool>) {
        self.show_series_name = value;
    }

    /// The number format, or `None` to inherit.
    #[must_use]
    pub fn number_format(&self) -> Option<&str> {
        self.number_format.as_deref()
    }

    /// Set the number format.
    pub fn set_number_format(&mut self, format: Option<&str>) {
        self.number_format = format.map(ToString::to_string);
    }

    /// The data label position, or `None` to inherit.
    #[must_use]
    pub const fn position(&self) -> Option<XlDataLabelPosition> {
        self.position
    }

    /// Set the data label position.
    pub fn set_position(&mut self, position: Option<XlDataLabelPosition>) {
        self.position = position;
    }

    /// The font for this data label, if set.
    #[must_use]
    pub const fn font(&self) -> Option<&Font> {
        self.font.as_ref()
    }

    /// Mutable access to the font. Creates a default if `None`.
    pub fn font_mut(&mut self) -> &mut Font {
        self.font.get_or_insert_with(Font::new)
    }

    /// Set the font for this data label.
    pub fn set_font(&mut self, font: Font) {
        self.font = Some(font);
    }

    /// Whether this data label has a text frame.
    #[must_use]
    pub const fn has_text_frame(&self) -> bool {
        self.has_text_frame
    }

    /// The text frame for this data label, if present.
    #[must_use]
    pub const fn text_frame(&self) -> Option<&TextFrame> {
        self.text_frame.as_ref()
    }

    /// Mutable access to the text frame. Creates a default if `None`.
    pub fn text_frame_mut(&mut self) -> &mut TextFrame {
        self.has_text_frame = true;
        self.text_frame.get_or_insert_with(TextFrame::new)
    }

    /// Set the text frame for this data label.
    pub fn set_text_frame(&mut self, tf: TextFrame) {
        self.has_text_frame = true;
        self.text_frame = Some(tf);
    }
}

/// Creates an empty data label with no visibility flags or formatting.
impl Default for DataLabel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "datalabel_tests.rs"]
mod tests;
