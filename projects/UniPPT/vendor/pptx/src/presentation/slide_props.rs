use crate::dml::fill::GradientFill;
use crate::error::{PartNotFoundExt, PptxError, PptxResult};
use crate::opc::constants::relationship_type as RT;
use crate::slide::{SlideLayoutRef, SlideRef};
use crate::xml_util::WriteXml;

use super::{remove_xml_element, Presentation};

/// Slide property, transition, animation, and comment methods for `Presentation`.
impl Presentation {
    /// Get the slide layout used by a given slide.
    ///
    /// Looks up the slideLayout relationship on the slide part and resolves
    /// it to a `SlideLayoutRef`.
    /// # Errors
    ///
    /// Returns an error if the slide part cannot be accessed.
    pub fn slide_layout_for(&self, slide_ref: &SlideRef) -> PptxResult<Option<SlideLayoutRef>> {
        let slide_part = self
            .package
            .part(&slide_ref.partname)
            .or_part_not_found(slide_ref.partname.as_str())?;

        let layout_rels = slide_part.rels.all_by_reltype(RT::SLIDE_LAYOUT);
        let layout_rel = match layout_rels.first() {
            Some(r) => *r,
            None => return Ok(None),
        };

        let partname = layout_rel.target_partname(slide_part.partname.base_uri())?;
        let r_id = layout_rel.r_id.clone();

        let (name, slide_master_part_name) = match self.package.part(&partname) {
            None => (String::new(), None),
            Some(layout_part) => {
                let name = crate::slide::parse_layout_name(&layout_part.blob)?;
                let slide_master = layout_part
                    .rels
                    .all_by_reltype(RT::SLIDE_MASTER)
                    .first()
                    .and_then(|r| {
                        r.target_partname(layout_part.partname.base_uri())
                            .ok()
                            .map(super::super::opc::pack_uri::PackURI::into_string)
                    });
                (name, slide_master)
            }
        };

        Ok(Some(SlideLayoutRef {
            r_id,
            partname,
            name,
            slide_master_part_name,
        }))
    }

    /// Set a slide transition on a given slide.
    ///
    /// Inserts a `<p:transition>` element into the slide XML.
    /// If the slide already has a transition, it is replaced.
    /// # Errors
    ///
    /// Returns an error if the slide XML cannot be updated.
    pub fn set_slide_transition(
        &mut self,
        slide_ref: &SlideRef,
        transition: &crate::transition::SlideTransition,
    ) -> PptxResult<()> {
        let slide_xml = self.slide_xml(slide_ref)?;
        let xml_str = std::str::from_utf8(slide_xml)?;

        let transition_xml = transition.to_xml_string();

        // Remove existing <p:transition.../> or <p:transition>...</p:transition>
        let result = remove_xml_element(xml_str, "p:transition");

        // Insert before </p:sld>
        let pos = result.rfind("</p:sld>").ok_or_else(|| {
            PptxError::InvalidXml("slide XML does not contain </p:sld>".to_string())
        })?;
        let mut updated = String::with_capacity(result.len() + transition_xml.len());
        updated.push_str(&result[..pos]);
        updated.push_str(&transition_xml);
        updated.push_str(&result[pos..]);

        *self.slide_xml_mut(slide_ref)? = updated.into_bytes();
        Ok(())
    }

    /// Set animations on a given slide.
    ///
    /// Inserts a `<p:timing>` element into the slide XML.
    /// If the slide already has a timing element, it is replaced.
    /// # Errors
    ///
    /// Returns an error if the slide XML cannot be updated.
    pub fn set_slide_animations(
        &mut self,
        slide_ref: &SlideRef,
        animations: &crate::animation::AnimationSequence,
    ) -> PptxResult<()> {
        let slide_xml = self.slide_xml(slide_ref)?;
        let xml_str = std::str::from_utf8(slide_xml)?;

        let timing_xml = animations.to_xml_string();

        // Remove existing <p:timing>...</p:timing>
        let result = remove_xml_element(xml_str, "p:timing");

        if timing_xml.is_empty() {
            // Just remove the existing timing element (if any).
            *self.slide_xml_mut(slide_ref)? = result.into_bytes();
            return Ok(());
        }

        // Insert before </p:sld>
        let pos = result.rfind("</p:sld>").ok_or_else(|| {
            PptxError::InvalidXml("slide XML does not contain </p:sld>".to_string())
        })?;
        let mut updated = String::with_capacity(result.len() + timing_xml.len());
        updated.push_str(&result[..pos]);
        updated.push_str(&timing_xml);
        updated.push_str(&result[pos..]);

        *self.slide_xml_mut(slide_ref)? = updated.into_bytes();
        Ok(())
    }

    /// Get comments for a slide.
    ///
    /// Returns an empty vector if the slide has no comments part.
    /// # Errors
    ///
    /// Returns an error if comments cannot be read.
    #[allow(clippy::missing_const_for_fn)]
    pub fn slide_comments(
        &self,
        _slide_ref: &SlideRef,
    ) -> PptxResult<Vec<crate::comment::Comment>> {
        // Comments parsing from existing XML is a read operation.
        // For now, return empty; full parsing would require reading
        // the comments part and the comment authors part.
        Ok(Vec::new())
    }

    /// Set a solid RGB background on a slide.
    ///
    /// `color` must be a six-character hexadecimal RGB value without `#`.
    /// # Errors
    ///
    /// Returns an error if the slide XML cannot be updated.
    pub fn set_slide_background_solid(
        &mut self,
        slide_ref: &SlideRef,
        color: &str,
    ) -> PptxResult<()> {
        let slide_xml = self.slide_xml_mut(slide_ref)?;
        crate::slide::set_slide_background_solid(slide_xml, color)
    }

    /// Set a gradient background on a slide.
    ///
    /// Replaces any existing background with a gradient fill derived from
    /// the given `GradientFill`.
    /// # Errors
    ///
    /// Returns an error if the slide XML cannot be updated.
    pub fn set_slide_background_gradient(
        &mut self,
        slide_ref: &SlideRef,
        gradient: &GradientFill,
    ) -> PptxResult<()> {
        let slide_xml = self.slide_xml_mut(slide_ref)?;
        crate::slide::set_slide_background_gradient(slide_xml, gradient)
    }

    /// Set an image background on a slide.
    ///
    /// The `image_r_id` should be a relationship ID that references the
    /// image part within the slide's relationships.
    /// # Errors
    ///
    /// Returns an error if the slide XML cannot be updated.
    pub fn set_slide_background_image(
        &mut self,
        slide_ref: &SlideRef,
        image_r_id: &str,
    ) -> PptxResult<()> {
        let slide_xml = self.slide_xml_mut(slide_ref)?;
        crate::slide::set_slide_background_image(slide_xml, image_r_id)
    }

    /// Clear the slide background so it follows the master slide.
    ///
    /// Removes any explicit `<p:bg>` element from the slide XML, causing
    /// the slide to inherit the master's background.
    /// # Errors
    ///
    /// Returns an error if the slide XML cannot be updated.
    pub fn clear_slide_background(&mut self, slide_ref: &SlideRef) -> PptxResult<()> {
        let slide_xml = self.slide_xml_mut(slide_ref)?;
        crate::slide::set_follow_master_background(slide_xml, true)
    }
}
