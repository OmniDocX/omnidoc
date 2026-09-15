//! Combo chart (bar/column + line) XML writer.

use std::fmt::Write;

use crate::error::PptxResult;
use crate::oxml::ns::{NS_A, NS_C, NS_R};

use super::super::data::ComboChartData;
use super::ChartXmlWriter;

// Axis IDs for the combo chart.
const CAT_AX_ID: &str = "-2068027336";
const PRIMARY_VAL_AX_ID: &str = "-2113994440";
const SECONDARY_VAL_AX_ID: &str = "2140495176";

impl ChartXmlWriter {
    /// Generate chart XML for a combo chart (bar/column + line).
    ///
    /// The bar series are placed in a `<c:barChart>` element referencing the
    /// primary value axis. The line series are placed in a `<c:lineChart>`
    /// element referencing a secondary value axis on the right side.
    ///
    /// # Errors
    /// Returns an error if XML generation fails.
    pub fn write_combo(data: &ComboChartData) -> PptxResult<String> {
        let mut xml = String::with_capacity(8192);
        write_str!(
            xml,
            "<?xml version='1.0' encoding='UTF-8' standalone='yes'?>\n\
             <c:chartSpace xmlns:c=\"{NS_C}\" xmlns:a=\"{NS_A}\" xmlns:r=\"{NS_R}\">\n\
             \x20 <c:date1904 val=\"0\"/>\n\
             \x20 <c:chart>\n\
             \x20   <c:autoTitleDeleted val=\"0\"/>\n\
             \x20   <c:plotArea>\n"
        );

        // -- barChart section (primary axis) --
        Self::write_combo_bar_section(&mut xml, data);

        // -- lineChart section (secondary axis) --
        Self::write_combo_line_section(&mut xml, data);

        // -- Category axis (shared, bottom) --
        Self::cat_ax_to(&mut xml, CAT_AX_ID, PRIMARY_VAL_AX_ID, "b");

        // -- Primary value axis (left, for bars) --
        write_str!(
            xml,
            "\x20     <c:valAx>\n\
             \x20       <c:axId val=\"{PRIMARY_VAL_AX_ID}\"/>\n\
             \x20       <c:scaling/>\n\
             \x20       <c:delete val=\"0\"/>\n\
             \x20       <c:axPos val=\"l\"/>\n\
             \x20       <c:majorGridlines/>\n\
             \x20       <c:majorTickMark val=\"out\"/>\n\
             \x20       <c:minorTickMark val=\"none\"/>\n\
             \x20       <c:tickLblPos val=\"nextTo\"/>\n\
             \x20       <c:crossAx val=\"{CAT_AX_ID}\"/>\n\
             \x20       <c:crosses val=\"autoZero\"/>\n\
             \x20     </c:valAx>\n"
        );

        // -- Secondary value axis (right, for lines) --
        write_str!(
            xml,
            "\x20     <c:valAx>\n\
             \x20       <c:axId val=\"{SECONDARY_VAL_AX_ID}\"/>\n\
             \x20       <c:scaling/>\n\
             \x20       <c:delete val=\"0\"/>\n\
             \x20       <c:axPos val=\"r\"/>\n\
             \x20       <c:majorTickMark val=\"out\"/>\n\
             \x20       <c:minorTickMark val=\"none\"/>\n\
             \x20       <c:tickLblPos val=\"nextTo\"/>\n\
             \x20       <c:crossAx val=\"{CAT_AX_ID}\"/>\n\
             \x20       <c:crosses val=\"max\"/>\n\
             \x20     </c:valAx>\n"
        );

        write_str!(
            xml,
            "\x20   </c:plotArea>\n\
             \x20   <c:legend>\n\
             \x20     <c:legendPos val=\"b\"/>\n\
             \x20     <c:layout/>\n\
             \x20     <c:overlay val=\"0\"/>\n\
             \x20   </c:legend>\n\
             \x20   <c:plotVisOnly val=\"1\"/>\n\
             \x20   <c:dispBlanksAs val=\"gap\"/>\n\
             \x20 </c:chart>\n"
        );
        Self::txpr_to(&mut xml);
        xml.push_str("</c:chartSpace>\n");
        Ok(xml)
    }

    /// Write the `<c:barChart>` section for the bar series in a combo chart.
    fn write_combo_bar_section(xml: &mut String, data: &ComboChartData) {
        write_str!(
            xml,
            "\x20     <c:barChart>\n\
             \x20       <c:barDir val=\"col\"/>\n\
             \x20       <c:grouping val=\"clustered\"/>\n"
        );

        let categories = data.categories();
        for series in data.bar_series() {
            let idx = series.index();
            write_str!(
                xml,
                "\x20       <c:ser>\n\
                 \x20         <c:idx val=\"{idx}\"/>\n\
                 \x20         <c:order val=\"{idx}\"/>\n"
            );
            Self::tx_to(xml, series.name());
            Self::combo_cat_to(xml, categories);
            Self::val_to(xml, series.as_category_series());
            xml.push_str("\x20       </c:ser>\n");
        }

        write_str!(
            xml,
            "\x20       <c:axId val=\"{CAT_AX_ID}\"/>\n\
             \x20       <c:axId val=\"{PRIMARY_VAL_AX_ID}\"/>\n\
             \x20     </c:barChart>\n"
        );
    }

    /// Write the `<c:lineChart>` section for the line series in a combo chart.
    fn write_combo_line_section(xml: &mut String, data: &ComboChartData) {
        write_str!(
            xml,
            "\x20     <c:lineChart>\n\
             \x20       <c:grouping val=\"standard\"/>\n\
             \x20       <c:varyColors val=\"0\"/>\n"
        );

        let categories = data.categories();
        for series in data.line_series() {
            let idx = series.index();
            write_str!(
                xml,
                "\x20       <c:ser>\n\
                 \x20         <c:idx val=\"{idx}\"/>\n\
                 \x20         <c:order val=\"{idx}\"/>\n"
            );
            Self::tx_to(xml, series.name());
            xml.push_str(
                "          <c:marker>\n\
                 \x20           <c:symbol val=\"none\"/>\n\
                 \x20         </c:marker>\n",
            );
            Self::combo_cat_to(xml, categories);
            Self::val_to(xml, series.as_category_series());
            xml.push_str(
                "\x20         <c:smooth val=\"0\"/>\n\
                 \x20       </c:ser>\n",
            );
        }

        write_str!(
            xml,
            "\x20       <c:marker val=\"1\"/>\n\
             \x20       <c:smooth val=\"0\"/>\n\
             \x20       <c:axId val=\"{CAT_AX_ID}\"/>\n\
             \x20       <c:axId val=\"{SECONDARY_VAL_AX_ID}\"/>\n\
             \x20     </c:lineChart>\n"
        );
    }

    /// Write `<c:cat>` from a plain category slice (used by combo charts that
    /// store categories directly rather than in `CategoryChartData`).
    fn combo_cat_to(w: &mut String, categories: &[String]) {
        let count = categories.len();
        write_str!(
            w,
            "\x20         <c:cat>\n\
             \x20           <c:strRef>\n\
             \x20             <c:f>Sheet1!$A$2:$A${}</c:f>\n\
             \x20             <c:strCache>\n\
             \x20               <c:ptCount val=\"{count}\"/>\n",
            count + 1
        );
        for (idx, cat) in categories.iter().enumerate() {
            write_str!(
                w,
                "\x20               <c:pt idx=\"{idx}\">\n\
                 \x20                 <c:v>"
            );
            super::xml_escape_to(w, cat);
            w.push_str(
                "</c:v>\n\
                 \x20               </c:pt>\n",
            );
        }
        w.push_str(
            "\x20             </c:strCache>\n\
             \x20           </c:strRef>\n\
             \x20         </c:cat>\n",
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::chart::data::{ComboChartData, ComboSeriesType};
    use crate::chart::xmlwriter::ChartXmlWriter;

    #[test]
    fn test_combo_chart_contains_both_chart_types() {
        let mut data = ComboChartData::new();
        data.add_category("Q1");
        data.add_category("Q2");
        data.add_series("Revenue", ComboSeriesType::Bar, &[100.0, 150.0]);
        data.add_series("Growth %", ComboSeriesType::Line, &[10.0, 15.0]);

        let xml = ChartXmlWriter::write_combo(&data).unwrap();

        assert!(xml.contains("<c:chartSpace"));
        assert!(xml.contains("<c:barChart>"));
        assert!(xml.contains("<c:lineChart>"));
        assert!(xml.contains("</c:chartSpace>"));
    }

    #[test]
    fn test_combo_chart_bar_properties() {
        let mut data = ComboChartData::new();
        data.add_category("A");
        data.add_series("Bars", ComboSeriesType::Bar, &[42.0]);

        let xml = ChartXmlWriter::write_combo(&data).unwrap();

        assert!(xml.contains("<c:barDir val=\"col\"/>"));
        assert!(xml.contains("<c:grouping val=\"clustered\"/>"));
        assert!(xml.contains("<c:v>42</c:v>"));
    }

    #[test]
    fn test_combo_chart_line_properties() {
        let mut data = ComboChartData::new();
        data.add_category("A");
        data.add_series("Lines", ComboSeriesType::Line, &[7.5]);

        let xml = ChartXmlWriter::write_combo(&data).unwrap();

        assert!(xml.contains("<c:grouping val=\"standard\"/>"));
        assert!(xml.contains("<c:symbol val=\"none\"/>"));
        assert!(xml.contains("<c:smooth val=\"0\"/>"));
    }

    #[test]
    fn test_combo_chart_dual_axes() {
        let mut data = ComboChartData::new();
        data.add_category("X");
        data.add_series("Bar", ComboSeriesType::Bar, &[1.0]);
        data.add_series("Line", ComboSeriesType::Line, &[2.0]);

        let xml = ChartXmlWriter::write_combo(&data).unwrap();

        // Primary value axis (left)
        assert!(xml.contains("<c:axPos val=\"l\"/>"));
        // Secondary value axis (right)
        assert!(xml.contains("<c:axPos val=\"r\"/>"));
        // Two valAx elements
        let val_ax_count = xml.matches("<c:valAx>").count();
        assert_eq!(val_ax_count, 2, "expected 2 value axes, got {val_ax_count}");
    }

    #[test]
    fn test_combo_chart_series_indices() {
        let mut data = ComboChartData::new();
        data.add_category("A");
        data.add_series("S0", ComboSeriesType::Bar, &[1.0]);
        data.add_series("S1", ComboSeriesType::Line, &[2.0]);
        data.add_series("S2", ComboSeriesType::Bar, &[3.0]);

        let xml = ChartXmlWriter::write_combo(&data).unwrap();

        // All three series should be present with correct order indices
        assert!(xml.contains("<c:order val=\"0\"/>"));
        assert!(xml.contains("<c:order val=\"1\"/>"));
        assert!(xml.contains("<c:order val=\"2\"/>"));
    }

    #[test]
    fn test_combo_chart_category_labels() {
        let mut data = ComboChartData::new();
        data.add_category("Jan");
        data.add_category("Feb");
        data.add_category("Mar");
        data.add_series("S", ComboSeriesType::Bar, &[1.0, 2.0, 3.0]);

        let xml = ChartXmlWriter::write_combo(&data).unwrap();

        assert!(xml.contains("<c:v>Jan</c:v>"));
        assert!(xml.contains("<c:v>Feb</c:v>"));
        assert!(xml.contains("<c:v>Mar</c:v>"));
        assert!(xml.contains("<c:ptCount val=\"3\"/>"));
    }

    #[test]
    fn test_combo_chart_xml_escape() {
        let mut data = ComboChartData::new();
        data.add_category("A & B");
        data.add_series("Sales & Revenue", ComboSeriesType::Bar, &[1.0]);

        let xml = ChartXmlWriter::write_combo(&data).unwrap();

        assert!(xml.contains("Sales &amp; Revenue"));
        assert!(xml.contains("A &amp; B"));
    }

    #[test]
    fn test_combo_chart_to_xml_method() {
        let mut data = ComboChartData::new();
        data.add_category("X");
        data.add_series("Bar", ComboSeriesType::Bar, &[10.0]);
        data.add_series("Line", ComboSeriesType::Line, &[20.0]);

        let xml = data.to_xml().unwrap();

        assert!(xml.contains("<c:barChart>"));
        assert!(xml.contains("<c:lineChart>"));
    }

    #[test]
    fn test_combo_chart_legend() {
        let mut data = ComboChartData::new();
        data.add_category("X");
        data.add_series("S", ComboSeriesType::Bar, &[1.0]);

        let xml = ChartXmlWriter::write_combo(&data).unwrap();

        assert!(xml.contains("<c:legend>"));
        assert!(xml.contains("<c:legendPos val=\"b\"/>"));
    }
}
