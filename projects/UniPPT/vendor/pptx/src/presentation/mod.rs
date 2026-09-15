mod export;
mod fonts;
mod layouts;
mod media;
mod notes;
mod print;
mod scene;
mod signature;
mod slide_props;
mod slides;
mod vba;

pub use scene::SceneAssetCache;

#[cfg(test)]
mod tests;

use std::io::{Read, Write};
use std::path::Path;

use crate::core_properties::CoreProperties;
use crate::error::{PptxError, PptxResult};
use crate::opc::constants::{content_type as CT, relationship_type as RT};
use crate::opc::pack_uri::PackURI;
use crate::opc::package::OpcPackage;
use crate::opc::part::Part;

/// A `PowerPoint` presentation.
///
/// This is the main entry point for creating and manipulating .pptx files.
/// It wraps an `OpcPackage` and provides a high-level API for working with
/// slides, slide layouts, and slide masters.
///
/// # Examples
///
/// ```no_run
/// use pptx::presentation::Presentation;
///
/// // Create a new presentation from the default template
/// let mut prs = Presentation::new().unwrap();
///
/// // Get available slide layouts
/// let layouts = prs.slide_layouts().unwrap();
///
/// // Add a slide using the first layout
/// if let Some(layout) = layouts.first() {
///     prs.add_slide(layout).unwrap();
/// }
///
/// // Save to a file
/// prs.save("output.pptx").unwrap();
/// ```
#[must_use]
pub struct Presentation {
    package: OpcPackage,
}

/// Core lifecycle methods: creation, opening, saving.
impl Presentation {
    /// Create a new presentation from the embedded default template.
    ///
    /// # Errors
    ///
    /// Returns an error if the default template cannot be parsed.
    pub fn new() -> PptxResult<Self> {
        Ok(Self {
            package: OpcPackage::new()?,
        })
    }

    /// Open an existing .pptx file from a filesystem path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn open(path: impl AsRef<Path>) -> PptxResult<Self> {
        Ok(Self {
            package: OpcPackage::open(path)?,
        })
    }

    /// Open a presentation from in-memory bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes are not a valid .pptx file.
    pub fn from_bytes(data: &[u8]) -> PptxResult<Self> {
        Ok(Self {
            package: OpcPackage::from_bytes(data)?,
        })
    }

    /// Open a presentation from anything implementing `Read`.
    ///
    /// Input is limited to 500 MB to prevent excessive memory usage.
    /// # Errors
    ///
    /// Returns an error if reading or parsing fails.
    pub fn from_reader(mut reader: impl Read) -> PptxResult<Self> {
        const MAX_INPUT_SIZE: u64 = 500 * 1024 * 1024; // 500 MB
        let mut buf = Vec::new();
        reader.by_ref().take(MAX_INPUT_SIZE).read_to_end(&mut buf)?;
        let mut extra = [0u8; 1];
        if reader.read(&mut extra)? > 0 {
            return Err(PptxError::ResourceLimit {
                message: format!("input exceeds maximum size of {} bytes", MAX_INPUT_SIZE),
            });
        }
        Self::from_bytes(&buf)
    }

    /// Save the presentation to a filesystem path.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or file I/O fails.
    pub fn save(&self, path: impl AsRef<Path>) -> PptxResult<()> {
        self.package.save(path)
    }

    /// Serialize the presentation to in-memory bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    pub fn to_bytes(&self) -> PptxResult<Vec<u8>> {
        self.package.to_bytes()
    }

    /// Write the presentation to anything implementing `Write`.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or writing fails.
    pub fn write_to(&self, mut writer: impl Write) -> PptxResult<()> {
        let bytes = self.to_bytes()?;
        writer.write_all(&bytes)?;
        Ok(())
    }
}

/// Core-properties and package-accessor methods.
impl Presentation {
    /// Get the core properties of the presentation.
    ///
    /// Reads from `docProps/core.xml` if it exists, otherwise returns empty properties.
    /// # Errors
    ///
    /// Returns an error if the XML cannot be parsed.
    pub fn core_properties(&self) -> PptxResult<CoreProperties> {
        // Try to find the core properties part via package-level relationship
        let core_rel = self.package.pkg_rels.by_reltype(RT::CORE_PROPERTIES);
        core_rel.map_or_else(
            |_| Ok(CoreProperties::new()),
            |rel| {
                let partname = rel.target_partname(self.package.pkg_rels.base_uri())?;
                self.package.part(&partname).map_or_else(
                    || Ok(CoreProperties::new()),
                    |part| CoreProperties::from_xml(&part.blob),
                )
            },
        )
    }

    /// Set the core properties of the presentation.
    ///
    /// Writes to `docProps/core.xml`, creating the part and relationship if needed.
    ///
    /// # Errors
    ///
    /// Returns an error if XML serialization fails.
    pub fn set_core_properties(&mut self, props: &CoreProperties) -> PptxResult<()> {
        let partname = PackURI::new("/docProps/core.xml")?;
        let xml = props.to_xml()?;

        let part = Part::new(partname, CT::OPC_CORE_PROPERTIES, xml);
        self.package.put_part(part);

        // Ensure the package-level relationship exists
        self.package
            .pkg_rels
            .or_add(RT::CORE_PROPERTIES, "docProps/core.xml", false);

        Ok(())
    }

    /// Get a reference to the underlying OPC package.
    #[must_use]
    pub const fn package(&self) -> &OpcPackage {
        &self.package
    }

    /// Get a mutable reference to the underlying OPC package.
    pub fn package_mut(&mut self) -> &mut OpcPackage {
        &mut self.package
    }

    /// Get the presentation part (the main XML part).
    pub(crate) fn presentation_part(&self) -> PptxResult<&Part> {
        self.package.part_by_reltype(RT::OFFICE_DOCUMENT)
    }

    /// Get the partname of the presentation part.
    pub(crate) fn presentation_partname(&self) -> PptxResult<PackURI> {
        let rel = self.package.pkg_rels.by_reltype(RT::OFFICE_DOCUMENT)?;
        rel.target_partname(self.package.pkg_rels.base_uri())
    }
}

pub(super) fn remove_xml_element(xml: &str, element_name: &str) -> String {
    let open_tag = format!("<{element_name}");
    let close_tag = format!("</{element_name}>");

    // Try to remove a paired element first: <element_name ...>...</element_name>
    if let Some(start) = xml.find(&open_tag) {
        if let Some(end) = xml[start..].find(&close_tag) {
            let end_pos = start + end + close_tag.len();
            let mut result = String::with_capacity(xml.len() - (end_pos - start));
            result.push_str(&xml[..start]);
            result.push_str(&xml[end_pos..]);
            return result;
        }
        // Try self-closing: <element_name ... />
        if let Some(end) = xml[start..].find("/>") {
            let end_pos = start + end + 2;
            let mut result = String::with_capacity(xml.len() - (end_pos - start));
            result.push_str(&xml[..start]);
            result.push_str(&xml[end_pos..]);
            return result;
        }
    }
    xml.to_string()
}
