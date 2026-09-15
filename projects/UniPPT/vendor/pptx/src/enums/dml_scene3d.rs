//! 3D scene enumerations: light rig types and light direction.

/// Light rig type for 3D scene (`<a:lightRig rig="...">`).
///
/// Based on the OOXML `ST_LightRigType` simple type.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LightRigType {
    LegacyFlat1,
    LegacyFlat2,
    LegacyFlat3,
    LegacyFlat4,
    LegacyNormal1,
    LegacyNormal2,
    LegacyNormal3,
    LegacyNormal4,
    LegacyHarsh1,
    LegacyHarsh2,
    LegacyHarsh3,
    LegacyHarsh4,
    ThreePt,
    Balanced,
    Soft,
    Harsh,
    Flood,
    Contrasting,
    Morning,
    Sunrise,
    Sunset,
    Chilly,
    Freezing,
    Flat,
    TwoPt,
    Glow,
    BrightRoom,
    /// Unknown / unrecognised light rig type preserved for round-tripping.
    Other(String),
}

impl LightRigType {
    /// Return the XML attribute value for this light rig type.
    #[must_use]
    pub fn to_xml_str(&self) -> &str {
        match self {
            Self::LegacyFlat1 => "legacyFlat1",
            Self::LegacyFlat2 => "legacyFlat2",
            Self::LegacyFlat3 => "legacyFlat3",
            Self::LegacyFlat4 => "legacyFlat4",
            Self::LegacyNormal1 => "legacyNormal1",
            Self::LegacyNormal2 => "legacyNormal2",
            Self::LegacyNormal3 => "legacyNormal3",
            Self::LegacyNormal4 => "legacyNormal4",
            Self::LegacyHarsh1 => "legacyHarsh1",
            Self::LegacyHarsh2 => "legacyHarsh2",
            Self::LegacyHarsh3 => "legacyHarsh3",
            Self::LegacyHarsh4 => "legacyHarsh4",
            Self::ThreePt => "threePt",
            Self::Balanced => "balanced",
            Self::Soft => "soft",
            Self::Harsh => "harsh",
            Self::Flood => "flood",
            Self::Contrasting => "contrasting",
            Self::Morning => "morning",
            Self::Sunrise => "sunrise",
            Self::Sunset => "sunset",
            Self::Chilly => "chilly",
            Self::Freezing => "freezing",
            Self::Flat => "flat",
            Self::TwoPt => "twoPt",
            Self::Glow => "glow",
            Self::BrightRoom => "brightRoom",
            Self::Other(s) => s.as_str(),
        }
    }

    /// Parse an XML light rig type attribute value.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Self {
        match s {
            "legacyFlat1" => Self::LegacyFlat1,
            "legacyFlat2" => Self::LegacyFlat2,
            "legacyFlat3" => Self::LegacyFlat3,
            "legacyFlat4" => Self::LegacyFlat4,
            "legacyNormal1" => Self::LegacyNormal1,
            "legacyNormal2" => Self::LegacyNormal2,
            "legacyNormal3" => Self::LegacyNormal3,
            "legacyNormal4" => Self::LegacyNormal4,
            "legacyHarsh1" => Self::LegacyHarsh1,
            "legacyHarsh2" => Self::LegacyHarsh2,
            "legacyHarsh3" => Self::LegacyHarsh3,
            "legacyHarsh4" => Self::LegacyHarsh4,
            "threePt" => Self::ThreePt,
            "balanced" => Self::Balanced,
            "soft" => Self::Soft,
            "harsh" => Self::Harsh,
            "flood" => Self::Flood,
            "contrasting" => Self::Contrasting,
            "morning" => Self::Morning,
            "sunrise" => Self::Sunrise,
            "sunset" => Self::Sunset,
            "chilly" => Self::Chilly,
            "freezing" => Self::Freezing,
            "flat" => Self::Flat,
            "twoPt" => Self::TwoPt,
            "glow" => Self::Glow,
            "brightRoom" => Self::BrightRoom,
            other => Self::Other(other.to_string()),
        }
    }
}

impl From<&str> for LightRigType {
    fn from(s: &str) -> Self {
        Self::from_xml_str(s)
    }
}

impl PartialEq<&str> for LightRigType {
    fn eq(&self, other: &&str) -> bool {
        self.to_xml_str() == *other
    }
}

// ---------------------------------------------------------------------------
// LightDirection (ST_LightRigDirection)
// ---------------------------------------------------------------------------

/// Light direction for 3D scene (`<a:lightRig dir="...">`).
///
/// Based on the OOXML `ST_LightRigDirection` simple type.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LightDirection {
    Top,
    TopLeft,
    TopRight,
    Left,
    Right,
    Bottom,
    BottomLeft,
    BottomRight,
    /// Unknown / unrecognised light direction preserved for round-tripping.
    Other(String),
}

impl LightDirection {
    /// Return the XML attribute value for this light direction.
    #[must_use]
    pub fn to_xml_str(&self) -> &str {
        match self {
            Self::Top => "t",
            Self::TopLeft => "tl",
            Self::TopRight => "tr",
            Self::Left => "l",
            Self::Right => "r",
            Self::Bottom => "b",
            Self::BottomLeft => "bl",
            Self::BottomRight => "br",
            Self::Other(s) => s.as_str(),
        }
    }

    /// Parse an XML light direction attribute value.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Self {
        match s {
            "t" => Self::Top,
            "tl" => Self::TopLeft,
            "tr" => Self::TopRight,
            "l" => Self::Left,
            "r" => Self::Right,
            "b" => Self::Bottom,
            "bl" => Self::BottomLeft,
            "br" => Self::BottomRight,
            other => Self::Other(other.to_string()),
        }
    }
}

impl From<&str> for LightDirection {
    fn from(s: &str) -> Self {
        Self::from_xml_str(s)
    }
}

impl PartialEq<&str> for LightDirection {
    fn eq(&self, other: &&str) -> bool {
        self.to_xml_str() == *other
    }
}
