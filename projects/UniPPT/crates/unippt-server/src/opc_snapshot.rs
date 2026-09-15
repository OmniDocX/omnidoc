//! Self-contained, OPC-native snapshot storage for PowerPoint packages.
//!
//! The snapshot deliberately stores no complete `.pptx` byte blob. Every ZIP
//! member points at an independently addressable, uncompressed SHA-256 blob,
//! while the index keeps the original member order and the metadata needed to
//! produce an equivalent OPC ZIP package.
//!
//! Fidelity boundary: [`rebuild`] preserves member names, order, uncompressed
//! bytes, compression method, DOS modification time, Unix permissions and the
//! archive comment. It does not yet preserve the original compressed streams,
//! local/central header extra fields, flags, data-descriptor layout, file
//! comments, offsets or archive prefix/trailing bytes. Consequently an
//! unedited rebuilt PPTX is OPC-member equivalent, but its ZIP envelope is not
//! guaranteed to be byte-identical. A future raw-entry fidelity layer can be
//! added without changing the content-addressed blob model.
//!
//! Integration is intentionally callback-based so a UDoc decoder may hydrate
//! only the requested content-addressed part:
//! ```ignore
//! let exploded = opc_snapshot::explode(&pptx)?;
//! let rebuilt = opc_snapshot::rebuild(&exploded.index, |path| {
//!     exploded.blobs.get(path).map(AsRef::as_ref)
//! })?;
//! ```

use std::collections::{BTreeMap, HashSet};
use std::io::{Cursor, Read, Write};
use std::sync::Arc;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipArchive, ZipWriter};

const SNAPSHOT_FORMAT: &str = "unippt-opc-snapshot";
const SNAPSHOT_VERSION: u32 = 1;
const BLOB_PREFIX: &str = "blobs/sha256/";

/// Keep these limits aligned with the native PPTX parser. They apply to both
/// exploding an untrusted archive and rebuilding an untrusted UDoc index.
pub(crate) const MAX_ENTRY_COUNT: usize = 10_000;
pub(crate) const MAX_SINGLE_ENTRY_SIZE: u64 = 100 * 1024 * 1024;
pub(crate) const MAX_TOTAL_SIZE: u64 = 500 * 1024 * 1024;
const MAX_ARCHIVE_COMMENT_SIZE: usize = u16::MAX as usize;

/// Independently owned content-addressed OPC member bytes.
pub(crate) type Blob = Arc<[u8]>;
pub(crate) type BlobMap = BTreeMap<String, Blob>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OpcPackageIndex {
    pub(crate) format: String,
    pub(crate) version: u32,
    pub(crate) archive_comment_base64: String,
    pub(crate) entries: Vec<OpcPackageEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OpcPackageEntry {
    /// ZIP member name, without a leading slash. Directory names end in `/`.
    pub(crate) name: String,
    /// Safe UDoc-internal content address. It is never derived from `name`.
    pub(crate) blob_path: String,
    pub(crate) compression: OpcCompression,
    pub(crate) size: u64,
    pub(crate) sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) last_modified: Option<OpcDateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) unix_mode: Option<u32>,
    #[serde(default)]
    pub(crate) directory: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum OpcCompression {
    Store,
    Deflate,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OpcDateTime {
    pub(crate) year: u16,
    pub(crate) month: u8,
    pub(crate) day: u8,
    pub(crate) hour: u8,
    pub(crate) minute: u8,
    pub(crate) second: u8,
}

#[derive(Clone, Debug)]
pub(crate) struct OpcSnapshot {
    pub(crate) index: OpcPackageIndex,
    pub(crate) blobs: BlobMap,
}

impl OpcSnapshot {
    /// Stable identity of the logical OPC package. Unlike the outer ZIP byte
    /// stream this remains stable after the package is reconstructed.
    pub(crate) fn digest(&self) -> [u8; 32] {
        let mut digest = Sha256::new();
        digest.update(b"unippt-opc-snapshot-digest-v1\0");
        digest.update(self.index.archive_comment_base64.as_bytes());
        for entry in &self.index.entries {
            digest.update([0]);
            digest.update(entry.name.as_bytes());
            digest.update([entry.directory as u8]);
            digest.update(entry.size.to_le_bytes());
            digest.update(entry.sha256.as_bytes());
            digest.update([match entry.compression {
                OpcCompression::Store => 0,
                OpcCompression::Deflate => 1,
            }]);
        }
        digest.finalize().into()
    }

    pub(crate) fn estimated_bytes(&self) -> usize {
        let index_bytes = self.index.entries.iter().fold(0usize, |total, entry| {
            total
                .saturating_add(entry.name.len())
                .saturating_add(entry.blob_path.len())
                .saturating_add(entry.sha256.len())
                .saturating_add(std::mem::size_of::<OpcPackageEntry>())
        });
        self.blobs.values().fold(index_bytes, |total, bytes| {
            total.saturating_add(bytes.len())
        })
    }

    pub(crate) fn rebuild(&self) -> Result<Vec<u8>, String> {
        rebuild(&self.index, |path| self.blobs.get(path).map(AsRef::as_ref))
    }
}

#[derive(Debug, Error)]
pub(crate) enum OpcSnapshotError {
    #[error("invalid OPC snapshot: {0}")]
    Invalid(String),
    #[error("OPC ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("OPC ZIP I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid OPC archive comment encoding: {0}")]
    Base64(#[from] base64::DecodeError),
}

/// Explode a PPTX ZIP into an ordered index and independently owned blobs.
///
/// The input is borrowed only while this function runs; no returned value
/// contains the original complete PPTX or a slice that keeps it alive.
pub(crate) fn explode(pptx: &[u8]) -> Result<OpcSnapshot, String> {
    explode_checked(pptx).map_err(|error| error.to_string())
}

fn explode_checked(pptx: &[u8]) -> Result<OpcSnapshot, OpcSnapshotError> {
    let mut archive = ZipArchive::new(Cursor::new(pptx))?;
    if archive.len() > MAX_ENTRY_COUNT {
        return Err(OpcSnapshotError::Invalid(format!(
            "ZIP contains {} entries; limit is {MAX_ENTRY_COUNT}",
            archive.len()
        )));
    }
    if archive.comment().len() > MAX_ARCHIVE_COMMENT_SIZE {
        return Err(OpcSnapshotError::Invalid(format!(
            "archive comment is {} bytes; limit is {MAX_ARCHIVE_COMMENT_SIZE}",
            archive.comment().len()
        )));
    }

    let archive_comment_base64 = STANDARD.encode(archive.comment());
    let mut entries = Vec::with_capacity(archive.len());
    let mut blobs = BlobMap::new();
    let mut names = HashSet::with_capacity(archive.len());
    let mut total_size = 0u64;

    for entry_index in 0..archive.len() {
        let mut member = archive.by_index(entry_index)?;
        if member.encrypted() {
            return Err(OpcSnapshotError::Invalid(format!(
                "encrypted ZIP member is not supported: {}",
                member.name()
            )));
        }

        let name = member.name().to_owned();
        let directory = member.is_dir();
        validate_member_name(&name, directory)?;
        if !names.insert(name.clone()) {
            return Err(OpcSnapshotError::Invalid(format!(
                "duplicate OPC member name: {name}"
            )));
        }

        let size = member.size();
        validate_declared_size(&name, size, &mut total_size)?;
        let compression = OpcCompression::try_from(member.compression()).map_err(|method| {
            OpcSnapshotError::Invalid(format!(
                "unsupported ZIP compression method {method:?} for {name}"
            ))
        })?;
        let last_modified = member.last_modified().map(OpcDateTime::from);
        let unix_mode = member.unix_mode();

        let capacity = usize::try_from(size).map_err(|_| {
            OpcSnapshotError::Invalid(format!("member size does not fit memory: {name}"))
        })?;
        let mut bytes = Vec::with_capacity(capacity);
        member
            .by_ref()
            .take(size.saturating_add(1))
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 != size {
            return Err(OpcSnapshotError::Invalid(format!(
                "member size mismatch for {name}: declared {size}, decoded {}",
                bytes.len()
            )));
        }
        if directory && !bytes.is_empty() {
            return Err(OpcSnapshotError::Invalid(format!(
                "directory member contains data: {name}"
            )));
        }

        let sha256 = sha256_hex(&bytes);
        let blob_path = blob_path_for_digest(&sha256);
        let bytes: Arc<[u8]> = Arc::from(bytes);
        if let Some(existing) = blobs.get(&blob_path) {
            if existing.as_ref() != bytes.as_ref() {
                return Err(OpcSnapshotError::Invalid(format!(
                    "SHA-256 collision for member {name}"
                )));
            }
        } else {
            blobs.insert(blob_path.clone(), bytes);
        }

        entries.push(OpcPackageEntry {
            name,
            blob_path,
            compression,
            size,
            sha256,
            last_modified,
            unix_mode,
            directory,
        });
    }

    Ok(OpcSnapshot {
        index: OpcPackageIndex {
            format: SNAPSHOT_FORMAT.into(),
            version: SNAPSHOT_VERSION,
            archive_comment_base64,
            entries,
        },
        blobs,
    })
}

/// Hydrate a self-contained snapshot directly from a UDoc content-addressed
/// blob table. This validates every name, size and hash without ever building
/// an intermediate `.pptx` ZIP.
pub(crate) fn hydrate(
    index: OpcPackageIndex,
    mut get_blob: impl FnMut(&str) -> Option<Blob>,
) -> Result<OpcSnapshot, String> {
    validate_index_header(&index).map_err(|error| error.to_string())?;
    let archive_comment = STANDARD
        .decode(&index.archive_comment_base64)
        .map_err(|error| OpcSnapshotError::Base64(error).to_string())?;
    if archive_comment.len() > MAX_ARCHIVE_COMMENT_SIZE {
        return Err(format!(
            "invalid OPC snapshot: archive comment is {} bytes; limit is {MAX_ARCHIVE_COMMENT_SIZE}",
            archive_comment.len()
        ));
    }

    let mut names = HashSet::with_capacity(index.entries.len());
    let mut blobs = BlobMap::new();
    let mut total_size = 0u64;
    for entry in &index.entries {
        validate_member_name(&entry.name, entry.directory).map_err(|error| error.to_string())?;
        if !names.insert(entry.name.clone()) {
            return Err(format!(
                "invalid OPC snapshot: duplicate OPC member name: {}",
                entry.name
            ));
        }
        validate_declared_size(&entry.name, entry.size, &mut total_size)
            .map_err(|error| error.to_string())?;
        validate_sha256(&entry.sha256).map_err(|error| error.to_string())?;
        let expected_path = blob_path_for_digest(&entry.sha256);
        if entry.blob_path != expected_path {
            return Err(format!(
                "invalid OPC snapshot: non-content-addressed blob path for {}: expected {expected_path}, got {}",
                entry.name, entry.blob_path
            ));
        }
        let bytes = get_blob(&entry.blob_path).ok_or_else(|| {
            format!(
                "invalid OPC snapshot: missing blob {} for member {}",
                entry.blob_path, entry.name
            )
        })?;
        if bytes.len() as u64 != entry.size || sha256_hex(&bytes) != entry.sha256 {
            return Err(format!(
                "invalid OPC snapshot: blob integrity mismatch for {}",
                entry.name
            ));
        }
        if entry.directory && !bytes.is_empty() {
            return Err(format!(
                "invalid OPC snapshot: directory member contains data: {}",
                entry.name
            ));
        }
        if let Some(existing) = blobs.get(&entry.blob_path) {
            if existing.as_ref() != bytes.as_ref() {
                return Err(format!(
                    "invalid OPC snapshot: conflicting content-addressed blob {}",
                    entry.blob_path
                ));
            }
        } else {
            blobs.insert(entry.blob_path.clone(), bytes);
        }
    }
    Ok(OpcSnapshot { index, blobs })
}

/// Rebuild a PPTX ZIP exclusively from a snapshot index and its blob map.
pub(crate) fn rebuild<'a>(
    index: &OpcPackageIndex,
    get_blob: impl FnMut(&str) -> Option<&'a [u8]>,
) -> Result<Vec<u8>, String> {
    rebuild_checked(index, get_blob).map_err(|error| error.to_string())
}

fn rebuild_checked<'a>(
    index: &OpcPackageIndex,
    mut get_blob: impl FnMut(&str) -> Option<&'a [u8]>,
) -> Result<Vec<u8>, OpcSnapshotError> {
    validate_index_header(index)?;
    let archive_comment = STANDARD.decode(&index.archive_comment_base64)?;
    if archive_comment.len() > MAX_ARCHIVE_COMMENT_SIZE {
        return Err(OpcSnapshotError::Invalid(format!(
            "archive comment is {} bytes; limit is {MAX_ARCHIVE_COMMENT_SIZE}",
            archive_comment.len()
        )));
    }

    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);
    writer.set_raw_comment(archive_comment.into_boxed_slice());

    let mut names = HashSet::with_capacity(index.entries.len());
    let mut total_size = 0u64;
    for entry in &index.entries {
        validate_member_name(&entry.name, entry.directory)?;
        if !names.insert(entry.name.clone()) {
            return Err(OpcSnapshotError::Invalid(format!(
                "duplicate OPC member name: {}",
                entry.name
            )));
        }
        validate_declared_size(&entry.name, entry.size, &mut total_size)?;
        validate_sha256(&entry.sha256)?;
        let expected_path = blob_path_for_digest(&entry.sha256);
        if entry.blob_path != expected_path {
            return Err(OpcSnapshotError::Invalid(format!(
                "non-content-addressed blob path for {}: expected {expected_path}, got {}",
                entry.name, entry.blob_path
            )));
        }

        let bytes = get_blob(&entry.blob_path).ok_or_else(|| {
            OpcSnapshotError::Invalid(format!(
                "missing blob {} for member {}",
                entry.blob_path, entry.name
            ))
        })?;
        if bytes.len() as u64 != entry.size {
            return Err(OpcSnapshotError::Invalid(format!(
                "blob size mismatch for {}: declared {}, actual {}",
                entry.name,
                entry.size,
                bytes.len()
            )));
        }
        if sha256_hex(bytes) != entry.sha256 {
            return Err(OpcSnapshotError::Invalid(format!(
                "blob SHA-256 mismatch for {}",
                entry.name
            )));
        }
        if entry.directory && !bytes.is_empty() {
            return Err(OpcSnapshotError::Invalid(format!(
                "directory member contains data: {}",
                entry.name
            )));
        }

        let mut options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::from(entry.compression));
        if let Some(last_modified) = entry.last_modified {
            options = options.last_modified_time(last_modified.try_into()?);
        }
        if let Some(unix_mode) = entry.unix_mode {
            options = options.unix_permissions(unix_mode);
        }

        if entry.directory {
            writer.add_directory(&entry.name, options)?;
        } else {
            writer.start_file(&entry.name, options)?;
            writer.write_all(bytes)?;
        }
    }

    Ok(writer.finish()?.into_inner())
}

fn validate_index_header(index: &OpcPackageIndex) -> Result<(), OpcSnapshotError> {
    if index.format != SNAPSHOT_FORMAT || index.version != SNAPSHOT_VERSION {
        return Err(OpcSnapshotError::Invalid(format!(
            "unsupported snapshot identity {}/{}",
            index.format, index.version
        )));
    }
    if index.entries.len() > MAX_ENTRY_COUNT {
        return Err(OpcSnapshotError::Invalid(format!(
            "snapshot contains {} entries; limit is {MAX_ENTRY_COUNT}",
            index.entries.len()
        )));
    }
    Ok(())
}

fn validate_declared_size(
    name: &str,
    size: u64,
    total_size: &mut u64,
) -> Result<(), OpcSnapshotError> {
    if size > MAX_SINGLE_ENTRY_SIZE {
        return Err(OpcSnapshotError::Invalid(format!(
            "member {name} is {size} bytes; per-entry limit is {MAX_SINGLE_ENTRY_SIZE}"
        )));
    }
    *total_size = total_size
        .checked_add(size)
        .ok_or_else(|| OpcSnapshotError::Invalid("total uncompressed size overflow".into()))?;
    if *total_size > MAX_TOTAL_SIZE {
        return Err(OpcSnapshotError::Invalid(format!(
            "total uncompressed size exceeds {MAX_TOTAL_SIZE} bytes"
        )));
    }
    Ok(())
}

fn validate_member_name(name: &str, directory: bool) -> Result<(), OpcSnapshotError> {
    if name.is_empty()
        || name.starts_with('/')
        || name.starts_with('\\')
        || name.contains('\\')
        || name.contains('\0')
        || directory != name.ends_with('/')
    {
        return Err(OpcSnapshotError::Invalid(format!(
            "unsafe or inconsistent OPC member name: {name:?}"
        )));
    }

    let path = name.strip_suffix('/').unwrap_or(name);
    if path.is_empty()
        || path
            .split('/')
            .any(|segment| segment.is_empty() || matches!(segment, "." | ".."))
        || path
            .split('/')
            .next()
            .is_some_and(|segment| segment.contains(':'))
    {
        return Err(OpcSnapshotError::Invalid(format!(
            "unsafe OPC member path: {name:?}"
        )));
    }
    Ok(())
}

fn validate_sha256(value: &str) -> Result<(), OpcSnapshotError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(OpcSnapshotError::Invalid(format!(
            "invalid lowercase SHA-256 digest: {value:?}"
        )));
    }
    Ok(())
}

pub(crate) fn blob_path_for_digest(sha256: &str) -> String {
    format!("{BLOB_PREFIX}{sha256}")
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut value = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(value, "{byte:02x}");
    }
    value
}

impl TryFrom<CompressionMethod> for OpcCompression {
    type Error = CompressionMethod;

    fn try_from(value: CompressionMethod) -> Result<Self, Self::Error> {
        match value {
            CompressionMethod::Stored => Ok(Self::Store),
            CompressionMethod::Deflated => Ok(Self::Deflate),
            other => Err(other),
        }
    }
}

impl From<OpcCompression> for CompressionMethod {
    fn from(value: OpcCompression) -> Self {
        match value {
            OpcCompression::Store => Self::Stored,
            OpcCompression::Deflate => Self::Deflated,
        }
    }
}

impl From<DateTime> for OpcDateTime {
    fn from(value: DateTime) -> Self {
        Self {
            year: value.year(),
            month: value.month(),
            day: value.day(),
            hour: value.hour(),
            minute: value.minute(),
            second: value.second(),
        }
    }
}

impl TryFrom<OpcDateTime> for DateTime {
    type Error = OpcSnapshotError;

    fn try_from(value: OpcDateTime) -> Result<Self, Self::Error> {
        DateTime::from_date_and_time(
            value.year,
            value.month,
            value.day,
            value.hour,
            value.minute,
            value.second,
        )
        .map_err(|_| {
            OpcSnapshotError::Invalid(format!(
                "invalid ZIP timestamp {:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                value.year, value.month, value.day, value.hour, value.minute, value.second
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTENT_TYPES: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/></Types>"#;
    const ROOT_RELS: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="urn:test:officeDocument" Target="ppt/presentation.xml"/></Relationships>"#;
    const SLIDE_RELS: &[u8] = br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId9" Type="urn:test:unknown" Target="../custom/opaque.bin"/></Relationships>"#;

    #[derive(Debug, Eq, PartialEq)]
    struct MemberView {
        name: String,
        content: Vec<u8>,
        compression: CompressionMethod,
        last_modified: Option<OpcDateTime>,
        unix_mode: Option<u32>,
    }

    fn make_zip(entries: &[(&str, &[u8], CompressionMethod)]) -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        writer.set_raw_comment(b"opc archive comment\xff".to_vec().into_boxed_slice());
        let modified = DateTime::from_date_and_time(2025, 6, 7, 8, 9, 10).unwrap();
        for (name, bytes, compression) in entries {
            let options = SimpleFileOptions::default()
                .compression_method(*compression)
                .last_modified_time(modified)
                .unix_permissions(0o640);
            writer.start_file(*name, options).unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    fn members(bytes: &[u8]) -> Vec<MemberView> {
        let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut result = Vec::with_capacity(archive.len());
        for index in 0..archive.len() {
            let mut member = archive.by_index(index).unwrap();
            let name = member.name().to_owned();
            let compression = member.compression();
            let last_modified = member.last_modified().map(OpcDateTime::from);
            let unix_mode = member.unix_mode();
            let mut content = Vec::new();
            member.read_to_end(&mut content).unwrap();
            result.push(MemberView {
                name,
                content,
                compression,
                last_modified,
                unix_mode,
            });
        }
        result
    }

    fn rebuild_snapshot(snapshot: &OpcSnapshot) -> Result<Vec<u8>, String> {
        rebuild(&snapshot.index, |path| {
            snapshot.blobs.get(path).map(AsRef::as_ref)
        })
    }

    #[test]
    fn unknown_parts_relationships_and_content_types_round_trip_verbatim_in_order() {
        let source = make_zip(&[
            (
                "[Content_Types].xml",
                CONTENT_TYPES,
                CompressionMethod::Deflated,
            ),
            ("_rels/.rels", ROOT_RELS, CompressionMethod::Deflated),
            (
                "ppt/presentation.xml",
                br#"<p:presentation xmlns:p="urn:test:p"/>"#,
                CompressionMethod::Deflated,
            ),
            (
                "ppt/slides/_rels/slide1.xml.rels",
                SLIDE_RELS,
                CompressionMethod::Deflated,
            ),
            (
                "ppt/custom/opaque.bin",
                b"\x00unknown\xffpart\x10",
                CompressionMethod::Stored,
            ),
        ]);
        let snapshot = explode(&source).unwrap();
        let rebuilt = rebuild_snapshot(&snapshot).unwrap();

        assert_eq!(members(&rebuilt), members(&source));
        let source_archive = ZipArchive::new(Cursor::new(&source)).unwrap();
        let rebuilt_archive = ZipArchive::new(Cursor::new(&rebuilt)).unwrap();
        assert_eq!(rebuilt_archive.comment(), source_archive.comment());
    }

    #[test]
    fn repeated_content_uses_one_blob() {
        let source = make_zip(&[
            (
                "ppt/media/image1.bin",
                b"same bytes",
                CompressionMethod::Stored,
            ),
            (
                "ppt/media/image2.bin",
                b"same bytes",
                CompressionMethod::Deflated,
            ),
        ]);
        let snapshot = explode(&source).unwrap();

        assert_eq!(snapshot.index.entries.len(), 2);
        assert_eq!(snapshot.blobs.len(), 1);
        assert_eq!(
            snapshot.index.entries[0].blob_path,
            snapshot.index.entries[1].blob_path
        );
        let rebuilt = rebuild_snapshot(&snapshot).unwrap();
        assert_eq!(members(&rebuilt), members(&source));
    }

    #[test]
    fn traversal_member_is_rejected() {
        let source = make_zip(&[(
            "ppt/slides/../../evil.xml",
            b"nope",
            CompressionMethod::Deflated,
        )]);
        let error = explode(&source).unwrap_err().to_string();
        assert!(error.contains("unsafe OPC member path"), "{error}");
    }

    #[test]
    fn rebuild_validates_blob_hash_size_and_bomb_limits() {
        let source = make_zip(&[(
            "ppt/presentation.xml",
            b"<presentation/>",
            CompressionMethod::Deflated,
        )]);
        let mut snapshot = explode(&source).unwrap();
        let path = snapshot.index.entries[0].blob_path.clone();
        snapshot.blobs.insert(path, Arc::from(&b"tampered"[..]));
        let error = rebuild_snapshot(&snapshot).unwrap_err();
        assert!(
            error.contains("size mismatch") || error.contains("SHA-256 mismatch"),
            "{error}"
        );

        let mut bomb = explode(&source).unwrap();
        bomb.index.entries[0].size = MAX_SINGLE_ENTRY_SIZE + 1;
        let error = rebuild_snapshot(&bomb).unwrap_err();
        assert!(error.contains("per-entry limit"), "{error}");
    }

    #[test]
    fn rebuilt_pptx_is_readable_by_unippt_core() {
        let source = unippt_core::export_pptx(&unippt_core::Deck::demo(), None).unwrap();
        let snapshot = explode(&source).unwrap();
        let rebuilt = rebuild_snapshot(&snapshot).unwrap();
        let source_members = members(&source);
        let rebuilt_members = members(&rebuilt);

        assert_eq!(rebuilt_members, source_members);
        let imported = unippt_core::import_pptx(&rebuilt).unwrap();
        assert_eq!(imported.slides.len(), 1);
    }
}
