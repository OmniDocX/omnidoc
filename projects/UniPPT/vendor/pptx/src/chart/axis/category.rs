//! Category axis type.

use crate::enums::chart::{XlAxisCrosses, XlCategoryType, XlTickLabelPosition, XlTickMark};

use super::super::chart::ChartFormat;
use super::{AxisTitle, TickLabels};

/// A category axis on a chart.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct CategoryAxis {
    has_title: bool,
    title: Option<String>,
    axis_title: Option<AxisTitle>,
    visible: bool,
    major_tick_mark: XlTickMark,
    minor_tick_mark: XlTickMark,
    tick_label_position: XlTickLabelPosition,
    has_major_gridlines: bool,
    has_minor_gridlines: bool,
    crosses: XlAxisCrosses,
    category_type: XlCategoryType,
    number_format: String,
    reverse_order: bool,
    tick_labels: Option<TickLabels>,
    format: Option<ChartFormat>,
    major_gridline_format: Option<ChartFormat>,
    minor_gridline_format: Option<ChartFormat>,
}

impl CategoryAxis {
    /// Create a new category axis with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            has_title: false,
            title: None,
            axis_title: None,
            visible: true,
            major_tick_mark: XlTickMark::Outside,
            minor_tick_mark: XlTickMark::None,
            tick_label_position: XlTickLabelPosition::NextToAxis,
            has_major_gridlines: false,
            has_minor_gridlines: false,
            crosses: XlAxisCrosses::Automatic,
            category_type: XlCategoryType::AutomaticScale,
            number_format: "General".to_string(),
            reverse_order: false,
            tick_labels: None,
            format: None,
            major_gridline_format: None,
            minor_gridline_format: None,
        }
    }

    /// Whether the axis has a title.
    #[must_use]
    pub const fn has_title(&self) -> bool {
        self.has_title
    }

    /// Set whether the axis has a title.
    pub fn set_has_title(&mut self, value: bool) {
        self.has_title = value;
        if !value {
            self.title = None;
            self.axis_title = None;
        }
    }

    /// The axis title text (backward-compatible plain text accessor).
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Set the axis title text (backward-compatible plain text setter).
    pub fn set_title(&mut self, title: &str) {
        self.has_title = true;
        self.title = Some(title.to_string());
        self.axis_title = Some(AxisTitle::from_text(title));
    }

    /// The rich axis title, if present.
    #[must_use]
    pub const fn axis_title(&self) -> Option<&AxisTitle> {
        self.axis_title.as_ref()
    }

    /// Mutable access to the rich axis title. Creates one if absent.
    pub fn axis_title_mut(&mut self) -> &mut AxisTitle {
        self.has_title = true;
        self.axis_title.get_or_insert_with(AxisTitle::new)
    }

    /// Set the rich axis title.
    pub fn set_axis_title(&mut self, axis_title: AxisTitle) {
        self.has_title = true;
        // Sync the plain-text field for backward compatibility
        self.title = axis_title.text();
        self.axis_title = Some(axis_title);
    }

    /// Whether the axis is visible.
    #[must_use]
    pub const fn visible(&self) -> bool {
        self.visible
    }

    /// Set axis visibility.
    pub fn set_visible(&mut self, value: bool) {
        self.visible = value;
    }

    /// The major tick mark style.
    #[must_use]
    pub const fn major_tick_mark(&self) -> XlTickMark {
        self.major_tick_mark
    }

    /// Set the major tick mark style.
    pub fn set_major_tick_mark(&mut self, value: XlTickMark) {
        self.major_tick_mark = value;
    }

    /// The minor tick mark style.
    #[must_use]
    pub const fn minor_tick_mark(&self) -> XlTickMark {
        self.minor_tick_mark
    }

    /// Set the minor tick mark style.
    pub fn set_minor_tick_mark(&mut self, value: XlTickMark) {
        self.minor_tick_mark = value;
    }

    /// The tick label position.
    #[must_use]
    pub const fn tick_label_position(&self) -> XlTickLabelPosition {
        self.tick_label_position
    }

    /// Set the tick label position.
    pub fn set_tick_label_position(&mut self, value: XlTickLabelPosition) {
        self.tick_label_position = value;
    }

    /// Whether the axis has major gridlines.
    #[must_use]
    pub const fn has_major_gridlines(&self) -> bool {
        self.has_major_gridlines
    }

    /// Set whether the axis has major gridlines.
    pub fn set_has_major_gridlines(&mut self, value: bool) {
        self.has_major_gridlines = value;
    }

    /// Whether the axis has minor gridlines.
    #[must_use]
    pub const fn has_minor_gridlines(&self) -> bool {
        self.has_minor_gridlines
    }

    /// Set whether the axis has minor gridlines.
    pub fn set_has_minor_gridlines(&mut self, value: bool) {
        self.has_minor_gridlines = value;
    }

    /// The axis crosses setting.
    #[must_use]
    pub const fn crosses(&self) -> XlAxisCrosses {
        self.crosses
    }

    /// Set the axis crosses setting.
    pub fn set_crosses(&mut self, value: XlAxisCrosses) {
        self.crosses = value;
    }

    /// The category type.
    #[must_use]
    pub const fn category_type(&self) -> XlCategoryType {
        self.category_type
    }

    /// The number format.
    #[must_use]
    pub fn number_format(&self) -> &str {
        &self.number_format
    }

    /// Set the number format.
    pub fn set_number_format(&mut self, format: &str) {
        self.number_format = format.to_string();
    }

    /// Whether the axis order is reversed.
    #[must_use]
    pub const fn reverse_order(&self) -> bool {
        self.reverse_order
    }

    /// Set whether the axis order is reversed.
    pub fn set_reverse_order(&mut self, value: bool) {
        self.reverse_order = value;
    }

    /// The tick label formatting properties.
    #[must_use]
    pub const fn tick_labels(&self) -> Option<&TickLabels> {
        self.tick_labels.as_ref()
    }

    /// Get or create mutable tick labels.
    pub fn tick_labels_mut(&mut self) -> &mut TickLabels {
        self.tick_labels.get_or_insert_with(TickLabels::new)
    }

    /// Set the tick labels.
    pub fn set_tick_labels(&mut self, tick_labels: TickLabels) {
        self.tick_labels = Some(tick_labels);
    }

    /// The axis line format (fill + line).
    #[must_use]
    pub const fn format(&self) -> Option<&ChartFormat> {
        self.format.as_ref()
    }

    /// Get or create a mutable axis format.
    pub fn format_mut(&mut self) -> &mut ChartFormat {
        self.format.get_or_insert_with(ChartFormat::new)
    }

    /// Set the axis line format.
    pub fn set_format(&mut self, format: ChartFormat) {
        self.format = Some(format);
    }

    /// The major gridline format.
    #[must_use]
    pub const fn major_gridline_format(&self) -> Option<&ChartFormat> {
        self.major_gridline_format.as_ref()
    }

    /// Get or create a mutable major gridline format.
    pub fn major_gridline_format_mut(&mut self) -> &mut ChartFormat {
        self.major_gridline_format
            .get_or_insert_with(ChartFormat::new)
    }

    /// The minor gridline format.
    #[must_use]
    pub const fn minor_gridline_format(&self) -> Option<&ChartFormat> {
        self.minor_gridline_format.as_ref()
    }

    /// Get or create a mutable minor gridline format.
    pub fn minor_gridline_format_mut(&mut self) -> &mut ChartFormat {
        self.minor_gridline_format
            .get_or_insert_with(ChartFormat::new)
    }
}

/// Creates a category axis with no title and default tick/grid settings.
impl Default for CategoryAxis {
    fn default() -> Self {
        Self::new()
    }
}
