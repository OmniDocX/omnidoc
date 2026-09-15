//! XY (scatter) and bubble chart XML writers.

use std::fmt::Write;

use crate::enums::chart::XlChartType;
use crate::oxml::ns::{NS_A, NS_C, NS_R};

use super::super::data::{BubbleChartData, XyChartData};
use super::super::plot;
use super::ChartXmlWriter;

impl ChartXmlWriter {
    // -----------------------------------------------------------------------
    // Scatter (XY) chart
    // -----------------------------------------------------------------------

    #[allow(clippy::too_many_lines)]
    pub(super) fn write_scatter_chart(data: &XyChartData, chart_type: XlChartType) -> String {
        let scatter_style =
            plot::scatter_style_for(chart_type).unwrap_or(plot::ScatterStyle::LineMarker);

        let no_marker_types = [
            XlChartType::XyScatterLinesNoMarkers,
            XlChartType::XyScatterSmoothNoMarkers,
        ];
        let has_no_marker = no_marker_types.contains(&chart_type);
        let has_sp_pr = chart_type == XlChartType::XyScatter;

        let mut xml = String::with_capacity(8192);
        write_str!(
            xml,
            "<?xml version='1.0' encoding='UTF-8' standalone='yes'?>\n\
             <c:chartSpace xmlns:c=\"{NS_C}\" xmlns:a=\"{NS_A}\" xmlns:r=\"{NS_R}\">\n\
             \x20 <c:chart>\n\
             \x20   <c:plotArea>\n\
             \x20     <c:scatterChart>\n\
             \x20       <c:scatterStyle val=\"{}\"/>\n\
             \x20       <c:varyColors val=\"0\"/>\n",
            scatter_style.to_xml_str()
        );
        for series in data.series() {
            let x_values = series.x_values();
            let y_values = series.y_values();
            let idx = series.index();
            write_str!(
                xml,
                "\x20       <c:ser>\n\
                 \x20         <c:idx val=\"{idx}\"/>\n\
                 \x20         <c:order val=\"{idx}\"/>\n"
            );
            Self::tx_to(&mut xml, series.name());
            if has_sp_pr {
                xml.push_str(
                    "          <c:spPr>\n\
                     \x20           <a:ln w=\"47625\">\n\
                     \x20             <a:noFill/>\n\
                     \x20           </a:ln>\n\
                     \x20         </c:spPr>\n",
                );
            }
            if has_no_marker {
                xml.push_str(
                    "          <c:marker>\n\
                     \x20           <c:symbol val=\"none\"/>\n\
                     \x20         </c:marker>\n",
                );
            }
            Self::num_val_to(&mut xml, "xVal", &x_values, "General");
            Self::num_val_to(&mut xml, "yVal", &y_values, "General");
            xml.push_str(
                "\x20         <c:smooth val=\"0\"/>\n\
                 \x20       </c:ser>\n",
            );
        }
        xml.push_str(
            "\x20       <c:axId val=\"-2128940872\"/>\n\
             \x20       <c:axId val=\"-2129643912\"/>\n\
             \x20     </c:scatterChart>\n\
             \x20     <c:valAx>\n\
             \x20       <c:axId val=\"-2128940872\"/>\n\
             \x20       <c:scaling>\n\
             \x20         <c:orientation val=\"minMax\"/>\n\
             \x20       </c:scaling>\n\
             \x20       <c:delete val=\"0\"/>\n\
             \x20       <c:axPos val=\"b\"/>\n\
             \x20       <c:numFmt formatCode=\"General\" sourceLinked=\"1\"/>\n\
             \x20       <c:majorTickMark val=\"out\"/>\n\
             \x20       <c:minorTickMark val=\"none\"/>\n\
             \x20       <c:tickLblPos val=\"nextTo\"/>\n\
             \x20       <c:crossAx val=\"-2129643912\"/>\n\
             \x20       <c:crosses val=\"autoZero\"/>\n\
             \x20       <c:crossBetween val=\"midCat\"/>\n\
             \x20     </c:valAx>\n\
             \x20     <c:valAx>\n\
             \x20       <c:axId val=\"-2129643912\"/>\n\
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
             \x20       <c:crossAx val=\"-2128940872\"/>\n\
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
             \x20   <c:dispBlanksAs val=\"gap\"/>\n\
             \x20   <c:showDLblsOverMax val=\"0\"/>\n\
             \x20 </c:chart>\n",
        );
        Self::txpr_to(&mut xml);
        xml.push_str("</c:chartSpace>\n");
        xml
    }

    // -----------------------------------------------------------------------
    // Bubble chart
    // -----------------------------------------------------------------------

    #[allow(clippy::too_many_lines)]
    pub(super) fn write_bubble_chart(data: &BubbleChartData, chart_type: XlChartType) -> String {
        let bubble_3d_val = if chart_type == XlChartType::BubbleThreeDEffect {
            "1"
        } else {
            "0"
        };

        let mut xml = String::with_capacity(8192);
        write_str!(
            xml,
            "<?xml version='1.0' encoding='UTF-8' standalone='yes'?>\n\
             <c:chartSpace xmlns:c=\"{NS_C}\" xmlns:a=\"{NS_A}\" xmlns:r=\"{NS_R}\">\n\
             \x20 <c:chart>\n\
             \x20   <c:autoTitleDeleted val=\"0\"/>\n\
             \x20   <c:plotArea>\n\
             \x20     <c:layout/>\n\
             \x20     <c:bubbleChart>\n\
             \x20       <c:varyColors val=\"0\"/>\n"
        );
        for series in data.series() {
            let x_values = series.x_values();
            let y_values = series.y_values();
            let bubble_sizes = series.bubble_sizes();
            let idx = series.index();
            write_str!(
                xml,
                "\x20       <c:ser>\n\
                 \x20         <c:idx val=\"{idx}\"/>\n\
                 \x20         <c:order val=\"{idx}\"/>\n"
            );
            Self::tx_to(&mut xml, series.name());
            xml.push_str("\x20         <c:invertIfNegative val=\"0\"/>\n");
            Self::num_val_to(&mut xml, "xVal", &x_values, "General");
            Self::num_val_to(&mut xml, "yVal", &y_values, "General");
            Self::num_val_to(&mut xml, "bubbleSize", &bubble_sizes, "General");
            write_str!(
                xml,
                "\x20         <c:bubble3D val=\"{bubble_3d_val}\"/>\n\
                 \x20       </c:ser>\n"
            );
        }
        xml.push_str(
            "\x20       <c:dLbls>\n\
             \x20         <c:showLegendKey val=\"0\"/>\n\
             \x20         <c:showVal val=\"0\"/>\n\
             \x20         <c:showCatName val=\"0\"/>\n\
             \x20         <c:showSerName val=\"0\"/>\n\
             \x20         <c:showPercent val=\"0\"/>\n\
             \x20         <c:showBubbleSize val=\"0\"/>\n\
             \x20       </c:dLbls>\n\
             \x20       <c:bubbleScale val=\"100\"/>\n\
             \x20       <c:showNegBubbles val=\"0\"/>\n\
             \x20       <c:axId val=\"-2115720072\"/>\n\
             \x20       <c:axId val=\"-2115723560\"/>\n\
             \x20     </c:bubbleChart>\n\
             \x20     <c:valAx>\n\
             \x20       <c:axId val=\"-2115720072\"/>\n\
             \x20       <c:scaling>\n\
             \x20         <c:orientation val=\"minMax\"/>\n\
             \x20       </c:scaling>\n\
             \x20       <c:delete val=\"0\"/>\n\
             \x20       <c:axPos val=\"b\"/>\n\
             \x20       <c:numFmt formatCode=\"General\" sourceLinked=\"1\"/>\n\
             \x20       <c:majorTickMark val=\"out\"/>\n\
             \x20       <c:minorTickMark val=\"none\"/>\n\
             \x20       <c:tickLblPos val=\"nextTo\"/>\n\
             \x20       <c:crossAx val=\"-2115723560\"/>\n\
             \x20       <c:crosses val=\"autoZero\"/>\n\
             \x20       <c:crossBetween val=\"midCat\"/>\n\
             \x20     </c:valAx>\n\
             \x20     <c:valAx>\n\
             \x20       <c:axId val=\"-2115723560\"/>\n\
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
             \x20       <c:crossAx val=\"-2115720072\"/>\n\
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
             \x20   <c:dispBlanksAs val=\"gap\"/>\n\
             \x20   <c:showDLblsOverMax val=\"0\"/>\n\
             \x20 </c:chart>\n",
        );
        Self::txpr_to(&mut xml);
        xml.push_str("</c:chartSpace>\n");
        xml
    }
}

#[cfg(test)]
mod tests {
    use super::super::ChartXmlWriter;
    use crate::chart::data::{BubbleChartData, XyChartData};
    use crate::enums::chart::XlChartType;

    #[test]
    fn test_scatter_chart_xml() {
        let mut data = XyChartData::new();
        {
            let series = data.add_series("Measurements");
            series.add_data_point(1.0, 2.0);
            series.add_data_point(3.0, 4.0);
        }

        let xml = ChartXmlWriter::write_xy(&data, XlChartType::XyScatterLines).unwrap();

        assert!(xml.contains("<c:scatterChart>"));
        assert!(xml.contains("<c:scatterStyle val=\"lineMarker\"/>"));
        assert!(xml.contains("<c:xVal>"));
        assert!(xml.contains("<c:yVal>"));
    }

    #[test]
    fn test_bubble_chart_xml() {
        let mut data = BubbleChartData::new();
        {
            let series = data.add_series("Dataset");
            series.add_data_point(1.0, 2.0, 10.0);
        }

        let xml = ChartXmlWriter::write_bubble(&data, XlChartType::Bubble).unwrap();

        assert!(xml.contains("<c:bubbleChart>"));
        assert!(xml.contains("<c:bubbleSize>"));
        assert!(xml.contains("<c:bubble3D val=\"0\"/>"));
    }

    #[test]
    fn test_bubble_3d_chart_xml() {
        let mut data = BubbleChartData::new();
        {
            let series = data.add_series("Dataset");
            series.add_data_point(1.0, 2.0, 10.0);
        }

        let xml = ChartXmlWriter::write_bubble(&data, XlChartType::BubbleThreeDEffect).unwrap();

        assert!(xml.contains("<c:bubble3D val=\"1\"/>"));
    }
}
