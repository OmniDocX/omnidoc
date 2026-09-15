//! Enumerations used by Chart objects.
//!
//! Types are defined in sub-modules and re-exported here for backwards
//! compatibility.

pub use super::chart_enums::{
    XlAxisCrosses, XlCategoryType, XlDataLabelPosition, XlLabelPosition, XlLegendPosition,
    XlMarkerStyle, XlTickLabelPosition, XlTickMark,
};
pub use super::chart_type::XlChartType;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legend_position_roundtrip() {
        let positions = [
            XlLegendPosition::Bottom,
            XlLegendPosition::Corner,
            XlLegendPosition::Left,
            XlLegendPosition::Right,
            XlLegendPosition::Top,
        ];
        for p in positions {
            let xml = p.to_xml_str();
            assert_eq!(XlLegendPosition::from_xml_str(xml), Some(p));
        }
    }

    #[test]
    fn test_marker_style_roundtrip() {
        let styles = [
            XlMarkerStyle::Automatic,
            XlMarkerStyle::Circle,
            XlMarkerStyle::Diamond,
            XlMarkerStyle::None,
            XlMarkerStyle::Square,
            XlMarkerStyle::Triangle,
        ];
        for s in styles {
            let xml = s.to_xml_str();
            assert_eq!(XlMarkerStyle::from_xml_str(xml), Some(s));
        }
    }

    #[test]
    fn test_data_label_position_roundtrip() {
        let positions = [
            XlDataLabelPosition::Above,
            XlDataLabelPosition::Below,
            XlDataLabelPosition::Center,
            XlDataLabelPosition::InsideEnd,
            XlDataLabelPosition::OutsideEnd,
        ];
        for p in positions {
            let xml = p.to_xml_str();
            assert_eq!(XlDataLabelPosition::from_xml_str(xml), Some(p));
        }
    }

    #[test]
    fn test_tick_mark_roundtrip() {
        let marks = [
            XlTickMark::Cross,
            XlTickMark::Inside,
            XlTickMark::None,
            XlTickMark::Outside,
        ];
        for m in marks {
            let xml = m.to_xml_str();
            assert_eq!(XlTickMark::from_xml_str(xml), Some(m));
        }
    }

    #[test]
    fn test_chart_type_classification() {
        assert!(XlChartType::BarClustered.is_bar_type());
        assert!(XlChartType::ColumnClustered.is_column_type());
        assert!(XlChartType::ColumnClustered.is_bar_or_column());
        assert!(XlChartType::Line.is_line_type());
        assert!(XlChartType::Pie.is_pie_type());
        assert!(XlChartType::Doughnut.is_doughnut_type());
        assert!(XlChartType::Area.is_area_type());
        assert!(XlChartType::XyScatter.is_xy_type());
        assert!(XlChartType::Bubble.is_bubble_type());
        assert!(XlChartType::Radar.is_radar_type());
        assert!(XlChartType::ColumnClustered.is_category_type());
        assert!(!XlChartType::XyScatter.is_category_type());
        assert!(!XlChartType::Bubble.is_category_type());
    }

    #[test]
    fn test_3d_type_classification() {
        assert!(XlChartType::BarClustered3D.is_3d_type());
        assert!(XlChartType::BarStacked3D.is_3d_type());
        assert!(XlChartType::BarStacked100_3D.is_3d_type());
        assert!(XlChartType::ColumnClustered3D.is_3d_type());
        assert!(XlChartType::ColumnStacked3D.is_3d_type());
        assert!(XlChartType::ColumnStacked100_3D.is_3d_type());
        assert!(XlChartType::Line3D.is_3d_type());
        assert!(XlChartType::Pie3D.is_3d_type());
        assert!(XlChartType::ExplodedPie3D.is_3d_type());
        assert!(XlChartType::Area3D.is_3d_type());
        assert!(XlChartType::AreaStacked3D.is_3d_type());
        assert!(XlChartType::AreaStacked100_3D.is_3d_type());
        // 2D types should not be 3D
        assert!(!XlChartType::BarClustered.is_3d_type());
        assert!(!XlChartType::Pie.is_3d_type());
        assert!(!XlChartType::Line.is_3d_type());
    }

    #[test]
    fn test_3d_types_are_also_base_types() {
        assert!(XlChartType::BarClustered3D.is_bar_type());
        assert!(XlChartType::BarClustered3D.is_bar_or_column());
        assert!(XlChartType::ColumnClustered3D.is_column_type());
        assert!(XlChartType::ColumnClustered3D.is_bar_or_column());
        assert!(XlChartType::Line3D.is_line_type());
        assert!(XlChartType::Pie3D.is_pie_type());
        assert!(XlChartType::ExplodedPie3D.is_pie_type());
        assert!(XlChartType::Area3D.is_area_type());
    }

    #[test]
    fn test_stock_type_classification() {
        assert!(XlChartType::StockHLC.is_stock_type());
        assert!(XlChartType::StockOHLC.is_stock_type());
        assert!(XlChartType::StockVHLC.is_stock_type());
        assert!(XlChartType::StockVOHLC.is_stock_type());
        assert!(!XlChartType::BarClustered.is_stock_type());
        assert!(XlChartType::StockHLC.is_category_type());
    }

    #[test]
    fn test_surface_type_classification() {
        assert!(XlChartType::Surface.is_surface_type());
        assert!(XlChartType::SurfaceWireframe.is_surface_type());
        assert!(XlChartType::SurfaceTop.is_surface_type());
        assert!(XlChartType::SurfaceTopWireframe.is_surface_type());
        assert!(!XlChartType::BarClustered.is_surface_type());
        assert!(XlChartType::Surface.is_category_type());
    }

    #[test]
    fn test_cone_type_classification() {
        assert!(XlChartType::ConeBarClustered.is_bar_type());
        assert!(XlChartType::ConeBarStacked.is_bar_type());
        assert!(XlChartType::ConeBarStacked100.is_bar_type());
        assert!(XlChartType::ConeBarClustered.is_bar_or_column());
        assert!(XlChartType::ConeCol.is_column_type());
        assert!(XlChartType::ConeColClustered.is_column_type());
        assert!(XlChartType::ConeColStacked.is_column_type());
        assert!(XlChartType::ConeColStacked100.is_column_type());
        assert!(XlChartType::ConeBarClustered.is_3d_type());
        assert!(XlChartType::ConeCol.is_3d_type());
        assert!(XlChartType::ConeColClustered.is_3d_type());
        assert!(XlChartType::ConeBarClustered.is_cone_type());
        assert!(!XlChartType::CylinderBarClustered.is_cone_type());
        assert_eq!(XlChartType::ConeBarClustered.chart_shape(), Some("cone"));
        assert_eq!(XlChartType::ConeCol.chart_shape(), Some("cone"));
        assert!(XlChartType::ConeBarClustered.is_category_type());
    }

    #[test]
    fn test_cylinder_type_classification() {
        assert!(XlChartType::CylinderBarClustered.is_bar_type());
        assert!(XlChartType::CylinderBarStacked.is_bar_type());
        assert!(XlChartType::CylinderBarStacked100.is_bar_type());
        assert!(XlChartType::CylinderCol.is_column_type());
        assert!(XlChartType::CylinderColClustered.is_column_type());
        assert!(XlChartType::CylinderColStacked.is_column_type());
        assert!(XlChartType::CylinderColStacked100.is_column_type());
        assert!(XlChartType::CylinderBarClustered.is_3d_type());
        assert!(XlChartType::CylinderCol.is_3d_type());
        assert!(XlChartType::CylinderBarClustered.is_cylinder_type());
        assert!(!XlChartType::ConeBarClustered.is_cylinder_type());
        assert_eq!(
            XlChartType::CylinderBarClustered.chart_shape(),
            Some("cylinder")
        );
        assert_eq!(
            XlChartType::CylinderColStacked.chart_shape(),
            Some("cylinder")
        );
    }

    #[test]
    fn test_pyramid_type_classification() {
        assert!(XlChartType::PyramidBarClustered.is_bar_type());
        assert!(XlChartType::PyramidBarStacked.is_bar_type());
        assert!(XlChartType::PyramidBarStacked100.is_bar_type());
        assert!(XlChartType::PyramidCol.is_column_type());
        assert!(XlChartType::PyramidColClustered.is_column_type());
        assert!(XlChartType::PyramidColStacked.is_column_type());
        assert!(XlChartType::PyramidColStacked100.is_column_type());
        assert!(XlChartType::PyramidBarClustered.is_3d_type());
        assert!(XlChartType::PyramidCol.is_3d_type());
        assert!(XlChartType::PyramidBarClustered.is_pyramid_type());
        assert!(!XlChartType::ConeBarClustered.is_pyramid_type());
        assert_eq!(
            XlChartType::PyramidBarClustered.chart_shape(),
            Some("pyramid")
        );
        assert_eq!(
            XlChartType::PyramidColStacked100.chart_shape(),
            Some("pyramid")
        );
    }

    #[test]
    fn test_combo_type_classification() {
        assert!(XlChartType::ColumnLineCombo.is_combo_type());
        assert!(XlChartType::ColumnLineCombo.is_category_type());
        assert!(!XlChartType::ColumnClustered.is_combo_type());
        assert!(!XlChartType::Line.is_combo_type());
    }

    #[test]
    fn test_regular_types_have_no_chart_shape() {
        assert_eq!(XlChartType::BarClustered.chart_shape(), None);
        assert_eq!(XlChartType::ColumnClustered.chart_shape(), None);
        assert_eq!(XlChartType::BarClustered3D.chart_shape(), None);
        assert_eq!(XlChartType::Line.chart_shape(), None);
        assert_eq!(XlChartType::Pie.chart_shape(), None);
    }
}
