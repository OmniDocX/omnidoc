//! Date-axis chart data types.

use super::{CategoryChartData, CategorySeriesData};

// ---------------------------------------------------------------------------
// DateAxisChartData
// ---------------------------------------------------------------------------

/// Accumulates data for charts with a date axis, similar to `CategoryChartData`
/// but categories are date strings (e.g. `"2024-01-15"`, `"2024/02/20"`).
///
/// # Example
/// ```
/// use pptx::chart::data::DateAxisChartData;
///
/// let mut chart_data = DateAxisChartData::new();
/// chart_data.add_date("2024-01-01");
/// chart_data.add_date("2024-02-01");
/// chart_data.add_series("Revenue", &[100.0, 150.0]);
/// ```
#[derive(Debug, Clone)]
pub struct DateAxisChartData {
    dates: Vec<String>,
    series: Vec<CategorySeriesData>,
    number_format: String,
    date_format: String,
}

impl DateAxisChartData {
    /// Create a new empty date-axis chart data object.
    #[must_use]
    pub fn new() -> Self {
        Self {
            dates: Vec::new(),
            series: Vec::new(),
            number_format: "General".to_string(),
            date_format: "yyyy-mm-dd".to_string(),
        }
    }

    /// Create with a custom number format for the values.
    #[must_use]
    pub fn with_number_format(number_format: &str) -> Self {
        Self {
            dates: Vec::new(),
            series: Vec::new(),
            number_format: number_format.to_string(),
            date_format: "yyyy-mm-dd".to_string(),
        }
    }

    /// Add a date category label (string representation of a date).
    pub fn add_date(&mut self, date: &str) {
        self.dates.push(date.to_string());
    }

    /// Add a series with a name and values.
    pub fn add_series(&mut self, name: &str, values: &[f64]) {
        let values = values.iter().map(|v| Some(*v)).collect();
        let index = self.series.len();
        self.series.push(CategorySeriesData {
            name: name.to_string(),
            values,
            number_format: None,
            index,
        });
    }

    /// Add a series with optional values (`None` represents missing data points).
    pub fn add_series_with_options(&mut self, name: &str, values: &[Option<f64>]) {
        let index = self.series.len();
        self.series.push(CategorySeriesData {
            name: name.to_string(),
            values: values.to_vec(),
            number_format: None,
            index,
        });
    }

    /// Get the date strings.
    #[must_use]
    pub fn dates(&self) -> &[String] {
        &self.dates
    }

    /// Get the series data.
    #[must_use]
    pub fn series(&self) -> &[CategorySeriesData] {
        &self.series
    }

    /// Get the number format for values.
    #[must_use]
    pub fn number_format(&self) -> &str {
        &self.number_format
    }

    /// Get the date format string.
    #[must_use]
    pub fn date_format(&self) -> &str {
        &self.date_format
    }

    /// Set the date format string used for the date axis.
    pub fn set_date_format(&mut self, format: &str) {
        self.date_format = format.to_string();
    }

    /// Convert to a `CategoryChartData` using the dates as category labels.
    /// This allows reuse of existing category chart XML generation.
    #[must_use]
    pub fn to_category_chart_data(&self) -> CategoryChartData {
        let mut data = CategoryChartData::with_number_format(&self.number_format);
        for date in &self.dates {
            data.add_category(date);
        }
        for series in &self.series {
            data.add_series_with_options(series.name(), series.values());
        }
        data
    }
}

/// Creates an empty date-axis chart data set with no series.
impl Default for DateAxisChartData {
    fn default() -> Self {
        Self::new()
    }
}
