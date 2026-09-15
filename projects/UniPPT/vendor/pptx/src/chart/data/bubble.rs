//! Bubble chart data types.

use crate::enums::chart::XlChartType;
use crate::error::PptxResult;

use super::super::xmlwriter::ChartXmlWriter;

// ---------------------------------------------------------------------------
// BubbleChartData
// ---------------------------------------------------------------------------

/// Accumulates data for bubble charts.
///
/// # Example
/// ```
/// use pptx::chart::data::BubbleChartData;
///
/// let mut chart_data = BubbleChartData::new();
/// let mut series = chart_data.add_series("Dataset");
/// series.add_data_point(1.0, 2.5, 10.0);
/// series.add_data_point(3.0, 4.0, 20.0);
/// ```
#[derive(Debug, Clone)]
pub struct BubbleChartData {
    series: Vec<BubbleSeriesData>,
    number_format: String,
}

impl BubbleChartData {
    /// Create a new empty bubble chart data object.
    #[must_use]
    pub fn new() -> Self {
        Self {
            series: Vec::new(),
            number_format: "General".to_string(),
        }
    }

    /// Add a series and return a mutable reference to populate its data points.
    ///
    /// # Panics
    /// This method will not panic; the internal `unwrap()` is safe because
    /// an element was just pushed.
    pub fn add_series(&mut self, name: &str) -> &mut BubbleSeriesData {
        let index = self.series.len();
        self.series.push(BubbleSeriesData {
            name: name.to_string(),
            data_points: Vec::new(),
            number_format: None,
            index,
        });
        let idx = self.series.len() - 1;
        &mut self.series[idx]
    }

    /// Get the series data.
    #[must_use]
    pub fn series(&self) -> &[BubbleSeriesData] {
        &self.series
    }

    /// Get the number format.
    #[must_use]
    pub fn number_format(&self) -> &str {
        &self.number_format
    }

    /// Generate chart XML for the given chart type.
    ///
    /// # Errors
    /// Returns an error if the chart type is unsupported or XML generation fails.
    pub fn to_xml(&self, chart_type: XlChartType) -> PptxResult<String> {
        ChartXmlWriter::write_bubble(self, chart_type)
    }
}

/// Creates an empty bubble chart data set with no series.
impl Default for BubbleChartData {
    fn default() -> Self {
        Self::new()
    }
}

/// Data for a single series in a bubble chart.
#[derive(Debug, Clone)]
pub struct BubbleSeriesData {
    name: String,
    data_points: Vec<BubbleDataPoint>,
    number_format: Option<String>,
    index: usize,
}

impl BubbleSeriesData {
    /// Add a data point with X, Y, and bubble size values.
    pub fn add_data_point(&mut self, x: f64, y: f64, size: f64) {
        self.data_points.push(BubbleDataPoint { x, y, size });
    }

    /// The series name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The data points.
    #[must_use]
    pub fn data_points(&self) -> &[BubbleDataPoint] {
        &self.data_points
    }

    /// The X values.
    #[must_use]
    pub fn x_values(&self) -> Vec<f64> {
        self.data_points.iter().map(|dp| dp.x).collect()
    }

    /// The Y values.
    #[must_use]
    pub fn y_values(&self) -> Vec<f64> {
        self.data_points.iter().map(|dp| dp.y).collect()
    }

    /// The bubble sizes.
    #[must_use]
    pub fn bubble_sizes(&self) -> Vec<f64> {
        self.data_points.iter().map(|dp| dp.size).collect()
    }

    /// The zero-based index of this series.
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// The number format, or `None` to inherit from chart data.
    #[must_use]
    pub fn number_format(&self) -> Option<&str> {
        self.number_format.as_deref()
    }
}

/// A data point in a bubble chart series.
#[derive(Debug, Clone, Copy)]
pub struct BubbleDataPoint {
    /// The X value.
    pub x: f64,
    /// The Y value.
    pub y: f64,
    /// The bubble size.
    pub size: f64,
}
