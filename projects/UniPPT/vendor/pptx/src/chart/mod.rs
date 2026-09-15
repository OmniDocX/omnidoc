//! Chart types and XML generation for `PowerPoint` charts.
//!
//! Charts in OOXML are stored as separate part files
//! (`ppt/charts/chart1.xml`) with a relationship from the slide's
//! `GraphicFrame` element.
//!
//! # Example
//!
//! ```rust
//! use pptx::chart::data::CategoryChartData;
//! use pptx::enums::chart::XlChartType;
//!
//! let mut chart_data = CategoryChartData::new();
//! chart_data.add_category("Q1");
//! chart_data.add_category("Q2");
//! chart_data.add_series("Sales", &[100.0, 150.0]);
//! chart_data.add_series("Expenses", &[80.0, 120.0]);
//!
//! let chart_xml = chart_data.to_xml(XlChartType::ColumnClustered).unwrap();
//! assert!(chart_xml.contains("<c:barChart>"));
//! ```

pub mod axis;
// Allow module_inception: `chart::chart` mirrors the python-pptx structure
// (`pptx.chart.chart.Chart`) for familiarity.
#[allow(clippy::module_inception)]
pub mod chart;
pub mod chart_format;
pub mod chart_plot;
pub mod data;
pub mod datalabel;
pub mod legend;
pub mod marker;
pub mod plot;
pub mod series;
pub mod xlsx;
pub mod xmlwriter;

// Re-exports for convenience
pub use axis::{AxisTitle, CategoryAxis, DateAxis, TickLabels, ValueAxis};
pub use chart::{Chart, ChartFormat, ChartTitle, Plot};
pub use data::{
    BubbleChartData, Categories, Category, CategoryChartData, CategoryLevel, ComboChartData,
    ComboSeriesData, ComboSeriesType, DateAxisChartData, XyChartData,
};
pub use datalabel::{DataLabel, DataLabels};
pub use legend::{Legend, LegendEntry};
pub use marker::{Marker, MarkerFormat};
pub use plot::PlotProperties;
pub use series::{Point, Series, SeriesCollection, SeriesFormat};
pub use xmlwriter::ChartXmlWriter;
