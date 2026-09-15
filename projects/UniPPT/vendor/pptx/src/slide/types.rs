//! Data types for slide, layout, master, and notes references.

use crate::enums::shapes::PpPlaceholderType;
use crate::opc::pack_uri::PackURI;
use crate::shapes::shapetree::ShapeTree;
use crate::shapes::Shape;
use crate::text::TextFrame;
use crate::units::{RelationshipId, SlideId};

/// A reference to a slide within a presentation.
///
/// This is a lightweight handle that stores the slide's partname and relationship ID.
/// Actual slide content (the XML blob) is accessed through the [`OpcPackage`](crate::opc::OpcPackage).
#[derive(Debug, Clone)]
pub struct SlideRef {
    /// The rId linking this slide from the presentation part.
    pub r_id: RelationshipId,
    /// Absolute partname of the slide part.
    pub partname: PackURI,
}

/// A reference to a slide layout within the package.
#[derive(Debug, Clone)]
pub struct SlideLayoutRef {
    /// The rId linking this layout from its slide master.
    pub r_id: RelationshipId,
    /// Absolute partname of the slide layout part.
    pub partname: PackURI,
    /// The layout name from the XML (e.g. "Title Slide", "Title and Content").
    pub name: String,
    /// Absolute partname of the parent slide master, if resolved.
    pub slide_master_part_name: Option<String>,
}

/// A reference to a slide master within the package.
#[derive(Debug, Clone)]
pub struct SlideMasterRef {
    /// The rId linking this master from the presentation part.
    pub r_id: RelationshipId,
    /// Absolute partname of the slide master part.
    pub partname: PackURI,
}

/// A reference to a notes slide within the package.
#[derive(Debug, Clone)]
pub struct NotesSlideRef {
    /// Absolute partname of the notes slide part.
    pub partname: PackURI,
}

/// A reference to the notes master within the package.
#[derive(Debug, Clone)]
pub struct NotesMasterRef {
    /// The rId linking this notes master from the presentation part.
    pub r_id: RelationshipId,
    /// Absolute partname of the notes master part.
    pub partname: PackURI,
}

/// Properties of a slide.
#[derive(Debug, Clone)]
pub struct SlideProperties {
    /// The numeric slide ID from the presentation XML.
    pub slide_id: SlideId,
    /// The optional name from the `<p:cSld name="...">` element.
    pub name: String,
    /// Whether this slide has an associated notes slide.
    pub has_notes_slide: bool,
}

/// A parsed notes slide with its shapes and text content.
///
/// Provides access to the notes slide's shape tree, placeholder shapes,
/// and the notes text body.
#[derive(Debug, Clone)]
pub struct NotesSlide {
    /// The optional name from the `<p:cSld name="...">` element.
    pub name: Option<String>,
    /// All shapes parsed from the notes slide's shape tree.
    pub shapes: ShapeTree,
    /// The plain text content of the notes body placeholder.
    pub notes_text: String,
    /// The part name (URI) of this notes slide within the package.
    pub part_name: Option<String>,
}

impl NotesSlide {
    /// Get the optional name of the notes slide.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get the part name (URI) of this notes slide within the package.
    #[must_use]
    pub fn part_name(&self) -> Option<&str> {
        self.part_name.as_deref()
    }

    /// Get the shape tree containing all shapes on this notes slide.
    #[must_use]
    pub const fn shapes(&self) -> &ShapeTree {
        &self.shapes
    }

    /// Get all placeholder shapes on this notes slide.
    #[must_use]
    pub fn placeholders(&self) -> Vec<&Shape> {
        self.shapes
            .shapes
            .iter()
            .filter(|s| s.is_placeholder())
            .collect()
    }

    /// Find the notes body placeholder (typically `type="body"` with `idx=1`).
    #[must_use]
    pub fn notes_placeholder(&self) -> Option<&Shape> {
        self.shapes.shapes.iter().find(|s| {
            s.placeholder()
                .is_some_and(|ph| ph.ph_type == Some(PpPlaceholderType::Body))
        })
    }

    /// Get the `TextFrame` from the notes body placeholder, if it is an `AutoShape`.
    #[must_use]
    pub fn notes_text_frame(&self) -> Option<&TextFrame> {
        self.notes_placeholder()
            .and_then(|s| s.as_autoshape())
            .and_then(|a| a.text_frame())
    }

    /// Get the plain text content of the notes.
    #[must_use]
    pub fn notes_text(&self) -> &str {
        &self.notes_text
    }
}
