//! Pie and doughnut chart XML writers.

use std::fmt::Write;

use crate::enums::chart::XlChartType;
use crate::oxml::ns::{NS_A, NS_C, NS_R};

use super::super::data::CategoryChartData;
use super::ChartXmlWriter;

impl ChartXmlWriter {
    // -----------------------------------------------------------------------
    // Pie chart (2D and 3D)
    // -----------------------------------------------------------------------

    pub(super) fn write_pie_chart(data: &CategoryChartData, chart_type: XlChartType) -> String {
        let is_3d = chart_type.is_3d_type();
        let has_explosion = matches!(
            chart_type,
            XlChartType::PieExploded | XlChartType::ExplodedPie3D
        );

        let chart_tag = if is_3d { "pie3DChart" } else { "pieChart" };

        let mut xml = String::with_capacity(4096);
        write_str!(
            xml,
            "<?xml version='1.0' encoding='UTF-8' standalone='yes'?>\n\
             <c:chartSpace xmlns:c=\"{NS_C}\" xmlns:a=\"{NS_A}\" xmlns:r=\"{NS_R}\">\n\
             \x20 <c:chart>\n"
        );
        if is_3d {
            Self::view3d_to(&mut xml, 30, 0, 0);
        }
        write_str!(
            xml,
            "\x20   <c:autoTitleDeleted val=\"0\"/>\n\
             \x20   <c:plotArea>\n\
             \x20     <c:{chart_tag}>\n\
             \x20       <c:varyColors val=\"1\"/>\n"
        );
        if let Some(series) = data.series().first() {
            xml.push_str(
                "\x20       <c:ser>\n\
                 \x20         <c:idx val=\"0\"/>\n\
                 \x20         <c:order val=\"0\"/>\n",
            );
            Self::tx_to(&mut xml, series.name());
            if has_explosion {
                xml.push_str("          <c:explosion val=\"25\"/>\n");
            }
            Self::cat_to(&mut xml, data);
            Self::val_to(&mut xml, series);
            xml.push_str("\x20       </c:ser>\n");
        }
        write_str!(
            xml,
            "\x20     </c:{chart_tag}>\n\
             \x20   </c:plotArea>\n\
             \x20   <c:dispBlanksAs val=\"gap\"/>\n\
             \x20 </c:chart>\n"
        );
        Self::txpr_to(&mut xml);
        xml.push_str("</c:chartSpace>\n");
        xml
    }

    // -----------------------------------------------------------------------
    // Doughnut chart
    // -----------------------------------------------------------------------

    pub(super) fn write_doughnut_chart(
        data: &CategoryChartData,
        chart_type: XlChartType,
    ) -> String {
        let has_explosion = chart_type == XlChartType::DoughnutExploded;

        let mut xml = String::with_capacity(8192);
        write_str!(
            xml,
            "<?xml version='1.0' encoding='UTF-8' standalone='yes'?>\n\
             <c:chartSpace xmlns:c=\"{NS_C}\" xmlns:a=\"{NS_A}\" xmlns:r=\"{NS_R}\">\n\
             \x20 <c:date1904 val=\"0\"/>\n\
             \x20 <c:roundedCorners val=\"0\"/>\n\
             \x20 <c:chart>\n\
             \x20   <c:autoTitleDeleted val=\"0\"/>\n\
             \x20   <c:plotArea>\n\
             \x20     <c:layout/>\n\
             \x20     <c:doughnutChart>\n\
             \x20       <c:varyColors val=\"1\"/>\n"
        );
        if has_explosion {
            Self::category_series_with_explosion_to(
                &mut xml,
                data,
                "          <c:explosion val=\"25\"/>\n",
            );
        } else {
            Self::category_series_with_explosion_to(&mut xml, data, "");
        }
        xml.push_str(
            "\x20       <c:dLbls>\n\
             \x20         <c:showLegendKey val=\"0\"/>\n\
             \x20         <c:showVal val=\"0\"/>\n\
             \x20         <c:showCatName val=\"0\"/>\n\
             \x20         <c:showSerName val=\"0\"/>\n\
             \x20         <c:showPercent val=\"0\"/>\n\
             \x20         <c:showBubbleSize val=\"0\"/>\n\
             \x20         <c:showLeaderLines val=\"1\"/>\n\
             \x20       </c:dLbls>\n\
             \x20       <c:firstSliceAng val=\"0\"/>\n\
             \x20       <c:holeSize val=\"50\"/>\n\
             \x20     </c:doughnutChart>\n\
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
