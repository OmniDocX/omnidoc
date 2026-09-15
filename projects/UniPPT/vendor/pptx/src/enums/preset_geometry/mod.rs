//! Preset geometry (shape) type enumeration for OOXML.

mod from_xml;
mod to_xml;

/// Specifies a preset geometry type for a shape or connector.
///
/// Corresponds to the `prst` attribute on `<a:prstGeom>`.
/// Covers all standard OOXML preset geometry names.
/// Unknown values are captured in the `Other` variant.
#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresetGeometry {
    AccentBorderCallout1,
    AccentBorderCallout2,
    AccentBorderCallout3,
    AccentCallout1,
    AccentCallout2,
    AccentCallout3,
    ActionButtonBackPrevious,
    ActionButtonBeginning,
    ActionButtonBlank,
    ActionButtonDocument,
    ActionButtonEnd,
    ActionButtonForwardNext,
    ActionButtonHelp,
    ActionButtonHome,
    ActionButtonInformation,
    ActionButtonMovie,
    ActionButtonReturn,
    ActionButtonSound,
    Arc,
    BentArrow,
    BentConnector2,
    BentConnector3,
    BentConnector4,
    BentConnector5,
    BentUpArrow,
    Bevel,
    BlockArc,
    BorderCallout1,
    BorderCallout2,
    BorderCallout3,
    BracePair,
    BracketPair,
    Callout1,
    Callout2,
    Callout3,
    Can,
    ChartPlus,
    ChartStar,
    ChartX,
    Chevron,
    Chord,
    CircularArrow,
    Cloud,
    CloudCallout,
    Corner,
    CornerTabs,
    Cube,
    CurvedConnector2,
    CurvedConnector3,
    CurvedConnector4,
    CurvedConnector5,
    CurvedDownArrow,
    CurvedLeftArrow,
    CurvedRightArrow,
    CurvedUpArrow,
    Decagon,
    DiagStripe,
    Diamond,
    Dodecagon,
    Donut,
    DoubleWave,
    DownArrow,
    DownArrowCallout,
    Ellipse,
    EllipseRibbon,
    EllipseRibbon2,
    FlowChartAlternateProcess,
    FlowChartCollate,
    FlowChartConnector,
    FlowChartDecision,
    FlowChartDelay,
    FlowChartDisplay,
    FlowChartDocument,
    FlowChartExtract,
    FlowChartInputOutput,
    FlowChartInternalStorage,
    FlowChartMagneticDisk,
    FlowChartMagneticDrum,
    FlowChartMagneticTape,
    FlowChartManualInput,
    FlowChartManualOperation,
    FlowChartMerge,
    FlowChartMultidocument,
    FlowChartOfflineStorage,
    FlowChartOffpageConnector,
    FlowChartOnlineStorage,
    FlowChartOr,
    FlowChartPredefinedProcess,
    FlowChartPreparation,
    FlowChartProcess,
    FlowChartPunchedCard,
    FlowChartPunchedTape,
    FlowChartSort,
    FlowChartSummingJunction,
    FlowChartTerminator,
    FoldedCorner,
    Frame,
    Funnel,
    Gear6,
    Gear9,
    HalfFrame,
    Heart,
    Heptagon,
    Hexagon,
    HomePlate,
    HorizontalScroll,
    IrregularSeal1,
    IrregularSeal2,
    LeftArrow,
    LeftArrowCallout,
    LeftBrace,
    LeftBracket,
    LeftCircularArrow,
    LeftRightArrow,
    LeftRightArrowCallout,
    LeftRightCircularArrow,
    LeftRightRibbon,
    LeftRightUpArrow,
    LeftUpArrow,
    LightningBolt,
    Line,
    LineInv,
    MathDivide,
    MathEqual,
    MathMinus,
    MathMultiply,
    MathNotEqual,
    MathPlus,
    Moon,
    NoSmoking,
    NonIsoscelesTrapezoid,
    NotchedRightArrow,
    Octagon,
    Parallelogram,
    Pentagon,
    Pie,
    PieWedge,
    Plaque,
    PlaqueTabs,
    Plus,
    QuadArrow,
    QuadArrowCallout,
    Rect,
    Ribbon,
    Ribbon2,
    RightArrow,
    RightArrowCallout,
    RightBrace,
    RightBracket,
    Round1Rect,
    Round2DiagRect,
    Round2SameRect,
    RoundRect,
    RtTriangle,
    SmileyFace,
    Snip1Rect,
    Snip2DiagRect,
    Snip2SameRect,
    SnipRoundRect,
    SquareTabs,
    Star10,
    Star12,
    Star16,
    Star24,
    Star32,
    Star4,
    Star5,
    Star6,
    Star7,
    Star8,
    StraightConnector1,
    StripedRightArrow,
    Sun,
    SwooshArrow,
    Teardrop,
    Trapezoid,
    Triangle,
    UpArrow,
    UpArrowCallout,
    UpDownArrow,
    UpDownArrowCallout,
    UturnArrow,
    VerticalScroll,
    Wave,
    WedgeEllipseCallout,
    WedgeRectCallout,
    WedgeRoundRectCallout,
    /// Catch-all for unrecognized preset geometry values.
    Other(String),
}

impl std::fmt::Display for PresetGeometry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_xml_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_geometry_known_roundtrip() {
        let cases = [
            ("rect", PresetGeometry::Rect),
            ("ellipse", PresetGeometry::Ellipse),
            ("roundRect", PresetGeometry::RoundRect),
            ("line", PresetGeometry::Line),
            ("bentConnector3", PresetGeometry::BentConnector3),
            ("curvedConnector3", PresetGeometry::CurvedConnector3),
            ("diamond", PresetGeometry::Diamond),
            ("star5", PresetGeometry::Star5),
            ("heart", PresetGeometry::Heart),
            ("cloud", PresetGeometry::Cloud),
            ("triangle", PresetGeometry::Triangle),
        ];
        for (xml, variant) in cases {
            assert_eq!(PresetGeometry::from_xml_str(xml), variant);
            assert_eq!(variant.to_xml_str(), xml);
        }
    }

    #[test]
    fn test_preset_geometry_other_fallback() {
        let pg = PresetGeometry::from_xml_str("someNewShape");
        assert_eq!(pg, PresetGeometry::Other("someNewShape".to_string()));
        assert_eq!(pg.to_xml_str(), "someNewShape");
    }

    #[test]
    fn test_preset_geometry_display() {
        assert_eq!(format!("{}", PresetGeometry::Rect), "rect");
        assert_eq!(
            format!("{}", PresetGeometry::Other("custom".to_string())),
            "custom"
        );
    }
}
