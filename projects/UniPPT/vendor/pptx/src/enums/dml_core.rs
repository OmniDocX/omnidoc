//! Core `DrawingML` enumerations: theme colors, line dash styles, fill types,
//! and color types.

/// An Office theme color, one of those shown in the color gallery on the
/// formatting ribbon.
///
/// Alias: `MsoThemeColor`
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsoThemeColorIndex {
    NotThemeColor,
    Accent1,
    Accent2,
    Accent3,
    Accent4,
    Accent5,
    Accent6,
    Background1,
    Background2,
    Dark1,
    Dark2,
    FollowedHyperlink,
    Hyperlink,
    Light1,
    Light2,
    Text1,
    Text2,
}

/// Alias matching the python-pptx `MSO_THEME_COLOR` name.
pub type MsoThemeColor = MsoThemeColorIndex;

impl MsoThemeColorIndex {
    /// Return the XML attribute value for this theme color.
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::NotThemeColor => "",
            Self::Accent1 => "accent1",
            Self::Accent2 => "accent2",
            Self::Accent3 => "accent3",
            Self::Accent4 => "accent4",
            Self::Accent5 => "accent5",
            Self::Accent6 => "accent6",
            Self::Background1 => "bg1",
            Self::Background2 => "bg2",
            Self::Dark1 => "dk1",
            Self::Dark2 => "dk2",
            Self::FollowedHyperlink => "folHlink",
            Self::Hyperlink => "hlink",
            Self::Light1 => "lt1",
            Self::Light2 => "lt2",
            Self::Text1 => "tx1",
            Self::Text2 => "tx2",
        }
    }

    /// Parse an XML theme color attribute value.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Option<Self> {
        match s {
            "" => Some(Self::NotThemeColor),
            "accent1" => Some(Self::Accent1),
            "accent2" => Some(Self::Accent2),
            "accent3" => Some(Self::Accent3),
            "accent4" => Some(Self::Accent4),
            "accent5" => Some(Self::Accent5),
            "accent6" => Some(Self::Accent6),
            "bg1" => Some(Self::Background1),
            "bg2" => Some(Self::Background2),
            "dk1" => Some(Self::Dark1),
            "dk2" => Some(Self::Dark2),
            "folHlink" => Some(Self::FollowedHyperlink),
            "hlink" => Some(Self::Hyperlink),
            "lt1" => Some(Self::Light1),
            "lt2" => Some(Self::Light2),
            "tx1" => Some(Self::Text1),
            "tx2" => Some(Self::Text2),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// MSO_LINE_DASH_STYLE (MSO_LINE)
// ---------------------------------------------------------------------------

/// Specifies the dash style for a line.
///
/// Alias: `MsoLineDashStyle`
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsoLineDashStyle {
    Dash,
    DashDot,
    DashDotDot,
    LongDash,
    LongDashDot,
    RoundDot,
    Solid,
    SquareDot,
}

impl MsoLineDashStyle {
    /// Return the XML attribute value for this line dash style.
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::Dash => "dash",
            Self::DashDot => "dashDot",
            Self::DashDotDot => "lgDashDotDot",
            Self::LongDash => "lgDash",
            Self::LongDashDot => "lgDashDot",
            Self::RoundDot => "sysDot",
            Self::Solid => "solid",
            Self::SquareDot => "sysDash",
        }
    }

    /// Parse an XML line dash style attribute value.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Option<Self> {
        match s {
            "dash" => Some(Self::Dash),
            "dashDot" => Some(Self::DashDot),
            "lgDashDotDot" => Some(Self::DashDotDot),
            "lgDash" => Some(Self::LongDash),
            "lgDashDot" => Some(Self::LongDashDot),
            "sysDot" => Some(Self::RoundDot),
            "solid" => Some(Self::Solid),
            "sysDash" => Some(Self::SquareDot),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// MSO_FILL_TYPE (MSO_FILL)
// ---------------------------------------------------------------------------

/// Specifies the type of fill for a shape.
///
/// Alias: `MsoFillType`
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsoFillType {
    /// The shape is transparent.
    Background,
    /// Shape is filled with a gradient.
    Gradient,
    /// Shape inherits fill from group.
    Group,
    /// Shape is filled with a pattern.
    Patterned,
    /// Shape is filled with a bitmapped image.
    Picture,
    /// Shape is filled with a solid color.
    Solid,
    /// Shape is filled with a texture.
    Textured,
}

// ---------------------------------------------------------------------------
// MSO_COLOR_TYPE
// ---------------------------------------------------------------------------

/// Specifies the color specification scheme.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsoColorType {
    /// Color is specified by an RGB value.
    Rgb,
    /// Color is one of the preset theme colors.
    Scheme,
    /// Color is specified using Hue, Saturation, and Luminosity values.
    Hsl,
    /// Color is specified using a named built-in color.
    Preset,
    /// Color is an scRGB color.
    ScRgb,
    /// Color is one specified by the operating system.
    System,
}
