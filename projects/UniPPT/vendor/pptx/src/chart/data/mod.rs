//! Chart data types for building chart content.
//!
//! These types accumulate category labels, series names, and data values
//! used to generate chart XML.

mod bubble;
mod category;
mod date_axis;
mod xy;

pub use bubble::{BubbleChartData, BubbleDataPoint, BubbleSeriesData};
pub use category::{
    CategoryChartData, CategorySeriesData, ComboChartData, ComboSeriesData, ComboSeriesType,
};
pub use date_axis::DateAxisChartData;
pub use xy::{XyChartData, XyDataPoint, XySeriesData};

// ---------------------------------------------------------------------------
// Categories / CategoryLevel / Category
// ---------------------------------------------------------------------------

/// Represents a single category with optional sub-categories (for hierarchical data).
#[derive(Debug, Clone)]
pub struct Category {
    /// The zero-based index of this category.
    pub idx: usize,
    /// The label text for this category.
    pub label: String,
    /// Child categories (populated for hierarchical category data).
    pub children: Vec<Category>,
}

/// Categories at a single level of a hierarchy.
#[derive(Debug, Clone)]
pub struct CategoryLevel {
    pub(crate) categories: Vec<Category>,
}

impl CategoryLevel {
    /// Get the categories at this level.
    #[must_use]
    pub fn categories(&self) -> &[Category] {
        &self.categories
    }

    /// Iterate over the categories at this level.
    pub fn iter(&self) -> std::slice::Iter<'_, Category> {
        self.categories.iter()
    }

    /// The number of categories at this level.
    #[must_use]
    pub fn len(&self) -> usize {
        self.categories.len()
    }

    /// Whether this level is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.categories.is_empty()
    }
}

impl<'a> IntoIterator for &'a CategoryLevel {
    type Item = &'a Category;
    type IntoIter = std::slice::Iter<'a, Category>;

    fn into_iter(self) -> Self::IntoIter {
        self.categories.iter()
    }
}

/// The full categories collection with hierarchical support.
///
/// For flat (single-level) categories, `depth()` returns 1 and there is one level.
/// For multi-level (hierarchical) categories, `depth()` returns the number of levels,
/// where level 0 is the leaf (most specific) level and the last level is the root.
#[derive(Debug, Clone)]
pub struct Categories {
    pub(crate) levels: Vec<CategoryLevel>,
}

impl Categories {
    /// The number of hierarchy levels (1 for flat categories, >1 for hierarchical).
    #[must_use]
    pub fn depth(&self) -> usize {
        self.levels.len()
    }

    /// Access levels directly. Level 0 is the leaf level, the last is the root.
    #[must_use]
    pub fn levels(&self) -> &[CategoryLevel] {
        &self.levels
    }

    /// Iterate over the top-level (leaf) categories.
    pub fn iter(&self) -> std::slice::Iter<'_, Category> {
        if self.levels.is_empty() {
            [].iter()
        } else {
            self.levels[0].categories.iter()
        }
    }

    /// Returns flattened labels as a vector of tuples (represented as `Vec<String>`).
    ///
    /// Each inner Vec contains one label per hierarchy level for a single data point,
    /// ordered from leaf (index 0) to root (last index). This matches the tuple
    /// sequence returned by python-pptx's `Categories.flattened_labels`.
    ///
    /// For flat categories, each inner Vec contains a single label.
    ///
    /// # Example
    /// ```
    /// use pptx::chart::data::CategoryChartData;
    ///
    /// let mut data = CategoryChartData::new();
    /// data.set_hierarchical_categories(vec![
    ///     vec!["Q1".into(), "Q2".into(), "Q3".into(), "Q4".into()],
    ///     vec!["H1".into(), "H1".into(), "H2".into(), "H2".into()],
    /// ]);
    /// let cats = data.categories_object();
    /// let labels = cats.flattened_labels();
    /// assert_eq!(labels[0], vec!["Q1", "H1"]);
    /// assert_eq!(labels[3], vec!["Q4", "H2"]);
    /// ```
    #[must_use]
    pub fn flattened_labels(&self) -> Vec<Vec<String>> {
        if self.levels.is_empty() {
            return Vec::new();
        }

        let count = self.levels[0].categories.len();
        let mut result = Vec::with_capacity(count);

        // Index-based loop is intentional: this is a transpose operation that
        // gathers the i-th element from each level into a single tuple. An
        // iterator-based approach (e.g. `levels.iter().map(|l| l[i])`) would
        // still need the outer index, so `0..count` is the clearest form.
        for i in 0..count {
            let mut tuple = Vec::with_capacity(self.levels.len());
            for level in &self.levels {
                let label = level
                    .categories
                    .get(i)
                    .map(|c| c.label.clone())
                    .unwrap_or_default();
                tuple.push(label);
            }
            result.push(tuple);
        }

        result
    }
}

impl<'a> IntoIterator for &'a Categories {
    type Item = &'a Category;
    type IntoIter = std::slice::Iter<'a, Category>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests;
