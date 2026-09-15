//! Surface chart XML writer.

use std::fmt::Write;

use crate::enums::chart::XlChartType;
use crate::oxml::ns::{NS_A, NS_C, NS_R};

use super::super::data::CategoryChartData;
use super::ChartXmlWriter;

impl ChartXmlWriter {
    #[allow(clippy::too_many_lines)]
    pub(super) fn write_surface_chart(data: &CategoryChartData, chart_type: XlChartType) -> String {
        let is_wireframe = matches!(
            chart_type,
            XlChartType::SurfaceWireframe | XlChartType::SurfaceTopWireframe
        );
        let is_top_view = matches!(
            chart_type,
            XlChartType::SurfaceTop | XlChartType::SurfaceTopWireframe
        );
        let is_3d = !is_top_view;
        let xml_tag = if is_top_view {
            "surfaceChart"
        } else {
            "surface3DChart"
        };

        let mut xml = String::with_capacity(8192);
        write_str!(
            xml,
            "<?xml version='1.0' encoding='UTF-8' standalone='yes'?>\n\
             <c:chartSpace xmlns:c=\"{NS_C}\" xmlns:a=\"{NS_A}\" xmlns:r=\"{NS_R}\">\n\
             \x20 <c:date1904 val=\"0\"/>\n\
             \x20 <c:chart>\n"
        );
        if is_3d {
            Self::view3d_to(&mut xml, 15, 20, 0);
        }
        write_str!(
            xml,
            "\x20   <c:autoTitleDeleted val=\"0\"/>\n\
             \x20   <c:plotArea>\n\
             \x20     <c:layout/>\n\
             \x20     <c:{xml_tag}>\n"
        );
        if is_wireframe {
            xml.push_str("\x20       <c:wireframe val=\"1\"/>\n");
        }
        Self::category_series_to(&mut xml, data);
        // Band formats for filled (non-wireframe) surface charts
        if !is_wireframe && data.series().len() > 1 {
            let count = data.series().len() - 1;
            xml.push_str("\x20       <c:bandFmts>\n");
            for i in 0..count {
                write_str!(
                    xml,
                    "\x20         <c:bandFmt>\n\
                     \x20           <c:idx val=\"{i}\"/>\n\
                     \x20         </c:bandFmt>\n"
                );
            }
            xml.push_str("\x20       </c:bandFmts>\n");
        }
        xml.push_str(
            "\x20       <c:axId val=\"-2068027336\"/>\n\
             \x20       <c:axId val=\"-2113994440\"/>\n",
        );
        if is_3d {
            xml.push_str("\x20       <c:axId val=\"-2082876632\"/>\n");
        }
        write_str!(xml, "\x20     </c:{xml_tag}>\n");
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
             \x20     </c:valAx>\n",
        );
        if is_3d {
            xml.push_str(
                "\x20     <c:serAx>\n\
                 \x20       <c:axId val=\"-2082876632\"/>\n\
                 \x20       <c:scaling>\n\
                 \x20         <c:orientation val=\"minMax\"/>\n\
                 \x20       </c:scaling>\n\
                 \x20       <c:delete val=\"0\"/>\n\
                 \x20       <c:axPos val=\"b\"/>\n\
                 \x20       <c:majorTickMark val=\"out\"/>\n\
                 \x20       <c:minorTickMark val=\"none\"/>\n\
                 \x20       <c:tickLblPos val=\"nextTo\"/>\n\
                 \x20       <c:crossAx val=\"-2113994440\"/>\n\
                 \x20       <c:crosses val=\"autoZero\"/>\n\
                 \x20     </c:serAx>\n",
            );
        }
        xml.push_str(
            "\x20   </c:plotArea>\n\
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
    fn test_surface_chart_xml() {
        let mut data = CategoryChartData::new();
        data.add_category("A");
        data.add_category("B");
        data.add_series("S1", &[1.0, 2.0]);
        data.add_series("S2", &[3.0, 4.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::Surface).unwrap();

        assert!(xml.contains("<c:surface3DChart>"));
        assert!(xml.contains("</c:surface3DChart>"));
        assert!(xml.contains("<c:view3D>"));
        assert!(!xml.contains("<c:wireframe"));
        assert!(xml.contains("<c:serAx>"));
        assert!(xml.contains("</c:serAx>"));
        assert!(xml.contains("<c:bandFmts>"));
        assert!(xml.contains("<c:bandFmt>"));
    }

    #[test]
    fn test_surface_chart_3d_axis_ids() {
        let mut data = CategoryChartData::new();
        data.add_category("A");
        data.add_series("S1", &[1.0]);
        data.add_series("S2", &[2.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::Surface).unwrap();

        let chart_content = xml
            .split("<c:surface3DChart>")
            .nth(1)
            .unwrap()
            .split("</c:surface3DChart>")
            .next()
            .unwrap();
        let ax_id_count = chart_content.matches("<c:axId").count();
        assert_eq!(ax_id_count, 3, "surface3DChart should reference 3 axis IDs");
    }

    #[test]
    fn test_surface_wireframe_chart_xml() {
        let mut data = CategoryChartData::new();
        data.add_category("A");
        data.add_series("S", &[1.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::SurfaceWireframe).unwrap();

        assert!(xml.contains("<c:wireframe val=\"1\"/>"));
        assert!(xml.contains("<c:view3D>"));
        assert!(xml.contains("<c:serAx>"));
        assert!(!xml.contains("<c:bandFmts>"));
    }

    #[test]
    fn test_surface_top_chart_xml() {
        let mut data = CategoryChartData::new();
        data.add_category("A");
        data.add_series("S", &[1.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::SurfaceTop).unwrap();

        assert!(xml.contains("<c:surfaceChart>"));
        assert!(xml.contains("</c:surfaceChart>"));
        assert!(!xml.contains("<c:view3D>"));
        assert!(!xml.contains("<c:serAx>"));
    }

    #[test]
    fn test_surface_top_chart_axis_ids() {
        let mut data = CategoryChartData::new();
        data.add_category("A");
        data.add_series("S", &[1.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::SurfaceTop).unwrap();

        let chart_content = xml
            .split("<c:surfaceChart>")
            .nth(1)
            .unwrap()
            .split("</c:surfaceChart>")
            .next()
            .unwrap();
        let ax_id_count = chart_content.matches("<c:axId").count();
        assert_eq!(
            ax_id_count, 2,
            "surfaceChart (top view) should reference 2 axis IDs"
        );
    }

    #[test]
    fn test_surface_top_wireframe_chart_xml() {
        let mut data = CategoryChartData::new();
        data.add_category("A");
        data.add_series("S", &[1.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::SurfaceTopWireframe).unwrap();

        assert!(xml.contains("<c:surfaceChart>"));
        assert!(xml.contains("<c:wireframe val=\"1\"/>"));
        assert!(!xml.contains("<c:view3D>"));
        assert!(!xml.contains("<c:serAx>"));
    }
}
