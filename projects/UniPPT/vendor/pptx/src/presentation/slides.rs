use crate::error::{PackageError, PartNotFoundExt, PptxError, PptxResult};
use crate::opc::constants::content_type as CT;
use crate::opc::constants::relationship_type as RT;
use crate::opc::part::Part;
use crate::slide::{
    add_slide_id_to_presentation_xml, new_slide_xml, next_slide_id, parse_slide_ids,
    parse_slide_name, parse_slide_size, remove_slide_id_from_presentation_xml,
    reorder_slide_in_presentation_xml, set_slide_size_in_xml, SlideLayoutRef, SlideRef,
};
use crate::units::RelationshipId;

use super::Presentation;

/// Slide management methods for `Presentation`.
impl Presentation {
    /// Get references to all slides in the presentation, in order.
    ///
    /// # Errors
    ///
    /// Returns an error if the presentation XML cannot be parsed.
    pub fn slides(&self) -> PptxResult<Vec<SlideRef>> {
        let pres_part = self.presentation_part()?;
        let slide_ids = parse_slide_ids(&pres_part.blob)?;
        let base_uri = pres_part.partname.base_uri();

        let mut slides = Vec::with_capacity(slide_ids.len());
        for (r_id_str, _id) in slide_ids {
            let rel = pres_part.rels.get(&r_id_str).ok_or_else(|| {
                PptxError::Package(PackageError::RelationshipNotFound(format!(
                    "slide relationship {r_id_str} not found"
                )))
            })?;
            let partname = rel.target_partname(base_uri)?;
            let r_id = RelationshipId::try_from(r_id_str.as_str())
                .map_err(|_| PptxError::InvalidXml(format!("invalid slide rId: {r_id_str}")))?;
            slides.push(SlideRef { r_id, partname });
        }

        Ok(slides)
    }

    /// Get the number of slides in the presentation.
    ///
    /// # Errors
    ///
    /// Returns an error if the presentation XML cannot be parsed.
    pub fn slide_count(&self) -> PptxResult<usize> {
        let pres_part = self.presentation_part()?;
        let slide_ids = parse_slide_ids(&pres_part.blob)?;
        Ok(slide_ids.len())
    }

    /// Get a slide by its 0-based index in the presentation.
    ///
    /// Returns `Err` if the index is out of bounds.
    /// # Errors
    ///
    /// Returns an error if the index is out of bounds.
    pub fn slides_get(&self, index: usize) -> PptxResult<SlideRef> {
        let slides = self.slides()?;
        let len = slides.len();
        slides.into_iter().nth(index).ok_or_else(|| {
            PptxError::InvalidXml(format!(
                "slide index {index} out of range (presentation has {len} slides)",
            ))
        })
    }

    /// Return the 0-based index of a slide in the presentation.
    ///
    /// Matches by partname. Returns `Err` if the slide is not found.
    /// # Errors
    ///
    /// Returns an error if the slide is not found.
    pub fn slide_index(&self, slide_ref: &SlideRef) -> PptxResult<usize> {
        let slides = self.slides()?;
        slides
            .iter()
            .position(|s| s.partname.as_str() == slide_ref.partname.as_str())
            .ok_or_else(|| {
                PptxError::Package(PackageError::PartNotFound(format!(
                    "slide {} not found in presentation",
                    slide_ref.partname
                )))
            })
    }

    /// Move a slide from one position to another.
    ///
    /// Both `from_index` and `to_index` are 0-based. This reorders the
    /// `<p:sldIdLst>` entries in the presentation XML.
    /// # Errors
    ///
    /// Returns an error if the indices are out of range.
    pub fn move_slide(&mut self, from_index: usize, to_index: usize) -> PptxResult<()> {
        if from_index == to_index {
            return Ok(());
        }
        let pres_partname = self.presentation_partname()?;
        let pres_part = self
            .package
            .part_mut(&pres_partname)
            .or_part_not_found(pres_partname.as_str())?;
        pres_part.blob = reorder_slide_in_presentation_xml(&pres_part.blob, from_index, to_index)?;
        Ok(())
    }

    /// Add a new blank slide to the presentation using the given layout.
    ///
    /// Returns a `SlideRef` for the newly created slide.
    /// # Errors
    ///
    /// Returns an error if the slide cannot be created.
    pub fn add_slide(&mut self, layout: &SlideLayoutRef) -> PptxResult<SlideRef> {
        // Determine the next slide partname
        let slide_partname = self.package.next_partname("/ppt/slides/slide{}.xml")?;

        // Pre-compute relative references before consuming the partname
        let layout_target_ref = layout.partname.relative_ref(slide_partname.base_uri());
        let pres_partname = self.presentation_partname()?;
        let slide_target_ref = slide_partname.relative_ref(pres_partname.base_uri());

        // Create the new slide part with a blank slide XML
        let slide_blob = new_slide_xml();
        let mut slide_part = Part::new(slide_partname.clone(), CT::PML_SLIDE, slide_blob);

        // Add a relationship from the slide to its layout
        slide_part
            .rels
            .add_relationship(RT::SLIDE_LAYOUT, &layout_target_ref, false);

        // Add the slide part to the package
        self.package.put_part(slide_part);

        let pres_part = self
            .package
            .part_mut(&pres_partname)
            .or_part_not_found(pres_partname.as_str())?;

        let r_id_str = pres_part
            .rels
            .add_relationship(RT::SLIDE, &slide_target_ref, false);

        // Update the presentation XML to include the new slide in sldIdLst
        let existing_ids = parse_slide_ids(&pres_part.blob)?;
        let slide_id = next_slide_id(&existing_ids);
        let updated_xml = add_slide_id_to_presentation_xml(&pres_part.blob, &r_id_str, slide_id)?;
        pres_part.blob = updated_xml;

        let r_id = RelationshipId::try_from(r_id_str.as_str())
            .map_err(|_| PptxError::InvalidXml(format!("invalid slide rId: {r_id_str}")))?;

        Ok(SlideRef {
            r_id,
            partname: slide_partname,
        })
    }

    /// Get the XML blob of a slide by its reference.
    ///
    /// # Errors
    ///
    /// Returns an error if the slide part is not found.
    pub fn slide_xml(&self, slide_ref: &SlideRef) -> PptxResult<&[u8]> {
        let part = self
            .package
            .part(&slide_ref.partname)
            .or_part_not_found(slide_ref.partname.as_str())?;
        Ok(&part.blob)
    }

    /// Get a mutable reference to the XML blob of a slide.
    ///
    /// # Errors
    ///
    /// Returns an error if the slide part is not found.
    pub fn slide_xml_mut(&mut self, slide_ref: &SlideRef) -> PptxResult<&mut Vec<u8>> {
        let part = self
            .package
            .part_mut(&slide_ref.partname)
            .or_part_not_found(slide_ref.partname.as_str())?;
        Ok(&mut part.blob)
    }

    /// Get the slide size in EMU (English Metric Units).
    /// Returns (width, height) or None if not defined.
    /// # Errors
    ///
    /// Returns an error if the presentation XML cannot be parsed.
    pub fn slide_size(&self) -> PptxResult<Option<(i64, i64)>> {
        let pres_part = self.presentation_part()?;
        parse_slide_size(&pres_part.blob)
    }

    /// Set the slide width in EMU (English Metric Units).
    ///
    /// # Errors
    ///
    /// Returns an error if the presentation XML cannot be updated.
    pub fn set_slide_width(&mut self, width: i64) -> PptxResult<()> {
        let pres_partname = self.presentation_partname()?;
        let pres_part = self
            .package
            .part_mut(&pres_partname)
            .or_part_not_found(pres_partname.as_str())?;
        // Default to standard OOXML slide size (10" x 7.5") when <p:sldSz> is absent.
        let current_size = parse_slide_size(&pres_part.blob)?.unwrap_or((9_144_000, 6_858_000));
        pres_part.blob = set_slide_size_in_xml(&pres_part.blob, width, current_size.1)?;
        Ok(())
    }

    /// Set the slide height in EMU (English Metric Units).
    ///
    /// # Errors
    ///
    /// Returns an error if the presentation XML cannot be updated.
    pub fn set_slide_height(&mut self, height: i64) -> PptxResult<()> {
        let pres_partname = self.presentation_partname()?;
        let pres_part = self
            .package
            .part_mut(&pres_partname)
            .or_part_not_found(pres_partname.as_str())?;
        // Default to standard OOXML slide size (10" x 7.5") when <p:sldSz> is absent.
        let current_size = parse_slide_size(&pres_part.blob)?.unwrap_or((9_144_000, 6_858_000));
        pres_part.blob = set_slide_size_in_xml(&pres_part.blob, current_size.0, height)?;
        Ok(())
    }

    /// Delete a slide from the presentation.
    ///
    /// Removes the slide part, its relationship from the presentation part,
    /// and its entry in `<p:sldIdLst>`.
    /// # Errors
    ///
    /// Returns an error if the slide cannot be removed.
    pub fn delete_slide(&mut self, slide_ref: &SlideRef) -> PptxResult<()> {
        // Remove the slide part from the package
        self.package.remove_part(&slide_ref.partname);

        // Remove the relationship and sldId entry from the presentation part
        let pres_partname = self.presentation_partname()?;
        let pres_part = self
            .package
            .part_mut(&pres_partname)
            .or_part_not_found(pres_partname.as_str())?;

        // Remove the relationship
        pres_part.rels.remove(slide_ref.r_id.as_str());

        // Remove the sldId entry from presentation XML
        pres_part.blob =
            remove_slide_id_from_presentation_xml(&pres_part.blob, slide_ref.r_id.as_str())?;

        Ok(())
    }

    /// Get the name of a slide from its `<p:cSld name="...">` attribute.
    ///
    /// Returns `Ok(Some(name))` if the slide has a non-empty name attribute,
    /// `Ok(None)` otherwise.
    /// # Errors
    ///
    /// Returns an error if the slide part is not found.
    pub fn slide_name(&self, slide_ref: &SlideRef) -> PptxResult<Option<String>> {
        let part = self
            .package
            .part(&slide_ref.partname)
            .or_part_not_found(slide_ref.partname.as_str())?;
        parse_slide_name(&part.blob)
    }
}
