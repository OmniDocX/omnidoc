use super::*;

#[test]
fn test_grouping_for_3d_types() {
    assert_eq!(
        grouping_for(XlChartType::BarClustered3D),
        Some(ChartGrouping::Clustered)
    );
    assert_eq!(
        grouping_for(XlChartType::ColumnStacked3D),
        Some(ChartGrouping::Stacked)
    );
    assert_eq!(
        grouping_for(XlChartType::BarStacked100_3D),
        Some(ChartGrouping::PercentStacked)
    );
    assert_eq!(
        grouping_for(XlChartType::Line3D),
        Some(ChartGrouping::Standard)
    );
    assert_eq!(
        grouping_for(XlChartType::Area3D),
        Some(ChartGrouping::Standard)
    );
    assert_eq!(
        grouping_for(XlChartType::AreaStacked3D),
        Some(ChartGrouping::Stacked)
    );
}

#[test]
fn test_bar_direction_for_3d() {
    assert_eq!(
        bar_direction_for(XlChartType::BarClustered3D),
        Some(BarDirection::Bar)
    );
    assert_eq!(
        bar_direction_for(XlChartType::ColumnClustered3D),
        Some(BarDirection::Column)
    );
}

#[test]
fn test_needs_overlap_3d() {
    assert!(needs_overlap(XlChartType::BarStacked3D));
    assert!(needs_overlap(XlChartType::ColumnStacked3D));
    assert!(!needs_overlap(XlChartType::BarClustered3D));
}

#[test]
fn test_plot_properties_defaults() {
    let pp = PlotProperties::new();
    assert_eq!(pp.gap_width(), None);
    assert_eq!(pp.overlap(), None);
    assert_eq!(pp.vary_by_categories(), None);
}

#[test]
fn test_plot_properties_setters() {
    let mut pp = PlotProperties::new();
    pp.set_gap_width(Some(200));
    pp.set_overlap(Some(-50));
    pp.set_vary_by_categories(Some(true));
    assert_eq!(pp.gap_width(), Some(200));
    assert_eq!(pp.overlap(), Some(-50));
    assert_eq!(pp.vary_by_categories(), Some(true));
}

#[test]
fn test_bubble_scale_property() {
    let mut pp = PlotProperties::new();
    assert_eq!(pp.bubble_scale(), None);
    pp.set_bubble_scale(Some(150));
    assert_eq!(pp.bubble_scale(), Some(150));
    pp.set_bubble_scale(None);
    assert_eq!(pp.bubble_scale(), None);
}

#[test]
fn test_grouping_for_cone_types() {
    assert_eq!(
        grouping_for(XlChartType::ConeBarClustered),
        Some(ChartGrouping::Clustered)
    );
    assert_eq!(
        grouping_for(XlChartType::ConeColClustered),
        Some(ChartGrouping::Clustered)
    );
    assert_eq!(
        grouping_for(XlChartType::ConeBarStacked),
        Some(ChartGrouping::Stacked)
    );
    assert_eq!(
        grouping_for(XlChartType::ConeColStacked),
        Some(ChartGrouping::Stacked)
    );
    assert_eq!(
        grouping_for(XlChartType::ConeBarStacked100),
        Some(ChartGrouping::PercentStacked)
    );
    assert_eq!(
        grouping_for(XlChartType::ConeColStacked100),
        Some(ChartGrouping::PercentStacked)
    );
    assert_eq!(
        grouping_for(XlChartType::ConeCol),
        Some(ChartGrouping::Clustered)
    );
}

#[test]
fn test_grouping_for_cylinder_types() {
    assert_eq!(
        grouping_for(XlChartType::CylinderBarClustered),
        Some(ChartGrouping::Clustered)
    );
    assert_eq!(
        grouping_for(XlChartType::CylinderColStacked),
        Some(ChartGrouping::Stacked)
    );
    assert_eq!(
        grouping_for(XlChartType::CylinderBarStacked100),
        Some(ChartGrouping::PercentStacked)
    );
    assert_eq!(
        grouping_for(XlChartType::CylinderCol),
        Some(ChartGrouping::Clustered)
    );
}

#[test]
fn test_grouping_for_pyramid_types() {
    assert_eq!(
        grouping_for(XlChartType::PyramidBarClustered),
        Some(ChartGrouping::Clustered)
    );
    assert_eq!(
        grouping_for(XlChartType::PyramidColStacked),
        Some(ChartGrouping::Stacked)
    );
    assert_eq!(
        grouping_for(XlChartType::PyramidBarStacked100),
        Some(ChartGrouping::PercentStacked)
    );
    assert_eq!(
        grouping_for(XlChartType::PyramidCol),
        Some(ChartGrouping::Clustered)
    );
}

#[test]
fn test_bar_direction_for_shaped_types() {
    assert_eq!(
        bar_direction_for(XlChartType::ConeBarClustered),
        Some(BarDirection::Bar)
    );
    assert_eq!(
        bar_direction_for(XlChartType::ConeColClustered),
        Some(BarDirection::Column)
    );
    assert_eq!(
        bar_direction_for(XlChartType::CylinderBarStacked),
        Some(BarDirection::Bar)
    );
    assert_eq!(
        bar_direction_for(XlChartType::CylinderCol),
        Some(BarDirection::Column)
    );
    assert_eq!(
        bar_direction_for(XlChartType::PyramidBarStacked100),
        Some(BarDirection::Bar)
    );
    assert_eq!(
        bar_direction_for(XlChartType::PyramidColStacked100),
        Some(BarDirection::Column)
    );
}

#[test]
fn test_needs_overlap_shaped_types() {
    assert!(needs_overlap(XlChartType::ConeBarStacked));
    assert!(needs_overlap(XlChartType::ConeColStacked100));
    assert!(needs_overlap(XlChartType::CylinderBarStacked));
    assert!(needs_overlap(XlChartType::CylinderColStacked100));
    assert!(needs_overlap(XlChartType::PyramidBarStacked));
    assert!(needs_overlap(XlChartType::PyramidColStacked100));
    assert!(!needs_overlap(XlChartType::ConeBarClustered));
    assert!(!needs_overlap(XlChartType::CylinderCol));
    assert!(!needs_overlap(XlChartType::PyramidColClustered));
}
