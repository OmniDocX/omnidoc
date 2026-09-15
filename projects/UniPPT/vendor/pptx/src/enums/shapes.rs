//! Enumerations used by shapes and related objects.

// Re-export the large AutoShape type enum from its dedicated module.
pub use super::autoshape_type::{MsoAutoShapeType, MsoShape};

// Re-export PresetGeometry from its dedicated module.
pub use super::preset_geometry::PresetGeometry;

/// Specifies one of the 18 distinct types of placeholder.
///
/// Alias: `PpPlaceholder`
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PpPlaceholderType {
    Bitmap,
    Body,
    CenterTitle,
    Chart,
    Date,
    Footer,
    Header,
    MediaClip,
    Object,
    OrgChart,
    Picture,
    SlideImage,
    SlideNumber,
    Subtitle,
    Table,
    Title,
    VerticalBody,
    VerticalObject,
    VerticalTitle,
}

/// Alias matching the python-pptx `PP_PLACEHOLDER` name.
pub type PpPlaceholder = PpPlaceholderType;

impl PpPlaceholderType {
    /// Return the OOXML placeholder type string (the `type` attribute on `<p:ph>`).
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::Bitmap => "clipArt",
            Self::Body => "body",
            Self::CenterTitle => "ctrTitle",
            Self::Chart => "chart",
            Self::Date => "dt",
            Self::Footer => "ftr",
            Self::Header => "hdr",
            Self::MediaClip => "media",
            Self::Object => "obj",
            Self::OrgChart => "dgm",
            Self::Picture => "pic",
            Self::SlideImage => "sldImg",
            Self::SlideNumber => "sldNum",
            Self::Subtitle => "subTitle",
            Self::Table => "tbl",
            Self::Title => "title",
            Self::VerticalBody | Self::VerticalObject | Self::VerticalTitle => "",
        }
    }

    /// Parse an OOXML placeholder type string into a variant.
    ///
    /// Returns `None` if the string does not match any known placeholder type.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Option<Self> {
        match s {
            "clipArt" => Some(Self::Bitmap),
            "body" => Some(Self::Body),
            "ctrTitle" => Some(Self::CenterTitle),
            "chart" => Some(Self::Chart),
            "dt" => Some(Self::Date),
            "ftr" => Some(Self::Footer),
            "hdr" => Some(Self::Header),
            "media" => Some(Self::MediaClip),
            "obj" => Some(Self::Object),
            "dgm" => Some(Self::OrgChart),
            "pic" => Some(Self::Picture),
            "sldImg" => Some(Self::SlideImage),
            "sldNum" => Some(Self::SlideNumber),
            "subTitle" => Some(Self::Subtitle),
            "tbl" => Some(Self::Table),
            "title" => Some(Self::Title),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// MSO_SHAPE_TYPE
// ---------------------------------------------------------------------------

/// Specifies the type of a shape, more specifically than the five base types.
///
/// Alias: `Mso`
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsoShapeType {
    AutoShape,
    Callout,
    Canvas,
    Chart,
    Comment,
    Diagram,
    EmbeddedOleObject,
    FormControl,
    Freeform,
    Group,
    IgxGraphic,
    Ink,
    InkComment,
    Line,
    LinkedOleObject,
    LinkedPicture,
    Media,
    OleControlObject,
    Picture,
    Placeholder,
    ScriptAnchor,
    Table,
    TextBox,
    TextEffect,
    WebVideo,
}

// ---------------------------------------------------------------------------
// MSO_CONNECTOR_TYPE
// ---------------------------------------------------------------------------

/// Specifies a type of connector.
///
/// Alias: `MsoConnector`
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsoConnectorType {
    Curve,
    Elbow,
    Straight,
}

impl MsoConnectorType {
    /// Return the OOXML connector type string.
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::Curve => "curvedConnector3",
            Self::Elbow => "bentConnector3",
            Self::Straight => "line",
        }
    }

    /// Parse an OOXML connector type string into a variant.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Option<Self> {
        match s {
            "curvedConnector3" => Some(Self::Curve),
            "bentConnector3" => Some(Self::Elbow),
            "line" => Some(Self::Straight),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// PlaceholderOrientation
// ---------------------------------------------------------------------------

/// Specifies the orientation of a placeholder.
///
/// Corresponds to the `orient` attribute on `<p:ph>`.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceholderOrientation {
    Horizontal,
    Vertical,
}

impl PlaceholderOrientation {
    /// Return the XML attribute value for this orientation.
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::Horizontal => "horz",
            Self::Vertical => "vert",
        }
    }

    /// Parse an XML orientation attribute value.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Option<Self> {
        match s {
            "horz" => Some(Self::Horizontal),
            "vert" => Some(Self::Vertical),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// PlaceholderSize
// ---------------------------------------------------------------------------

/// Specifies the size of a placeholder.
///
/// Corresponds to the `sz` attribute on `<p:ph>`.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceholderSize {
    Full,
    Half,
    Quarter,
}

impl PlaceholderSize {
    /// Return the XML attribute value for this size.
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Half => "half",
            Self::Quarter => "quarter",
        }
    }

    /// Parse an XML size attribute value.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Option<Self> {
        match s {
            "full" => Some(Self::Full),
            "half" => Some(Self::Half),
            "quarter" => Some(Self::Quarter),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_placeholder_to_xml_str() {
        assert_eq!(PpPlaceholderType::Title.to_xml_str(), "title");
        assert_eq!(PpPlaceholderType::Body.to_xml_str(), "body");
        assert_eq!(PpPlaceholderType::Date.to_xml_str(), "dt");
    }

    #[test]
    fn test_placeholder_from_xml_str() {
        assert_eq!(
            PpPlaceholderType::from_xml_str("title"),
            Some(PpPlaceholderType::Title)
        );
        assert_eq!(
            PpPlaceholderType::from_xml_str("body"),
            Some(PpPlaceholderType::Body)
        );
        assert_eq!(PpPlaceholderType::from_xml_str("unknown"), None);
    }

    #[test]
    fn test_connector_roundtrip() {
        let connectors = [
            MsoConnectorType::Curve,
            MsoConnectorType::Elbow,
            MsoConnectorType::Straight,
        ];
        for c in connectors {
            let xml = c.to_xml_str();
            assert_eq!(MsoConnectorType::from_xml_str(xml), Some(c));
        }
    }

    #[test]
    fn test_placeholder_orientation_roundtrip() {
        assert_eq!(
            PlaceholderOrientation::from_xml_str("horz"),
            Some(PlaceholderOrientation::Horizontal)
        );
        assert_eq!(
            PlaceholderOrientation::from_xml_str("vert"),
            Some(PlaceholderOrientation::Vertical)
        );
        assert_eq!(PlaceholderOrientation::from_xml_str("unknown"), None);
        assert_eq!(PlaceholderOrientation::Horizontal.to_xml_str(), "horz");
        assert_eq!(PlaceholderOrientation::Vertical.to_xml_str(), "vert");
    }

    #[test]
    fn test_placeholder_size_roundtrip() {
        assert_eq!(
            PlaceholderSize::from_xml_str("full"),
            Some(PlaceholderSize::Full)
        );
        assert_eq!(
            PlaceholderSize::from_xml_str("half"),
            Some(PlaceholderSize::Half)
        );
        assert_eq!(
            PlaceholderSize::from_xml_str("quarter"),
            Some(PlaceholderSize::Quarter)
        );
        assert_eq!(PlaceholderSize::from_xml_str("unknown"), None);
        assert_eq!(PlaceholderSize::Full.to_xml_str(), "full");
        assert_eq!(PlaceholderSize::Half.to_xml_str(), "half");
        assert_eq!(PlaceholderSize::Quarter.to_xml_str(), "quarter");
    }
}
