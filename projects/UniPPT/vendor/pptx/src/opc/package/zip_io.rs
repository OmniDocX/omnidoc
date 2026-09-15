use std::collections::HashMap;
use std::io::{Cursor, Read, Write};

use zip::read::ZipArchive;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::error::{PptxError, PptxResult};
use crate::opc::content_type::ContentTypeMap;
use crate::opc::pack_uri::PackURI;
use crate::opc::part::Part;
use crate::opc::relationship::Relationships;

use super::OpcPackage;

/// Maximum total decompressed size across all ZIP entries (500 MB).
const MAX_TOTAL_SIZE: u64 = 500 * 1024 * 1024;

/// Maximum number of entries in a ZIP archive.
const MAX_ENTRY_COUNT: usize = 10_000;

/// Maximum decompressed size of a single ZIP entry (100 MB).
const MAX_SINGLE_ENTRY_SIZE: u64 = 100 * 1024 * 1024;

/// Read a package from in-memory bytes.
pub(super) fn read_from_bytes(data: &[u8]) -> PptxResult<OpcPackage> {
    let cursor = Cursor::new(data);
    let mut archive = ZipArchive::new(cursor)?;
    read_from_zip(&mut archive)
}

/// Serialize the package to in-memory bytes (a valid .pptx file).
pub(super) fn write_to_bytes(pkg: &OpcPackage) -> PptxResult<Vec<u8>> {
    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // Write [Content_Types].xml
    let ct_map = pkg.build_content_type_map();
    let ct_xml = ct_map.to_xml()?;
    zip.start_file("[Content_Types].xml", options)?;
    zip.write_all(&ct_xml)?;

    // Write package-level rels (_rels/.rels)
    if !pkg.pkg_rels.is_empty() {
        let rels_xml = pkg.pkg_rels.to_xml()?;
        zip.start_file("_rels/.rels", options)?;
        zip.write_all(&rels_xml)?;
    }

    // Write each part and its rels
    for part in pkg.parts.values() {
        let membername = part.partname.membername();
        zip.start_file(membername, options)?;
        zip.write_all(&part.blob)?;

        // Write part-level rels if any
        if !part.rels.is_empty() {
            let rels_membername = part.partname.rels_uri();
            let rels_xml = part.rels.to_xml()?;
            zip.start_file(rels_membername.membername(), options)?;
            zip.write_all(&rels_xml)?;
        }
    }

    let cursor = zip.finish()?;
    Ok(cursor.into_inner())
}

/// Read a package from a `ZipArchive`.
fn read_from_zip<R: Read + std::io::Seek>(archive: &mut ZipArchive<R>) -> PptxResult<OpcPackage> {
    // Check entry count limit.
    if archive.len() > MAX_ENTRY_COUNT {
        return Err(PptxError::ResourceLimit {
            message: format!(
                "ZIP archive contains {} entries, exceeding the limit of {}",
                archive.len(),
                MAX_ENTRY_COUNT
            ),
        });
    }

    // First pass: read all blobs from the ZIP.
    let mut blobs: HashMap<String, Vec<u8>> = HashMap::new();
    let mut total_size: u64 = 0;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;

        // Check single entry size limit.
        if file.size() > MAX_SINGLE_ENTRY_SIZE {
            return Err(PptxError::ResourceLimit {
                message: format!(
                    "ZIP entry '{}' has decompressed size {} bytes, exceeding the limit of {} bytes",
                    file.name(),
                    file.size(),
                    MAX_SINGLE_ENTRY_SIZE
                ),
            });
        }

        // Check cumulative total size limit.
        total_size = total_size.saturating_add(file.size());
        if total_size > MAX_TOTAL_SIZE {
            return Err(PptxError::ResourceLimit {
                message: format!(
                    "total decompressed size exceeds the limit of {MAX_TOTAL_SIZE} bytes"
                ),
            });
        }

        // Reject entries with path traversal sequences.
        if file.name().contains("..") {
            return Err(PptxError::Package(
                crate::error::PackageError::InvalidPackUri(format!(
                    "zip entry contains path traversal: {}",
                    file.name()
                )),
            ));
        }

        let name = file.name().to_string();
        if blobs.contains_key(&name) {
            return Err(PptxError::Package(
                crate::error::PackageError::DuplicatePart(name),
            ));
        }
        let mut buf = Vec::with_capacity(usize::try_from(file.size()).unwrap_or(0));
        file.read_to_end(&mut buf)?;
        blobs.insert(name, buf);
    }

    // Parse [Content_Types].xml
    let ct_xml = blobs.get("[Content_Types].xml").ok_or_else(|| {
        PptxError::InvalidXml("[Content_Types].xml not found in package".to_string())
    })?;
    let content_types = ContentTypeMap::from_xml(ct_xml)?;

    // Parse package-level rels (_rels/.rels)
    let pkg_rels = match blobs.get("_rels/.rels") {
        Some(xml) => Relationships::from_xml("/", xml)?,
        None => Relationships::new("/"),
    };

    // Separate rels blobs from content blobs so we can take ownership of
    // content blobs without cloning.
    let mut rels_blobs: HashMap<String, Vec<u8>> = HashMap::new();
    let mut content_blobs: HashMap<String, Vec<u8>> = HashMap::new();
    for (name, blob) in blobs {
        if name == "[Content_Types].xml" || name == "_rels/.rels" {
            continue;
        }
        if name.contains("/_rels/") {
            rels_blobs.insert(name, blob);
        } else {
            content_blobs.insert(name, blob);
        }
    }

    // Build parts, consuming content_blobs to avoid cloning.
    let mut parts = HashMap::new();
    for (name, blob) in content_blobs {
        let partname = PackURI::new(format!("/{name}"))?;
        let ct = content_types
            .get(&partname)
            .unwrap_or("application/octet-stream");

        let rels_membername = partname.rels_uri();
        let rels = match rels_blobs.get(rels_membername.membername()) {
            Some(rels_xml) => Relationships::from_xml(partname.base_uri(), rels_xml)?,
            None => Relationships::new(partname.base_uri()),
        };

        let part = Part::with_rels(partname, ct.to_string(), blob, rels);
        parts.insert(part.partname.to_string(), part);
    }

    Ok(OpcPackage { parts, pkg_rels })
}
