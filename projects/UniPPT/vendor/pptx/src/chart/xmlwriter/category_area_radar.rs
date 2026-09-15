//! Area and radar chart XML writers.

use std::fmt::Write;

use crate::enums::chart::XlChartType;
use crate::oxml::ns::{NS_A, NS_C, NS_R};

use super::super::data::CategoryChartData;
use super::super::plot;
use super::ChartXmlWriter;

impl ChartXmlWriter {
    // -----------------------------------------------------------------------
    // Area chart (2D and 3D)
    // -----------------------------------------------------------------------

    pub(super) fn write_area_chart(data: &CategoryChartData, chart_type: XlChartType) -> String {
        let is_3d = chart_type.is_3d_type();
        let grouping = plot::grouping_for(chart_type).unwrap_or(plot::ChartGrouping::Standard);
        let chart_tag = if is_3d { "area3DChart" } else { "areaChart" };

        let mut xml = String::with_capacity(8192);
        write_str!(
            xml,
            "<?xml version='1.0' encoding='UTF-8' standalone='yes'?>\n\
             <c:chartSpace xmlns:c=\"{NS_C}\" xmlns:a=\"{NS_A}\" xmlns:r=\"{NS_R}\">\n\
             \x20 <c:date1904 val=\"0\"/>\n\
             \x20 <c:roundedCorners val=\"0\"/>\n\
             \x20 <c:chart>\n"
        );
        if is_3d {
            Self::view3d_to(&mut xml, 15, 20, 15);
        }
        write_str!(
            xml,
            "\x20   <c:autoTitleDeleted val=\"0\"/>\n\
             \x20   <c:plotArea>\n\
             \x20     <c:layout/>\n\
             \x20     <c:{chart_tag}>\n\
             \x20       <c:grouping val=\"{}\"/>\n\
             \x20       <c:varyColors val=\"0\"/>\n",
            grouping.to_xml_str()
        );
        Self::category_series_to(&mut xml, data);
        xml.push_str(
            "\x20       <c:dLbls>\n\
             \x20         <c:showLegendKey val=\"0\"/>\n\
             \x20         <c:showVal val=\"0\"/>\n\
             \x20         <c:showCatName val=\"0\"/>\n\
             \x20         <c:showSerName val=\"0\"/>\n\
             \x20         <c:showPercent val=\"0\"/>\n\
             \x20         <c:showBubbleSize val=\"0\"/>\n\
             \x20       </c:dLbls>\n",
        );
        write_str!(
            xml,
            "\x20       <c:axId val=\"-2101159928\"/>\n\
             \x20       <c:axId val=\"-2100718248\"/>\n\
             \x20     </c:{chart_tag}>\n"
        );
        Self::cat_ax_to(&mut xml, "-2101159928", "-2100718248", "b");
        xml.push_str(
            "\x20     <c:valAx>\n\
             \x20       <c:axId val=\"-2100718248\"/>\n\
             \x20       <c:scaling>\n\
             \x20         <c:orientation val=\"minMax\"/>\n\
             \x20       </c:scaling>\n\
             \x20       <c:delete val=\"0\"/>\n\
             \x20       <c:axPos val=\"l\"/>\n\
             \x20       <c:majorGridlines/>\n\
             \x20       <c:numFmt formatCode=\"General\" sourceLinked=\"1\"/>\n\
             \x20       <c:majorTickMark val=\"out\"/>\n\
             \x20       <c:minorTickMark val=\"none\"/>\n\
             \x20       <c:tickLblPos val=\"nextTo\"/>\n\
             \x20       <c:crossAx val=\"-2101159928\"/>\n\
             \x20       <c:crosses val=\"autoZero\"/>\n\
             \x20       <c:crossBetween val=\"midCat\"/>\n\
             \x20     </c:valAx>\n\
             \x20   </c:plotArea>\n\
             \x20   <c:legend>\n\
             \x20     <c:legendPos val=\"r\"/>\n\
             \x20     <c:layout/>\n\
             \x20     <c:overlay val=\"0\"/>\n\
             \x20   </c:legend>\n\
             \x20   <c:plotVisOnly val=\"1\"/>\n\
             \x20   <c:dispBlanksAs val=\"zero\"/>\n\
             \x20   <c:showDLblsOverMax val=\"0\"/>\n\
             \x20 </c:chart>\n",
        );
        Self::txpr_to(&mut xml);
        xml.push_str("</c:chartSpace>\n");
        xml
    }

    // -----------------------------------------------------------------------
    // Radar chart
    // -----------------------------------------------------------------------

    pub(super) fn write_radar_chart(data: &CategoryChartData, chart_type: XlChartType) -> String {
        let radar_style = plot::radar_style_for(chart_type).unwrap_or(plot::RadarStyle::Marker);
        let has_no_marker = chart_type == XlChartType::Radar;

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
             \x20     <c:radarChart>\n\
             \x20       <c:radarStyle val=\"{}\"/>\n\
             \x20       <c:varyColors val=\"0\"/>\n",
            radar_style.to_xml_str()
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
        xml.push_str(
            "\x20       <c:axId val=\"2073612648\"/>\n\
             \x20       <c:axId val=\"-2112772216\"/>\n\
             \x20     </c:radarChart>\n\
             \x20     <c:catAx>\n\
             \x20       <c:axId val=\"2073612648\"/>\n\
             \x20       <c:scaling>\n\
             \x20         <c:orientation val=\"minMax\"/>\n\
             \x20       </c:scaling>\n\
             \x20       <c:delete val=\"0\"/>\n\
             \x20       <c:axPos val=\"b\"/>\n\
             \x20       <c:majorGridlines/>\n\
             \x20       <c:numFmt formatCode=\"m/d/yy\" sourceLinked=\"1\"/>\n\
             \x20       <c:majorTickMark val=\"out\"/>\n\
             \x20       <c:minorTickMark val=\"none\"/>\n\
             \x20       <c:tickLblPos val=\"nextTo\"/>\n\
             \x20       <c:crossAx val=\"-2112772216\"/>\n\
             \x20       <c:crosses val=\"autoZero\"/>\n\
             \x20       <c:auto val=\"1\"/>\n\
             \x20       <c:lblAlgn val=\"ctr\"/>\n\
             \x20       <c:lblOffset val=\"100\"/>\n\
             \x20       <c:noMultiLvlLbl val=\"0\"/>\n\
             \x20     </c:catAx>\n\
             \x20     <c:valAx>\n\
             \x20       <c:axId val=\"-2112772216\"/>\n\
             \x20       <c:scaling>\n\
             \x20         <c:orientation val=\"minMax\"/>\n\
             \x20       </c:scaling>\n\
             \x20       <c:delete val=\"0\"/>\n\
             \x20       <c:axPos val=\"l\"/>\n\
             \x20       <c:majorGridlines/>\n\
             \x20       <c:numFmt formatCode=\"General\" sourceLinked=\"1\"/>\n\
             \x20       <c:majorTickMark val=\"cross\"/>\n\
             \x20       <c:minorTickMark val=\"none\"/>\n\
             \x20       <c:tickLblPos val=\"nextTo\"/>\n\
             \x20       <c:crossAx val=\"2073612648\"/>\n\
             \x20       <c:crosses val=\"autoZero\"/>\n\
             \x20       <c:crossBetween val=\"between\"/>\n\
             \x20     </c:valAx>\n\
             \x20   </c:plotArea>\n\
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
