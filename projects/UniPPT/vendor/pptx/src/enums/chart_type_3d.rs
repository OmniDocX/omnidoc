//! 3D, stock, surface, and specialty chart classification methods.

use super::chart_type::XlChartType;

impl XlChartType {
    /// Returns true if this is a 3D chart type.
    #[must_use]
    pub const fn is_3d_type(self) -> bool {
        matches!(
            self,
            Self::BarClustered3D
                | Self::BarStacked3D
                | Self::BarStacked100_3D
                | Self::ColumnClustered3D
                | Self::ColumnStacked3D
                | Self::ColumnStacked100_3D
                | Self::Line3D
                | Self::Pie3D
                | Self::ExplodedPie3D
                | Self::Area3D
                | Self::AreaStacked3D
                | Self::AreaStacked100_3D
                | Self::ConeBarClustered
                | Self::ConeBarStacked
                | Self::ConeBarStacked100
                | Self::ConeCol
                | Self::ConeColClustered
                | Self::ConeColStacked
                | Self::ConeColStacked100
                | Self::CylinderBarClustered
                | Self::CylinderBarStacked
                | Self::CylinderBarStacked100
                | Self::CylinderCol
                | Self::CylinderColClustered
                | Self::CylinderColStacked
                | Self::CylinderColStacked100
                | Self::PyramidBarClustered
                | Self::PyramidBarStacked
                | Self::PyramidBarStacked100
                | Self::PyramidCol
                | Self::PyramidColClustered
                | Self::PyramidColStacked
                | Self::PyramidColStacked100
        )
    }

    /// Returns true if this is a stock chart type.
    #[must_use]
    pub const fn is_stock_type(self) -> bool {
        matches!(
            self,
            Self::StockHLC | Self::StockOHLC | Self::StockVHLC | Self::StockVOHLC
        )
    }

    /// Returns true if this is a surface chart type.
    #[must_use]
    pub const fn is_surface_type(self) -> bool {
        matches!(
            self,
            Self::Surface | Self::SurfaceWireframe | Self::SurfaceTop | Self::SurfaceTopWireframe
        )
    }

    /// Returns true if this is a cone chart type.
    #[must_use]
    pub const fn is_cone_type(self) -> bool {
        matches!(
            self,
            Self::ConeBarClustered
                | Self::ConeBarStacked
                | Self::ConeBarStacked100
                | Self::ConeCol
                | Self::ConeColClustered
                | Self::ConeColStacked
                | Self::ConeColStacked100
        )
    }

    /// Returns true if this is a cylinder chart type.
    #[must_use]
    pub const fn is_cylinder_type(self) -> bool {
        matches!(
            self,
            Self::CylinderBarClustered
                | Self::CylinderBarStacked
                | Self::CylinderBarStacked100
                | Self::CylinderCol
                | Self::CylinderColClustered
                | Self::CylinderColStacked
                | Self::CylinderColStacked100
        )
    }

    /// Returns true if this is a pyramid chart type.
    #[must_use]
    pub const fn is_pyramid_type(self) -> bool {
        matches!(
            self,
            Self::PyramidBarClustered
                | Self::PyramidBarStacked
                | Self::PyramidBarStacked100
                | Self::PyramidCol
                | Self::PyramidColClustered
                | Self::PyramidColStacked
                | Self::PyramidColStacked100
        )
    }

    /// Returns the bar 3D shape name for cone/cylinder/pyramid types, or `None`.
    #[must_use]
    pub const fn chart_shape(self) -> Option<&'static str> {
        if self.is_cone_type() {
            Some("cone")
        } else if self.is_cylinder_type() {
            Some("cylinder")
        } else if self.is_pyramid_type() {
            Some("pyramid")
        } else {
            None
        }
    }
}
