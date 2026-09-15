//! Supplementary chart enumerations: legend, marker, data-label, axis, and
//! tick-mark types.

/// Specifies the position of the legend on a chart.
///
/// MS API Name: `XlLegendPosition`
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XlLegendPosition {
    Bottom,
    Corner,
    Left,
    Right,
    Top,
}

impl XlLegendPosition {
    /// Return the XML attribute value for this legend position.
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::Bottom => "b",
            Self::Corner => "tr",
            Self::Left => "l",
            Self::Right => "r",
            Self::Top => "t",
        }
    }

    /// Parse an XML legend position attribute value.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Option<Self> {
        match s {
            "b" => Some(Self::Bottom),
            "tr" => Some(Self::Corner),
            "l" => Some(Self::Left),
            "r" => Some(Self::Right),
            "t" => Some(Self::Top),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// XL_MARKER_STYLE
// ---------------------------------------------------------------------------

/// Specifies the marker style for a point or series in a line, scatter, or
/// radar chart.
///
/// MS API Name: `XlMarkerStyle`
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XlMarkerStyle {
    Automatic,
    Circle,
    Dash,
    Diamond,
    Dot,
    None,
    Picture,
    Plus,
    Square,
    Star,
    Triangle,
    X,
}

impl XlMarkerStyle {
    /// Return the XML attribute value for this marker style.
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::Automatic => "auto",
            Self::Circle => "circle",
            Self::Dash => "dash",
            Self::Diamond => "diamond",
            Self::Dot => "dot",
            Self::None => "none",
            Self::Picture => "picture",
            Self::Plus => "plus",
            Self::Square => "square",
            Self::Star => "star",
            Self::Triangle => "triangle",
            Self::X => "x",
        }
    }

    /// Parse an XML marker style attribute value.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Option<Self> {
        match s {
            "auto" => Some(Self::Automatic),
            "circle" => Some(Self::Circle),
            "dash" => Some(Self::Dash),
            "diamond" => Some(Self::Diamond),
            "dot" => Some(Self::Dot),
            "none" => Some(Self::None),
            "picture" => Some(Self::Picture),
            "plus" => Some(Self::Plus),
            "square" => Some(Self::Square),
            "star" => Some(Self::Star),
            "triangle" => Some(Self::Triangle),
            "x" => Some(Self::X),
            _ => Option::None,
        }
    }
}

// ---------------------------------------------------------------------------
// XL_DATA_LABEL_POSITION
// ---------------------------------------------------------------------------

/// Specifies where the data label is positioned.
///
/// MS API Name: `XlDataLabelPosition`
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XlDataLabelPosition {
    Above,
    Below,
    BestFit,
    Center,
    InsideBase,
    InsideEnd,
    Left,
    OutsideEnd,
    Right,
}

/// Alias matching the python-pptx `XL_LABEL_POSITION` name.
pub type XlLabelPosition = XlDataLabelPosition;

impl XlDataLabelPosition {
    /// Return the XML attribute value for this data label position.
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::Above => "t",
            Self::Below => "b",
            Self::BestFit => "bestFit",
            Self::Center => "ctr",
            Self::InsideBase => "inBase",
            Self::InsideEnd => "inEnd",
            Self::Left => "l",
            Self::OutsideEnd => "outEnd",
            Self::Right => "r",
        }
    }

    /// Parse an XML data label position attribute value.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Option<Self> {
        match s {
            "t" => Some(Self::Above),
            "b" => Some(Self::Below),
            "bestFit" => Some(Self::BestFit),
            "ctr" => Some(Self::Center),
            "inBase" => Some(Self::InsideBase),
            "inEnd" => Some(Self::InsideEnd),
            "l" => Some(Self::Left),
            "outEnd" => Some(Self::OutsideEnd),
            "r" => Some(Self::Right),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// XL_AXIS_CROSSES
// ---------------------------------------------------------------------------

/// Specifies the point on an axis where the other axis crosses.
///
/// MS API Name: `XlAxisCrosses`
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XlAxisCrosses {
    Automatic,
    Custom,
    Maximum,
    Minimum,
}

impl XlAxisCrosses {
    /// Return the XML attribute value for this axis crossing point.
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::Automatic => "autoZero",
            Self::Custom => "",
            Self::Maximum => "max",
            Self::Minimum => "min",
        }
    }

    /// Parse an XML axis crosses attribute value.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Option<Self> {
        match s {
            "autoZero" => Some(Self::Automatic),
            "max" => Some(Self::Maximum),
            "min" => Some(Self::Minimum),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// XL_CATEGORY_TYPE
// ---------------------------------------------------------------------------

/// Specifies the type of the category axis.
///
/// MS API Name: `XlCategoryType`
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XlCategoryType {
    AutomaticScale,
    CategoryScale,
    TimeScale,
}

// ---------------------------------------------------------------------------
// XL_TICK_MARK
// ---------------------------------------------------------------------------

/// Specifies a type of axis tick for a chart.
///
/// MS API Name: `XlTickMark`
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XlTickMark {
    Cross,
    Inside,
    None,
    Outside,
}

impl XlTickMark {
    /// Return the XML attribute value for this tick mark.
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::Cross => "cross",
            Self::Inside => "in",
            Self::None => "none",
            Self::Outside => "out",
        }
    }

    /// Parse an XML tick mark attribute value.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Option<Self> {
        match s {
            "cross" => Some(Self::Cross),
            "in" => Some(Self::Inside),
            "none" => Some(Self::None),
            "out" => Some(Self::Outside),
            _ => Option::None,
        }
    }
}

// ---------------------------------------------------------------------------
// XL_TICK_LABEL_POSITION
// ---------------------------------------------------------------------------

/// Specifies the position of tick-mark labels on a chart axis.
///
/// MS API Name: `XlTickLabelPosition`
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XlTickLabelPosition {
    High,
    Low,
    NextToAxis,
    None,
}

impl XlTickLabelPosition {
    /// Return the XML attribute value for this tick label position.
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Low => "low",
            Self::NextToAxis => "nextTo",
            Self::None => "none",
        }
    }

    /// Parse an XML tick label position attribute value.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Option<Self> {
        match s {
            "high" => Some(Self::High),
            "low" => Some(Self::Low),
            "nextTo" => Some(Self::NextToAxis),
            "none" => Some(Self::None),
            _ => Option::None,
        }
    }
}
