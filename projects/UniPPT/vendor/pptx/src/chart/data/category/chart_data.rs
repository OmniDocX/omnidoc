//! Core category chart data types: `CategoryChartData` and `CategorySeriesData`.

use crate::enums::chart::XlChartType;
use crate::error::PptxResult;

use super::super::super::xmlwriter::ChartXmlWriter;
use super::super::{Categories, Category, CategoryLevel};

// ---------------------------------------------------------------------------
// CategoryChartData
// ---------------------------------------------------------------------------

/// Accumulates data for category-based charts (bar, column, line, pie, area,
/// doughnut, radar).
///
/// # Example
/// ```
/// use pptx::chart::data::CategoryChartData;
///
/// let mut chart_data = CategoryChartData::new();
/// chart_data.add_category("Q1");
/// chart_data.add_category("Q2");
/// chart_data.add_series("Sales", &[100.0, 150.0]);
/// chart_data.add_series("Expenses", &[80.0, 120.0]);
/// ```
#[derive(Debug, Clone)]
pub struct CategoryChartData {
    categories: Vec<String>,
    series: Vec<CategorySeriesData>,
    number_format: String,
    hierarchical_categories: Option<Vec<Vec<String>>>,
}

impl CategoryChartData {
    /// Create a new empty chart data object.
    #[must_use]
    pub fn new() -> Self {
        Self {
            categories: Vec::new(),
            series: Vec::new(),
            number_format: "General".to_string(),
            hierarchical_categories: None,
        }
    }

    /// Create with a custom number format.
    #[must_use]
    pub fn with_number_format(number_format: &str) -> Self {
        Self {
            categories: Vec::new(),
            series: Vec::new(),
            number_format: number_format.to_string(),
            hierarchical_categories: None,
        }
    }

    /// Add a category label.
    pub fn add_category(&mut self, label: &str) {
        self.categories.push(label.to_string());
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

    /// Add a series with optional values (None represents missing data points).
    pub fn add_series_with_options(&mut self, name: &str, values: &[Option<f64>]) {
        let index = self.series.len();
        self.series.push(CategorySeriesData {
            name: name.to_string(),
            values: values.to_vec(),
            number_format: None,
            index,
        });
    }

    /// Get the categories.
    #[must_use]
    pub fn categories(&self) -> &[String] {
        &self.categories
    }

    /// Get the series data.
    #[must_use]
    pub fn series(&self) -> &[CategorySeriesData] {
        &self.series
    }

    /// Get the number format.
    #[must_use]
    pub fn number_format(&self) -> &str {
        &self.number_format
    }

    /// Set hierarchical (multi-level) categories.
    ///
    /// Each inner `Vec<String>` represents one level of the hierarchy,
    /// from the deepest (leaf) level at index 0 to the topmost (root)
    /// level at the last index.
    ///
    /// # Example
    /// ```
    /// use pptx::chart::data::CategoryChartData;
    ///
    /// let mut data = CategoryChartData::new();
    /// // Level 0 (leaf): Q1, Q2, Q3, Q4
    /// // Level 1 (root): H1, H1, H2, H2
    /// data.set_hierarchical_categories(vec![
    ///     vec!["Q1".into(), "Q2".into(), "Q3".into(), "Q4".into()],
    ///     vec!["H1".into(), "H1".into(), "H2".into(), "H2".into()],
    /// ]);
    /// ```
    pub fn set_hierarchical_categories(&mut self, levels: Vec<Vec<String>>) {
        // Also set the leaf level as main categories for backwards compatibility
        if let Some(leaf) = levels.first() {
            self.categories.clone_from(leaf);
        }
        self.hierarchical_categories = Some(levels);
    }

    /// Get the hierarchical categories, if set.
    #[must_use]
    pub const fn hierarchical_categories(&self) -> Option<&Vec<Vec<String>>> {
        self.hierarchical_categories.as_ref()
    }

    /// Returns the number of hierarchy levels (1 for flat, >1 for multi-level).
    #[must_use]
    pub fn category_depth(&self) -> usize {
        match &self.hierarchical_categories {
            Some(levels) if levels.len() > 1 => levels.len(),
            _ => 1,
        }
    }

    /// Build a structured `Categories` object from this chart data.
    ///
    /// If hierarchical categories have been set, the returned `Categories` will
    /// have multiple levels. Otherwise it contains a single flat level built
    /// from the plain category labels.
    #[must_use]
    pub fn categories_object(&self) -> Categories {
        match &self.hierarchical_categories {
            Some(levels) if levels.len() > 1 => {
                let cat_levels = levels
                    .iter()
                    .map(|level_labels| CategoryLevel {
                        categories: level_labels
                            .iter()
                            .enumerate()
                            .map(|(idx, label)| Category {
                                idx,
                                label: label.clone(),
                                children: Vec::new(),
                            })
                            .collect(),
                    })
                    .collect();
                Categories { levels: cat_levels }
            }
            _ => {
                let level = CategoryLevel {
                    categories: self
                        .categories
                        .iter()
                        .enumerate()
                        .map(|(idx, label)| Category {
                            idx,
                            label: label.clone(),
                            children: Vec::new(),
                        })
                        .collect(),
                };
                Categories {
                    levels: vec![level],
                }
            }
        }
    }

    /// Generate chart XML for the given chart type.
    ///
    /// # Errors
    /// Returns an error if the chart type is unsupported or XML generation fails.
    pub fn to_xml(&self, chart_type: XlChartType) -> PptxResult<String> {
        ChartXmlWriter::write_category(self, chart_type)
    }
}

/// Creates an empty category chart data set with no categories or series.
impl Default for CategoryChartData {
    fn default() -> Self {
        Self::new()
    }
}

/// Data for a single series in a category chart.
#[derive(Debug, Clone)]
pub struct CategorySeriesData {
    pub(crate) name: String,
    pub(crate) values: Vec<Option<f64>>,
    pub(crate) number_format: Option<String>,
    pub(crate) index: usize,
}

impl CategorySeriesData {
    /// The series name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The series values (`None` for missing data points).
    #[must_use]
    pub fn values(&self) -> &[Option<f64>] {
        &self.values
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
