//! Chart XML generation.
//!
//! Generates complete chart XML documents (`ppt/charts/chart1.xml`) from
//! chart data objects. Each chart type has its own XML structure following
//! the OOXML `DrawingML` Chart specification.

/// Write to a String (infallible - `fmt::Write` for `String` never fails).
macro_rules! write_str {
    ($dst:expr, $($arg:tt)*) => {
        write!($dst, $($arg)*).unwrap_or_else(|_| unreachable!("fmt::Write for String is infallible"))
    };
}

mod category_area_radar;
mod category_bar_line;
mod category_pie;
mod category_tests;
mod category_tests_3d;
mod combo;
mod helpers;
mod stock;
mod surface;
mod xy_bubble;

use crate::enums::chart::XlChartType;
use crate::error::PptxResult;
use crate::xml_util::write_xml_escaped;

use super::data::CategoryChartData;

/// Generates chart XML from chart data.
pub struct ChartXmlWriter;

fn xml_escape_to(w: &mut String, s: &str) {
    write_xml_escaped(w, s).unwrap_or_else(|_| unreachable!("write to String should not fail"));
}

impl ChartXmlWriter {
    /// Generate chart XML for a category chart.
    ///
    /// # Errors
    /// Returns an error if the chart type is unsupported (e.g. combo or unknown type).
    pub fn write_category(data: &CategoryChartData, chart_type: XlChartType) -> PptxResult<String> {
        match chart_type {
            ct if ct.is_combo_type() => Err(crate::error::PptxError::InvalidXml(
                "combo charts require ComboChartData; use ComboChartData::to_xml() instead"
                    .to_string(),
            )),
            ct if ct.is_stock_type() => Ok(Self::write_stock_chart(data, chart_type)),
            ct if ct.is_surface_type() => Ok(Self::write_surface_chart(data, chart_type)),
            ct if ct.is_bar_or_column() => Ok(Self::write_bar_chart(data, chart_type)),
            ct if ct.is_line_type() => Ok(Self::write_line_chart(data, chart_type)),
            ct if ct.is_pie_type() => Ok(Self::write_pie_chart(data, chart_type)),
            ct if ct.is_doughnut_type() => Ok(Self::write_doughnut_chart(data, chart_type)),
            ct if ct.is_area_type() => Ok(Self::write_area_chart(data, chart_type)),
            ct if ct.is_radar_type() => Ok(Self::write_radar_chart(data, chart_type)),
            _ => Err(crate::error::PptxError::InvalidXml(format!(
                "unsupported category chart type: {chart_type:?}"
            ))),
        }
    }

    /// Generate chart XML for an XY (scatter) chart.
    ///
    /// # Errors
    /// Returns an error if XML generation fails.
    pub fn write_xy(
        data: &super::data::XyChartData,
        chart_type: XlChartType,
    ) -> PptxResult<String> {
        Ok(Self::write_scatter_chart(data, chart_type))
    }

    /// Generate chart XML for a bubble chart.
    ///
    /// # Errors
    /// Returns an error if XML generation fails.
    pub fn write_bubble(
        data: &super::data::BubbleChartData,
        chart_type: XlChartType,
    ) -> PptxResult<String> {
        Ok(Self::write_bubble_chart(data, chart_type))
    }
}
