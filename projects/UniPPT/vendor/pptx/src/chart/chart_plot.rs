//! Plot (chart group) within a chart.

use crate::enums::chart::XlChartType;

use super::datalabel::DataLabels;
use super::plot::PlotProperties;
use super::series::SeriesCollection;

/// A single plot (chart group) within a chart.
///
/// A plot represents one chart type element (e.g. `<c:barChart>`) in the
/// plot area.  A chart may contain multiple plots to create combo charts.
#[derive(Debug, Clone)]
pub struct Plot {
    /// The chart type for this plot.
    pub chart_type: XlChartType,
    /// The series belonging to this plot.
    pub series: SeriesCollection,
    /// Plot-level properties (`gap_width`, overlap, etc.).
    pub plot_properties: PlotProperties,
    /// Data labels for the entire plot (applied to all series unless overridden).
    pub data_labels: Option<DataLabels>,
    /// Whether the plot has data labels enabled.
    pub has_data_labels: bool,
    /// Category labels shared by all series in the plot.
    pub categories: Option<Vec<String>>,
}

impl Plot {
    /// Create a new plot for the given chart type.
    #[must_use]
    pub const fn new(chart_type: XlChartType) -> Self {
        Self {
            chart_type,
            series: SeriesCollection::new(),
            plot_properties: PlotProperties::new(),
            data_labels: None,
            has_data_labels: false,
            categories: None,
        }
    }

    /// The series collection for this plot.
    #[must_use]
    pub const fn series(&self) -> &SeriesCollection {
        &self.series
    }

    /// Mutable access to the series collection.
    pub fn series_mut(&mut self) -> &mut SeriesCollection {
        &mut self.series
    }

    /// The plot-level properties.
    #[must_use]
    pub const fn plot_properties(&self) -> &PlotProperties {
        &self.plot_properties
    }

    /// Mutable access to plot properties.
    pub fn plot_properties_mut(&mut self) -> &mut PlotProperties {
        &mut self.plot_properties
    }

    /// The data labels for this plot, if set.
    #[must_use]
    pub const fn data_labels(&self) -> Option<&DataLabels> {
        self.data_labels.as_ref()
    }

    /// Mutable access to the data labels. Creates a default if None.
    pub fn data_labels_mut(&mut self) -> &mut DataLabels {
        self.has_data_labels = true;
        self.data_labels.get_or_insert_with(DataLabels::new)
    }

    /// Set the data labels for this plot.
    pub fn set_data_labels(&mut self, data_labels: DataLabels) {
        self.has_data_labels = true;
        self.data_labels = Some(data_labels);
    }
}
