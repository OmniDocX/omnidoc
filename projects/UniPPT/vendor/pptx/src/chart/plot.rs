//! Plot types for different chart categories.
//!
//! A plot (called a "chart group" in Microsoft's API) is a distinct sequence
//! of one or more series depicted in a particular chart type within a chart.

use crate::enums::chart::XlChartType;

/// Grouping mode for bar/column, line, and area charts.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartGrouping {
    Standard,
    Stacked,
    PercentStacked,
    Clustered,
}

impl ChartGrouping {
    /// Return the XML attribute value.
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Stacked => "stacked",
            Self::PercentStacked => "percentStacked",
            Self::Clustered => "clustered",
        }
    }
}

/// Bar direction for bar/column charts.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarDirection {
    Bar,
    Column,
}

impl BarDirection {
    /// Return the XML attribute value.
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::Bar => "bar",
            Self::Column => "col",
        }
    }
}

/// Radar chart style.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadarStyle {
    Marker,
    Filled,
}

impl RadarStyle {
    /// Return the XML attribute value.
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::Marker => "marker",
            Self::Filled => "filled",
        }
    }
}

/// Scatter chart style.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScatterStyle {
    LineMarker,
    SmoothMarker,
}

impl ScatterStyle {
    /// Return the XML attribute value.
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::LineMarker => "lineMarker",
            Self::SmoothMarker => "smoothMarker",
        }
    }
}

/// Plot-level properties for bar/column charts (`gap_width`, `overlap`, `vary_by_categories`).
#[derive(Debug, Clone)]
pub struct PlotProperties {
    gap_width: Option<u32>,
    overlap: Option<i32>,
    vary_by_categories: Option<bool>,
    bubble_scale: Option<u32>,
}

impl PlotProperties {
    /// Create with default (unset) properties.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            gap_width: None,
            overlap: None,
            vary_by_categories: None,
            bubble_scale: None,
        }
    }

    /// Gap width between bar/column groups, as a percentage (0-500).
    /// Default in OOXML is 150%.
    #[must_use]
    pub const fn gap_width(&self) -> Option<u32> {
        self.gap_width
    }

    /// Set the gap width.
    pub fn set_gap_width(&mut self, value: Option<u32>) {
        self.gap_width = value;
    }

    /// Overlap between bars in a group, as a percentage (-100 to 100).
    /// Positive values overlap; negative values add gaps.
    #[must_use]
    pub const fn overlap(&self) -> Option<i32> {
        self.overlap
    }

    /// Set the overlap.
    pub fn set_overlap(&mut self, value: Option<i32>) {
        self.overlap = value;
    }

    /// Whether to vary colors by category (point).
    #[must_use]
    pub const fn vary_by_categories(&self) -> Option<bool> {
        self.vary_by_categories
    }

    /// Set vary-by-categories.
    pub fn set_vary_by_categories(&mut self, value: Option<bool>) {
        self.vary_by_categories = value;
    }

    /// Bubble scale as a percentage (0-300). Default in OOXML is 100%.
    #[must_use]
    pub const fn bubble_scale(&self) -> Option<u32> {
        self.bubble_scale
    }

    /// Set the bubble scale.
    pub fn set_bubble_scale(&mut self, value: Option<u32>) {
        self.bubble_scale = value;
    }
}

/// Creates empty plot properties with no overlap, gap, or grouping.
impl Default for PlotProperties {
    fn default() -> Self {
        Self::new()
    }
}

/// Determine the grouping for a given chart type.
#[must_use]
#[allow(clippy::too_many_lines)]
pub const fn grouping_for(chart_type: XlChartType) -> Option<ChartGrouping> {
    match chart_type {
        XlChartType::BarClustered
        | XlChartType::ColumnClustered
        | XlChartType::BarClustered3D
        | XlChartType::ColumnClustered3D
        | XlChartType::ConeBarClustered
        | XlChartType::ConeColClustered
        | XlChartType::CylinderBarClustered
        | XlChartType::CylinderColClustered
        | XlChartType::PyramidBarClustered
        | XlChartType::PyramidColClustered
        | XlChartType::ConeCol
        | XlChartType::CylinderCol
        | XlChartType::PyramidCol => Some(ChartGrouping::Clustered),
        XlChartType::BarStacked
        | XlChartType::ColumnStacked
        | XlChartType::BarStacked3D
        | XlChartType::ColumnStacked3D
        | XlChartType::ConeBarStacked
        | XlChartType::ConeColStacked
        | XlChartType::CylinderBarStacked
        | XlChartType::CylinderColStacked
        | XlChartType::PyramidBarStacked
        | XlChartType::PyramidColStacked
        | XlChartType::LineStacked
        | XlChartType::LineMarkersStacked
        | XlChartType::AreaStacked
        | XlChartType::AreaStacked3D => Some(ChartGrouping::Stacked),
        XlChartType::BarStacked100
        | XlChartType::ColumnStacked100
        | XlChartType::BarStacked100_3D
        | XlChartType::ColumnStacked100_3D
        | XlChartType::ConeBarStacked100
        | XlChartType::ConeColStacked100
        | XlChartType::CylinderBarStacked100
        | XlChartType::CylinderColStacked100
        | XlChartType::PyramidBarStacked100
        | XlChartType::PyramidColStacked100
        | XlChartType::LineStacked100
        | XlChartType::LineMarkersStacked100
        | XlChartType::AreaStacked100
        | XlChartType::AreaStacked100_3D => Some(ChartGrouping::PercentStacked),
        XlChartType::Line
        | XlChartType::LineMarkers
        | XlChartType::Line3D
        | XlChartType::Area
        | XlChartType::Area3D => Some(ChartGrouping::Standard),
        _ => None,
    }
}

/// Determine the bar direction for a chart type.
#[must_use]
pub const fn bar_direction_for(chart_type: XlChartType) -> Option<BarDirection> {
    if chart_type.is_bar_type() {
        Some(BarDirection::Bar)
    } else if chart_type.is_column_type() {
        Some(BarDirection::Column)
    } else {
        None
    }
}

/// Determine the radar style for a chart type.
#[must_use]
pub const fn radar_style_for(chart_type: XlChartType) -> Option<RadarStyle> {
    match chart_type {
        XlChartType::RadarFilled => Some(RadarStyle::Filled),
        XlChartType::Radar | XlChartType::RadarMarkers => Some(RadarStyle::Marker),
        _ => None,
    }
}

/// Determine the scatter style for a chart type.
#[must_use]
pub const fn scatter_style_for(chart_type: XlChartType) -> Option<ScatterStyle> {
    match chart_type {
        XlChartType::XyScatterSmooth | XlChartType::XyScatterSmoothNoMarkers => {
            Some(ScatterStyle::SmoothMarker)
        }
        XlChartType::XyScatter
        | XlChartType::XyScatterLines
        | XlChartType::XyScatterLinesNoMarkers => Some(ScatterStyle::LineMarker),
        _ => None,
    }
}

/// Whether stacked overlap (100%) should be applied.
#[must_use]
pub const fn needs_overlap(chart_type: XlChartType) -> bool {
    matches!(
        chart_type,
        XlChartType::BarStacked
            | XlChartType::BarStacked100
            | XlChartType::ColumnStacked
            | XlChartType::ColumnStacked100
            | XlChartType::BarStacked3D
            | XlChartType::BarStacked100_3D
            | XlChartType::ColumnStacked3D
            | XlChartType::ColumnStacked100_3D
            | XlChartType::ConeBarStacked
            | XlChartType::ConeBarStacked100
            | XlChartType::ConeColStacked
            | XlChartType::ConeColStacked100
            | XlChartType::CylinderBarStacked
            | XlChartType::CylinderBarStacked100
            | XlChartType::CylinderColStacked
            | XlChartType::CylinderColStacked100
            | XlChartType::PyramidBarStacked
            | XlChartType::PyramidBarStacked100
            | XlChartType::PyramidColStacked
            | XlChartType::PyramidColStacked100
    )
}

#[cfg(test)]
#[path = "plot_tests.rs"]
mod tests;
