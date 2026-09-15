//! Shared XML helper methods for chart XML generation.

use std::fmt::Write;

use super::super::data::{CategoryChartData, CategorySeriesData};
use super::{xml_escape_to, ChartXmlWriter};

impl ChartXmlWriter {
    /// Write `<c:tx>` element XML for a series name.
    pub(super) fn tx_to(w: &mut String, name: &str) {
        w.push_str(
            "\x20         <c:tx>\n\
             \x20           <c:strRef>\n\
             \x20             <c:f>Sheet1!$B$1</c:f>\n\
             \x20             <c:strCache>\n\
             \x20               <c:ptCount val=\"1\"/>\n\
             \x20               <c:pt idx=\"0\">\n\
             \x20                 <c:v>",
        );
        xml_escape_to(w, name);
        w.push_str(
            "</c:v>\n\
             \x20               </c:pt>\n\
             \x20             </c:strCache>\n\
             \x20           </c:strRef>\n\
             \x20         </c:tx>\n",
        );
    }

    /// Write `<c:cat>` element XML for category labels.
    pub(super) fn cat_to(w: &mut String, data: &CategoryChartData) {
        // If hierarchical categories are present, emit multi-level category XML
        if let Some(levels) = data.hierarchical_categories() {
            if levels.len() > 1 {
                Self::multi_level_cat_to(w, levels);
                return;
            }
        }

        let categories = data.categories();
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
            xml_escape_to(w, cat);
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

    /// Write multi-level `<c:cat>` XML for hierarchical categories.
    fn multi_level_cat_to(w: &mut String, levels: &[Vec<String>]) {
        let count = levels.first().map_or(0, Vec::len);

        write_str!(
            w,
            "\x20         <c:cat>\n\
             \x20           <c:multiLvlStrRef>\n\
             \x20             <c:f>Sheet1!$A$2:$A${}</c:f>\n\
             \x20             <c:multiLvlStrCache>\n\
             \x20               <c:ptCount val=\"{count}\"/>\n",
            count + 1
        );
        for level in levels {
            w.push_str("\x20                 <c:lvl>\n");
            for (idx, label) in level.iter().enumerate() {
                write_str!(
                    w,
                    "\x20                   <c:pt idx=\"{idx}\">\n\
                     \x20                     <c:v>"
                );
                xml_escape_to(w, label);
                w.push_str(
                    "</c:v>\n\
                     \x20                   </c:pt>\n",
                );
            }
            w.push_str("\x20                 </c:lvl>\n");
        }
        w.push_str(
            "\x20             </c:multiLvlStrCache>\n\
             \x20           </c:multiLvlStrRef>\n\
             \x20         </c:cat>\n",
        );
    }

    /// Write `<c:val>` element XML for series values.
    pub(super) fn val_to(w: &mut String, series: &CategorySeriesData) {
        let values = series.values();
        let count = values.len();
        let number_format = series.number_format().unwrap_or("General");

        write_str!(
            w,
            "\x20         <c:val>\n\
             \x20           <c:numRef>\n\
             \x20             <c:f>Sheet1!$B$2:$B${}</c:f>\n\
             \x20             <c:numCache>\n\
             \x20               <c:formatCode>{number_format}</c:formatCode>\n\
             \x20               <c:ptCount val=\"{count}\"/>\n",
            count + 1
        );
        for (idx, value) in values.iter().enumerate() {
            if let Some(v) = value {
                write_str!(
                    w,
                    "\x20               <c:pt idx=\"{idx}\">\n\
                     \x20                 <c:v>{v}</c:v>\n\
                     \x20               </c:pt>\n"
                );
            }
        }
        w.push_str(
            "\x20             </c:numCache>\n\
             \x20           </c:numRef>\n\
             \x20         </c:val>\n",
        );
    }

    /// Write `<c:xVal>`, `<c:yVal>`, or `<c:bubbleSize>` element XML
    /// for numeric data.
    pub(super) fn num_val_to(w: &mut String, tag: &str, values: &[f64], number_format: &str) {
        let count = values.len();

        write_str!(
            w,
            "\x20         <c:{tag}>\n\
             \x20           <c:numRef>\n\
             \x20             <c:f>Sheet1!$A$2:$A${}</c:f>\n\
             \x20             <c:numCache>\n\
             \x20               <c:formatCode>{number_format}</c:formatCode>\n\
             \x20               <c:ptCount val=\"{count}\"/>\n",
            count + 1
        );
        for (idx, v) in values.iter().enumerate() {
            write_str!(
                w,
                "\x20               <c:pt idx=\"{idx}\">\n\
                 \x20                 <c:v>{v}</c:v>\n\
                 \x20               </c:pt>\n"
            );
        }
        write_str!(
            w,
            "\x20             </c:numCache>\n\
             \x20           </c:numRef>\n\
             \x20         </c:{tag}>\n"
        );
    }

    /// Write series XML for all category series.
    pub(super) fn category_series_to(w: &mut String, data: &CategoryChartData) {
        for series in data.series() {
            let idx = series.index();
            write_str!(
                w,
                "\x20       <c:ser>\n\
                 \x20         <c:idx val=\"{idx}\"/>\n\
                 \x20         <c:order val=\"{idx}\"/>\n"
            );
            Self::tx_to(w, series.name());
            Self::cat_to(w, data);
            Self::val_to(w, series);
            w.push_str("\x20       </c:ser>\n");
        }
    }

    /// Write series XML with explosion element (for pie/doughnut).
    pub(super) fn category_series_with_explosion_to(
        w: &mut String,
        data: &CategoryChartData,
        explosion_xml: &str,
    ) {
        for series in data.series() {
            let idx = series.index();
            write_str!(
                w,
                "\x20       <c:ser>\n\
                 \x20         <c:idx val=\"{idx}\"/>\n\
                 \x20         <c:order val=\"{idx}\"/>\n"
            );
            Self::tx_to(w, series.name());
            w.push_str(explosion_xml);
            Self::cat_to(w, data);
            Self::val_to(w, series);
            w.push_str("\x20       </c:ser>\n");
        }
    }

    /// Write `<c:catAx>` element.
    pub(super) fn cat_ax_to(w: &mut String, cat_ax_id: &str, val_ax_id: &str, position: &str) {
        write_str!(
            w,
            "\x20     <c:catAx>\n\
             \x20       <c:axId val=\"{cat_ax_id}\"/>\n\
             \x20       <c:scaling>\n\
             \x20         <c:orientation val=\"minMax\"/>\n\
             \x20       </c:scaling>\n\
             \x20       <c:delete val=\"0\"/>\n\
             \x20       <c:axPos val=\"{position}\"/>\n\
             \x20       <c:majorTickMark val=\"out\"/>\n\
             \x20       <c:minorTickMark val=\"none\"/>\n\
             \x20       <c:tickLblPos val=\"nextTo\"/>\n\
             \x20       <c:crossAx val=\"{val_ax_id}\"/>\n\
             \x20       <c:crosses val=\"autoZero\"/>\n\
             \x20       <c:auto val=\"1\"/>\n\
             \x20       <c:lblAlgn val=\"ctr\"/>\n\
             \x20       <c:lblOffset val=\"100\"/>\n\
             \x20       <c:noMultiLvlLbl val=\"0\"/>\n\
             \x20     </c:catAx>\n"
        );
    }

    /// Write `<c:view3D>` element for 3D charts.
    pub(super) fn view3d_to(w: &mut String, rot_x: i32, rot_y: i32, perspective: i32) {
        write_str!(
            w,
            "\x20   <c:view3D>\n\
             \x20     <c:rotX val=\"{rot_x}\"/>\n\
             \x20     <c:rotY val=\"{rot_y}\"/>\n\
             \x20     <c:perspective val=\"{perspective}\"/>\n\
             \x20   </c:view3D>\n"
        );
    }

    /// Write `<c:txPr>` (text properties) element for the chart space.
    pub(super) fn txpr_to(w: &mut String) {
        w.push_str(
            "\x20 <c:txPr>\n\
             \x20   <a:bodyPr/>\n\
             \x20   <a:lstStyle/>\n\
             \x20   <a:p>\n\
             \x20     <a:pPr>\n\
             \x20       <a:defRPr sz=\"1800\"/>\n\
             \x20     </a:pPr>\n\
             \x20     <a:endParaRPr lang=\"en-US\"/>\n\
             \x20   </a:p>\n\
             \x20 </c:txPr>\n",
        );
    }
}
