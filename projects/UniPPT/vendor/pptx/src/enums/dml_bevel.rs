//! Bevel and material preset enumerations for `DrawingML` 3D shapes.

/// Bevel preset type for 3D shape (`<a:bevelT prst="...">`, `<a:bevelB prst="...">`).
///
/// Based on the OOXML `ST_BevelPresetType` simple type.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BevelType {
    Circle,
    RelaxedInset,
    Cross,
    CoolSlant,
    Angle,
    SoftRound,
    Convex,
    Slope,
    Divot,
    Riblet,
    HardEdge,
    ArtDeco,
    /// Unknown / unrecognised bevel type preserved for round-tripping.
    Other(String),
}

impl BevelType {
    /// Return the XML attribute value for this bevel type.
    #[must_use]
    pub fn to_xml_str(&self) -> &str {
        match self {
            Self::Circle => "circle",
            Self::RelaxedInset => "relaxedInset",
            Self::Cross => "cross",
            Self::CoolSlant => "coolSlant",
            Self::Angle => "angle",
            Self::SoftRound => "softRound",
            Self::Convex => "convex",
            Self::Slope => "slope",
            Self::Divot => "divot",
            Self::Riblet => "riblet",
            Self::HardEdge => "hardEdge",
            Self::ArtDeco => "artDeco",
            Self::Other(s) => s.as_str(),
        }
    }

    /// Parse an XML bevel type attribute value.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Self {
        match s {
            "circle" => Self::Circle,
            "relaxedInset" => Self::RelaxedInset,
            "cross" => Self::Cross,
            "coolSlant" => Self::CoolSlant,
            "angle" => Self::Angle,
            "softRound" => Self::SoftRound,
            "convex" => Self::Convex,
            "slope" => Self::Slope,
            "divot" => Self::Divot,
            "riblet" => Self::Riblet,
            "hardEdge" => Self::HardEdge,
            "artDeco" => Self::ArtDeco,
            other => Self::Other(other.to_string()),
        }
    }
}

impl From<&str> for BevelType {
    fn from(s: &str) -> Self {
        Self::from_xml_str(s)
    }
}

impl PartialEq<&str> for BevelType {
    fn eq(&self, other: &&str) -> bool {
        self.to_xml_str() == *other
    }
}

// ---------------------------------------------------------------------------
// MaterialPreset (ST_PresetMaterialType)
// ---------------------------------------------------------------------------

/// Material preset type for 3D shape (`<a:sp3d prstMaterial="...">`).
///
/// Based on the OOXML `ST_PresetMaterialType` simple type.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaterialPreset {
    LegacyMatte,
    LegacyPlastic,
    LegacyMetal,
    LegacyWireframe,
    Matte,
    Plastic,
    Metal,
    WarmMatte,
    TranslucentPowder,
    Powder,
    DkEdge,
    SoftEdge,
    Clear,
    Flat,
    SoftMetal,
    /// Unknown / unrecognised material preset preserved for round-tripping.
    Other(String),
}

impl MaterialPreset {
    /// Return the XML attribute value for this material preset.
    #[must_use]
    pub fn to_xml_str(&self) -> &str {
        match self {
            Self::LegacyMatte => "legacyMatte",
            Self::LegacyPlastic => "legacyPlastic",
            Self::LegacyMetal => "legacyMetal",
            Self::LegacyWireframe => "legacyWireframe",
            Self::Matte => "matte",
            Self::Plastic => "plastic",
            Self::Metal => "metal",
            Self::WarmMatte => "warmMatte",
            Self::TranslucentPowder => "translucentPowder",
            Self::Powder => "powder",
            Self::DkEdge => "dkEdge",
            Self::SoftEdge => "softEdge",
            Self::Clear => "clear",
            Self::Flat => "flat",
            Self::SoftMetal => "softMetal",
            Self::Other(s) => s.as_str(),
        }
    }

    /// Parse an XML material preset attribute value.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Self {
        match s {
            "legacyMatte" => Self::LegacyMatte,
            "legacyPlastic" => Self::LegacyPlastic,
            "legacyMetal" => Self::LegacyMetal,
            "legacyWireframe" => Self::LegacyWireframe,
            "matte" => Self::Matte,
            "plastic" => Self::Plastic,
            "metal" => Self::Metal,
            "warmMatte" => Self::WarmMatte,
            "translucentPowder" => Self::TranslucentPowder,
            "powder" => Self::Powder,
            "dkEdge" => Self::DkEdge,
            "softEdge" => Self::SoftEdge,
            "clear" => Self::Clear,
            "flat" => Self::Flat,
            "softMetal" => Self::SoftMetal,
            other => Self::Other(other.to_string()),
        }
    }
}

impl From<&str> for MaterialPreset {
    fn from(s: &str) -> Self {
        Self::from_xml_str(s)
    }
}

impl PartialEq<&str> for MaterialPreset {
    fn eq(&self, other: &&str) -> bool {
        self.to_xml_str() == *other
    }
}
