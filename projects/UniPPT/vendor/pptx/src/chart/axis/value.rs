//! Value (numeric) axis type.

use crate::enums::chart::{XlAxisCrosses, XlTickLabelPosition, XlTickMark};

use super::super::chart::ChartFormat;
use super::{AxisTitle, TickLabels};

/// A value (numeric) axis on a chart.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct ValueAxis {
    has_title: bool,
    title: Option<String>,
    axis_title: Option<AxisTitle>,
    visible: bool,
    major_tick_mark: XlTickMark,
    minor_tick_mark: XlTickMark,
    tick_label_position: XlTickLabelPosition,
    pub(super) has_major_gridlines: bool,
    pub(super) has_minor_gridlines: bool,
    crosses: XlAxisCrosses,
    pub(super) crosses_at: Option<f64>,
    number_format: String,
    number_format_is_linked: bool,
    pub(super) minimum_scale: Option<f64>,
    pub(super) maximum_scale: Option<f64>,
    pub(super) major_unit: Option<f64>,
    pub(super) minor_unit: Option<f64>,
    pub(super) reverse_order: bool,
    pub(super) tick_labels: Option<TickLabels>,
    pub(super) format: Option<ChartFormat>,
    pub(super) major_gridline_format: Option<ChartFormat>,
    pub(super) minor_gridline_format: Option<ChartFormat>,
}

impl ValueAxis {
    /// Create a new value axis with default settings.
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
            has_major_gridlines: true,
            has_minor_gridlines: false,
            crosses: XlAxisCrosses::Automatic,
            crosses_at: None,
            number_format: "General".to_string(),
            number_format_is_linked: true,
            minimum_scale: None,
            maximum_scale: None,
            major_unit: None,
            minor_unit: None,
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

    /// The axis crosses setting.
    #[must_use]
    pub const fn crosses(&self) -> XlAxisCrosses {
        self.crosses
    }

    /// Set the axis crosses setting.
    pub fn set_crosses(&mut self, value: XlAxisCrosses) {
        self.crosses = value;
    }

    /// The number format string.
    #[must_use]
    pub fn number_format(&self) -> &str {
        &self.number_format
    }

    /// Set the number format string.
    pub fn set_number_format(&mut self, format: &str) {
        self.number_format = format.to_string();
    }

    /// Whether the number format is linked to source data.
    #[must_use]
    pub const fn number_format_is_linked(&self) -> bool {
        self.number_format_is_linked
    }

    /// Set whether the number format is linked to source data.
    pub fn set_number_format_is_linked(&mut self, value: bool) {
        self.number_format_is_linked = value;
    }
}

/// Creates a value axis with no title and default tick/grid settings.
impl Default for ValueAxis {
    fn default() -> Self {
        Self::new()
    }
}
