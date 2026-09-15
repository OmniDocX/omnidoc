//! Bar/column and line chart XML writers.

use std::fmt::Write;

use crate::enums::chart::XlChartType;
use crate::oxml::ns::{NS_A, NS_C, NS_R};

use super::super::data::CategoryChartData;
use super::super::plot;
use super::ChartXmlWriter;

impl ChartXmlWriter {
    // -----------------------------------------------------------------------
    // Bar / Column chart (2D and 3D)
    // -----------------------------------------------------------------------

    pub(super) fn write_bar_chart(data: &CategoryChartData, chart_type: XlChartType) -> String {
        let is_3d = chart_type.is_3d_type();
        let bar_dir = plot::bar_direction_for(chart_type).unwrap_or(plot::BarDirection::Column);
        let grouping = plot::grouping_for(chart_type).unwrap_or(plot::ChartGrouping::Clustered);

        let (cat_ax_pos, val_ax_pos) = if bar_dir == plot::BarDirection::Bar {
            ("l", "b")
        } else {
            ("b", "l")
        };

        let chart_tag = if is_3d { "bar3DChart" } else { "barChart" };

        let mut xml = String::with_capacity(8192);
        write_str!(
            xml,
            "<?xml version='1.0' encoding='UTF-8' standalone='yes'?>\n\
             <c:chartSpace xmlns:c=\"{NS_C}\" xmlns:a=\"{NS_A}\" xmlns:r=\"{NS_R}\">\n\
             \x20 <c:date1904 val=\"0\"/>\n\
             \x20 <c:chart>\n"
        );
        if is_3d {
            Self::view3d_to(&mut xml, 15, 20, 15);
        }
        write_str!(
            xml,
            "\x20   <c:autoTitleDeleted val=\"0\"/>\n\
             \x20   <c:plotArea>\n\
             \x20     <c:{chart_tag}>\n\
             \x20       <c:barDir val=\"{}\"/>\n\
             \x20       <c:grouping val=\"{}\"/>\n",
            bar_dir.to_xml_str(),
            grouping.to_xml_str()
        );
        Self::category_series_to(&mut xml, data);
        if plot::needs_overlap(chart_type) {
            xml.push_str("        <c:overlap val=\"100\"/>\n");
        }
        if let Some(shape) = chart_type.chart_shape() {
            write_str!(xml, "        <c:shape val=\"{shape}\"/>\n");
        }
        write_str!(
            xml,
            "\x20       <c:axId val=\"-2068027336\"/>\n\
             \x20       <c:axId val=\"-2113994440\"/>\n\
             \x20     </c:{chart_tag}>\n"
        );
        Self::cat_ax_to(&mut xml, "-2068027336", "-2113994440", cat_ax_pos);
        write_str!(
            xml,
            "\x20     <c:valAx>\n\
             \x20       <c:axId val=\"-2113994440\"/>\n\
             \x20       <c:scaling/>\n\
             \x20       <c:delete val=\"0\"/>\n\
             \x20       <c:axPos val=\"{val_ax_pos}\"/>\n\
             \x20       <c:majorGridlines/>\n\
             \x20       <c:majorTickMark val=\"out\"/>\n\
             \x20       <c:minorTickMark val=\"none\"/>\n\
             \x20       <c:tickLblPos val=\"nextTo\"/>\n\
             \x20       <c:crossAx val=\"-2068027336\"/>\n\
             \x20       <c:crosses val=\"autoZero\"/>\n\
             \x20     </c:valAx>\n\
             \x20   </c:plotArea>\n\
             \x20   <c:dispBlanksAs val=\"gap\"/>\n\
             \x20 </c:chart>\n"
        );
        Self::txpr_to(&mut xml);
        xml.push_str("</c:chartSpace>\n");
        xml
    }

    // -----------------------------------------------------------------------
    // Line chart (2D and 3D)
    // -----------------------------------------------------------------------

    pub(super) fn write_line_chart(data: &CategoryChartData, chart_type: XlChartType) -> String {
        let is_3d = chart_type.is_3d_type();
        let grouping = plot::grouping_for(chart_type).unwrap_or(plot::ChartGrouping::Standard);

        let no_marker_types = [
            XlChartType::Line,
            XlChartType::LineStacked,
            XlChartType::LineStacked100,
            XlChartType::Line3D,
        ];
        let has_no_marker = no_marker_types.contains(&chart_type);

        let chart_tag = if is_3d { "line3DChart" } else { "lineChart" };

        let mut xml = String::with_capacity(8192);
        write_str!(
            xml,
            "<?xml version='1.0' encoding='UTF-8' standalone='yes'?>\n\
             <c:chartSpace xmlns:c=\"{NS_C}\" xmlns:a=\"{NS_A}\" xmlns:r=\"{NS_R}\">\n\
             \x20 <c:date1904 val=\"0\"/>\n\
             \x20 <c:chart>\n"
        );
        if is_3d {
            Self::view3d_to(&mut xml, 15, 20, 15);
        }
        write_str!(
            xml,
            "\x20   <c:autoTitleDeleted val=\"0\"/>\n\
             \x20   <c:plotArea>\n\
             \x20     <c:{chart_tag}>\n\
             \x20       <c:grouping val=\"{}\"/>\n\
             \x20       <c:varyColors val=\"0\"/>\n",
            grouping.to_xml_str()
        );
        for series in data.series() {
            let idx = series.index();
            write_str!(
                xml,
                "\x20       <c:ser>\n\
                 \x20         <c:idx val=\"{idx}\"/>\n\
                 \x20         <c:order val=\"{idx}\"/>\n"
            );
            Self::tx_to(&mut xml, series.name());
            if has_no_marker {
                xml.push_str(
                    "          <c:marker>\n\
                     \x20           <c:symbol val=\"none\"/>\n\
                     \x20         </c:marker>\n",
                );
            }
            Self::cat_to(&mut xml, data);
            Self::val_to(&mut xml, series);
            xml.push_str(
                "\x20         <c:smooth val=\"0\"/>\n\
                 \x20       </c:ser>\n",
            );
        }
        if !is_3d {
            xml.push_str(
                "\x20       <c:marker val=\"1\"/>\n\
                 \x20       <c:smooth val=\"0\"/>\n",
            );
        }
        write_str!(
            xml,
            "\x20       <c:axId val=\"2118791784\"/>\n\
             \x20       <c:axId val=\"2140495176\"/>\n\
             \x20     </c:{chart_tag}>\n"
        );
        Self::cat_ax_to(&mut xml, "2118791784", "2140495176", "b");
        xml.push_str(
            "\x20     <c:valAx>\n\
             \x20       <c:axId val=\"2140495176\"/>\n\
             \x20       <c:scaling/>\n\
             \x20       <c:delete val=\"0\"/>\n\
             \x20       <c:axPos val=\"l\"/>\n\
             \x20       <c:majorGridlines/>\n\
             \x20       <c:majorTickMark val=\"out\"/>\n\
             \x20       <c:minorTickMark val=\"none\"/>\n\
             \x20       <c:tickLblPos val=\"nextTo\"/>\n\
             \x20       <c:crossAx val=\"2118791784\"/>\n\
             \x20       <c:crosses val=\"autoZero\"/>\n\
             \x20     </c:valAx>\n\
             \x20   </c:plotArea>\n\
             \x20   <c:legend>\n\
             \x20     <c:legendPos val=\"r\"/>\n\
             \x20     <c:layout/>\n\
             \x20     <c:overlay val=\"0\"/>\n\
             \x20   </c:legend>\n\
             \x20   <c:plotVisOnly val=\"1\"/>\n\
             \x20   <c:dispBlanksAs val=\"gap\"/>\n\
             \x20   <c:showDLblsOverMax val=\"0\"/>\n\
             \x20 </c:chart>\n",
        );
        Self::txpr_to(&mut xml);
        xml.push_str("</c:chartSpace>\n");
        xml
    }
}
