use std::path::Path;

use crate::error::{PptxError, PptxResult};

const MAX_FONT_SIZE: u64 = 100 * 1024 * 1024; // 100 MB

/// An embedded font that can be stored within a .pptx package.
///
/// `PowerPoint` supports embedding fonts so that the presentation renders
/// correctly even on systems where the font is not installed. Font data
/// is stored as binary parts at `/ppt/fonts/fontN.fntdata` with content
/// type `application/x-fontdata`.
#[derive(Debug, Clone)]
pub struct EmbeddedFont {
    /// Font family name (typeface), e.g. "Calibri".
    pub typeface: String,
    /// Raw font file bytes (.ttf, .otf, or obfuscated .fntdata).
    pub font_data: Vec<u8>,
    /// Whether this font variant is bold.
    pub bold: bool,
    /// Whether this font variant is italic.
    pub italic: bool,
}

impl EmbeddedFont {
    /// Create an `EmbeddedFont` from a font file on disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn from_file(
        path: impl AsRef<Path>,
        typeface: impl Into<String>,
        bold: bool,
        italic: bool,
    ) -> PptxResult<Self> {
        let path = path.as_ref();
        let metadata = std::fs::metadata(path).map_err(PptxError::Io)?;
        if metadata.len() > MAX_FONT_SIZE {
            return Err(PptxError::ResourceLimit {
                message: format!(
                    "font file size {} bytes exceeds the limit of {} bytes",
                    metadata.len(),
                    MAX_FONT_SIZE
                ),
            });
        }
        let data = std::fs::read(path).map_err(PptxError::Io)?;
        Ok(Self {
            typeface: typeface.into(),
            font_data: data,
            bold,
            italic,
        })
    }

    /// Create an `EmbeddedFont` from raw bytes.
    pub fn from_bytes(
        data: Vec<u8>,
        typeface: impl Into<String>,
        bold: bool,
        italic: bool,
    ) -> Self {
        Self {
            typeface: typeface.into(),
            font_data: data,
            bold,
            italic,
        }
    }
}

/// The content type for embedded font data parts.
pub const FONT_DATA_CONTENT_TYPE: &str = "application/x-fontdata";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_font_from_bytes() {
        let font = EmbeddedFont::from_bytes(vec![0x00, 0x01, 0x00, 0x00], "TestFont", false, false);
        assert_eq!(font.typeface, "TestFont");
        assert_eq!(font.font_data, vec![0x00, 0x01, 0x00, 0x00]);
        assert!(!font.bold);
        assert!(!font.italic);
    }

    #[test]
    fn test_embedded_font_bold_italic() {
        let font = EmbeddedFont::from_bytes(vec![1, 2, 3], "Arial", true, true);
        assert_eq!(font.typeface, "Arial");
        assert!(font.bold);
        assert!(font.italic);
    }

    #[test]
    fn test_embedded_font_clone() {
        let font = EmbeddedFont::from_bytes(vec![1, 2], "Calibri", false, true);
        let cloned = font.clone();
        assert_eq!(cloned.typeface, "Calibri");
        assert_eq!(cloned.font_data, vec![1, 2]);
        assert!(!cloned.bold);
        assert!(cloned.italic);
    }
}
