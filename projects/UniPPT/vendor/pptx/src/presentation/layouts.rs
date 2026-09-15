use crate::error::{PackageError, PartNotFoundExt, PptxError, PptxResult};
use crate::opc::constants::relationship_type as RT;
use crate::slide::{
    extract_layout_r_ids, layout_used_by_slides, parse_layout_name, parse_slide_master_ids,
    remove_layout_from_master_xml, SlideLayoutRef, SlideMasterRef,
};
use crate::units::RelationshipId;

use super::Presentation;

/// Layout, master, theme, and `SmartArt` methods for `Presentation`.
impl Presentation {
    /// Get references to all slide masters in the presentation.
    ///
    /// # Errors
    ///
    /// Returns an error if the presentation XML cannot be parsed.
    pub fn slide_masters(&self) -> PptxResult<Vec<SlideMasterRef>> {
        let pres_part = self.presentation_part()?;
        let master_ids = parse_slide_master_ids(&pres_part.blob)?;
        let base_uri = pres_part.partname.base_uri();

        let mut masters = Vec::with_capacity(master_ids.len());
        for (r_id_str, _id) in master_ids {
            let rel = pres_part.rels.get(&r_id_str).ok_or_else(|| {
                PptxError::Package(PackageError::RelationshipNotFound(format!(
                    "slide master relationship {r_id_str} not found"
                )))
            })?;
            let partname = rel.target_partname(base_uri)?;
            let r_id = RelationshipId::try_from(r_id_str.as_str()).map_err(|_| {
                PptxError::InvalidXml(format!("invalid slide master rId: {r_id_str}"))
            })?;
            masters.push(SlideMasterRef { r_id, partname });
        }

        Ok(masters)
    }

    /// Get references to all slide layouts available in the presentation.
    ///
    /// Collects layouts from all slide masters, with the first master's layouts first.
    /// # Errors
    ///
    /// Returns an error if layouts cannot be resolved.
    pub fn slide_layouts(&self) -> PptxResult<Vec<SlideLayoutRef>> {
        let masters = self.slide_masters()?;
        let mut layouts = Vec::new();

        for master_ref in &masters {
            let master_part = self
                .package
                .part(&master_ref.partname)
                .or_part_not_found(master_ref.partname.as_str())?;

            let layout_r_ids = extract_layout_r_ids(&master_part.rels);
            let base_uri = master_part.partname.base_uri();

            for r_id_str in layout_r_ids {
                let rel = master_part.rels.get(&r_id_str).ok_or_else(|| {
                    PptxError::Package(PackageError::RelationshipNotFound(r_id_str.clone()))
                })?;
                let partname = rel.target_partname(base_uri)?;

                // Read the layout name and resolve its slide master
                let (name, slide_master_part_name) = match self.package.part(&partname) {
                    None => (String::new(), Some(master_ref.partname.to_string())),
                    Some(layout_part) => {
                        let name = parse_layout_name(&layout_part.blob)?;
                        let master_rels = layout_part.rels.all_by_reltype(RT::SLIDE_MASTER);
                        let slide_master = master_rels
                            .first()
                            .and_then(|r| {
                                r.target_partname(layout_part.partname.base_uri())
                                    .ok()
                                    .map(super::super::opc::pack_uri::PackURI::into_string)
                            })
                            // Fallback: use the master we're iterating from
                            .or_else(|| Some(master_ref.partname.to_string()));
                        (name, slide_master)
                    }
                };

                let r_id = RelationshipId::try_from(r_id_str.as_str()).map_err(|_| {
                    PptxError::InvalidXml(format!("invalid layout rId: {r_id_str}"))
                })?;
                layouts.push(SlideLayoutRef {
                    r_id,
                    partname,
                    name,
                    slide_master_part_name,
                });
            }
        }

        Ok(layouts)
    }

    /// Get the slide master for a given slide layout.
    ///
    /// Uses the `slide_master_part_name` stored on the layout reference to find
    /// the matching `SlideMasterRef`.
    /// # Errors
    ///
    /// Returns an error if slide masters cannot be resolved.
    pub fn slide_master_for_layout(
        &self,
        layout: &SlideLayoutRef,
    ) -> PptxResult<Option<SlideMasterRef>> {
        let Some(master_part_name) = &layout.slide_master_part_name else {
            return Ok(None);
        };

        let masters = self.slide_masters()?;
        for master in masters {
            if master.partname.as_str() == master_part_name {
                return Ok(Some(master));
            }
        }

        Ok(None)
    }

    /// Remove a slide layout from the presentation.
    ///
    /// Removes the layout part, its relationship from the parent slide master,
    /// and its `<p:sldLayoutId>` entry in the master XML.
    ///
    /// Returns an error if any slides are still using this layout.
    /// # Errors
    ///
    /// Returns an error if the layout is in use or cannot be found.
    pub fn remove_slide_layout(&mut self, layout_ref: &SlideLayoutRef) -> PptxResult<()> {
        // Check if any slides use this layout
        let slides = self.slides()?;
        let users = layout_used_by_slides(&layout_ref.partname, &slides, &self.package);
        if !users.is_empty() {
            return Err(PptxError::InvalidXml(format!(
                "cannot remove layout '{}': still used by {} slide(s)",
                layout_ref.name,
                users.len()
            )));
        }

        // Find the slide master that owns this layout
        let masters = self.slide_masters()?;
        for master_ref in &masters {
            let master_part = self
                .package
                .part(&master_ref.partname)
                .or_part_not_found(master_ref.partname.as_str())?;

            // Check if this master has the layout as a relationship
            let layout_rels = master_part.rels.all_by_reltype(RT::SLIDE_LAYOUT);
            let mut found_r_id = None;
            for rel in &layout_rels {
                if let Ok(target) = rel.target_partname(master_part.partname.base_uri()) {
                    if target.as_str() == layout_ref.partname.as_str() {
                        found_r_id = Some(rel.r_id.clone());
                        break;
                    }
                }
            }

            if let Some(r_id) = found_r_id {
                // Remove the layout from the master XML
                let master_partname = master_ref.partname.clone();
                let master_part_mut = self
                    .package
                    .part_mut(&master_partname)
                    .or_part_not_found(master_partname.as_str())?;
                master_part_mut.blob =
                    remove_layout_from_master_xml(&master_part_mut.blob, r_id.as_str())?;
                // Remove the relationship
                master_part_mut.rels.remove(r_id.as_str());

                // Remove the layout part
                self.package.remove_part(&layout_ref.partname);

                return Ok(());
            }
        }

        Err(PptxError::Package(PackageError::PartNotFound(format!(
            "layout '{}' not found in any slide master",
            layout_ref.name
        ))))
    }

    /// Get the theme color scheme from the first slide master's theme.
    ///
    /// Reads the theme part linked from the first slide master and extracts
    /// the 12 theme color slots (dk1, dk2, lt1, lt2, accent1-6, hlink, folHlink).
    ///
    /// Returns `None` if no theme can be found or parsed.
    /// # Errors
    ///
    /// Returns an error if the theme XML cannot be parsed.
    pub fn theme_colors(&self) -> PptxResult<Option<crate::theme::ThemeColorScheme>> {
        // Get the first slide master
        let masters = self.slide_masters()?;
        let Some(master_ref) = masters.first() else {
            return Ok(None);
        };

        // Get the master part and find its theme relationship
        let master_part = self
            .package
            .part(&master_ref.partname)
            .or_part_not_found(master_ref.partname.as_str())?;

        let theme_rels = master_part.rels.all_by_reltype(RT::THEME);
        let Some(theme_rel) = theme_rels.first() else {
            return Ok(None);
        };

        let theme_partname = theme_rel.target_partname(master_part.partname.base_uri())?;
        let Some(theme_part) = self.package.part(&theme_partname) else {
            return Ok(None);
        };

        crate::theme::parse_theme_color_scheme(&theme_part.blob)
    }

    /// Read `SmartArt` diagram parts for a given slide and relationship ID.
    ///
    /// The `r_id` should come from `GraphicFrame::smartart_r_id`. The method
    /// resolves the diagram data part from the slide's relationships and
    /// collects the associated colour, style, layout, and drawing parts.
    ///
    /// # Errors
    ///
    /// Returns an error if the parts cannot be resolved.
    pub fn smartart_data(
        &self,
        r_id: &str,
        slide_ref: &crate::slide::SlideRef,
    ) -> PptxResult<Option<crate::smartart::SmartArt>> {
        use crate::smartart::smartart_rel_type as SRT;

        let slide_part = self
            .package
            .part(&slide_ref.partname)
            .or_part_not_found(slide_ref.partname.as_str())?;

        // Resolve the diagram data part from the slide's relationship
        let Some(rel) = slide_part.rels.get(r_id) else {
            return Ok(None);
        };
        let data_partname = rel.target_partname(slide_part.partname.base_uri())?;
        let Some(data_part) = self.package.part(&data_partname) else {
            return Ok(None);
        };
        // Clone is required: SmartArt owns its Vec<u8> fields and outlives the borrow on `self`.
        let data_xml = data_part.blob.clone();

        // Helper: try to read a related part by relationship type from the slide.
        // Clone is required for each part because SmartArt owns its data.
        let read_optional = |reltype: &str| -> Option<Vec<u8>> {
            let rels = slide_part.rels.all_by_reltype(reltype);
            for r in rels {
                if let Ok(pn) = r.target_partname(slide_part.partname.base_uri()) {
                    if let Some(p) = self.package.part(&pn) {
                        return Some(p.blob.clone());
                    }
                }
            }
            None
        };

        Ok(Some(crate::smartart::SmartArt {
            data_xml,
            colors_xml: read_optional(SRT::DIAGRAM_COLORS),
            style_xml: read_optional(SRT::DIAGRAM_STYLE),
            layout_xml: read_optional(SRT::DIAGRAM_LAYOUT),
            drawing_xml: read_optional(SRT::DIAGRAM_DRAWING),
        }))
    }
}
