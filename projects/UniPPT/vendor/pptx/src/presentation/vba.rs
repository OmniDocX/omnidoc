//! Macro-enabled (.pptm) and VBA project operations on a [`Presentation`].

use std::path::Path;

use crate::error::{PartNotFoundExt, PptxResult};
use crate::opc::constants::{content_type as CT, relationship_type as RT};
use crate::opc::pack_uri::PackURI;
use crate::opc::part::Part;

use super::Presentation;

impl Presentation {
    /// Check if this presentation is macro-enabled.
    ///
    /// Returns `true` if the main presentation part has the macro-enabled
    /// content type.
    #[must_use]
    pub fn is_macro_enabled(&self) -> bool {
        self.presentation_part()
            .map(|p| p.content_type == CT::PML_PRESENTATION_MACRO)
            .unwrap_or(false)
    }

    /// Save the presentation as a macro-enabled .pptm file.
    ///
    /// This changes the content type of the main presentation part to
    /// the macro-enabled variant and saves to the given path.
    /// # Errors
    ///
    /// Returns an error if the file cannot be saved.
    pub fn save_as_pptm(&mut self, path: impl AsRef<Path>) -> PptxResult<()> {
        // Change the content type of the presentation part
        let pres_partname = self.presentation_partname()?;
        let pres_part = self
            .package
            .part_mut(&pres_partname)
            .or_part_not_found(pres_partname.as_str())?;
        pres_part.content_type = CT::PML_PRESENTATION_MACRO.to_string();

        self.package.save(path)
    }

    /// Read the VBA project binary from the package, if present.
    ///
    /// Returns a borrowed slice into the package's VBA part. If the caller
    /// needs owned data, use `.to_vec()` on the returned slice.
    ///
    /// Returns `None` if no VBA project is embedded.
    /// # Errors
    ///
    /// Returns an error if the presentation part cannot be accessed.
    pub fn vba_project(&self) -> PptxResult<Option<&[u8]>> {
        let pres_part = self.presentation_part()?;
        let vba_rels = pres_part.rels.all_by_reltype(RT::VBA_PROJECT);
        let rel = match vba_rels.first() {
            Some(r) => *r,
            None => return Ok(None),
        };
        let partname = rel.target_partname(pres_part.partname.base_uri())?;
        Ok(self
            .package
            .part(&partname)
            .map(|part| part.blob.as_slice()))
    }

    /// Set or replace the VBA project binary.
    ///
    /// The VBA project is stored at `/ppt/vbaProject.bin`. If no VBA
    /// project relationship exists yet, one is created.
    /// # Errors
    ///
    /// Returns an error if the VBA part cannot be created.
    pub fn set_vba_project(&mut self, data: Vec<u8>) -> PptxResult<()> {
        let vba_partname = PackURI::new("/ppt/vbaProject.bin")?;

        // Pre-compute relative ref before consuming the partname
        let pres_partname = self.presentation_partname()?;
        let rel_target = vba_partname.relative_ref(pres_partname.base_uri());

        let vba_part = Part::new(vba_partname, CT::VBA_PROJECT, data);
        self.package.put_part(vba_part);
        let pres_part = self
            .package
            .part_mut(&pres_partname)
            .or_part_not_found(pres_partname.as_str())?;
        pres_part.rels.or_add(RT::VBA_PROJECT, &rel_target, false);

        Ok(())
    }
}
