//! Tests for category chart XML writers (2D charts).

#[cfg(test)]
mod tests {
    use crate::chart::data::CategoryChartData;
    use crate::chart::xmlwriter::ChartXmlWriter;
    use crate::enums::chart::XlChartType;

    #[test]
    fn test_bar_chart_xml() {
        let mut data = CategoryChartData::new();
        data.add_category("Q1");
        data.add_category("Q2");
        data.add_series("Sales", &[100.0, 150.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::ColumnClustered).unwrap();

        assert!(xml.contains("<c:chartSpace"));
        assert!(xml.contains("<c:barChart>"));
        assert!(xml.contains("<c:barDir val=\"col\"/>"));
        assert!(xml.contains("<c:grouping val=\"clustered\"/>"));
        assert!(xml.contains("<c:v>Q1</c:v>"));
        assert!(xml.contains("<c:v>Q2</c:v>"));
        assert!(xml.contains("<c:v>100</c:v>"));
        assert!(xml.contains("<c:v>150</c:v>"));
        assert!(xml.contains("</c:chartSpace>"));
    }

    #[test]
    fn test_bar_stacked_chart() {
        let mut data = CategoryChartData::new();
        data.add_category("A");
        data.add_series("S1", &[10.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::BarStacked).unwrap();

        assert!(xml.contains("<c:barDir val=\"bar\"/>"));
        assert!(xml.contains("<c:grouping val=\"stacked\"/>"));
        assert!(xml.contains("<c:overlap val=\"100\"/>"));
    }

    #[test]
    fn test_line_chart_xml() {
        let mut data = CategoryChartData::new();
        data.add_category("Jan");
        data.add_category("Feb");
        data.add_series("Revenue", &[200.0, 300.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::Line).unwrap();

        assert!(xml.contains("<c:lineChart>"));
        assert!(xml.contains("<c:grouping val=\"standard\"/>"));
        assert!(xml.contains("<c:symbol val=\"none\"/>"));
    }

    #[test]
    fn test_pie_chart_xml() {
        let mut data = CategoryChartData::new();
        data.add_category("Apples");
        data.add_category("Oranges");
        data.add_series("Fruit", &[60.0, 40.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::Pie).unwrap();

        assert!(xml.contains("<c:pieChart>"));
        assert!(xml.contains("<c:varyColors val=\"1\"/>"));
        assert!(!xml.contains("<c:explosion"));
    }

    #[test]
    fn test_pie_exploded_chart_xml() {
        let mut data = CategoryChartData::new();
        data.add_category("A");
        data.add_series("S1", &[1.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::PieExploded).unwrap();

        assert!(xml.contains("<c:explosion val=\"25\"/>"));
    }

    #[test]
    fn test_xml_escape_in_series_name() {
        let mut data = CategoryChartData::new();
        data.add_category("A");
        data.add_series("Sales & Revenue", &[100.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::ColumnClustered).unwrap();

        assert!(xml.contains("Sales &amp; Revenue"));
    }

    #[test]
    fn test_area_chart_xml() {
        let mut data = CategoryChartData::new();
        data.add_category("X");
        data.add_series("S", &[5.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::Area).unwrap();

        assert!(xml.contains("<c:areaChart>"));
        assert!(xml.contains("<c:grouping val=\"standard\"/>"));
    }

    #[test]
    fn test_doughnut_chart_xml() {
        let mut data = CategoryChartData::new();
        data.add_category("X");
        data.add_series("S", &[5.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::Doughnut).unwrap();

        assert!(xml.contains("<c:doughnutChart>"));
        assert!(xml.contains("<c:holeSize val=\"50\"/>"));
    }

    #[test]
    fn test_radar_chart_xml() {
        let mut data = CategoryChartData::new();
        data.add_category("Speed");
        data.add_category("Power");
        data.add_series("Rating", &[8.0, 6.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::RadarFilled).unwrap();

        assert!(xml.contains("<c:radarChart>"));
        assert!(xml.contains("<c:radarStyle val=\"filled\"/>"));
    }

    #[test]
    fn test_hierarchical_categories_xml() {
        let mut data = CategoryChartData::new();
        data.set_hierarchical_categories(vec![
            vec!["Q1".into(), "Q2".into(), "Q3".into(), "Q4".into()],
            vec!["H1".into(), "H1".into(), "H2".into(), "H2".into()],
        ]);
        data.add_series("Sales", &[10.0, 20.0, 30.0, 40.0]);

        let xml = ChartXmlWriter::write_category(&data, XlChartType::ColumnClustered).unwrap();

        assert!(xml.contains("<c:multiLvlStrRef>"));
        assert!(xml.contains("<c:multiLvlStrCache>"));
        assert!(xml.contains("<c:lvl>"));
        assert!(xml.contains("<c:v>Q1</c:v>"));
        assert!(xml.contains("<c:v>H1</c:v>"));
        assert!(xml.contains("<c:v>H2</c:v>"));
    }
}
