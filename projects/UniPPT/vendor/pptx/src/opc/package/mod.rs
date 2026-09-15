mod media_parts;
mod zip_io;

use std::collections::HashMap;
use std::path::Path;

use crate::error::{PackageError, PartNotFoundExt, PptxError, PptxResult};
use crate::opc::content_type::ContentTypeMap;
use crate::opc::pack_uri::PackURI;
use crate::opc::part::Part;
use crate::opc::relationship::Relationships;

/// The default .pptx template embedded at compile time.
const DEFAULT_TEMPLATE: &[u8] = include_bytes!("../../templates/default.pptx");

/// Maximum allowed .pptx file size on disk (2 GB).
const MAX_PPTX_SIZE: u64 = 2 * 1024 * 1024 * 1024;

/// An OPC (Open Packaging Convention) package.
///
/// This is the primary struct for reading and writing .pptx files. An OPC package
/// is a ZIP archive containing XML and binary parts linked by relationships.
///
/// The package contains:
/// - A collection of parts indexed by their `PackURI`
/// - Package-level relationships (from `_rels/.rels`)
/// - A content type map (from `[Content_Types].xml`)
#[derive(Debug, Clone)]
pub struct OpcPackage {
    /// Parts indexed by their partname.
    pub(super) parts: HashMap<String, Part>,
    /// Package-level relationships (from `_rels/.rels`).
    pub(crate) pkg_rels: Relationships,
}

impl OpcPackage {
    /// Open an existing .pptx file from a filesystem path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or is not a valid package.
    pub fn open(path: impl AsRef<Path>) -> PptxResult<Self> {
        let metadata = std::fs::metadata(path.as_ref()).map_err(|e| {
            PptxError::Package(PackageError::PackageNotFound(format!(
                "{}: {}",
                path.as_ref().display(),
                e
            )))
        })?;
        if metadata.len() > MAX_PPTX_SIZE {
            return Err(PptxError::ResourceLimit {
                message: format!(
                    "PPTX file size {} bytes exceeds the limit of {} bytes",
                    metadata.len(),
                    MAX_PPTX_SIZE
                ),
            });
        }
        let data = std::fs::read(path.as_ref()).map_err(|e| {
            PptxError::Package(PackageError::PackageNotFound(format!(
                "{}: {}",
                path.as_ref().display(),
                e
            )))
        })?;
        Self::from_bytes(&data)
    }

    /// Open a package from in-memory bytes (a .pptx file loaded into memory).
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes are not a valid OPC package.
    pub fn from_bytes(data: &[u8]) -> PptxResult<Self> {
        zip_io::read_from_bytes(data)
    }

    /// Create a new package from the embedded default template.
    ///
    /// # Errors
    ///
    /// Returns an error if the default template cannot be parsed.
    pub fn new() -> PptxResult<Self> {
        Self::from_bytes(DEFAULT_TEMPLATE)
    }

    /// Save the package to a filesystem path.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or file I/O fails.
    pub fn save(&self, path: impl AsRef<Path>) -> PptxResult<()> {
        let bytes = self.to_bytes()?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    /// Serialize the package to in-memory bytes (a valid .pptx file).
    ///
    /// # Errors
    ///
    /// Returns an error if XML serialization or ZIP writing fails.
    pub fn to_bytes(&self) -> PptxResult<Vec<u8>> {
        zip_io::write_to_bytes(self)
    }

    /// Get a part by its partname.
    #[must_use]
    pub fn part(&self, partname: &PackURI) -> Option<&Part> {
        self.parts.get(partname.as_str())
    }

    /// Get a mutable reference to a part by its partname.
    pub fn part_mut(&mut self, partname: &PackURI) -> Option<&mut Part> {
        self.parts.get_mut(partname.as_str())
    }

    /// Add or replace a part in the package.
    pub fn put_part(&mut self, part: Part) {
        self.parts.insert(part.partname.to_string(), part);
    }

    /// Remove a part by its partname.
    pub fn remove_part(&mut self, partname: &PackURI) -> Option<Part> {
        self.parts.remove(partname.as_str())
    }

    /// Iterate over all parts in the package.
    pub fn parts(&self) -> impl Iterator<Item = &Part> {
        self.parts.values()
    }

    /// Iterate over all parts mutably.
    pub fn parts_mut(&mut self) -> impl Iterator<Item = &mut Part> {
        self.parts.values_mut()
    }

    /// Find the part related to the package by relationship type.
    ///
    /// # Errors
    ///
    /// Returns an error if the relationship or part is not found.
    pub fn part_by_reltype(&self, reltype: &str) -> PptxResult<&Part> {
        let rel = self.pkg_rels.by_reltype(reltype)?;
        let partname = rel.target_partname(self.pkg_rels.base_uri())?;
        self.part(&partname).or_part_not_found(partname.as_str())
    }

    /// Get a mutable reference to the part related to the package by relationship type.
    ///
    /// # Errors
    ///
    /// Returns an error if the relationship or part is not found.
    pub fn part_by_reltype_mut(&mut self, reltype: &str) -> PptxResult<&mut Part> {
        let rel = self.pkg_rels.by_reltype(reltype)?;
        let partname = rel.target_partname(self.pkg_rels.base_uri())?;
        let key = partname.to_string();
        self.parts
            .get_mut(&key)
            .or_part_not_found(partname.as_str())
    }

    /// Get the next available partname matching a pattern template.
    ///
    /// The template should contain `{}` where the number goes.
    /// For example, "/ppt/slides/slide{}.xml" might return "/ppt/slides/slide1.xml".
    /// # Errors
    ///
    /// Returns an error if no available partname can be found.
    pub fn next_partname(&self, template: &str) -> PptxResult<PackURI> {
        for n in 1..10000 {
            let candidate = template.replace("{}", &n.to_string());
            let uri = PackURI::new(&candidate)?;
            if self.part(&uri).is_none() {
                return Ok(uri);
            }
        }
        Err(PptxError::Package(PackageError::InvalidPackUri(
            "could not find available partname".to_string(),
        )))
    }

    /// Get the next available image partname with the given extension.
    ///
    /// Returns a partname like "/ppt/media/image1.png", "/ppt/media/image2.jpg", etc.
    /// # Errors
    ///
    /// Returns an error if no available partname can be found.
    pub fn next_image_partname(&self, ext: &str) -> PptxResult<PackURI> {
        let template = format!("/ppt/media/image{{}}.{ext}");
        self.next_partname(&template)
    }

    /// Get the next available media partname with the given extension.
    ///
    /// Returns a partname like "/ppt/media/media1.mp4", etc.
    /// # Errors
    ///
    /// Returns an error if no available partname can be found.
    pub fn next_media_partname(&self, ext: &str) -> PptxResult<PackURI> {
        let template = format!("/ppt/media/media{{}}.{ext}");
        self.next_partname(&template)
    }

    /// Build a `ContentTypeMap` from the current parts in the package.
    pub(super) fn build_content_type_map(&self) -> ContentTypeMap {
        ContentTypeMap::from_parts(
            self.parts
                .values()
                .map(|p| (&p.partname, p.content_type.as_str())),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_from_template() {
        let pkg = OpcPackage::new().unwrap();
        assert!(!pkg.pkg_rels.is_empty());
        let pres = pkg.part_by_reltype(
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument",
        );
        assert!(pres.is_ok());
    }

    #[test]
    fn test_round_trip() {
        let pkg = OpcPackage::new().unwrap();
        let part_count = pkg.parts.len();
        let bytes = pkg.to_bytes().unwrap();
        let pkg2 = OpcPackage::from_bytes(&bytes).unwrap();
        assert_eq!(pkg2.parts.len(), part_count);
        assert!(!pkg2.pkg_rels.is_empty());
    }

    #[test]
    fn test_part() {
        let pkg = OpcPackage::new().unwrap();
        let uri = PackURI::new("/ppt/presentation.xml").unwrap();
        assert!(pkg.part(&uri).is_some());
    }

    #[test]
    fn test_next_partname() {
        let pkg = OpcPackage::new().unwrap();
        let next = pkg.next_partname("/ppt/slides/slide{}.xml").unwrap();
        assert_eq!(next.as_str(), "/ppt/slides/slide1.xml");
    }

    #[test]
    fn test_save_and_reopen() {
        let pkg = OpcPackage::new().unwrap();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        pkg.save(&path).unwrap();
        let pkg2 = OpcPackage::open(&path).unwrap();
        assert!(!pkg2.pkg_rels.is_empty());
    }
}
