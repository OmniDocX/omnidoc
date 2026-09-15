/// Content type URIs (MIME-types) that specify a part's format.
pub mod content_type {
    pub const XML: &str = "application/xml";
    pub const OPC_RELATIONSHIPS: &str = "application/vnd.openxmlformats-package.relationships+xml";
    pub const OPC_CORE_PROPERTIES: &str =
        "application/vnd.openxmlformats-package.core-properties+xml";

    // Presentation ML
    pub const PML_PRESENTATION_MAIN: &str =
        "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml";
    pub const PML_SLIDE: &str =
        "application/vnd.openxmlformats-officedocument.presentationml.slide+xml";
    pub const PML_SLIDE_LAYOUT: &str =
        "application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml";
    pub const PML_SLIDE_MASTER: &str =
        "application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml";
    pub const PML_NOTES_MASTER: &str =
        "application/vnd.openxmlformats-officedocument.presentationml.notesMaster+xml";
    pub const PML_NOTES_SLIDE: &str =
        "application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml";
    pub const PML_PRINTER_SETTINGS: &str =
        "application/vnd.openxmlformats-officedocument.presentationml.printerSettings";
    pub const PML_COMMENTS: &str =
        "application/vnd.openxmlformats-officedocument.presentationml.comments+xml";

    // DrawingML
    pub const DML_CHART: &str = "application/vnd.openxmlformats-officedocument.drawingml.chart+xml";

    // SpreadsheetML (embedded xlsx for chart data)
    pub const SML_SHEET: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";

    // Office shared
    pub const OFC_THEME: &str = "application/vnd.openxmlformats-officedocument.theme+xml";

    // Image types
    pub const BMP: &str = "image/bmp";
    pub const GIF: &str = "image/gif";
    pub const JPEG: &str = "image/jpeg";
    pub const PNG: &str = "image/png";
    pub const TIFF: &str = "image/tiff";
    pub const X_EMF: &str = "image/x-emf";
    pub const X_WMF: &str = "image/x-wmf";

    // Video types
    pub const MP4: &str = "video/mp4";
    pub const MOV: &str = "video/quicktime";
    pub const WMV: &str = "video/x-ms-wmv";

    // Audio types
    pub const AUDIO_MPEG: &str = "audio/mpeg";
    pub const AUDIO_WAV: &str = "audio/wav";
    pub const AUDIO_MP4: &str = "audio/mp4";

    // SVG
    pub const SVG: &str = "image/svg+xml";

    // Other
    pub const X_FONTDATA: &str = "application/x-fontdata";
    pub const X_FONT_TTF: &str = "application/x-font-ttf";

    // Macro-enabled presentation
    pub const PML_PRESENTATION_MACRO: &str =
        "application/vnd.ms-powerpoint.presentation.macroEnabled.main+xml";

    // VBA
    pub const VBA_PROJECT: &str = "application/vnd.ms-office.vbaProject";
}

/// Relationship type URIs describing the relationship between source and target.
pub mod relationship_type {
    pub const OFFICE_DOCUMENT: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
    pub const SLIDE: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide";
    pub const SLIDE_LAYOUT: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout";
    pub const SLIDE_MASTER: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster";
    pub const NOTES_MASTER: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesMaster";
    pub const NOTES_SLIDE: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide";
    pub const THEME: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme";
    pub const IMAGE: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";
    pub const CHART: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart";
    pub const FONT: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/font";
    pub const VIDEO: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/video";
    pub const AUDIO: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/audio";
    pub const MEDIA: &str = "http://schemas.microsoft.com/office/2007/relationships/media";
    pub const CORE_PROPERTIES: &str =
        "http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties";
    pub const PACKAGE: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/package";
    pub const VBA_PROJECT: &str =
        "http://schemas.microsoft.com/office/2006/relationships/vbaProject";
}

/// XML namespace URIs used in OPC and Open XML.
pub mod namespace {
    pub const OPC_RELATIONSHIPS: &str =
        "http://schemas.openxmlformats.org/package/2006/relationships";
    pub const OPC_CONTENT_TYPES: &str =
        "http://schemas.openxmlformats.org/package/2006/content-types";
}

/// Default content types that can be inferred from file extension.
pub const DEFAULT_CONTENT_TYPES: &[(&str, &str)] = &[
    ("bin", content_type::PML_PRINTER_SETTINGS),
    ("fntdata", content_type::X_FONTDATA),
    ("bmp", content_type::BMP),
    ("emf", content_type::X_EMF),
    ("gif", content_type::GIF),
    ("jpe", content_type::JPEG),
    ("jpeg", content_type::JPEG),
    ("jpg", content_type::JPEG),
    ("m4a", content_type::AUDIO_MP4),
    ("mov", content_type::MOV),
    ("mp3", content_type::AUDIO_MPEG),
    ("mp4", content_type::MP4),
    ("png", content_type::PNG),
    ("rels", content_type::OPC_RELATIONSHIPS),
    ("svg", content_type::SVG),
    ("tif", content_type::TIFF),
    ("tiff", content_type::TIFF),
    ("wav", content_type::AUDIO_WAV),
    ("wmf", content_type::X_WMF),
    ("wmv", content_type::WMV),
    ("xml", content_type::XML),
];
