//! `AutoShape` (preset geometry) type enumeration.
//!
//! This large enum maps the ~190 OOXML preset geometry names to Rust
//! variants. It is separated from the main `shapes` module to keep
//! file sizes manageable.

mod from_xml;
mod to_xml;

/// Specifies a type of `AutoShape` (preset geometry).
///
/// Each variant maps to an OOXML preset geometry name used in `<a:prstGeom prst="...">`.
///
/// Alias: `MsoShape`
#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsoAutoShapeType {
    ActionButtonBackOrPrevious,
    ActionButtonBeginning,
    ActionButtonCustom,
    ActionButtonDocument,
    ActionButtonEnd,
    ActionButtonForwardOrNext,
    ActionButtonHelp,
    ActionButtonHome,
    ActionButtonInformation,
    ActionButtonMovie,
    ActionButtonReturn,
    ActionButtonSound,
    Arc,
    Balloon,
    BentArrow,
    BentUpArrow,
    Bevel,
    BlockArc,
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
    Cross,
    Cube,
    CurvedDownArrow,
    CurvedDownRibbon,
    CurvedLeftArrow,
    CurvedRightArrow,
    CurvedUpArrow,
    CurvedUpRibbon,
    Decagon,
    DiagonalStripe,
    Diamond,
    Dodecagon,
    Donut,
    DoubleBrace,
    DoubleBracket,
    DoubleWave,
    DownArrow,
    DownArrowCallout,
    DownRibbon,
    Explosion1,
    Explosion2,
    FlowchartAlternateProcess,
    FlowchartCard,
    FlowchartCollate,
    FlowchartConnector,
    FlowchartData,
    FlowchartDecision,
    FlowchartDelay,
    FlowchartDirectAccessStorage,
    FlowchartDisplay,
    FlowchartDocument,
    FlowchartExtract,
    FlowchartInternalStorage,
    FlowchartMagneticDisk,
    FlowchartManualInput,
    FlowchartManualOperation,
    FlowchartMerge,
    FlowchartMultidocument,
    FlowchartOfflineStorage,
    FlowchartOffpageConnector,
    FlowchartOr,
    FlowchartPredefinedProcess,
    FlowchartPreparation,
    FlowchartProcess,
    FlowchartPunchedTape,
    FlowchartSequentialAccessStorage,
    FlowchartSort,
    FlowchartStoredData,
    FlowchartSummingJunction,
    FlowchartTerminator,
    FoldedCorner,
    Frame,
    Funnel,
    Gear6,
    Gear9,
    HalfFrame,
    Heart,
    Heptagon,
    Hexagon,
    HorizontalScroll,
    IsoscelesTriangle,
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
    LineCallout1,
    LineCallout1AccentBar,
    LineCallout1BorderAndAccentBar,
    LineCallout1NoBorder,
    LineCallout2,
    LineCallout2AccentBar,
    LineCallout2BorderAndAccentBar,
    LineCallout2NoBorder,
    LineCallout3,
    LineCallout3AccentBar,
    LineCallout3BorderAndAccentBar,
    LineCallout3NoBorder,
    LineCallout4,
    LineCallout4AccentBar,
    LineCallout4BorderAndAccentBar,
    LineCallout4NoBorder,
    LineInverse,
    MathDivide,
    MathEqual,
    MathMinus,
    MathMultiply,
    MathNotEqual,
    MathPlus,
    Moon,
    NonIsoscelesTrapezoid,
    NotchedRightArrow,
    NoSymbol,
    Octagon,
    Oval,
    OvalCallout,
    Parallelogram,
    Pentagon,
    Pie,
    PieWedge,
    Plaque,
    PlaqueTabs,
    QuadArrow,
    QuadArrowCallout,
    Rectangle,
    RectangularCallout,
    RegularPentagon,
    RightArrow,
    RightArrowCallout,
    RightBrace,
    RightBracket,
    RightTriangle,
    RoundedRectangle,
    RoundedRectangularCallout,
    Round1Rectangle,
    Round2DiagRectangle,
    Round2SameRectangle,
    SmileyFace,
    Snip1Rectangle,
    Snip2DiagRectangle,
    Snip2SameRectangle,
    SnipRoundRectangle,
    SquareTabs,
    Star10Point,
    Star12Point,
    Star16Point,
    Star24Point,
    Star32Point,
    Star4Point,
    Star5Point,
    Star6Point,
    Star7Point,
    Star8Point,
    StripedRightArrow,
    Sun,
    SwooshArrow,
    Tear,
    Trapezoid,
    UpArrow,
    UpArrowCallout,
    UpDownArrow,
    UpDownArrowCallout,
    UpRibbon,
    UTurnArrow,
    VerticalScroll,
    Wave,
}

/// Alias matching the python-pptx `MSO_SHAPE` name.
pub type MsoShape = MsoAutoShapeType;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mso_shape_to_xml_str() {
        assert_eq!(MsoAutoShapeType::Rectangle.to_xml_str(), "rect");
        assert_eq!(MsoAutoShapeType::Oval.to_xml_str(), "ellipse");
        assert_eq!(MsoAutoShapeType::RoundedRectangle.to_xml_str(), "roundRect");
    }

    #[test]
    fn test_mso_shape_from_xml_str() {
        assert_eq!(
            MsoAutoShapeType::from_xml_str("rect"),
            Some(MsoAutoShapeType::Rectangle)
        );
        assert_eq!(
            MsoAutoShapeType::from_xml_str("roundRect"),
            Some(MsoAutoShapeType::RoundedRectangle)
        );
        assert_eq!(
            MsoAutoShapeType::from_xml_str("ellipse"),
            Some(MsoAutoShapeType::Oval)
        );
        assert_eq!(MsoAutoShapeType::from_xml_str("nonexistent"), None);
    }

    #[test]
    fn test_mso_shape_roundtrip() {
        let shapes = [
            MsoAutoShapeType::Rectangle,
            MsoAutoShapeType::Oval,
            MsoAutoShapeType::Cloud,
            MsoAutoShapeType::Star5Point,
            MsoAutoShapeType::Hexagon,
        ];
        for shape in shapes {
            let xml = shape.to_xml_str();
            assert_eq!(MsoAutoShapeType::from_xml_str(xml), Some(shape));
        }
    }
}
