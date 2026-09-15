//! Notes slide and notes master operations on a [`Presentation`].

use crate::error::{PartNotFoundExt, PptxError, PptxResult};
use crate::opc::constants::{content_type as CT, relationship_type as RT};
use crate::opc::part::Part;
use crate::shapes::Shape;
use crate::slide::{
    new_notes_master_xml, new_notes_slide_xml, parse_notes_slide_text,
    parse_notes_slide_with_part_name, set_slide_background_solid, NotesMasterRef, NotesSlide,
    NotesSlideRef,
};
use crate::units::RelationshipId;

use super::Presentation;

impl Presentation {
    /// Get or create a notes slide for the given slide.
    ///
    /// If the slide already has a notes slide relationship, that reference is returned.
    /// Otherwise a new notes slide is created and linked.
    /// # Errors
    ///
    /// Returns an error if the notes slide cannot be created.
    pub fn notes_slide_or_create(
        &mut self,
        slide_ref: &crate::slide::SlideRef,
    ) -> PptxResult<NotesSlideRef> {
        // Check if the slide already has a notes slide relationship
        {
            let slide_part = self
                .package
                .part(&slide_ref.partname)
                .or_part_not_found(slide_ref.partname.as_str())?;
            let notes_rels = slide_part.rels.all_by_reltype(RT::NOTES_SLIDE);
            if let Some(notes_rel) = notes_rels.first() {
                let partname = notes_rel.target_partname(slide_part.partname.base_uri())?;
                return Ok(NotesSlideRef { partname });
            }
        }

        // Create a new notes slide
        let notes_partname = self
            .package
            .next_partname("/ppt/notesSlides/notesSlide{}.xml")?;
        let notes_blob = new_notes_slide_xml();

        // Pre-compute relative refs before consuming the partname
        let slide_target_ref = slide_ref.partname.relative_ref(notes_partname.base_uri());
        let notes_target_ref = notes_partname.relative_ref(slide_ref.partname.base_uri());

        let mut notes_part = Part::new(notes_partname.clone(), CT::PML_NOTES_SLIDE, notes_blob);
        notes_part
            .rels
            .add_relationship(RT::SLIDE, &slide_target_ref, false);

        self.package.put_part(notes_part);
        let slide_part = self
            .package
            .part_mut(&slide_ref.partname)
            .or_part_not_found(slide_ref.partname.as_str())?;
        slide_part
            .rels
            .add_relationship(RT::NOTES_SLIDE, &notes_target_ref, false);

        Ok(NotesSlideRef {
            partname: notes_partname,
        })
    }

    /// Get the notes master for this presentation, if one exists.
    ///
    /// Returns `None` if the presentation does not have a notes master part.
    /// # Errors
    ///
    /// Returns an error if the presentation part cannot be accessed.
    pub fn notes_master(&self) -> PptxResult<Option<NotesMasterRef>> {
        let pres_part = self.presentation_part()?;
        let notes_master_rels = pres_part.rels.all_by_reltype(RT::NOTES_MASTER);
        match notes_master_rels.first() {
            Some(rel) => {
                let partname = rel.target_partname(pres_part.partname.base_uri())?;
                Ok(Some(NotesMasterRef {
                    r_id: rel.r_id.clone(),
                    partname,
                }))
            }
            None => Ok(None),
        }
    }

    /// Get or create a notes master for this presentation.
    ///
    /// If the presentation already has a notes master, its reference is returned.
    /// Otherwise a new minimal notes master is created and linked to the
    /// presentation part.
    /// # Errors
    ///
    /// Returns an error if the notes master cannot be created.
    pub fn notes_master_or_create(&mut self) -> PptxResult<NotesMasterRef> {
        // Check if one already exists
        if let Some(existing) = self.notes_master()? {
            return Ok(existing);
        }

        // Create a new notes master
        let nm_partname = self
            .package
            .next_partname("/ppt/notesMasters/notesMaster{}.xml")?;
        let nm_blob = new_notes_master_xml();

        // Link notes master to the first theme
        let mut nm_part = Part::new(nm_partname.clone(), CT::PML_NOTES_MASTER, nm_blob);

        // Find the first theme and add a relationship from the notes master to it
        let pres_part = self.presentation_part()?;
        let theme_rels = pres_part.rels.all_by_reltype(RT::THEME);
        if let Some(theme_rel) = theme_rels.first() {
            let theme_partname = theme_rel.target_partname(pres_part.partname.base_uri())?;
            let theme_target_ref = theme_partname.relative_ref(nm_partname.base_uri());
            nm_part
                .rels
                .add_relationship(RT::THEME, &theme_target_ref, false);
        }

        self.package.put_part(nm_part);

        // Add relationship from presentation to notes master
        let pres_partname = self.presentation_partname()?;
        let nm_target_ref = nm_partname.relative_ref(pres_partname.base_uri());
        let pres_part_mut = self
            .package
            .part_mut(&pres_partname)
            .or_part_not_found(pres_partname.as_str())?;
        let r_id_str = pres_part_mut
            .rels
            .add_relationship(RT::NOTES_MASTER, &nm_target_ref, false);
        let r_id = RelationshipId::try_from(r_id_str.as_str())
            .map_err(|_| PptxError::InvalidXml(format!("invalid notes master rId: {r_id_str}")))?;

        Ok(NotesMasterRef {
            r_id,
            partname: nm_partname,
        })
    }

    /// Get the XML blob of the notes master by its reference.
    ///
    /// # Errors
    ///
    /// Returns an error if the notes master part is not found.
    pub fn notes_master_xml(&self, nm_ref: &NotesMasterRef) -> PptxResult<&[u8]> {
        let part = self
            .package
            .part(&nm_ref.partname)
            .or_part_not_found(nm_ref.partname.as_str())?;
        Ok(&part.blob)
    }

    /// Check whether a slide has an associated notes slide.
    #[must_use]
    pub fn has_notes_slide(&self, slide_ref: &crate::slide::SlideRef) -> bool {
        self.package
            .part(&slide_ref.partname)
            .is_some_and(|p| !p.rels.all_by_reltype(RT::NOTES_SLIDE).is_empty())
    }

    /// Get the text content of a slide's notes.
    ///
    /// Returns `Ok(Some(text))` if the slide has a notes slide with text,
    /// `Ok(None)` if there is no notes slide.
    /// # Errors
    ///
    /// Returns an error if the notes slide cannot be parsed.
    pub fn notes_slide_text(
        &self,
        slide_ref: &crate::slide::SlideRef,
    ) -> PptxResult<Option<String>> {
        let slide_part = self
            .package
            .part(&slide_ref.partname)
            .or_part_not_found(slide_ref.partname.as_str())?;

        let notes_rels = slide_part.rels.all_by_reltype(RT::NOTES_SLIDE);
        let notes_rel = match notes_rels.first() {
            Some(r) => *r,
            None => return Ok(None),
        };

        let notes_partname = notes_rel.target_partname(slide_part.partname.base_uri())?;
        let notes_part = self
            .package
            .part(&notes_partname)
            .or_part_not_found(notes_partname.as_str())?;

        let text = parse_notes_slide_text(&notes_part.blob)?;
        Ok(Some(text))
    }

    /// Parse and return a `NotesSlide` for the given slide, if one exists.
    ///
    /// Returns `Ok(Some(NotesSlide))` if the slide has an associated notes slide,
    /// `Ok(None)` otherwise.
    /// # Errors
    ///
    /// Returns an error if the notes slide cannot be parsed.
    pub fn notes_slide(
        &self,
        slide_ref: &crate::slide::SlideRef,
    ) -> PptxResult<Option<NotesSlide>> {
        let slide_part = self
            .package
            .part(&slide_ref.partname)
            .or_part_not_found(slide_ref.partname.as_str())?;

        let notes_rels = slide_part.rels.all_by_reltype(RT::NOTES_SLIDE);
        let notes_rel = match notes_rels.first() {
            Some(r) => *r,
            None => return Ok(None),
        };

        let notes_partname = notes_rel.target_partname(slide_part.partname.base_uri())?;
        let notes_part = self
            .package
            .part(&notes_partname)
            .or_part_not_found(notes_partname.as_str())?;

        let notes_slide = parse_notes_slide_with_part_name(
            &notes_part.blob,
            Some(notes_partname.as_str().to_owned()),
        )?;
        Ok(Some(notes_slide))
    }

    /// Get the name of a slide's notes slide, if one exists.
    ///
    /// Returns `Ok(Some(name))` if the notes slide exists and has a name,
    /// `Ok(None)` if there is no notes slide or no name attribute.
    /// # Errors
    ///
    /// Returns an error if the notes slide cannot be parsed.
    pub fn notes_slide_name(
        &self,
        slide_ref: &crate::slide::SlideRef,
    ) -> PptxResult<Option<String>> {
        let ns = self.notes_slide(slide_ref)?;
        Ok(ns.and_then(|n| n.name))
    }

    /// Set a solid color background on a slide's notes slide.
    ///
    /// The color should be a 6-char hex RGB string (e.g. "FF0000" for red).
    /// The slide must already have a notes slide; use `notes_slide_or_create`
    /// first if needed.
    /// # Errors
    ///
    /// Returns an error if the notes slide cannot be found or updated.
    pub fn set_notes_slide_background_solid(
        &mut self,
        slide_ref: &crate::slide::SlideRef,
        color: &str,
    ) -> PptxResult<()> {
        let notes_partname = {
            let slide_part = self
                .package
                .part(&slide_ref.partname)
                .or_part_not_found(slide_ref.partname.as_str())?;

            let notes_rels = slide_part.rels.all_by_reltype(RT::NOTES_SLIDE);
            let notes_rel = notes_rels
                .first()
                .or_part_not_found("slide has no notes slide")?;

            notes_rel.target_partname(slide_part.partname.base_uri())?
        };

        let notes_part = self
            .package
            .part_mut(&notes_partname)
            .or_part_not_found(notes_partname.as_str())?;

        set_slide_background_solid(&mut notes_part.blob, color)
    }

    /// Get the placeholder shapes on a slide.
    ///
    /// Parses the slide's shape tree and returns only shapes where
    /// `is_placeholder()` is true.
    /// # Errors
    ///
    /// Returns an error if the slide XML cannot be parsed.
    pub fn slide_placeholders(&self, slide_ref: &crate::slide::SlideRef) -> PptxResult<Vec<Shape>> {
        let slide_xml = self.slide_xml(slide_ref)?;
        let tree = crate::shapes::shapetree::ShapeTree::from_slide_xml(slide_xml)?;
        Ok(tree
            .shapes
            .into_iter()
            .filter(super::super::shapes::Shape::is_placeholder)
            .collect())
    }
}
