//! Stock chart XML writer.

use std::fmt::Write;

use crate::enums::chart::XlChartType;
use crate::oxml::ns::{NS_A, NS_C, NS_R};

use super::super::data::CategoryChartData;
use super::ChartXmlWriter;

impl ChartXmlWriter {
    pub(super) fn write_stock_chart(data: &CategoryChartData, chart_type: XlChartType) -> String {
        let has_volume = matches!(chart_type, XlChartType::StockVHLC | XlChartType::StockVOHLC);
        let has_open = matches!(chart_type, XlChartType::StockOHLC | XlChartType::StockVOHLC);

        let mut xml = String::with_capacity(8192);
        write_str!(
            xml,
            "<?xml version='1.0' encoding='UTF-8' standalone='yes'?>\n\
             <c:chartSpace xmlns:c=\"{NS_C}\" xmlns:a=\"{NS_A}\" xmlns:r=\"{NS_R}\">\n\
             \x20 <c:date1904 val=\"0\"/>\n\
             \x20 <c:chart>\n\
             \x20   <c:autoTitleDeleted val=\"0\"/>\n\
             \x20   <c:plotArea>\n\
             \x20     <c:layout/>\n"
        );

        // For volume charts, emit a barChart for the volume series
        if has_volume && !data.series().is_empty() {
            let volume_series = &data.series()[0];
            xml.push_str(
                "\x20     <c:barChart>\n\
                 \x20       <c:barDir val=\"col\"/>\n\
                 \x20       <c:grouping val=\"clustered\"/>\n\
                 \x20       <c:varyColors val=\"0\"/>\n\
                 \x20       <c:ser>\n\
                 \x20         <c:idx val=\"0\"/>\n\
                 \x20         <c:order val=\"0\"/>\n",
            );
            Self::tx_to(&mut xml, volume_series.name());
            Self::cat_to(&mut xml, data);
            Self::val_to(&mut xml, volume_series);
            xml.push_str(
                "\x20       </c:ser>\n\
                 \x20       <c:axId val=\"-2068027336\"/>\n\
                 \x20       <c:axId val=\"-2113994440\"/>\n\
                 \x20     </c:barChart>\n",
            );
        }

        xml.push_str("\x20     <c:stockChart>\n");
        let stock_start = usize::from(has_volume);
        for series in data.series().iter().skip(stock_start) {
            let idx = series.index();
            write_str!(
                xml,
                "\x20       <c:ser>\n\
                 \x20         <c:idx val=\"{idx}\"/>\n\
                 \x20         <c:order val=\"{idx}\"/>\n"
            );
            Self::tx_to(&mut xml, series.name());
            Self::cat_to(&mut xml, data);
            Self::val_to(&mut xml, series);
            xml.push_str("\x20       </c:ser>\n");
        }
        xml.push_str("\x20       <c:hiLowLines/>\n");
        if has_open {
            xml.push_str(
                "\x20       <c:upDownBars>\n\
                 \x20         <c:gapWidth val=\"150\"/>\n\
                 \x20         <c:upBars/>\n\
                 \x20         <c:downBars/>\n\
                 \x20       </c:upDownBars>\n",
            );
        }
        xml.push_str(
            "\x20       <c:axId val=\"-2068027336\"/>\n\
             \x20       <c:axId val=\"-2113994440\"/>\n\
             \x20     </c:stockChart>\n",
        );
        Self::cat_ax_to(&mut xml, "-2068027336", "-2113994440", "b");
        xml.push_str(
            "\x20     <c:valAx>\n\
             \x20       <c:axId val=\"-2113994440\"/>\n\
             \x20       <c:scaling/>\n\
             \x20       <c:delete val=\"0\"/>\n\
             \x20       <c:axPos val=\"l\"/>\n\
             \x20       <c:majorGridlines/>\n\
             \x20       <c:majorTickMark val=\"out\"/>\n\
             \x20       <c:minorTickMark val=\"none\"/>\n\
             \x20       <c:tickLblPos val=\"nextTo\"/>\n\
             \x20       <c:crossAx val=\"-2068027336\"/>\n\
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

#[cfg(test)]
mod tests {
    use crate::chart::data::CategoryChartData;
    use crate::chart::xmlwriter::ChartXmlWriter;
    use crate::enums::chart::XlChartType;

    #[test]
    fn test_stock_hlc_chart_xml() {
        let mut data = CategoryChartData::new();
        data.add_category("Day1");
        data.add_category("Day2");
        data.add_series("High", &[100.0, 110.0]);
        data.add_series("Low", &[80.0, 85.0]);
        data.add_series("Close", &[90.0, 105.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::StockHLC).unwrap();

        assert!(xml.contains("<c:stockChart>"));
        assert!(xml.contains("</c:stockChart>"));
        assert!(!xml.contains("<c:barChart>"));
        assert!(xml.contains("<c:v>High</c:v>"));
        assert!(xml.contains("<c:v>Low</c:v>"));
        assert!(xml.contains("<c:v>Close</c:v>"));
        assert!(xml.contains("<c:hiLowLines/>"));
        assert!(!xml.contains("<c:upDownBars>"));
    }

    #[test]
    fn test_stock_ohlc_chart_xml() {
        let mut data = CategoryChartData::new();
        data.add_category("Day1");
        data.add_category("Day2");
        data.add_series("Open", &[85.0, 90.0]);
        data.add_series("High", &[100.0, 110.0]);
        data.add_series("Low", &[80.0, 85.0]);
        data.add_series("Close", &[90.0, 105.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::StockOHLC).unwrap();

        assert!(xml.contains("<c:stockChart>"));
        assert!(!xml.contains("<c:barChart>"));
        assert!(xml.contains("<c:hiLowLines/>"));
        assert!(xml.contains("<c:upDownBars>"));
        assert!(xml.contains("<c:upBars/>"));
        assert!(xml.contains("<c:downBars/>"));
        assert!(xml.contains("<c:gapWidth val=\"150\"/>"));
        assert!(xml.contains("<c:v>Open</c:v>"));
        assert!(xml.contains("<c:v>High</c:v>"));
        assert!(xml.contains("<c:v>Low</c:v>"));
        assert!(xml.contains("<c:v>Close</c:v>"));
    }

    #[test]
    fn test_stock_vhlc_chart_xml() {
        let mut data = CategoryChartData::new();
        data.add_category("Day1");
        data.add_series("Volume", &[1000.0]);
        data.add_series("High", &[100.0]);
        data.add_series("Low", &[80.0]);
        data.add_series("Close", &[90.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::StockVHLC).unwrap();

        assert!(xml.contains("<c:stockChart>"));
        assert!(xml.contains("<c:barChart>"));
        assert!(xml.contains("<c:v>Volume</c:v>"));
        assert!(xml.contains("<c:hiLowLines/>"));
        assert!(!xml.contains("<c:upDownBars>"));
    }

    #[test]
    fn test_stock_vohlc_chart_xml() {
        let mut data = CategoryChartData::new();
        data.add_category("Day1");
        data.add_series("Volume", &[1000.0]);
        data.add_series("Open", &[85.0]);
        data.add_series("High", &[100.0]);
        data.add_series("Low", &[80.0]);
        data.add_series("Close", &[90.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::StockVOHLC).unwrap();

        assert!(xml.contains("<c:stockChart>"));
        assert!(xml.contains("<c:barChart>"));
        assert!(xml.contains("<c:v>Volume</c:v>"));
        assert!(xml.contains("<c:hiLowLines/>"));
        assert!(xml.contains("<c:upDownBars>"));
        assert!(xml.contains("<c:upBars/>"));
        assert!(xml.contains("<c:downBars/>"));
    }
}
