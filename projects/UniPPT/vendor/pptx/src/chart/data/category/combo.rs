//! Combo chart data types: `ComboSeriesType`, `ComboSeriesData`, and `ComboChartData`.

use crate::error::PptxResult;

use super::super::super::xmlwriter::ChartXmlWriter;
use super::chart_data::CategorySeriesData;

// ---------------------------------------------------------------------------
// ComboChartData
// ---------------------------------------------------------------------------

/// Specifies whether a series in a combo chart is rendered as a bar/column
/// or as a line.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComboSeriesType {
    /// Rendered as a bar/column in the primary value axis.
    Bar,
    /// Rendered as a line on the secondary value axis.
    Line,
}

/// A single series in a combo chart, tagged with its sub-chart type.
#[derive(Debug, Clone)]
pub struct ComboSeriesData {
    pub(crate) inner: CategorySeriesData,
    pub(crate) combo_type: ComboSeriesType,
}

impl ComboSeriesData {
    /// The series name.
    #[must_use]
    pub fn name(&self) -> &str {
        self.inner.name()
    }

    /// The series values.
    #[must_use]
    pub fn values(&self) -> &[Option<f64>] {
        self.inner.values()
    }

    /// The zero-based index of this series.
    #[must_use]
    pub const fn index(&self) -> usize {
        self.inner.index
    }

    /// The number format, or `None` to inherit.
    #[must_use]
    pub fn number_format(&self) -> Option<&str> {
        self.inner.number_format()
    }

    /// Which sub-chart type this series belongs to.
    #[must_use]
    pub const fn combo_type(&self) -> ComboSeriesType {
        self.combo_type
    }

    /// Access as a plain `CategorySeriesData`.
    #[must_use]
    pub const fn as_category_series(&self) -> &CategorySeriesData {
        &self.inner
    }
}

/// Accumulates data for a combo chart (bar/column + line in one chart).
///
/// Each series is tagged as either `Bar` or `Line`. Bar series are rendered
/// in a `<c:barChart>` element on the primary value axis; line series are
/// rendered in a `<c:lineChart>` element on a secondary value axis.
///
/// # Example
/// ```
/// use pptx::chart::data::{ComboChartData, ComboSeriesType};
///
/// let mut data = ComboChartData::new();
/// data.add_category("Q1");
/// data.add_category("Q2");
/// data.add_series("Revenue", ComboSeriesType::Bar, &[100.0, 150.0]);
/// data.add_series("Growth %", ComboSeriesType::Line, &[10.0, 15.0]);
///
/// let xml = data.to_xml().unwrap();
/// assert!(xml.contains("<c:barChart>"));
/// assert!(xml.contains("<c:lineChart>"));
/// ```
#[derive(Debug, Clone)]
pub struct ComboChartData {
    categories: Vec<String>,
    series: Vec<ComboSeriesData>,
    number_format: String,
}

impl ComboChartData {
    /// Create a new empty combo chart data object.
    #[must_use]
    pub fn new() -> Self {
        Self {
            categories: Vec::new(),
            series: Vec::new(),
            number_format: "General".to_string(),
        }
    }

    /// Add a category label.
    pub fn add_category(&mut self, label: &str) {
        self.categories.push(label.to_string());
    }

    /// Add a series tagged with its sub-chart type.
    pub fn add_series(&mut self, name: &str, combo_type: ComboSeriesType, values: &[f64]) {
        let values = values.iter().map(|v| Some(*v)).collect();
        let index = self.series.len();
        self.series.push(ComboSeriesData {
            inner: CategorySeriesData {
                name: name.to_string(),
                values,
                number_format: None,
                index,
            },
            combo_type,
        });
    }

    /// Add a series with optional values (None for missing data points).
    pub fn add_series_with_options(
        &mut self,
        name: &str,
        combo_type: ComboSeriesType,
        values: &[Option<f64>],
    ) {
        let index = self.series.len();
        self.series.push(ComboSeriesData {
            inner: CategorySeriesData {
                name: name.to_string(),
                values: values.to_vec(),
                number_format: None,
                index,
            },
            combo_type,
        });
    }

    /// Get the category labels.
    #[must_use]
    pub fn categories(&self) -> &[String] {
        &self.categories
    }

    /// Get all series data.
    #[must_use]
    pub fn series(&self) -> &[ComboSeriesData] {
        &self.series
    }

    /// Get the number format.
    #[must_use]
    pub fn number_format(&self) -> &str {
        &self.number_format
    }

    /// Get only the bar/column series.
    #[must_use]
    pub fn bar_series(&self) -> Vec<&ComboSeriesData> {
        self.series
            .iter()
            .filter(|s| s.combo_type == ComboSeriesType::Bar)
            .collect()
    }

    /// Get only the line series.
    #[must_use]
    pub fn line_series(&self) -> Vec<&ComboSeriesData> {
        self.series
            .iter()
            .filter(|s| s.combo_type == ComboSeriesType::Line)
            .collect()
    }

    /// Generate the combo chart XML.
    ///
    /// # Errors
    /// Returns an error if XML generation fails.
    pub fn to_xml(&self) -> PptxResult<String> {
        ChartXmlWriter::write_combo(self)
    }
}

impl Default for ComboChartData {
    fn default() -> Self {
        Self::new()
    }
}
