//! Tests for 3D category chart XML writers and special shapes.

#[cfg(test)]
mod tests {
    use crate::chart::data::CategoryChartData;
    use crate::chart::xmlwriter::ChartXmlWriter;
    use crate::enums::chart::XlChartType;

    #[test]
    fn test_bar_3d_chart_xml() {
        let mut data = CategoryChartData::new();
        data.add_category("A");
        data.add_series("S1", &[10.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::ColumnClustered3D).unwrap();

        assert!(xml.contains("<c:bar3DChart>"));
        assert!(xml.contains("</c:bar3DChart>"));
        assert!(xml.contains("<c:barDir val=\"col\"/>"));
        assert!(xml.contains("<c:grouping val=\"clustered\"/>"));
        assert!(xml.contains("<c:view3D>"));
        assert!(xml.contains("<c:rotX"));
        assert!(xml.contains("<c:rotY"));
        assert!(xml.contains("<c:perspective"));
    }

    #[test]
    fn test_bar_3d_horizontal_chart() {
        let mut data = CategoryChartData::new();
        data.add_category("A");
        data.add_series("S1", &[10.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::BarClustered3D).unwrap();

        assert!(xml.contains("<c:bar3DChart>"));
        assert!(xml.contains("<c:barDir val=\"bar\"/>"));
    }

    #[test]
    fn test_bar_3d_stacked_chart() {
        let mut data = CategoryChartData::new();
        data.add_category("A");
        data.add_series("S1", &[10.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::ColumnStacked3D).unwrap();

        assert!(xml.contains("<c:bar3DChart>"));
        assert!(xml.contains("<c:grouping val=\"stacked\"/>"));
        assert!(xml.contains("<c:overlap val=\"100\"/>"));
    }

    #[test]
    fn test_line_3d_chart_xml() {
        let mut data = CategoryChartData::new();
        data.add_category("A");
        data.add_category("B");
        data.add_series("S", &[1.0, 2.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::Line3D).unwrap();

        assert!(xml.contains("<c:line3DChart>"));
        assert!(xml.contains("</c:line3DChart>"));
        assert!(xml.contains("<c:view3D>"));
        assert!(xml.contains("<c:grouping val=\"standard\"/>"));
        assert!(!xml.contains("<c:marker val=\"1\"/>"));
    }

    #[test]
    fn test_pie_3d_chart_xml() {
        let mut data = CategoryChartData::new();
        data.add_category("A");
        data.add_category("B");
        data.add_series("S", &[60.0, 40.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::Pie3D).unwrap();

        assert!(xml.contains("<c:pie3DChart>"));
        assert!(xml.contains("</c:pie3DChart>"));
        assert!(xml.contains("<c:view3D>"));
        assert!(!xml.contains("<c:explosion"));
    }

    #[test]
    fn test_exploded_pie_3d_chart_xml() {
        let mut data = CategoryChartData::new();
        data.add_category("A");
        data.add_series("S", &[1.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::ExplodedPie3D).unwrap();

        assert!(xml.contains("<c:pie3DChart>"));
        assert!(xml.contains("<c:explosion val=\"25\"/>"));
        assert!(xml.contains("<c:view3D>"));
    }

    #[test]
    fn test_area_3d_chart_xml() {
        let mut data = CategoryChartData::new();
        data.add_category("X");
        data.add_series("S", &[5.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::Area3D).unwrap();

        assert!(xml.contains("<c:area3DChart>"));
        assert!(xml.contains("</c:area3DChart>"));
        assert!(xml.contains("<c:view3D>"));
        assert!(xml.contains("<c:grouping val=\"standard\"/>"));
    }

    #[test]
    fn test_area_stacked_3d_chart_xml() {
        let mut data = CategoryChartData::new();
        data.add_category("X");
        data.add_series("S", &[5.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::AreaStacked3D).unwrap();

        assert!(xml.contains("<c:area3DChart>"));
        assert!(xml.contains("<c:grouping val=\"stacked\"/>"));
    }

    // -----------------------------------------------------------------------
    // Cone / Cylinder / Pyramid chart tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_cone_bar_clustered_chart_xml() {
        let mut data = CategoryChartData::new();
        data.add_category("A");
        data.add_series("S1", &[10.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::ConeBarClustered).unwrap();

        assert!(xml.contains("<c:bar3DChart>"));
        assert!(xml.contains("<c:barDir val=\"bar\"/>"));
        assert!(xml.contains("<c:grouping val=\"clustered\"/>"));
        assert!(xml.contains("<c:shape val=\"cone\"/>"));
        assert!(xml.contains("<c:view3D>"));
        assert!(!xml.contains("<c:overlap"));
    }

    #[test]
    fn test_cone_col_stacked_chart_xml() {
        let mut data = CategoryChartData::new();
        data.add_category("A");
        data.add_series("S1", &[10.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::ConeColStacked).unwrap();

        assert!(xml.contains("<c:bar3DChart>"));
        assert!(xml.contains("<c:barDir val=\"col\"/>"));
        assert!(xml.contains("<c:grouping val=\"stacked\"/>"));
        assert!(xml.contains("<c:shape val=\"cone\"/>"));
        assert!(xml.contains("<c:overlap val=\"100\"/>"));
    }

    #[test]
    fn test_cylinder_col_clustered_chart_xml() {
        let mut data = CategoryChartData::new();
        data.add_category("A");
        data.add_series("S1", &[10.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::CylinderColClustered).unwrap();

        assert!(xml.contains("<c:bar3DChart>"));
        assert!(xml.contains("<c:barDir val=\"col\"/>"));
        assert!(xml.contains("<c:grouping val=\"clustered\"/>"));
        assert!(xml.contains("<c:shape val=\"cylinder\"/>"));
        assert!(xml.contains("<c:view3D>"));
    }

    #[test]
    fn test_cylinder_bar_stacked100_chart_xml() {
        let mut data = CategoryChartData::new();
        data.add_category("A");
        data.add_series("S1", &[10.0]);

        let xml =
            ChartXmlWriter::write_category(&data, XlChartType::CylinderBarStacked100).unwrap();

        assert!(xml.contains("<c:bar3DChart>"));
        assert!(xml.contains("<c:barDir val=\"bar\"/>"));
        assert!(xml.contains("<c:grouping val=\"percentStacked\"/>"));
        assert!(xml.contains("<c:shape val=\"cylinder\"/>"));
        assert!(xml.contains("<c:overlap val=\"100\"/>"));
    }

    #[test]
    fn test_pyramid_col_chart_xml() {
        let mut data = CategoryChartData::new();
        data.add_category("A");
        data.add_series("S1", &[10.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::PyramidCol).unwrap();

        assert!(xml.contains("<c:bar3DChart>"));
        assert!(xml.contains("<c:barDir val=\"col\"/>"));
        assert!(xml.contains("<c:shape val=\"pyramid\"/>"));
        assert!(xml.contains("<c:view3D>"));
    }

    #[test]
    fn test_pyramid_bar_stacked_chart_xml() {
        let mut data = CategoryChartData::new();
        data.add_category("A");
        data.add_series("S1", &[10.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::PyramidBarStacked).unwrap();

        assert!(xml.contains("<c:bar3DChart>"));
        assert!(xml.contains("<c:barDir val=\"bar\"/>"));
        assert!(xml.contains("<c:grouping val=\"stacked\"/>"));
        assert!(xml.contains("<c:shape val=\"pyramid\"/>"));
        assert!(xml.contains("<c:overlap val=\"100\"/>"));
    }

    #[test]
    fn test_regular_bar_chart_has_no_shape() {
        let mut data = CategoryChartData::new();
        data.add_category("A");
        data.add_series("S1", &[10.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::BarClustered3D).unwrap();

        assert!(xml.contains("<c:bar3DChart>"));
        assert!(!xml.contains("<c:shape"));
    }
}
