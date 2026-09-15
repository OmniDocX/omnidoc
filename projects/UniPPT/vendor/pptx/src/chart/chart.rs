//! Chart struct representing a chart object on a slide.

use crate::enums::chart::{XlChartType, XlLegendPosition};
use crate::error::PptxResult;
use crate::text::font::Font;

use super::axis::{CategoryAxis, ValueAxis};
// Re-export ChartFormat and ChartTitle so existing `super::chart::ChartFormat` paths still work.
pub use super::chart_format::{ChartFormat, ChartTitle};
// Re-export Plot so existing `chart::chart::Plot` paths still work.
pub use super::chart_plot::Plot;
use super::data::CategoryChartData;
use super::legend::Legend;
use super::plot::PlotProperties;
use super::series::SeriesCollection;
use super::xmlwriter::ChartXmlWriter;

/// Represents a chart embedded in a slide.
///
/// A `Chart` holds metadata about the chart type and layout, along with
/// references to axes, legend, and series data.  The actual chart XML
/// is generated separately via `ChartXmlWriter`.
#[derive(Debug, Clone)]
pub struct Chart {
    chart_type: XlChartType,
    has_legend: bool,
    has_title: bool,
    title: Option<String>,
    title_obj: Option<ChartTitle>,
    style: Option<u32>,
    legend: Option<Legend>,
    category_axis: Option<CategoryAxis>,
    value_axis: Option<ValueAxis>,
    plots: Vec<Plot>,
    format: Option<ChartFormat>,
    /// Default font for the chart (emitted as `<c:txPr>` in chart XML).
    font: Option<Font>,
}

impl Chart {
    /// Create a new chart of the specified type.
    #[must_use]
    pub fn new(chart_type: XlChartType) -> Self {
        let has_axes = !chart_type.is_pie_type()
            && !chart_type.is_doughnut_type()
            && !chart_type.is_surface_type();
        let default_plot = Plot::new(chart_type);
        Self {
            chart_type,
            has_legend: false,
            has_title: false,
            title: None,
            title_obj: None,
            style: None,
            legend: None,
            category_axis: if has_axes {
                Some(CategoryAxis::new())
            } else {
                None
            },
            value_axis: if has_axes {
                Some(ValueAxis::new())
            } else {
                None
            },
            plots: vec![default_plot],
            format: None,
            font: None,
        }
    }

    /// The chart type.
    #[must_use]
    pub const fn chart_type(&self) -> XlChartType {
        self.chart_type
    }

    /// Whether the chart has a legend.
    #[must_use]
    pub const fn has_legend(&self) -> bool {
        self.has_legend
    }

    /// Set whether the chart has a legend.
    pub fn set_has_legend(&mut self, value: bool) {
        self.has_legend = value;
        if value && self.legend.is_none() {
            self.legend = Some(Legend::new());
        }
        if !value {
            self.legend = None;
        }
    }

    /// Whether the chart has a title.
    #[must_use]
    pub const fn has_title(&self) -> bool {
        self.has_title
    }

    /// Set whether the chart has a title.
    pub fn set_has_title(&mut self, value: bool) {
        self.has_title = value;
        if !value {
            self.title = None;
            self.title_obj = None;
        }
    }

    /// The chart title text (backward-compatible plain text accessor).
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Set the chart title text. Also sets `has_title` to true.
    pub fn set_title(&mut self, title: &str) {
        self.has_title = true;
        self.title = Some(title.to_string());
        self.title_obj = Some(ChartTitle::from_text(title));
    }

    /// The rich chart title, if present.
    #[must_use]
    pub const fn chart_title(&self) -> Option<&ChartTitle> {
        self.title_obj.as_ref()
    }

    /// Mutable access to the rich chart title. Creates one if absent.
    pub fn chart_title_mut(&mut self) -> &mut ChartTitle {
        self.has_title = true;
        self.title_obj.get_or_insert_with(ChartTitle::new)
    }

    /// Set the rich chart title.
    pub fn set_chart_title(&mut self, chart_title: ChartTitle) {
        self.has_title = true;
        self.title = chart_title.text();
        self.title_obj = Some(chart_title);
    }

    /// The chart style index (1-48), or `None` for default.
    #[must_use]
    pub const fn chart_style(&self) -> Option<u32> {
        self.style
    }

    /// Set the chart style index.
    pub fn set_chart_style(&mut self, style: Option<u32>) {
        self.style = style;
    }

    /// The legend, if present.
    #[must_use]
    pub const fn legend(&self) -> Option<&Legend> {
        self.legend.as_ref()
    }

    /// Mutable access to the legend, if present.
    pub fn legend_mut(&mut self) -> Option<&mut Legend> {
        self.legend.as_mut()
    }

    /// Set the legend position. Creates a legend if one doesn't exist.
    pub fn set_legend_position(&mut self, position: XlLegendPosition) {
        self.has_legend = true;
        if let Some(legend) = &mut self.legend {
            legend.set_position(position);
        } else {
            let mut legend = Legend::new();
            legend.set_position(position);
            self.legend = Some(legend);
        }
    }

    /// The category axis, if present.
    #[must_use]
    pub const fn category_axis(&self) -> Option<&CategoryAxis> {
        self.category_axis.as_ref()
    }

    /// Mutable access to the category axis.
    pub fn category_axis_mut(&mut self) -> Option<&mut CategoryAxis> {
        self.category_axis.as_mut()
    }

    /// The value axis, if present.
    #[must_use]
    pub const fn value_axis(&self) -> Option<&ValueAxis> {
        self.value_axis.as_ref()
    }

    /// Mutable access to the value axis.
    pub fn value_axis_mut(&mut self) -> Option<&mut ValueAxis> {
        self.value_axis.as_mut()
    }

    /// The plots (chart groups) in this chart.
    #[must_use]
    pub fn plots(&self) -> &[Plot] {
        &self.plots
    }

    /// Mutable access to the plots vector.
    pub fn plots_mut(&mut self) -> &mut Vec<Plot> {
        &mut self.plots
    }

    /// Add a plot to the chart (for combo charts).
    pub fn add_plot(&mut self, plot: Plot) {
        self.plots.push(plot);
    }

    /// The series collection (delegates to the first plot).
    #[must_use]
    pub fn series(&self) -> &SeriesCollection {
        &self.plots[0].series
    }

    /// Mutable access to the series collection (delegates to the first plot).
    pub fn series_mut(&mut self) -> &mut SeriesCollection {
        &mut self.plots[0].series
    }

    /// The plot-level properties (delegates to the first plot).
    #[must_use]
    pub fn plot_properties(&self) -> &PlotProperties {
        &self.plots[0].plot_properties
    }

    /// Mutable access to plot properties (delegates to the first plot).
    pub fn plot_properties_mut(&mut self) -> &mut PlotProperties {
        &mut self.plots[0].plot_properties
    }

    /// The chart-level format (fill + line for the chart area).
    #[must_use]
    pub const fn chart_format(&self) -> Option<&ChartFormat> {
        self.format.as_ref()
    }

    /// Mutable access to the chart format. Creates a default if `None`.
    pub fn chart_format_mut(&mut self) -> &mut ChartFormat {
        self.format.get_or_insert_with(ChartFormat::new)
    }

    /// Set the chart format.
    pub fn set_chart_format(&mut self, format: ChartFormat) {
        self.format = Some(format);
    }

    /// The default font for the chart, if set.
    #[must_use]
    pub const fn font(&self) -> Option<&Font> {
        self.font.as_ref()
    }

    /// Mutable access to the default chart font. Creates a default if `None`.
    pub fn font_mut(&mut self) -> &mut Font {
        self.font.get_or_insert_with(Font::new)
    }

    /// Set the default font for the chart.
    pub fn set_font(&mut self, font: Font) {
        self.font = Some(font);
    }

    /// Replace the chart data and regenerate the chart XML.
    ///
    /// # Errors
    /// Returns an error if the chart type is unsupported or XML generation fails.
    pub fn replace_data(&self, chart_data: &CategoryChartData) -> PptxResult<String> {
        ChartXmlWriter::write_category(chart_data, self.chart_type)
    }
}

#[cfg(test)]
#[path = "chart_tests.rs"]
mod tests;
