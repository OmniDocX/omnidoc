//! Chart type enumeration and basic classification methods.

/// Specifies the type of a chart.
///
/// MS API Name: `XlChartType`
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum XlChartType {
    Area,
    AreaStacked,
    AreaStacked100,
    BarClustered,
    BarStacked,
    BarStacked100,
    Bubble,
    BubbleThreeDEffect,
    ColumnClustered,
    ColumnStacked,
    ColumnStacked100,
    Doughnut,
    DoughnutExploded,
    Line,
    LineMarkers,
    LineMarkersStacked,
    LineMarkersStacked100,
    LineStacked,
    LineStacked100,
    Pie,
    PieExploded,
    Radar,
    RadarFilled,
    RadarMarkers,
    XyScatter,
    XyScatterLines,
    XyScatterLinesNoMarkers,
    XyScatterSmooth,
    XyScatterSmoothNoMarkers,
    // 3D variants
    BarClustered3D,
    BarStacked3D,
    BarStacked100_3D,
    ColumnClustered3D,
    ColumnStacked3D,
    ColumnStacked100_3D,
    Line3D,
    Pie3D,
    ExplodedPie3D,
    Area3D,
    AreaStacked3D,
    AreaStacked100_3D,
    // Surface charts
    Surface,
    SurfaceWireframe,
    SurfaceTop,
    SurfaceTopWireframe,
    // Stock charts
    StockHLC,
    StockOHLC,
    StockVHLC,
    StockVOHLC,
    // Cone charts
    ConeBarClustered,
    ConeBarStacked,
    ConeBarStacked100,
    ConeCol,
    ConeColClustered,
    ConeColStacked,
    ConeColStacked100,
    // Cylinder charts
    CylinderBarClustered,
    CylinderBarStacked,
    CylinderBarStacked100,
    CylinderCol,
    CylinderColClustered,
    CylinderColStacked,
    CylinderColStacked100,
    // Pyramid charts
    PyramidBarClustered,
    PyramidBarStacked,
    PyramidBarStacked100,
    PyramidCol,
    PyramidColClustered,
    PyramidColStacked,
    PyramidColStacked100,
    // Combo charts
    ColumnLineCombo,
}

impl XlChartType {
    /// Returns true if this is a bar-direction chart type (horizontal bars).
    #[must_use]
    pub const fn is_bar_type(self) -> bool {
        matches!(
            self,
            Self::BarClustered
                | Self::BarStacked
                | Self::BarStacked100
                | Self::BarClustered3D
                | Self::BarStacked3D
                | Self::BarStacked100_3D
                | Self::ConeBarClustered
                | Self::ConeBarStacked
                | Self::ConeBarStacked100
                | Self::CylinderBarClustered
                | Self::CylinderBarStacked
                | Self::CylinderBarStacked100
                | Self::PyramidBarClustered
                | Self::PyramidBarStacked
                | Self::PyramidBarStacked100
        )
    }

    /// Returns true if this is a column-direction chart type (vertical bars).
    #[must_use]
    pub const fn is_column_type(self) -> bool {
        matches!(
            self,
            Self::ColumnClustered
                | Self::ColumnStacked
                | Self::ColumnStacked100
                | Self::ColumnClustered3D
                | Self::ColumnStacked3D
                | Self::ColumnStacked100_3D
                | Self::ConeCol
                | Self::ConeColClustered
                | Self::ConeColStacked
                | Self::ConeColStacked100
                | Self::CylinderCol
                | Self::CylinderColClustered
                | Self::CylinderColStacked
                | Self::CylinderColStacked100
                | Self::PyramidCol
                | Self::PyramidColClustered
                | Self::PyramidColStacked
                | Self::PyramidColStacked100
        )
    }

    /// Returns true if this is a bar or column chart type.
    #[must_use]
    pub const fn is_bar_or_column(self) -> bool {
        self.is_bar_type() || self.is_column_type()
    }

    /// Returns true if this is a line chart type.
    #[must_use]
    pub const fn is_line_type(self) -> bool {
        matches!(
            self,
            Self::Line
                | Self::LineMarkers
                | Self::LineStacked
                | Self::LineMarkersStacked
                | Self::LineStacked100
                | Self::LineMarkersStacked100
                | Self::Line3D
        )
    }

    /// Returns true if this is a pie chart type.
    #[must_use]
    pub const fn is_pie_type(self) -> bool {
        matches!(
            self,
            Self::Pie | Self::PieExploded | Self::Pie3D | Self::ExplodedPie3D
        )
    }

    /// Returns true if this is a doughnut chart type.
    #[must_use]
    pub const fn is_doughnut_type(self) -> bool {
        matches!(self, Self::Doughnut | Self::DoughnutExploded)
    }

    /// Returns true if this is an area chart type.
    #[must_use]
    pub const fn is_area_type(self) -> bool {
        matches!(
            self,
            Self::Area
                | Self::AreaStacked
                | Self::AreaStacked100
                | Self::Area3D
                | Self::AreaStacked3D
                | Self::AreaStacked100_3D
        )
    }

    /// Returns true if this is a scatter (XY) chart type.
    #[must_use]
    pub const fn is_xy_type(self) -> bool {
        matches!(
            self,
            Self::XyScatter
                | Self::XyScatterLines
                | Self::XyScatterLinesNoMarkers
                | Self::XyScatterSmooth
                | Self::XyScatterSmoothNoMarkers
        )
    }

    /// Returns true if this is a bubble chart type.
    #[must_use]
    pub const fn is_bubble_type(self) -> bool {
        matches!(self, Self::Bubble | Self::BubbleThreeDEffect)
    }

    /// Returns true if this is a radar chart type.
    #[must_use]
    pub const fn is_radar_type(self) -> bool {
        matches!(self, Self::Radar | Self::RadarFilled | Self::RadarMarkers)
    }

    /// Returns true if this is a combo chart type (e.g. column + line).
    #[must_use]
    pub const fn is_combo_type(self) -> bool {
        matches!(self, Self::ColumnLineCombo)
    }

    /// Returns true if this chart type uses categories (not XY/Bubble).
    #[must_use]
    pub const fn is_category_type(self) -> bool {
        !self.is_xy_type() && !self.is_bubble_type()
    }
}
