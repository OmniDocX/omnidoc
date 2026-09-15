use crate::error::PptxResult;
use crate::opc::constants::content_type as CT;
use crate::opc::pack_uri::PackURI;
use crate::opc::relationship::Relationships;

/// Classification of a part based on its content type.
///
/// Provides Rust-idiomatic dispatch based on content type, replacing
/// the `PartFactory` pattern from python-pptx.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartType {
    Slide,
    SlideLayout,
    SlideMaster,
    NotesSlide,
    NotesMaster,
    Chart,
    Image,
    Video,
    Audio,
    Theme,
    Presentation,
    CoreProperties,
    CustomXml,
    Comments,
    Font,
    SmartArt,
    Unknown,
}

/// Map a content type string to a `PartType`.
#[must_use]
pub fn part_type_from_content_type(ct: &str) -> PartType {
    match ct {
        CT::PML_SLIDE => PartType::Slide,
        CT::PML_SLIDE_LAYOUT => PartType::SlideLayout,
        CT::PML_SLIDE_MASTER => PartType::SlideMaster,
        CT::PML_NOTES_SLIDE => PartType::NotesSlide,
        CT::PML_NOTES_MASTER => PartType::NotesMaster,
        CT::DML_CHART => PartType::Chart,
        CT::PML_PRESENTATION_MAIN => PartType::Presentation,
        CT::OPC_CORE_PROPERTIES => PartType::CoreProperties,
        CT::OFC_THEME => PartType::Theme,
        CT::PML_COMMENTS => PartType::Comments,
        CT::X_FONTDATA | CT::X_FONT_TTF => PartType::Font,
        _ if ct.starts_with("image/") => PartType::Image,
        _ if ct.starts_with("video/") => PartType::Video,
        _ if ct.starts_with("audio/") => PartType::Audio,
        _ => PartType::Unknown,
    }
}

/// A part within an OPC package.
///
/// Parts are the fundamental units of content in an OPC package. Each part has:
/// - A partname (`PackURI`) identifying it within the package
/// - A content type (MIME type)
/// - Binary content stored as a `Vec<u8>` blob
/// - A collection of relationships to other parts
#[derive(Debug, Clone)]
pub struct Part {
    /// The pack URI (partname) of this part within the package.
    pub partname: PackURI,
    /// The content type (MIME type) of this part.
    pub content_type: String,
    /// The raw binary content of this part.
    pub blob: Vec<u8>,
    /// Relationships from this part to other parts or external resources.
    pub rels: Relationships,
}

impl Part {
    /// Create a new part with the given partname, content type, and blob.
    pub fn new(partname: PackURI, content_type: impl Into<String>, blob: Vec<u8>) -> Self {
        let base_uri = partname.base_uri().to_string();
        Self {
            partname,
            content_type: content_type.into(),
            blob,
            rels: Relationships::new(base_uri),
        }
    }

    /// Create a new part with pre-existing relationships.
    pub fn with_rels(
        partname: PackURI,
        content_type: impl Into<String>,
        blob: Vec<u8>,
        rels: Relationships,
    ) -> Self {
        Self {
            partname,
            content_type: content_type.into(),
            blob,
            rels,
        }
    }

    /// Get the single related part's target ref for the given relationship type.
    ///
    /// # Errors
    ///
    /// Returns an error if no relationship of the given type exists.
    pub fn related_part_ref(&self, reltype: &str) -> PptxResult<&str> {
        let rel = self.rels.by_reltype(reltype)?;
        Ok(&rel.target_ref)
    }

    /// Resolve a relationship's target to an absolute `PackURI`.
    ///
    /// # Errors
    ///
    /// Returns an error if the relationship ID is not found or resolution fails.
    pub fn related_partname(&self, r_id: &str) -> PptxResult<PackURI> {
        let rel = self.rels.get(r_id).ok_or_else(|| {
            crate::error::PptxError::Package(crate::error::PackageError::RelationshipNotFound(
                r_id.to_string(),
            ))
        })?;
        rel.target_partname(self.partname.base_uri())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_part_type_slide() {
        assert_eq!(part_type_from_content_type(CT::PML_SLIDE), PartType::Slide);
    }

    #[test]
    fn test_part_type_slide_layout() {
        assert_eq!(
            part_type_from_content_type(CT::PML_SLIDE_LAYOUT),
            PartType::SlideLayout
        );
    }

    #[test]
    fn test_part_type_slide_master() {
        assert_eq!(
            part_type_from_content_type(CT::PML_SLIDE_MASTER),
            PartType::SlideMaster
        );
    }

    #[test]
    fn test_part_type_notes_slide() {
        assert_eq!(
            part_type_from_content_type(CT::PML_NOTES_SLIDE),
            PartType::NotesSlide
        );
    }

    #[test]
    fn test_part_type_chart() {
        assert_eq!(part_type_from_content_type(CT::DML_CHART), PartType::Chart);
    }

    #[test]
    fn test_part_type_image_png() {
        assert_eq!(part_type_from_content_type("image/png"), PartType::Image);
    }

    #[test]
    fn test_part_type_image_jpeg() {
        assert_eq!(part_type_from_content_type("image/jpeg"), PartType::Image);
    }

    #[test]
    fn test_part_type_video() {
        assert_eq!(part_type_from_content_type("video/mp4"), PartType::Video);
    }

    #[test]
    fn test_part_type_audio() {
        assert_eq!(part_type_from_content_type("audio/mpeg"), PartType::Audio);
    }

    #[test]
    fn test_part_type_theme() {
        assert_eq!(part_type_from_content_type(CT::OFC_THEME), PartType::Theme);
    }

    #[test]
    fn test_part_type_presentation() {
        assert_eq!(
            part_type_from_content_type(CT::PML_PRESENTATION_MAIN),
            PartType::Presentation
        );
    }

    #[test]
    fn test_part_type_core_properties() {
        assert_eq!(
            part_type_from_content_type(CT::OPC_CORE_PROPERTIES),
            PartType::CoreProperties
        );
    }

    #[test]
    fn test_part_type_font() {
        assert_eq!(part_type_from_content_type(CT::X_FONTDATA), PartType::Font);
        assert_eq!(part_type_from_content_type(CT::X_FONT_TTF), PartType::Font);
    }

    #[test]
    fn test_part_type_comments() {
        assert_eq!(
            part_type_from_content_type(CT::PML_COMMENTS),
            PartType::Comments
        );
    }

    #[test]
    fn test_part_type_unknown() {
        assert_eq!(
            part_type_from_content_type("application/something-weird"),
            PartType::Unknown
        );
    }
}
