//! Miscellaneous enumerations.

/// Identifies a particular OLE program identifier for embedded objects.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgId {
    /// Microsoft Excel worksheet.
    ExcelWorksheet,
    /// Microsoft Excel chart.
    ExcelChart,
    /// Microsoft Word document.
    WordDocument,
    /// Microsoft `PowerPoint` presentation.
    PowerPointPresentation,
    /// Microsoft `PowerPoint` slide.
    PowerPointSlide,
    /// Microsoft Visio drawing.
    VisioDrawing,
    /// Adobe Acrobat PDF.
    AcrobatDocument,
    /// Package (generic embedded file).
    Package,
}

impl ProgId {
    /// Return the OLE `ProgId` string for this variant.
    #[must_use]
    pub const fn to_prog_id_str(self) -> &'static str {
        match self {
            Self::ExcelWorksheet => "Excel.Sheet.12",
            Self::ExcelChart => "Excel.Chart.8",
            Self::WordDocument => "Word.Document.12",
            Self::PowerPointPresentation => "PowerPoint.Show.12",
            Self::PowerPointSlide => "PowerPoint.Slide.12",
            Self::VisioDrawing => "Visio.Drawing.15",
            Self::AcrobatDocument => "AcroExch.Document",
            Self::Package => "Package",
        }
    }

    /// Parse an OLE `ProgId` string.
    #[must_use]
    pub fn from_prog_id_str(s: &str) -> Option<Self> {
        match s {
            "Excel.Sheet.12" => Some(Self::ExcelWorksheet),
            "Excel.Chart.8" => Some(Self::ExcelChart),
            "Word.Document.12" => Some(Self::WordDocument),
            "PowerPoint.Show.12" => Some(Self::PowerPointPresentation),
            "PowerPoint.Slide.12" => Some(Self::PowerPointSlide),
            "Visio.Drawing.15" => Some(Self::VisioDrawing),
            "AcroExch.Document" => Some(Self::AcrobatDocument),
            "Package" => Some(Self::Package),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// PP_MEDIA_TYPE
// ---------------------------------------------------------------------------

/// Specifies the type of media in a shape.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PpMediaType {
    /// A movie (video) clip.
    Movie,
    /// A sound (audio) clip.
    Sound,
    /// Another media type not covered by Movie or Sound.
    Other,
}

impl PpMediaType {
    /// Return the XML attribute value for this media type.
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::Movie => "movie",
            Self::Sound => "sound",
            Self::Other => "other",
        }
    }

    /// Parse an XML media type attribute value.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Option<Self> {
        match s {
            "movie" => Some(Self::Movie),
            "sound" => Some(Self::Sound),
            "other" => Some(Self::Other),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// ExcelNumFormat - common Excel number format strings
// ---------------------------------------------------------------------------

/// Common Excel number format strings used in chart data labels and table cells.
///
/// These are the standard format codes recognized by Excel and `PowerPoint`.
pub struct ExcelNumFormat;

impl ExcelNumFormat {
    /// General format (no specific formatting).
    pub const GENERAL: &str = "General";
    /// Number with two decimal places: `0.00`
    pub const NUMBER: &str = "0.00";
    /// Number with no decimal places: `0`
    pub const NUMBER_NO_DECIMAL: &str = "0";
    /// Currency with two decimal places: `$#,##0.00`
    pub const CURRENCY: &str = "$#,##0.00";
    /// Percentage with no decimal places: `0%`
    pub const PERCENTAGE: &str = "0%";
    /// Percentage with two decimal places: `0.00%`
    pub const PERCENTAGE_DECIMAL: &str = "0.00%";
    /// Date format: `m/d/yyyy`
    pub const DATE: &str = "m/d/yyyy";
    /// Time format: `h:mm:ss`
    pub const TIME: &str = "h:mm:ss";
}

// ---------------------------------------------------------------------------
// PrintColorMode
// ---------------------------------------------------------------------------

/// Specifies how a presentation should be printed with respect to color.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintColorMode {
    /// Print in full color.
    Color,
    /// Print in grayscale.
    Grayscale,
    /// Print in pure black and white.
    PureBlackWhite,
}

impl PrintColorMode {
    /// Return the XML attribute value for this color mode.
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::Color => "clr",
            Self::Grayscale => "gray",
            Self::PureBlackWhite => "pureBlkWht",
        }
    }

    /// Parse an XML color mode attribute value.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Option<Self> {
        match s {
            "clr" => Some(Self::Color),
            "gray" => Some(Self::Grayscale),
            "pureBlkWht" => Some(Self::PureBlackWhite),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// PrintWhat
// ---------------------------------------------------------------------------

/// Specifies what content should be printed.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintWhat {
    /// Print slides.
    Slides,
    /// Print handouts.
    Handouts,
    /// Print notes pages.
    Notes,
    /// Print outline view.
    Outline,
}

impl PrintWhat {
    /// Return the XML attribute value for this print target.
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::Slides => "slides",
            Self::Handouts => "handouts",
            Self::Notes => "notes",
            Self::Outline => "outline",
        }
    }

    /// Parse an XML print target attribute value.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Option<Self> {
        match s {
            "slides" => Some(Self::Slides),
            "handouts" => Some(Self::Handouts),
            "notes" => Some(Self::Notes),
            "outline" => Some(Self::Outline),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// HandoutLayout
// ---------------------------------------------------------------------------

/// Specifies the number of slides per page for handout printing.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandoutLayout {
    /// One slide per page.
    One,
    /// Two slides per page.
    Two,
    /// Three slides per page.
    Three,
    /// Four slides per page.
    Four,
    /// Six slides per page.
    Six,
    /// Nine slides per page.
    Nine,
}

impl HandoutLayout {
    /// Return the XML attribute value for this handout layout.
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::One => "handouts1",
            Self::Two => "handouts2",
            Self::Three => "handouts3",
            Self::Four => "handouts4",
            Self::Six => "handouts6",
            Self::Nine => "handouts9",
        }
    }

    /// Parse an XML handout layout attribute value.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Option<Self> {
        match s {
            "handouts1" => Some(Self::One),
            "handouts2" => Some(Self::Two),
            "handouts3" => Some(Self::Three),
            "handouts4" => Some(Self::Four),
            "handouts6" => Some(Self::Six),
            "handouts9" => Some(Self::Nine),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// PrintOrientation
// ---------------------------------------------------------------------------

/// Specifies the orientation for printing.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintOrientation {
    /// Portrait orientation.
    Portrait,
    /// Landscape orientation.
    Landscape,
}

impl PrintOrientation {
    /// Return the XML attribute value for this orientation.
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::Portrait => "portrait",
            Self::Landscape => "landscape",
        }
    }

    /// Parse an XML orientation attribute value.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Option<Self> {
        match s {
            "portrait" => Some(Self::Portrait),
            "landscape" => Some(Self::Landscape),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prog_id_roundtrip() {
        let ids = [
            ProgId::ExcelWorksheet,
            ProgId::ExcelChart,
            ProgId::WordDocument,
            ProgId::PowerPointPresentation,
            ProgId::Package,
        ];
        for id in ids {
            let s = id.to_prog_id_str();
            assert_eq!(ProgId::from_prog_id_str(s), Some(id));
        }
    }

    #[test]
    fn test_unknown_prog_id() {
        assert_eq!(ProgId::from_prog_id_str("Unknown.App"), None);
    }

    #[test]
    fn test_pp_media_type_roundtrip() {
        let types = [PpMediaType::Movie, PpMediaType::Sound, PpMediaType::Other];
        for t in types {
            let s = t.to_xml_str();
            assert_eq!(PpMediaType::from_xml_str(s), Some(t));
        }
    }

    #[test]
    fn test_pp_media_type_unknown() {
        assert_eq!(PpMediaType::from_xml_str("unknown"), None);
    }

    #[test]
    fn test_pp_media_type_values() {
        assert_eq!(PpMediaType::Movie.to_xml_str(), "movie");
        assert_eq!(PpMediaType::Sound.to_xml_str(), "sound");
        assert_eq!(PpMediaType::Other.to_xml_str(), "other");
    }

    #[test]
    fn test_excel_num_format_constants() {
        assert_eq!(ExcelNumFormat::GENERAL, "General");
        assert_eq!(ExcelNumFormat::NUMBER, "0.00");
        assert_eq!(ExcelNumFormat::NUMBER_NO_DECIMAL, "0");
        assert_eq!(ExcelNumFormat::CURRENCY, "$#,##0.00");
        assert_eq!(ExcelNumFormat::PERCENTAGE, "0%");
        assert_eq!(ExcelNumFormat::PERCENTAGE_DECIMAL, "0.00%");
        assert_eq!(ExcelNumFormat::DATE, "m/d/yyyy");
        assert_eq!(ExcelNumFormat::TIME, "h:mm:ss");
    }

    #[test]
    fn test_print_color_mode_roundtrip() {
        let modes = [
            PrintColorMode::Color,
            PrintColorMode::Grayscale,
            PrintColorMode::PureBlackWhite,
        ];
        for m in modes {
            let s = m.to_xml_str();
            assert_eq!(PrintColorMode::from_xml_str(s), Some(m));
        }
    }

    #[test]
    fn test_print_color_mode_unknown() {
        assert_eq!(PrintColorMode::from_xml_str("unknown"), None);
    }

    #[test]
    fn test_print_what_roundtrip() {
        let targets = [
            PrintWhat::Slides,
            PrintWhat::Handouts,
            PrintWhat::Notes,
            PrintWhat::Outline,
        ];
        for t in targets {
            let s = t.to_xml_str();
            assert_eq!(PrintWhat::from_xml_str(s), Some(t));
        }
    }

    #[test]
    fn test_print_what_unknown() {
        assert_eq!(PrintWhat::from_xml_str("unknown"), None);
    }

    #[test]
    fn test_handout_layout_roundtrip() {
        let layouts = [
            HandoutLayout::One,
            HandoutLayout::Two,
            HandoutLayout::Three,
            HandoutLayout::Four,
            HandoutLayout::Six,
            HandoutLayout::Nine,
        ];
        for l in layouts {
            let s = l.to_xml_str();
            assert_eq!(HandoutLayout::from_xml_str(s), Some(l));
        }
    }

    #[test]
    fn test_handout_layout_unknown() {
        assert_eq!(HandoutLayout::from_xml_str("unknown"), None);
    }

    #[test]
    fn test_print_orientation_roundtrip() {
        let orientations = [PrintOrientation::Portrait, PrintOrientation::Landscape];
        for o in orientations {
            let s = o.to_xml_str();
            assert_eq!(PrintOrientation::from_xml_str(s), Some(o));
        }
    }

    #[test]
    fn test_print_orientation_unknown() {
        assert_eq!(PrintOrientation::from_xml_str("unknown"), None);
    }
}
