//! Series types and collections for charts.

use crate::dml::fill::FillFormat;
use crate::dml::line::LineFormat;
use crate::enums::chart::XlChartType;

use super::datalabel::DataLabels;
use super::marker::Marker;

/// A collection of series in a chart.
#[derive(Debug, Clone, Default)]
pub struct SeriesCollection {
    series: Vec<Series>,
}

impl SeriesCollection {
    /// Create a new empty series collection.
    #[must_use]
    pub const fn new() -> Self {
        Self { series: Vec::new() }
    }

    /// Add a series to the collection.
    pub fn add(&mut self, series: Series) {
        self.series.push(series);
    }

    /// Get the series at the given index.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&Series> {
        self.series.get(index)
    }

    /// Get a mutable reference to the series at the given index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Series> {
        self.series.get_mut(index)
    }

    /// The number of series in the collection.
    #[must_use]
    pub fn len(&self) -> usize {
        self.series.len()
    }

    /// Whether the collection is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.series.is_empty()
    }

    /// Iterate over the series.
    pub fn iter(&self) -> std::slice::Iter<'_, Series> {
        self.series.iter()
    }

    /// Mutable iteration over the series.
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Series> {
        self.series.iter_mut()
    }
}

impl<'a> IntoIterator for &'a SeriesCollection {
    type Item = &'a Series;
    type IntoIter = std::slice::Iter<'a, Series>;

    fn into_iter(self) -> Self::IntoIter {
        self.series.iter()
    }
}

impl<'a> IntoIterator for &'a mut SeriesCollection {
    type Item = &'a mut Series;
    type IntoIter = std::slice::IterMut<'a, Series>;

    fn into_iter(self) -> Self::IntoIter {
        self.series.iter_mut()
    }
}

/// Format properties for a series (fill and line).
#[derive(Debug, Clone)]
pub struct SeriesFormat {
    /// Fill format for the series (bars, area, etc).
    pub fill: Option<FillFormat>,
    /// Line format for the series outline / line.
    pub line: Option<LineFormat>,
}

impl SeriesFormat {
    /// Create an empty format (all inherited).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            fill: None,
            line: None,
        }
    }
}

/// Creates an empty series format with no fill, line, or marker.
impl Default for SeriesFormat {
    fn default() -> Self {
        Self::new()
    }
}

/// A single data point with optional individual formatting.
#[derive(Debug, Clone)]
pub struct Point {
    index: usize,
    format: Option<SeriesFormat>,
}

impl Point {
    /// Create a point for the given data-point index.
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self {
            index,
            format: None,
        }
    }

    /// The zero-based index of this data point.
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// The format for this point.
    #[must_use]
    pub const fn format(&self) -> Option<&SeriesFormat> {
        self.format.as_ref()
    }

    /// Set format for this point.
    pub fn set_format(&mut self, format: SeriesFormat) {
        self.format = Some(format);
    }
}

/// A single data series in a chart.
///
/// The specific series behavior varies by chart type. For example, a bar
/// series supports gap width while a line series supports markers.
#[derive(Debug, Clone)]
pub struct Series {
    name: String,
    index: usize,
    chart_type: XlChartType,
    marker: Option<Marker>,
    data_labels: Option<DataLabels>,
    smooth: bool,
    invert_if_negative: bool,
    format: Option<SeriesFormat>,
    values: Vec<Option<f64>>,
    points: Vec<Point>,
}

impl Series {
    /// Create a new series.
    #[must_use]
    pub fn new(name: &str, index: usize, chart_type: XlChartType) -> Self {
        Self {
            name: name.to_string(),
            index,
            chart_type,
            marker: None,
            data_labels: None,
            smooth: false,
            invert_if_negative: false,
            format: None,
            values: Vec::new(),
            points: Vec::new(),
        }
    }

    /// The series name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The zero-based series index.
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// The chart type of this series.
    #[must_use]
    pub const fn chart_type(&self) -> XlChartType {
        self.chart_type
    }

    /// The marker for this series, if set.
    #[must_use]
    pub const fn marker(&self) -> Option<&Marker> {
        self.marker.as_ref()
    }

    /// Set the marker for this series.
    pub fn set_marker(&mut self, marker: Marker) {
        self.marker = Some(marker);
    }

    /// The data labels for this series, if set.
    #[must_use]
    pub const fn data_labels(&self) -> Option<&DataLabels> {
        self.data_labels.as_ref()
    }

    /// Set the data labels for this series.
    pub fn set_data_labels(&mut self, data_labels: DataLabels) {
        self.data_labels = Some(data_labels);
    }

    /// Whether lines are smoothed (for line/scatter charts).
    #[must_use]
    pub const fn smooth(&self) -> bool {
        self.smooth
    }

    /// Set line smoothing.
    pub fn set_smooth(&mut self, value: bool) {
        self.smooth = value;
    }

    /// Whether negative values are inverted.
    #[must_use]
    pub const fn invert_if_negative(&self) -> bool {
        self.invert_if_negative
    }

    /// Set invert-if-negative behavior.
    pub fn set_invert_if_negative(&mut self, value: bool) {
        self.invert_if_negative = value;
    }

    /// The format (fill + line) for this series.
    #[must_use]
    pub const fn format(&self) -> Option<&SeriesFormat> {
        self.format.as_ref()
    }

    /// Mutable access to the format. Creates a default if `None`.
    pub fn format_mut(&mut self) -> &mut SeriesFormat {
        self.format.get_or_insert_with(SeriesFormat::new)
    }

    /// Set the format for this series.
    pub fn set_format(&mut self, format: SeriesFormat) {
        self.format = Some(format);
    }

    /// Read-access to the series data values.
    #[must_use]
    pub fn values(&self) -> &[Option<f64>] {
        &self.values
    }

    /// Set the series data values.
    pub fn set_values(&mut self, values: Vec<Option<f64>>) {
        self.values = values;
    }

    /// Per-data-point access.
    #[must_use]
    pub fn points(&self) -> &[Point] {
        &self.points
    }

    /// Mutable per-data-point access.
    pub fn points_mut(&mut self) -> &mut Vec<Point> {
        &mut self.points
    }

    /// Add a point override at the given index.
    pub fn add_point(&mut self, point: Point) {
        self.points.push(point);
    }
}

#[cfg(test)]
#[path = "series_tests.rs"]
mod tests;
