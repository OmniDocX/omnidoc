//! Extended value axis methods (gridlines, scale, format).

use super::super::chart::ChartFormat;
use super::value::ValueAxis;
use super::TickLabels;

impl ValueAxis {
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

    /// The value at which the other axis crosses this axis, or `None` for automatic.
    #[must_use]
    pub const fn crosses_at(&self) -> Option<f64> {
        self.crosses_at
    }

    /// Set the value at which the other axis crosses this axis.
    pub fn set_crosses_at(&mut self, value: Option<f64>) {
        self.crosses_at = value;
    }

    /// The minimum scale value, or `None` for automatic.
    #[must_use]
    pub const fn minimum_scale(&self) -> Option<f64> {
        self.minimum_scale
    }

    /// Set the minimum scale value.
    pub fn set_minimum_scale(&mut self, value: Option<f64>) {
        self.minimum_scale = value;
    }

    /// The maximum scale value, or `None` for automatic.
    #[must_use]
    pub const fn maximum_scale(&self) -> Option<f64> {
        self.maximum_scale
    }

    /// Set the maximum scale value.
    pub fn set_maximum_scale(&mut self, value: Option<f64>) {
        self.maximum_scale = value;
    }

    /// The major unit (tick interval) for the value axis, or `None` for automatic.
    #[must_use]
    pub const fn major_unit(&self) -> Option<f64> {
        self.major_unit
    }

    /// Set the major unit.
    pub fn set_major_unit(&mut self, value: Option<f64>) {
        self.major_unit = value;
    }

    /// The minor unit (tick interval) for the value axis, or `None` for automatic.
    #[must_use]
    pub const fn minor_unit(&self) -> Option<f64> {
        self.minor_unit
    }

    /// Set the minor unit.
    pub fn set_minor_unit(&mut self, value: Option<f64>) {
        self.minor_unit = value;
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
