//! Enumerations used by text and related objects.

/// Determines the type of automatic sizing allowed.
///
/// Alias: `MsoAutoSize`
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsoAutoSize {
    /// No automatic sizing of the shape or text will be done.
    None,
    /// The shape height and possibly width are adjusted to fit the text.
    ShapeToFitText,
    /// The font size is reduced as necessary to fit the text within the shape.
    TextToFitShape,
}

impl MsoAutoSize {
    /// Return the XML attribute value for this auto-size setting.
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::ShapeToFitText => "spAutoFit",
            Self::TextToFitShape => "normAutofit",
        }
    }

    /// Parse an XML auto-size attribute value.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Option<Self> {
        match s {
            "none" => Some(Self::None),
            "spAutoFit" => Some(Self::ShapeToFitText),
            "normAutofit" => Some(Self::TextToFitShape),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// MSO_VERTICAL_ANCHOR (MSO_ANCHOR)
// ---------------------------------------------------------------------------

/// Specifies the vertical alignment of text in a text frame.
///
/// Alias: `MsoAnchor`
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsoVerticalAnchor {
    /// Aligns text to top of text frame.
    Top,
    /// Centers text vertically.
    Middle,
    /// Aligns text to bottom of text frame.
    Bottom,
}

/// Alias matching the python-pptx `MSO_ANCHOR` name.
pub type MsoAnchor = MsoVerticalAnchor;

impl MsoVerticalAnchor {
    /// Return the XML attribute value for this vertical anchor.
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::Top => "t",
            Self::Middle => "ctr",
            Self::Bottom => "b",
        }
    }

    /// Parse an XML vertical anchor attribute value.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Option<Self> {
        match s {
            "t" => Some(Self::Top),
            "ctr" => Some(Self::Middle),
            "b" => Some(Self::Bottom),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// PP_PARAGRAPH_ALIGNMENT (PP_ALIGN)
// ---------------------------------------------------------------------------

/// Specifies the horizontal alignment for one or more paragraphs.
///
/// Alias: `PpAlign`
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PpParagraphAlignment {
    Center,
    Distribute,
    Justify,
    JustifyLow,
    Left,
    Right,
    ThaiDistribute,
}

/// Alias matching the python-pptx `PP_ALIGN` name.
pub type PpAlign = PpParagraphAlignment;

impl PpParagraphAlignment {
    /// Return the XML attribute value for this paragraph alignment.
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::Center => "ctr",
            Self::Distribute => "dist",
            Self::Justify => "just",
            Self::JustifyLow => "justLow",
            Self::Left => "l",
            Self::Right => "r",
            Self::ThaiDistribute => "thaiDist",
        }
    }

    /// Parse an XML paragraph alignment attribute value.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Option<Self> {
        match s {
            "ctr" => Some(Self::Center),
            "dist" => Some(Self::Distribute),
            "just" => Some(Self::Justify),
            "justLow" => Some(Self::JustifyLow),
            "l" => Some(Self::Left),
            "r" => Some(Self::Right),
            "thaiDist" => Some(Self::ThaiDistribute),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// TEXT_DIRECTION
// ---------------------------------------------------------------------------

/// Specifies the text direction (left-to-right or right-to-left) for a
/// paragraph.
///
/// Used to support Arabic, Hebrew, and other RTL languages.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextDirection {
    /// Left-to-right text flow (default for Latin scripts).
    LeftToRight,
    /// Right-to-left text flow (for Arabic, Hebrew, etc.).
    RightToLeft,
}

impl TextDirection {
    /// Return the XML attribute value for the `rtl` attribute on `<a:pPr>`.
    #[must_use]
    pub const fn to_xml_attr(self) -> &'static str {
        match self {
            Self::LeftToRight => "0",
            Self::RightToLeft => "1",
        }
    }

    /// Parse an XML `rtl` attribute value.
    #[must_use]
    pub fn from_xml_attr(s: &str) -> Option<Self> {
        match s {
            "0" | "false" => Some(Self::LeftToRight),
            "1" | "true" => Some(Self::RightToLeft),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// MSO_TEXT_UNDERLINE_TYPE (MSO_UNDERLINE)
// ---------------------------------------------------------------------------

/// Indicates the type of underline for text.
///
/// Alias: `MsoUnderline`
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsoTextUnderlineType {
    None,
    DashHeavyLine,
    DashLine,
    DashLongHeavyLine,
    DashLongLine,
    DotDashHeavyLine,
    DotDashLine,
    DotDotDashHeavyLine,
    DotDotDashLine,
    DottedHeavyLine,
    DottedLine,
    DoubleLine,
    HeavyLine,
    SingleLine,
    WavyDoubleLine,
    WavyHeavyLine,
    WavyLine,
    Words,
}

/// Alias matching the python-pptx `MSO_UNDERLINE` name.
pub type MsoUnderline = MsoTextUnderlineType;

impl MsoTextUnderlineType {
    /// Return the XML attribute value for this underline type.
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::DashHeavyLine => "dashHeavy",
            Self::DashLine => "dash",
            Self::DashLongHeavyLine => "dashLongHeavy",
            Self::DashLongLine => "dashLong",
            Self::DotDashHeavyLine => "dotDashHeavy",
            Self::DotDashLine => "dotDash",
            Self::DotDotDashHeavyLine => "dotDotDashHeavy",
            Self::DotDotDashLine => "dotDotDash",
            Self::DottedHeavyLine => "dottedHeavy",
            Self::DottedLine => "dotted",
            Self::DoubleLine => "dbl",
            Self::HeavyLine => "heavy",
            Self::SingleLine => "sng",
            Self::WavyDoubleLine => "wavyDbl",
            Self::WavyHeavyLine => "wavyHeavy",
            Self::WavyLine => "wavy",
            Self::Words => "words",
        }
    }

    /// Parse an XML underline type attribute value.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Option<Self> {
        match s {
            "none" => Some(Self::None),
            "dashHeavy" => Some(Self::DashHeavyLine),
            "dash" => Some(Self::DashLine),
            "dashLongHeavy" => Some(Self::DashLongHeavyLine),
            "dashLong" => Some(Self::DashLongLine),
            "dotDashHeavy" => Some(Self::DotDashHeavyLine),
            "dotDash" => Some(Self::DotDashLine),
            "dotDotDashHeavy" => Some(Self::DotDotDashHeavyLine),
            "dotDotDash" => Some(Self::DotDotDashLine),
            "dottedHeavy" => Some(Self::DottedHeavyLine),
            "dotted" => Some(Self::DottedLine),
            "dbl" => Some(Self::DoubleLine),
            "heavy" => Some(Self::HeavyLine),
            "sng" => Some(Self::SingleLine),
            "wavyDbl" => Some(Self::WavyDoubleLine),
            "wavyHeavy" => Some(Self::WavyHeavyLine),
            "wavy" => Some(Self::WavyLine),
            "words" => Some(Self::Words),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pp_align_roundtrip() {
        let aligns = [
            PpParagraphAlignment::Center,
            PpParagraphAlignment::Left,
            PpParagraphAlignment::Right,
            PpParagraphAlignment::Justify,
            PpParagraphAlignment::Distribute,
            PpParagraphAlignment::JustifyLow,
            PpParagraphAlignment::ThaiDistribute,
        ];
        for a in aligns {
            let xml = a.to_xml_str();
            assert_eq!(PpParagraphAlignment::from_xml_str(xml), Some(a));
        }
    }

    #[test]
    fn test_mso_anchor_roundtrip() {
        let anchors = [
            MsoVerticalAnchor::Top,
            MsoVerticalAnchor::Middle,
            MsoVerticalAnchor::Bottom,
        ];
        for a in anchors {
            let xml = a.to_xml_str();
            assert_eq!(MsoVerticalAnchor::from_xml_str(xml), Some(a));
        }
    }

    #[test]
    fn test_mso_auto_size_roundtrip() {
        let sizes = [
            MsoAutoSize::None,
            MsoAutoSize::ShapeToFitText,
            MsoAutoSize::TextToFitShape,
        ];
        for s in sizes {
            let xml = s.to_xml_str();
            assert_eq!(MsoAutoSize::from_xml_str(xml), Some(s));
        }
    }

    #[test]
    fn test_underline_roundtrip() {
        let underlines = [
            MsoTextUnderlineType::None,
            MsoTextUnderlineType::SingleLine,
            MsoTextUnderlineType::DoubleLine,
            MsoTextUnderlineType::WavyLine,
            MsoTextUnderlineType::Words,
        ];
        for u in underlines {
            let xml = u.to_xml_str();
            assert_eq!(MsoTextUnderlineType::from_xml_str(xml), Some(u));
        }
    }

    #[test]
    fn test_text_direction_roundtrip() {
        let dirs = [TextDirection::LeftToRight, TextDirection::RightToLeft];
        for d in dirs {
            let xml = d.to_xml_attr();
            assert_eq!(TextDirection::from_xml_attr(xml), Some(d));
        }
    }

    #[test]
    fn test_text_direction_from_bool_strings() {
        assert_eq!(
            TextDirection::from_xml_attr("true"),
            Some(TextDirection::RightToLeft)
        );
        assert_eq!(
            TextDirection::from_xml_attr("false"),
            Some(TextDirection::LeftToRight)
        );
    }

    #[test]
    fn test_text_direction_unknown() {
        assert_eq!(TextDirection::from_xml_attr("unknown"), None);
    }

    #[test]
    fn test_unknown_alignment() {
        assert_eq!(PpParagraphAlignment::from_xml_str("unknown"), None);
    }
}
