use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read, Write};
use thiserror::Error;
use unippt_core::{Deck, SceneObject, ASSET_REFERENCE_PREFIX};

use crate::asset_transport::{
    externalize_deck_assets, materialize_deck_assets, portableize_deck_assets,
    rebind_deck_asset_cache_id, AssetCatalog,
};
use crate::media_derivative;
use crate::opc_snapshot::{self, OpcPackageIndex, OpcSnapshot};

const HEADER: &[u8; 8] = b"UDOC3PKG";
const FOOTER_MAGIC: &[u8; 8] = b"UD3DIR01";
const FOOTER_SIZE: usize = 64;
const MAX_ENTRY_SIZE: usize = 512 * 1024 * 1024;
const MAX_ZIP_PART_SIZE: usize = MAX_ENTRY_SIZE + 1024 * 1024;
const MAX_TOTAL_SIZE: usize = 2 * 1024 * 1024 * 1024;
const MAX_ASSET_COUNT: usize = 100_000;
const OUTER_ZIP_ENTRY: &str = "package.udoc3";
pub(crate) const CURRENT_UNIDOC_TYPE: &str = "pptx";
const LEGACY_UNIDOC_TYPE: &str = "ppt";

#[derive(Debug, Error)]
pub enum UdocError {
    #[error("UDOC3 序列化失败: {0}")]
    Json(#[from] serde_json::Error),
    #[error("UDOC3 Brotli 编解码失败: {0}")]
    Brotli(String),
    #[error("UDOC3 资产处理失败: {0}")]
    Asset(String),
    #[error("不是有效的 UniPPT UDOC3：{0}")]
    Invalid(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Directory {
    format: String,
    version: u32,
    basename: String,
    root: String,
    entries: Vec<DirectoryEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DirectoryEntry {
    path: String,
    offset: u64,
    compressed_size: usize,
    size: usize,
    codec: String,
    mime: String,
    sha256: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    format: String,
    version: u32,
    unidoc_type: String,
    app: String,
    basename: String,
    root: String,
    /// Legacy UDoc3 packages embedded one complete PPTX here. New writers do
    /// not emit this field; it remains optional so those files can be migrated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    presentation: Option<String>,
    /// Native UDoc3 representation of the ordered OPC package. Every OPC
    /// member points at a content-addressed UDoc blob; no nested PPTX exists.
    #[serde(
        default,
        rename = "opcPackage",
        skip_serializing_if = "Option::is_none"
    )]
    opc_package: Option<String>,
    relationships: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    assets: Option<String>,
    features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RelationshipPart {
    version: u32,
    relationships: Vec<PackageRelationship>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PackageRelationship {
    source: String,
    #[serde(rename = "type")]
    kind: String,
    target: String,
    mode: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssetIndex {
    format: String,
    version: u32,
    cache_id: String,
    entries: Vec<AssetIndexEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssetIndexEntry {
    id: String,
    path: String,
    mime: String,
    data_uri_prefix: String,
    size: usize,
    sha256: String,
    /// Read-only compatibility for the short-lived embedded-PPTX writer.
    /// New packages always point directly at a UDoc-native blob.
    #[serde(default, rename = "sourcePartName", skip_serializing)]
    legacy_source_part_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DocumentPart {
    format: String,
    version: u32,
    unidoc_type: String,
    app: String,
    deck: Deck,
    /// Immutable import baseline used for loss-aware native XML patching.
    /// Older UDoc3 files omit it and reopen with `deck` as their baseline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    baseline: Option<Deck>,
}

#[derive(Debug, Serialize)]
pub struct UdocStructure {
    // Keep these first and adjacent: the structure viewer intentionally opens
    // with the portable-document identity instead of burying it in metadata.
    app: &'static str,
    unidoc_type: &'static str,
    format: &'static str,
    version: u32,
    manifest: Manifest,
    document: DocumentPart,
    relationships: RelationshipPart,
}

#[allow(dead_code)]
pub struct DecodedUdoc {
    pub deck: Deck,
    pub presentation: Vec<u8>,
}

pub(crate) struct DecodedCachedUdoc {
    pub(crate) deck: Deck,
    pub(crate) original_deck: Deck,
    pub(crate) assets: AssetCatalog,
    pub(crate) opc: OpcSnapshot,
    pub(crate) presentation_digest: [u8; 32],
}

pub fn structure(deck: &Deck) -> UdocStructure {
    let basename = safe_basename(&deck.title);
    let mut persisted_deck = deck.clone();
    persisted_deck.source_import_id = None;
    let manifest = Manifest {
        format: "udoc-package".into(),
        version: 3,
        unidoc_type: CURRENT_UNIDOC_TYPE.into(),
        app: "UniPPT".into(),
        basename,
        root: "document/document.json".into(),
        presentation: None,
        opc_package: Some("pptx/package.json".into()),
        relationships: "rels/relationships.json".into(),
        assets: Some("assets/index.json".into()),
        features: vec![
            "unippt".into(),
            "pptx".into(),
            "animations".into(),
            "omml".into(),
            "lossless-html".into(),
            "tail-directory".into(),
            "independent-compression".into(),
            "br".into(),
            "zip-static-resources".into(),
            "hybrid-br-zip".into(),
            "sha256-integrity".into(),
            "content-addressed-blobs".into(),
            "opc-parts-v1".into(),
            "scene-baseline-v2".into(),
            "regenerable-media-v1".into(),
        ],
    };
    UdocStructure {
        app: "UniPPT",
        unidoc_type: CURRENT_UNIDOC_TYPE,
        format: "udoc",
        version: 3,
        manifest,
        document: DocumentPart {
            format: "udoc".into(),
            version: 3,
            unidoc_type: CURRENT_UNIDOC_TYPE.into(),
            app: "UniPPT".into(),
            baseline: None,
            deck: persisted_deck,
        },
        relationships: canonical_relationships(),
    }
}

fn canonical_relationships() -> RelationshipPart {
    RelationshipPart {
        version: 1,
        relationships: vec![
            PackageRelationship {
                source: "document/document.json".into(),
                kind: "native-opc-package".into(),
                target: "pptx/package.json".into(),
                mode: "internal".into(),
            },
            PackageRelationship {
                source: "document/document.json".into(),
                kind: "asset-index".into(),
                target: "assets/index.json".into(),
                mode: "internal".into(),
            },
        ],
    }
}

pub fn encode(deck: &Deck, presentation: &[u8], basename: &str) -> Result<Vec<u8>, UdocError> {
    encode_with_type(deck, presentation, basename, CURRENT_UNIDOC_TYPE)
}

fn encode_with_type(
    deck: &Deck,
    presentation: &[u8],
    basename: &str,
    unidoc_type: &str,
) -> Result<Vec<u8>, UdocError> {
    let cache_id = deck
        .source_import_id
        .clone()
        .unwrap_or_else(|| "udoc3-assets".into());
    let mut compact = deck.clone();
    let mut assets = AssetCatalog::default();
    externalize_deck_assets(&mut compact, &cache_id, &mut assets).map_err(UdocError::Asset)?;
    let opc = opc_snapshot::explode(presentation).map_err(UdocError::Invalid)?;
    encode_compact_with_type(
        &compact,
        &compact,
        &assets,
        &cache_id,
        &opc,
        basename,
        unidoc_type,
    )
}

/// Encode an already compact server-cached scene without rebuilding hundreds
/// of megabytes of repeated data URIs. Any newly introduced inline asset is
/// folded into a private catalog clone before the package is written.
pub fn encode_cached(
    deck: &Deck,
    baseline: &Deck,
    catalog: &AssetCatalog,
    opc: &OpcSnapshot,
    basename: &str,
) -> Result<Vec<u8>, UdocError> {
    let cache_id = deck
        .source_import_id
        .as_deref()
        .ok_or_else(|| UdocError::Asset("cached Deck has no source_import_id".into()))?;
    let mut compact = deck.clone();
    let mut compact_baseline = baseline.clone();
    let mut assets = catalog.clone();
    externalize_deck_assets(&mut compact, cache_id, &mut assets).map_err(UdocError::Asset)?;
    externalize_deck_assets(&mut compact_baseline, cache_id, &mut assets)
        .map_err(UdocError::Asset)?;
    encode_compact_with_type(
        &compact,
        &compact_baseline,
        &assets,
        cache_id,
        opc,
        basename,
        CURRENT_UNIDOC_TYPE,
    )
}

fn encode_compact_with_type(
    compact: &Deck,
    baseline: &Deck,
    assets: &AssetCatalog,
    cache_id: &str,
    opc: &OpcSnapshot,
    basename: &str,
    unidoc_type: &str,
) -> Result<Vec<u8>, UdocError> {
    validate_cache_id(cache_id)?;
    let basename = safe_basename(basename);
    let mut persisted_deck = compact.clone();
    let mut persisted_baseline = baseline.clone();
    persisted_deck.source_import_id = None;
    persisted_baseline.source_import_id = None;
    media_derivative::strip_regenerable_playback_assets(
        &mut [&mut persisted_deck, &mut persisted_baseline],
        assets,
        opc,
    );
    portableize_deck_assets(&mut persisted_deck, cache_id, assets).map_err(UdocError::Asset)?;
    portableize_deck_assets(&mut persisted_baseline, cache_id, assets).map_err(UdocError::Asset)?;
    let mut referenced_assets = portable_asset_ids(&persisted_deck);
    referenced_assets.extend(portable_asset_ids(&persisted_baseline));
    let baseline = (persisted_baseline != persisted_deck).then_some(persisted_baseline);
    let document = DocumentPart {
        format: "udoc".into(),
        version: 3,
        unidoc_type: unidoc_type.into(),
        app: "UniPPT".into(),
        deck: persisted_deck,
        baseline,
    };
    let manifest = Manifest {
        format: "udoc-package".into(),
        version: 3,
        unidoc_type: unidoc_type.into(),
        app: "UniPPT".into(),
        basename: basename.clone(),
        root: "document/document.json".into(),
        presentation: None,
        opc_package: Some("pptx/package.json".into()),
        relationships: "rels/relationships.json".into(),
        assets: Some("assets/index.json".into()),
        features: vec![
            "unippt".into(),
            "pptx".into(),
            "animations".into(),
            "omml".into(),
            "lossless-html".into(),
            "tail-directory".into(),
            "independent-compression".into(),
            "br".into(),
            "zip-static-resources".into(),
            "hybrid-br-zip".into(),
            "sha256-integrity".into(),
            "content-addressed-blobs".into(),
            "opc-parts-v1".into(),
            "scene-baseline-v2".into(),
            "regenerable-media-v1".into(),
        ],
    };
    let relationships = canonical_relationships();
    let mut sorted_assets: Vec<_> = assets
        .iter()
        .filter(|(id, _)| referenced_assets.contains(*id))
        .collect();
    sorted_assets.sort_unstable_by(|left, right| left.0.cmp(right.0));
    let content_paths: HashMap<(usize, String), String> = opc
        .index
        .entries
        .iter()
        .map(|entry| {
            (
                (entry.size as usize, entry.sha256.clone()),
                entry.blob_path.clone(),
            )
        })
        .collect();
    let asset_index = AssetIndex {
        format: "unippt-asset-index".into(),
        version: 1,
        cache_id: cache_id.into(),
        entries: sorted_assets
            .iter()
            .map(|(id, asset)| AssetIndexEntry {
                id: (*id).into(),
                path: content_paths
                    .get(&(asset.bytes.len(), sha256_hex(&asset.bytes)))
                    .cloned()
                    .unwrap_or_else(|| format!("blobs/sha256/{}", sha256_hex(&asset.bytes))),
                mime: asset.mime_type.to_string(),
                data_uri_prefix: asset.data_uri_prefix.to_string(),
                size: asset.bytes.len(),
                sha256: sha256_hex(&asset.bytes),
                legacy_source_part_name: None,
            })
            .collect(),
    };
    let mut parts = vec![
        Part::json("manifest.json", &manifest)?,
        Part::json("document/document.json", &document)?,
        Part::json("rels/relationships.json", &relationships)?,
        Part::json("assets/index.json", &asset_index)?,
        Part::json("pptx/package.json", &opc.index)?,
    ];
    let blob_names: HashMap<_, _> = opc
        .index
        .entries
        .iter()
        .map(|entry| (entry.blob_path.as_str(), entry.name.as_str()))
        .collect();
    for (path, bytes) in &opc.blobs {
        let member_name = blob_names.get(path.as_str()).copied().unwrap_or_default();
        parts.push(Part::borrowed(
            path,
            bytes.as_ref(),
            opc_part_mime(member_name),
            PartCompression::for_resource(member_name, opc_part_mime(member_name)),
        ));
    }
    let mut written_blob_paths: HashSet<String> = opc.blobs.keys().cloned().collect();
    for (_id, asset) in sorted_assets {
        if content_paths.contains_key(&(asset.bytes.len(), sha256_hex(&asset.bytes))) {
            continue;
        }
        let path = format!("blobs/sha256/{}", sha256_hex(&asset.bytes));
        if !written_blob_paths.insert(path.clone()) {
            continue;
        }
        parts.push(Part::borrowed(
            &path,
            &asset.bytes,
            &asset.mime_type,
            PartCompression::for_resource(&path, &asset.mime_type),
        ));
    }
    encode_parts(parts, &basename)
}

fn opc_part_mime(name: &str) -> &'static str {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".xml") || lower.ends_with(".rels") {
        "application/xml"
    } else if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".avif") {
        "image/avif"
    } else if lower.ends_with(".bmp") {
        "image/bmp"
    } else if lower.ends_with(".svg") {
        "image/svg+xml"
    } else if lower.ends_with(".mp4") {
        "video/mp4"
    } else if lower.ends_with(".webm") {
        "video/webm"
    } else if lower.ends_with(".mp3") {
        "audio/mpeg"
    } else if lower.ends_with(".wav") {
        "audio/wav"
    } else if lower.ends_with(".ogg") {
        "audio/ogg"
    } else if lower.ends_with(".aac") {
        "audio/aac"
    } else if lower.ends_with(".flac") {
        "audio/flac"
    } else if lower.ends_with(".pdf") {
        "application/pdf"
    } else if lower.ends_with(".woff2") {
        "font/woff2"
    } else if lower.ends_with(".woff") {
        "font/woff"
    } else if lower.ends_with(".ttf") {
        "font/ttf"
    } else if lower.ends_with(".otf") {
        "font/otf"
    } else {
        "application/octet-stream"
    }
}

fn portable_asset_ids(deck: &Deck) -> HashSet<String> {
    let mut ids = HashSet::new();
    for font in &deck.fonts {
        collect_portable_ids(&font.data_uri, &mut ids);
    }
    for slide in &deck.slides {
        if let Some(asset) = slide.background_asset.as_deref() {
            collect_portable_ids(asset, &mut ids);
        }
        collect_object_asset_ids(&slide.master_objects, &mut ids);
        collect_object_asset_ids(&slide.layout_objects, &mut ids);
        collect_object_asset_ids(&slide.objects, &mut ids);
    }
    ids
}

fn collect_object_asset_ids(objects: &[SceneObject], ids: &mut HashSet<String>) {
    for object in objects {
        if let Some(asset) = object.asset.as_deref() {
            collect_portable_ids(asset, ids);
        }
        if let Some(asset) = object.shape_fill_asset.as_deref() {
            collect_portable_ids(asset, ids);
        }
        collect_portable_ids(&object.style.fill, ids);
        if let Some(media) = &object.media {
            if let Some(asset) = media.asset.as_deref() {
                collect_portable_ids(asset, ids);
            }
            if let Some(asset) = media.playback_asset.as_deref() {
                collect_portable_ids(asset, ids);
            }
        }
        collect_object_asset_ids(&object.children, ids);
    }
}

fn collect_portable_ids(value: &str, ids: &mut HashSet<String>) {
    let mut remaining = value;
    while let Some(index) = remaining.find(ASSET_REFERENCE_PREFIX) {
        let candidate = &remaining[index + ASSET_REFERENCE_PREFIX.len()..];
        if candidate.len() >= 64 {
            let id = &candidate[..64];
            if id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                ids.insert(id.to_ascii_lowercase());
            }
        }
        remaining = &candidate[candidate.len().min(64)..];
    }
}

pub(crate) fn decode_cached(bytes: &[u8]) -> Result<DecodedCachedUdoc, UdocError> {
    let package = unwrap_outer_zip(bytes)?;
    let bytes = package.as_ref();
    if bytes.len() < HEADER.len() + FOOTER_SIZE || &bytes[..HEADER.len()] != HEADER {
        return Err(UdocError::Invalid("缺少 UDOC3PKG 文件头".into()));
    }
    let footer = &bytes[bytes.len() - FOOTER_SIZE..];
    if &footer[..8] != FOOTER_MAGIC {
        return Err(UdocError::Invalid("UD3DIR01 尾目录签名损坏".into()));
    }
    let directory_offset = read_u64(footer, 8)? as usize;
    let directory_length = read_u64(footer, 16)? as usize;
    let directory_raw_length = read_u32(footer, 60)? as usize;
    if read_u32(footer, 56)? != 3
        || directory_offset < HEADER.len()
        || directory_raw_length == 0
        || directory_raw_length > MAX_ENTRY_SIZE
        || directory_offset
            .checked_add(directory_length)
            .is_none_or(|end| end != bytes.len() - FOOTER_SIZE)
    {
        return Err(UdocError::Invalid("尾目录范围或版本无效".into()));
    }
    let directory_bytes = br_decompress(
        &bytes[directory_offset..directory_offset + directory_length],
        directory_raw_length,
    )?;
    if Sha256::digest(&directory_bytes).as_slice() != &footer[24..56] {
        return Err(UdocError::Invalid("尾目录 SHA-256 校验失败".into()));
    }
    let directory: Directory = serde_json::from_slice(&directory_bytes)?;
    if directory.format != "udoc-directory" || directory.version != 3 {
        return Err(UdocError::Invalid("目录结构无效".into()));
    }

    let mut total = 0usize;
    let mut parts = HashMap::new();
    let mut ranges = Vec::new();
    for entry in &directory.entries {
        validate_path(&entry.path)?;
        if !matches!(entry.codec.as_str(), "store" | "br" | "zip")
            || entry.size > MAX_ENTRY_SIZE
            || entry.compressed_size > MAX_ZIP_PART_SIZE
            || entry.offset < HEADER.len() as u64
        {
            return Err(UdocError::Invalid(format!("目录项无效：{}", entry.path)));
        }
        total = total
            .checked_add(entry.size)
            .filter(|value| *value <= MAX_TOTAL_SIZE)
            .ok_or_else(|| UdocError::Invalid("解压后总尺寸超过安全上限".into()))?;
        let start =
            usize::try_from(entry.offset).map_err(|_| UdocError::Invalid("部件偏移溢出".into()))?;
        let end = start
            .checked_add(entry.compressed_size)
            .filter(|end| *end <= directory_offset)
            .ok_or_else(|| UdocError::Invalid(format!("部件范围越界：{}", entry.path)))?;
        ranges.push((start, end, &entry.path));
        let packed = &bytes[start..end];
        let raw: std::sync::Arc<[u8]> = match entry.codec.as_str() {
            "br" => std::sync::Arc::from(br_decompress(packed, entry.size)?),
            "zip" => std::sync::Arc::from(zip_unpack_part(packed, &entry.path, entry.size)?),
            "store" => std::sync::Arc::from(packed.to_vec()),
            _ => unreachable!("codec validated above"),
        };
        if raw.len() != entry.size || sha256_hex(&raw) != entry.sha256 {
            return Err(UdocError::Invalid(format!("部件校验失败：{}", entry.path)));
        }
        if parts.insert(entry.path.clone(), raw).is_some() {
            return Err(UdocError::Invalid(format!("重复目录项：{}", entry.path)));
        }
    }
    ranges.sort_by_key(|range| range.0);
    if ranges.windows(2).any(|pair| pair[1].0 < pair[0].1) {
        return Err(UdocError::Invalid("目录项范围重叠".into()));
    }

    let manifest: Manifest = serde_json::from_slice(required(&parts, &directory.root)?)?;
    if manifest.format != "udoc-package"
        || manifest.version != 3
        || !is_supported_unidoc_type(&manifest.unidoc_type)
        || manifest.app != "UniPPT"
    {
        return Err(UdocError::Invalid(
            "manifest 必须声明 unidoc_type=\"pptx\"（兼容旧值 \"ppt\"）".into(),
        ));
    }
    validate_path(&manifest.root)?;
    validate_manifest_relationships(&manifest, &parts)?;
    let document: DocumentPart = serde_json::from_slice(required(&parts, &manifest.root)?)?;
    if document.format != "udoc"
        || document.version != 3
        || !is_supported_unidoc_type(&document.unidoc_type)
        || document.unidoc_type != manifest.unidoc_type
        || document.app != "UniPPT"
        || document.deck.format != "unippt"
    {
        return Err(UdocError::Invalid(
            "document.json 不是 UniPPT 的 pptx 类型 UDOC3，或与 manifest 类型不一致".into(),
        ));
    }
    let mut deck = document.deck;
    let mut original_deck = document.baseline.unwrap_or_else(|| deck.clone());
    let (opc, legacy_presentation) = match (
        manifest.opc_package.as_deref(),
        manifest.presentation.as_deref(),
    ) {
        (Some(opc_path), None) => {
            validate_path(opc_path)?;
            let index: OpcPackageIndex = serde_json::from_slice(required(&parts, opc_path)?)?;
            let snapshot = opc_snapshot::hydrate(index, |blob_path| parts.get(blob_path).cloned())
                .map_err(UdocError::Invalid)?;
            (snapshot, None)
        }
        (None, Some(presentation_path)) => {
            validate_path(presentation_path)?;
            let presentation = parts
                .get(presentation_path)
                .map(|bytes| bytes.as_ref().to_vec())
                .ok_or_else(|| UdocError::Invalid(format!("缺少部件：{presentation_path}")))?;
            if !presentation.starts_with(b"PK") {
                return Err(UdocError::Invalid("原生 PPTX 部件不是 OPC ZIP".into()));
            }
            let snapshot = opc_snapshot::explode(&presentation).map_err(UdocError::Invalid)?;
            (snapshot, Some(presentation))
        }
        (Some(_), Some(_)) => {
            return Err(UdocError::Invalid(
                "manifest 不能同时包含 opcPackage 与旧 presentation".into(),
            ));
        }
        (None, None) => {
            return Err(UdocError::Invalid("manifest 缺少原生 opcPackage".into()));
        }
    };
    let mut catalog = AssetCatalog::default();
    let mut package_cache_id = None;
    if let Some(asset_index_path) = manifest.assets.as_deref() {
        validate_path(asset_index_path)?;
        let asset_index: AssetIndex = serde_json::from_slice(required(&parts, asset_index_path)?)?;
        if asset_index.format != "unippt-asset-index"
            || asset_index.version != 1
            || asset_index.entries.len() > MAX_ASSET_COUNT
        {
            return Err(UdocError::Invalid("UDOC3 资产索引结构无效".into()));
        }
        validate_cache_id(&asset_index.cache_id)?;
        package_cache_id = Some(asset_index.cache_id.clone());
        let has_legacy_asset_refs = asset_index
            .entries
            .iter()
            .any(|entry| entry.legacy_source_part_name.is_some());
        if has_legacy_asset_refs && legacy_presentation.is_none() {
            return Err(UdocError::Invalid(
                "opcPackage 格式禁止使用旧 sourcePartName 资产回指".into(),
            ));
        }
        let mut pptx_archive = if has_legacy_asset_refs {
            Some(
                zip::ZipArchive::new(Cursor::new(
                    legacy_presentation
                        .as_deref()
                        .expect("legacy presentation exists for sourcePartName"),
                ))
                .map_err(|error| UdocError::Invalid(format!("原生 PPTX ZIP 损坏：{error}")))?,
            )
        } else {
            None
        };
        let mut ids = HashSet::with_capacity(asset_index.entries.len());
        let mut accounted_paths = HashSet::with_capacity(asset_index.entries.len());
        let mut indexed_asset_bytes = 0usize;
        for entry in asset_index.entries {
            validate_asset_id(&entry.id)?;
            validate_path(&entry.path)?;
            let legacy_path = format!("assets/{}.bin", entry.id);
            let blob_path = opc_snapshot::blob_path_for_digest(&entry.sha256);
            if (entry.path != legacy_path && entry.path != blob_path)
                || !ids.insert(entry.id.clone())
            {
                return Err(UdocError::Invalid(format!(
                    "UDOC3 资产索引项重复或路径无效：{}",
                    entry.path
                )));
            }
            if accounted_paths.insert(entry.path.clone()) {
                indexed_asset_bytes = indexed_asset_bytes
                    .checked_add(entry.size)
                    .filter(|value| *value <= MAX_TOTAL_SIZE)
                    .ok_or_else(|| UdocError::Invalid("UDOC3 资产总尺寸超过安全上限".into()))?;
            }
            let raw: std::sync::Arc<[u8]> =
                if let Some(source_part_name) = entry.legacy_source_part_name.as_deref() {
                    std::sync::Arc::from(read_legacy_pptx_part(
                        pptx_archive
                            .as_mut()
                            .expect("archive exists when sourcePartName is present"),
                        source_part_name,
                        entry.size,
                    )?)
                } else {
                    parts.get(&entry.path).cloned().ok_or_else(|| {
                        UdocError::Invalid(format!("缺少 UDOC3 资产部件：{}", entry.path))
                    })?
                };
            if raw.len() != entry.size || sha256_hex(&raw) != entry.sha256 {
                return Err(UdocError::Invalid(format!(
                    "UDOC3 资产索引校验失败：{}",
                    entry.path
                )));
            }
            catalog
                .insert_original_asset_arc(&entry.id, &entry.mime, &entry.data_uri_prefix, raw)
                .map_err(UdocError::Asset)?;
        }
    }
    let media_report = media_derivative::enrich_playback_assets_for_decks(
        &mut [&mut deck, &mut original_deck],
        &catalog,
    );
    if media_report.generated > 0 {
        eprintln!(
            "regenerated {} browser media derivative(s) while opening UDoc3",
            media_report.generated
        );
    }
    for warning in media_report.warnings {
        eprintln!("UDoc3 media derivative skipped: {warning}");
    }
    let presentation_digest = opc.digest();
    let target_cache_id = format!("pptx-{}", hex_digest(&presentation_digest));
    if let Some(package_cache_id) = package_cache_id {
        rebind_deck_asset_cache_id(&mut deck, &package_cache_id, &target_cache_id, &catalog)
            .map_err(UdocError::Asset)?;
        rebind_deck_asset_cache_id(
            &mut original_deck,
            &package_cache_id,
            &target_cache_id,
            &catalog,
        )
        .map_err(UdocError::Asset)?;
    }
    // Legacy inline packages are compacted here; modern packages only run the
    // cheap URL/catalog validation path. Neither case materializes base64.
    externalize_deck_assets(&mut deck, &target_cache_id, &mut catalog).map_err(UdocError::Asset)?;
    externalize_deck_assets(&mut original_deck, &target_cache_id, &mut catalog)
        .map_err(UdocError::Asset)?;
    deck.source_import_id = Some(target_cache_id.clone());
    original_deck.source_import_id = Some(target_cache_id);
    Ok(DecodedCachedUdoc {
        deck,
        original_deck,
        assets: catalog,
        opc,
        presentation_digest,
    })
}

#[cfg(test)]
fn wrap_outer_zip(inner: &[u8]) -> Result<Vec<u8>, UdocError> {
    let cursor = Cursor::new(Vec::with_capacity(inner.len()));
    let mut writer = zip::ZipWriter::new(cursor);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        // The outer stream can still find repetition across independently
        // Brotli-packed JSON/XML parts and the content-addressed directory.
        // UDoc is a saved artifact rather than an interactive transport, so
        // prefer maximum Deflate density; payload hashes remain independent.
        .compression_level(Some(9));
    writer
        .start_file(OUTER_ZIP_ENTRY, options)
        .map_err(|error| UdocError::Invalid(format!("UDoc3 外层 ZIP 写入失败：{error}")))?;
    writer
        .write_all(inner)
        .map_err(|error| UdocError::Invalid(format!("UDoc3 外层 ZIP 写入失败：{error}")))?;
    writer
        .finish()
        .map(|cursor| cursor.into_inner())
        .map_err(|error| UdocError::Invalid(format!("UDoc3 外层 ZIP 收尾失败：{error}")))
}

fn unwrap_outer_zip(bytes: &[u8]) -> Result<Cow<'_, [u8]>, UdocError> {
    if bytes.starts_with(HEADER) {
        return Ok(Cow::Borrowed(bytes));
    }
    if !bytes.starts_with(b"PK") {
        return Err(UdocError::Invalid("缺少 UDoc3 或外层 ZIP 文件头".into()));
    }
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| UdocError::Invalid(format!("UDoc3 外层 ZIP 损坏：{error}")))?;
    if archive.len() != 1 {
        return Err(UdocError::Invalid(
            "UDoc3 外层 ZIP 必须且只能包含 package.udoc3".into(),
        ));
    }
    let mut entry = archive
        .by_index(0)
        .map_err(|error| UdocError::Invalid(format!("UDoc3 外层 ZIP 目录损坏：{error}")))?;
    if entry.is_dir() || entry.name() != OUTER_ZIP_ENTRY || entry.size() > MAX_TOTAL_SIZE as u64 {
        return Err(UdocError::Invalid(
            "UDoc3 外层 ZIP 部件名称或尺寸无效".into(),
        ));
    }
    let expected = entry.size() as usize;
    let mut inner = Vec::with_capacity(expected.min(8 * 1024 * 1024));
    entry
        .by_ref()
        .take(expected.saturating_add(1) as u64)
        .read_to_end(&mut inner)
        .map_err(|error| UdocError::Invalid(format!("UDoc3 外层 ZIP 解压失败：{error}")))?;
    if inner.len() != expected || !inner.starts_with(HEADER) {
        return Err(UdocError::Invalid("UDoc3 外层 ZIP 内部容器不完整".into()));
    }
    Ok(Cow::Owned(inner))
}

#[allow(dead_code)]
pub fn decode(bytes: &[u8]) -> Result<DecodedUdoc, UdocError> {
    let decoded = decode_cached(bytes)?;
    let presentation = decoded.opc.rebuild().map_err(UdocError::Invalid)?;
    let cache_id = decoded
        .deck
        .source_import_id
        .as_deref()
        .ok_or_else(|| UdocError::Asset("decoded compact Deck has no cache id".into()))?;
    let mut deck = materialize_deck_assets(&decoded.deck, cache_id, &decoded.assets)
        .map_err(UdocError::Asset)?;
    deck.source_import_id = None;
    Ok(DecodedUdoc { deck, presentation })
}

pub(crate) fn is_supported_unidoc_type(value: &str) -> bool {
    matches!(value, CURRENT_UNIDOC_TYPE | LEGACY_UNIDOC_TYPE)
}

struct Part<'a> {
    path: String,
    bytes: Cow<'a, [u8]>,
    mime: String,
    compression: PartCompression,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PartCompression {
    Brotli,
    ZipStored,
    ZipDeflated,
    /// Compatibility tests still reproduce parts written by the former
    /// Store/Brotli-only encoder.
    #[cfg(test)]
    Store,
}

impl PartCompression {
    fn for_resource(path_hint: &str, mime: &str) -> Self {
        if is_brotli_resource(path_hint, mime) {
            Self::Brotli
        } else if is_precompressed_resource(path_hint, mime) {
            Self::ZipStored
        } else {
            Self::ZipDeflated
        }
    }
}

impl<'a> Part<'a> {
    fn json(path: &str, value: &impl Serialize) -> Result<Self, UdocError> {
        Ok(Self {
            path: path.into(),
            bytes: Cow::Owned(serde_json::to_vec(value)?),
            mime: "application/json".into(),
            compression: PartCompression::Brotli,
        })
    }

    fn borrowed(path: &str, bytes: &'a [u8], mime: &str, compression: PartCompression) -> Self {
        Self {
            path: path.into(),
            bytes: Cow::Borrowed(bytes),
            mime: mime.into(),
            compression,
        }
    }
}

fn resource_extension(path: &str) -> &str {
    path.rsplit_once('.')
        .map(|(_, extension)| extension)
        .unwrap_or_default()
}

fn is_brotli_resource(path: &str, mime: &str) -> bool {
    let mime = mime
        .split_once(';')
        .map(|(mime, _)| mime)
        .unwrap_or(mime)
        .trim()
        .to_ascii_lowercase();
    let extension = resource_extension(path).to_ascii_lowercase();
    mime.starts_with("text/")
        || matches!(
            mime.as_str(),
            "application/json"
                | "application/xml"
                | "application/javascript"
                | "application/xhtml+xml"
                | "image/svg+xml"
        )
        || matches!(
            extension.as_str(),
            "json"
                | "html"
                | "htm"
                | "svg"
                | "xml"
                | "rels"
                | "vml"
                | "css"
                | "js"
                | "mjs"
                | "md"
                | "markdown"
                | "txt"
        )
}

fn is_precompressed_resource(path: &str, mime: &str) -> bool {
    let mime = mime
        .split_once(';')
        .map(|(mime, _)| mime)
        .unwrap_or(mime)
        .trim()
        .to_ascii_lowercase();
    let extension = resource_extension(path).to_ascii_lowercase();
    matches!(
        extension.as_str(),
        "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "webp"
            | "apng"
            | "avif"
            | "mp4"
            | "webm"
            | "ogv"
            | "mov"
            | "mp3"
            | "ogg"
            | "m4a"
            | "aac"
            | "flac"
            | "weba"
            | "zip"
            | "gz"
            | "br"
            | "pdf"
            | "woff"
            | "woff2"
    ) || mime.starts_with("image/") && mime != "image/bmp" && mime != "image/svg+xml"
        || matches!(
            mime.as_str(),
            "video/mp4"
                | "video/webm"
                | "video/quicktime"
                | "audio/mpeg"
                | "audio/ogg"
                | "audio/mp4"
                | "audio/aac"
                | "audio/flac"
                | "audio/webm"
                | "application/pdf"
                | "font/woff"
                | "font/woff2"
                | "application/zip"
                | "application/gzip"
                | "application/vnd.openxmlformats-officedocument.presentationml.presentation"
        )
}

fn encode_parts(parts: Vec<Part<'_>>, basename: &str) -> Result<Vec<u8>, UdocError> {
    let mut output = HEADER.to_vec();
    let mut entries = Vec::with_capacity(parts.len());
    for part in parts {
        validate_path(&part.path)?;
        let raw = part.bytes.as_ref();
        if raw.len() > MAX_ENTRY_SIZE {
            return Err(UdocError::Invalid(format!(
                "部件原始尺寸超过安全上限：{}",
                part.path
            )));
        }
        let (codec, payload): (&str, Cow<'_, [u8]>) = match part.compression {
            PartCompression::Brotli => {
                let compressed = (raw.len() >= 96).then(|| br_compress(raw)).transpose()?;
                let use_compressed = compressed
                    .as_ref()
                    .is_some_and(|compressed| compressed.len() + 16 < raw.len());
                if use_compressed {
                    ("br", Cow::Owned(compressed.expect("checked compressed")))
                } else {
                    ("store", Cow::Borrowed(raw))
                }
            }
            PartCompression::ZipStored | PartCompression::ZipDeflated => (
                "zip",
                Cow::Owned(zip_pack_part(
                    raw,
                    &part.path,
                    part.compression == PartCompression::ZipDeflated,
                )?),
            ),
            #[cfg(test)]
            PartCompression::Store => ("store", Cow::Borrowed(raw)),
        };
        let entry = DirectoryEntry {
            path: part.path,
            offset: output.len() as u64,
            compressed_size: payload.len(),
            size: raw.len(),
            codec: codec.into(),
            mime: part.mime,
            sha256: sha256_hex(raw),
        };
        output.extend_from_slice(&payload);
        entries.push(entry);
    }
    let directory = Directory {
        format: "udoc-directory".into(),
        version: 3,
        basename: basename.into(),
        root: "manifest.json".into(),
        entries,
    };
    let directory_bytes = serde_json::to_vec(&directory)?;
    let directory_hash = Sha256::digest(&directory_bytes);
    let packed_directory = br_compress(&directory_bytes)?;
    let directory_offset = output.len() as u64;
    output.extend_from_slice(&packed_directory);
    let mut footer = [0u8; FOOTER_SIZE];
    footer[..8].copy_from_slice(FOOTER_MAGIC);
    footer[8..16].copy_from_slice(&directory_offset.to_le_bytes());
    footer[16..24].copy_from_slice(&(packed_directory.len() as u64).to_le_bytes());
    footer[24..56].copy_from_slice(&directory_hash);
    footer[56..60].copy_from_slice(&3u32.to_le_bytes());
    footer[60..64].copy_from_slice(&(directory_bytes.len() as u32).to_le_bytes());
    output.extend_from_slice(&footer);
    Ok(output)
}

/// Wrap one binary resource in an independently readable, deterministic ZIP
/// member. The UDOC3 tail directory still owns addressing and SHA-256; this
/// inner ZIP contributes standard Store/Deflate framing and CRC without
/// turning the complete document into one monolithic archive.
fn zip_pack_part(input: &[u8], entry_path: &str, deflate: bool) -> Result<Vec<u8>, UdocError> {
    validate_path(entry_path)?;
    if input.len() > MAX_ENTRY_SIZE {
        return Err(UdocError::Invalid("ZIP 资源超过安全上限".into()));
    }
    let cursor = Cursor::new(Vec::with_capacity(
        input
            .len()
            .saturating_add(entry_path.len())
            .saturating_add(256),
    ));
    let mut writer = zip::ZipWriter::new(cursor);
    let method = if deflate {
        zip::CompressionMethod::Deflated
    } else {
        zip::CompressionMethod::Stored
    };
    let mut options = zip::write::SimpleFileOptions::default()
        .compression_method(method)
        .unix_permissions(0o644);
    if deflate {
        options = options.compression_level(Some(6));
    }
    writer
        .start_file(entry_path, options)
        .map_err(|error| UdocError::Invalid(format!("ZIP 资源建档失败：{error}")))?;
    writer
        .write_all(input)
        .map_err(|error| UdocError::Invalid(format!("ZIP 资源写入失败：{error}")))?;
    let packed = writer
        .finish()
        .map(|cursor| cursor.into_inner())
        .map_err(|error| UdocError::Invalid(format!("ZIP 资源收尾失败：{error}")))?;
    if packed.len() > MAX_ZIP_PART_SIZE {
        return Err(UdocError::Invalid("ZIP 资源编码后超过安全上限".into()));
    }
    Ok(packed)
}

fn zip_unpack_part(
    input: &[u8],
    expected_path: &str,
    expected_size: usize,
) -> Result<Vec<u8>, UdocError> {
    validate_path(expected_path)?;
    if input.len() > MAX_ZIP_PART_SIZE || expected_size > MAX_ENTRY_SIZE {
        return Err(UdocError::Invalid("ZIP 资源尺寸超过安全上限".into()));
    }
    let mut archive = zip::ZipArchive::new(Cursor::new(input))
        .map_err(|error| UdocError::Invalid(format!("ZIP 资源结构无效：{error}")))?;
    if archive.len() != 1 {
        return Err(UdocError::Invalid("ZIP 资源必须且只能包含一个文件".into()));
    }
    let mut member = archive
        .by_index(0)
        .map_err(|error| UdocError::Invalid(format!("ZIP 资源读取失败：{error}")))?;
    if member.is_dir()
        || member.encrypted()
        || member.name() != expected_path
        || member.size() != expected_size as u64
        || !matches!(
            member.compression(),
            zip::CompressionMethod::Stored | zip::CompressionMethod::Deflated
        )
    {
        return Err(UdocError::Invalid(
            "ZIP 资源路径、尺寸、加密标志或压缩算法无效".into(),
        ));
    }
    let mut output = Vec::with_capacity(expected_size.min(8 * 1024 * 1024));
    member
        .by_ref()
        .take(expected_size.saturating_add(1) as u64)
        .read_to_end(&mut output)
        .map_err(|error| UdocError::Invalid(format!("ZIP 资源解压失败：{error}")))?;
    if output.len() != expected_size {
        return Err(UdocError::Invalid(format!(
            "ZIP 资源解压尺寸不匹配：声明 {expected_size}，实际 {}",
            output.len()
        )));
    }
    Ok(output)
}

fn br_compress(input: &[u8]) -> Result<Vec<u8>, UdocError> {
    let params = brotli::enc::BrotliEncoderParams {
        // Scene/baseline JSON and OPC XML are the remaining overhead above a
        // native PPTX.  Quality 9 plus a 16 MiB window removes substantially
        // more repeated property names without touching already-compressed
        // image/audio/video blobs (those parts are stored verbatim).
        quality: 9,
        lgwin: 24,
        ..Default::default()
    };
    let mut output = Vec::new();
    let mut reader = input;
    brotli::BrotliCompress(&mut reader, &mut output, &params)
        .map_err(|error| UdocError::Brotli(error.to_string()))?;
    Ok(output)
}

fn br_decompress(input: &[u8], expected: usize) -> Result<Vec<u8>, UdocError> {
    if expected > MAX_ENTRY_SIZE {
        return Err(UdocError::Invalid("Brotli 解压尺寸超过安全上限".into()));
    }
    // Never reserve an attacker-controlled declared size up front.  Reading
    // through a hard `expected + 1` ceiling also stops a Brotli bomb before it
    // can grow the process to the old 512 MiB per-part allocation.
    let mut output = Vec::with_capacity(expected.min(8 * 1024 * 1024));
    let decoder = brotli::Decompressor::new(Cursor::new(input), 64 * 1024);
    decoder
        .take(expected.saturating_add(1) as u64)
        .read_to_end(&mut output)
        .map_err(|error| UdocError::Brotli(error.to_string()))?;
    if output.len() != expected {
        return Err(UdocError::Invalid(format!(
            "Brotli 解压尺寸不匹配：声明 {expected}，实际 {}",
            output.len()
        )));
    }
    Ok(output)
}

/// Read-only migration path for packages produced by the abandoned
/// `sourcePartName` design. New UDoc3 files never call this function.
fn read_legacy_pptx_part(
    archive: &mut zip::ZipArchive<Cursor<&[u8]>>,
    part_name: &str,
    expected_size: usize,
) -> Result<Vec<u8>, UdocError> {
    validate_path(part_name)?;
    if expected_size > MAX_ENTRY_SIZE {
        return Err(UdocError::Invalid(format!(
            "旧 UDoc3 PPTX 资产部件过大：{part_name}"
        )));
    }
    let mut member = archive
        .by_name(part_name)
        .map_err(|_| UdocError::Invalid(format!("旧 UDoc3 PPTX 缺少资产：{part_name}")))?;
    if member.is_dir() || member.size() != expected_size as u64 {
        return Err(UdocError::Invalid(format!(
            "旧 UDoc3 PPTX 资产尺寸不一致：{part_name}"
        )));
    }
    let mut bytes = Vec::with_capacity(expected_size);
    member
        .by_ref()
        .take(expected_size as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            UdocError::Invalid(format!("读取旧 UDoc3 PPTX 资产失败：{part_name}: {error}"))
        })?;
    if bytes.len() != expected_size {
        return Err(UdocError::Invalid(format!(
            "旧 UDoc3 PPTX 资产尺寸不一致：{part_name}"
        )));
    }
    Ok(bytes)
}

fn required<'a>(
    parts: &'a HashMap<String, std::sync::Arc<[u8]>>,
    path: &str,
) -> Result<&'a [u8], UdocError> {
    parts
        .get(path)
        .map(AsRef::as_ref)
        .ok_or_else(|| UdocError::Invalid(format!("缺少部件：{path}")))
}

fn validate_manifest_relationships(
    manifest: &Manifest,
    parts: &HashMap<String, std::sync::Arc<[u8]>>,
) -> Result<(), UdocError> {
    validate_path(&manifest.relationships)?;
    let relationship_part: RelationshipPart =
        serde_json::from_slice(required(parts, &manifest.relationships)?)?;
    if relationship_part.version != 1 || relationship_part.relationships.len() > MAX_ASSET_COUNT {
        return Err(UdocError::Invalid(
            "UDOC3 relationships 部件版本或关系数量无效".into(),
        ));
    }

    let mut unique = HashSet::with_capacity(relationship_part.relationships.len());
    for relationship in &relationship_part.relationships {
        validate_path(&relationship.source)?;
        if !parts.contains_key(&relationship.source) {
            return Err(UdocError::Invalid(format!(
                "UDOC3 relationship source 部件不存在：{}",
                relationship.source
            )));
        }
        if relationship.kind.is_empty()
            || relationship.kind.len() > 256
            || relationship.kind.chars().any(char::is_control)
        {
            return Err(UdocError::Invalid("UDOC3 relationship type 无效".into()));
        }
        match relationship.mode.as_str() {
            "internal" => {
                validate_path(&relationship.target)?;
                if !parts.contains_key(&relationship.target) {
                    return Err(UdocError::Invalid(format!(
                        "UDOC3 relationship target 部件不存在：{}",
                        relationship.target
                    )));
                }
            }
            "external" => {
                if relationship.target.trim().is_empty()
                    || relationship.target.chars().any(char::is_control)
                {
                    return Err(UdocError::Invalid(
                        "UDOC3 external relationship target 无效".into(),
                    ));
                }
            }
            _ => {
                return Err(UdocError::Invalid(format!(
                    "UDOC3 relationship mode 无效：{}",
                    relationship.mode
                )));
            }
        }
        if !unique.insert((
            relationship.source.as_str(),
            relationship.kind.as_str(),
            relationship.target.as_str(),
            relationship.mode.as_str(),
        )) {
            return Err(UdocError::Invalid("UDOC3 relationship 重复".into()));
        }
    }

    let has_internal = |kind: &str, target: &str| {
        relationship_part.relationships.iter().any(|relationship| {
            relationship.source == manifest.root
                && relationship.kind == kind
                && relationship.target == target
                && relationship.mode == "internal"
        })
    };
    if let Some(opc_package) = manifest.opc_package.as_deref() {
        if !has_internal("native-opc-package", opc_package) {
            return Err(UdocError::Invalid(
                "UDOC3 relationships 缺少 manifest.opcPackage 关系".into(),
            ));
        }
    }
    if let Some(presentation) = manifest.presentation.as_deref() {
        if !has_internal("native-presentation", presentation) {
            return Err(UdocError::Invalid(
                "UDOC3 relationships 缺少 manifest.presentation 关系".into(),
            ));
        }
    }
    if let Some(assets) = manifest.assets.as_deref() {
        if !has_internal("asset-index", assets) {
            return Err(UdocError::Invalid(
                "UDOC3 relationships 缺少 manifest.assets 关系".into(),
            ));
        }
    }
    Ok(())
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, UdocError> {
    let value = bytes
        .get(offset..offset + 8)
        .ok_or_else(|| UdocError::Invalid("尾目录整数越界".into()))?;
    Ok(u64::from_le_bytes(value.try_into().expect("8 bytes")))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, UdocError> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| UdocError::Invalid("尾目录整数越界".into()))?;
    Ok(u32::from_le_bytes(value.try_into().expect("4 bytes")))
}

fn validate_path(path: &str) -> Result<(), UdocError> {
    if path.is_empty()
        || path.len() > 1024
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains('\0')
        || path
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
    {
        return Err(UdocError::Invalid(format!("非法部件路径：{path}")));
    }
    Ok(())
}

fn validate_cache_id(value: &str) -> Result<(), UdocError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(UdocError::Invalid("UDOC3 资产 cacheId 非法".into()));
    }
    Ok(())
}

fn validate_asset_id(value: &str) -> Result<(), UdocError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(UdocError::Invalid(format!("UDOC3 资产 id 非法：{value}")));
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex_digest(&Sha256::digest(bytes))
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn safe_basename(value: &str) -> String {
    let basename: String = value
        .chars()
        .filter(|character| {
            !matches!(
                character,
                '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
            )
        })
        .take(80)
        .collect();
    let basename = basename.trim();
    if basename.is_empty() {
        "presentation".into()
    } else {
        basename.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use std::io::Write;

    fn directory(bytes: &[u8]) -> Directory {
        let package = unwrap_outer_zip(bytes).unwrap();
        let bytes = package.as_ref();
        let footer = &bytes[bytes.len() - FOOTER_SIZE..];
        let offset = read_u64(footer, 8).unwrap() as usize;
        let length = read_u64(footer, 16).unwrap() as usize;
        let raw_length = read_u32(footer, 60).unwrap() as usize;
        let raw = br_decompress(&bytes[offset..offset + length], raw_length).unwrap();
        serde_json::from_slice(&raw).unwrap()
    }

    fn part(bytes: &[u8], path: &str) -> Vec<u8> {
        let directory = directory(bytes);
        let package = unwrap_outer_zip(bytes).unwrap();
        let bytes = package.as_ref();
        let entry = directory
            .entries
            .iter()
            .find(|entry| entry.path == path)
            .unwrap_or_else(|| panic!("missing UDoc part {path}"));
        let start = entry.offset as usize;
        let end = start + entry.compressed_size;
        match entry.codec.as_str() {
            "br" => br_decompress(&bytes[start..end], entry.size).unwrap(),
            "zip" => zip_unpack_part(&bytes[start..end], &entry.path, entry.size).unwrap(),
            "store" => bytes[start..end].to_vec(),
            codec => panic!("unsupported test codec {codec}"),
        }
    }

    fn replace_part(bytes: &[u8], path: &str, replacement: Vec<u8>) -> Vec<u8> {
        let directory = directory(bytes);
        let owned: Vec<_> = directory
            .entries
            .iter()
            .map(|entry| {
                let raw = if entry.path == path {
                    replacement.clone()
                } else {
                    part(bytes, &entry.path)
                };
                let compression = match entry.codec.as_str() {
                    "br" => PartCompression::Brotli,
                    "zip" => PartCompression::for_resource(&entry.path, &entry.mime),
                    "store" => PartCompression::Store,
                    codec => panic!("unsupported test codec {codec}"),
                };
                (entry.path.clone(), raw, entry.mime.clone(), compression)
            })
            .collect();
        let parts = owned
            .iter()
            .map(|(path, raw, mime, compression)| Part::borrowed(path, raw, mime, *compression))
            .collect();
        encode_parts(parts, &directory.basename).unwrap()
    }

    fn opc_members(bytes: &[u8]) -> Vec<(String, Vec<u8>, zip::CompressionMethod)> {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut members = Vec::with_capacity(archive.len());
        for index in 0..archive.len() {
            let mut member = archive.by_index(index).unwrap();
            let name = member.name().to_string();
            let compression = member.compression();
            let mut raw = Vec::new();
            member.read_to_end(&mut raw).unwrap();
            members.push((name, raw, compression));
        }
        members
    }

    fn pptx_with_media(media: &[u8]) -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for (name, contents) in [
            ("[Content_Types].xml", b"<Types/>".as_slice()),
            ("_rels/.rels", b"<Relationships/>".as_slice()),
            ("ppt/presentation.xml", b"<p:presentation/>".as_slice()),
            ("ppt/media/image1.png", media),
        ] {
            writer.start_file(name, options).unwrap();
            writer.write_all(contents).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    fn encode_legacy_inline_with_type(
        deck: &Deck,
        presentation: &[u8],
        unidoc_type: &str,
    ) -> Vec<u8> {
        let document = DocumentPart {
            format: "udoc".into(),
            version: 3,
            unidoc_type: unidoc_type.into(),
            app: "UniPPT".into(),
            deck: deck.clone(),
            baseline: None,
        };
        let manifest = Manifest {
            format: "udoc-package".into(),
            version: 3,
            unidoc_type: unidoc_type.into(),
            app: "UniPPT".into(),
            basename: "legacy".into(),
            root: "document/document.json".into(),
            presentation: Some("document/presentation.pptx".into()),
            opc_package: None,
            relationships: "rels/relationships.json".into(),
            assets: None,
            features: vec!["unippt".into(), "pptx".into(), "br".into()],
        };
        let relationships = serde_json::json!({
            "version": 1,
            "relationships": [{
                "source": "document/document.json",
                "type": "native-presentation",
                "target": "document/presentation.pptx",
                "mode": "internal"
            }]
        });
        encode_parts(
            vec![
                Part::json("manifest.json", &manifest).unwrap(),
                Part::json("document/document.json", &document).unwrap(),
                Part::borrowed(
                    "document/presentation.pptx",
                    presentation,
                    "application/vnd.openxmlformats-officedocument.presentationml.presentation",
                    PartCompression::Store,
                ),
                Part::json("rels/relationships.json", &relationships).unwrap(),
            ],
            "legacy",
        )
        .unwrap()
    }

    #[test]
    fn pptx_udoc3_round_trip_and_type_guard() {
        let mut deck = Deck::demo();
        deck.extensions.insert(
            "org.unippt.ai".into(),
            serde_json::json!({"schemaVersion": 1, "objects": {"shape-title": {"role": "slide-title"}}}),
        );
        let pptx = pptx_with_media(b"native pptx proof");
        let bytes = encode(&deck, &pptx, "动画演示").unwrap();
        assert!(bytes.starts_with(b"UDOC3PKG"));
        assert_eq!(&bytes[bytes.len() - 64..bytes.len() - 56], b"UD3DIR01");
        let directory = directory(&bytes);
        assert!(directory
            .entries
            .iter()
            .any(|entry| entry.path == "pptx/package.json"));
        assert!(!directory
            .entries
            .iter()
            .any(|entry| entry.path.ends_with(".pptx")));
        let manifest: serde_json::Value =
            serde_json::from_slice(&part(&bytes, "manifest.json")).unwrap();
        assert_eq!(manifest["opcPackage"], "pptx/package.json");
        assert!(manifest.get("presentation").is_none());
        assert!(manifest["features"]
            .as_array()
            .unwrap()
            .iter()
            .any(|feature| feature == "hybrid-br-zip"));
        assert!(!manifest["features"]
            .as_array()
            .unwrap()
            .iter()
            .any(|feature| feature == "outer-zip-v1"));
        assert!(!String::from_utf8(part(&bytes, "assets/index.json"))
            .unwrap()
            .contains("sourcePartName"));
        let document: serde_json::Value =
            serde_json::from_slice(&part(&bytes, "document/document.json")).unwrap();
        assert!(
            document.get("baseline").is_none(),
            "identical scene baseline must be deduplicated"
        );
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded.deck, deck);
        assert_eq!(
            decoded.deck.extensions["org.unippt.ai"]["objects"]["shape-title"]["role"],
            "slide-title"
        );
        assert_eq!(opc_members(&decoded.presentation), opc_members(&pptx));

        let mut damaged = bytes;
        damaged[0] = b'X';
        assert!(decode(&damaged).is_err());
    }

    #[test]
    fn decoder_keeps_legacy_outer_zip_read_compatibility() {
        let deck = Deck::demo();
        let pptx = pptx_with_media(b"legacy outer ZIP");
        let direct = encode(&deck, &pptx, "legacy-outer").unwrap();
        assert!(direct.starts_with(HEADER));
        let legacy_outer = wrap_outer_zip(&direct).unwrap();
        assert!(legacy_outer.starts_with(b"PK"));
        let decoded = decode(&legacy_outer).unwrap();
        assert_eq!(decoded.deck, deck);
        assert_eq!(opc_members(&decoded.presentation), opc_members(&pptx));
    }

    #[test]
    fn binary_parts_use_single_member_zip_with_store_or_deflate() {
        let compressed_source = (0..=255).cycle().take(32 * 1024).collect::<Vec<_>>();
        let compressed_path = "media/0123456789abcdef.png";
        let stored = zip_pack_part(&compressed_source, compressed_path, false).unwrap();
        assert_eq!(
            stored,
            zip_pack_part(&compressed_source, compressed_path, false).unwrap(),
            "same resource must produce deterministic ZIP bytes"
        );
        let mut stored_archive = zip::ZipArchive::new(Cursor::new(&stored)).unwrap();
        assert_eq!(stored_archive.len(), 1);
        assert_eq!(
            stored_archive.by_index(0).unwrap().compression(),
            zip::CompressionMethod::Stored
        );
        assert_eq!(
            zip_unpack_part(&stored, compressed_path, compressed_source.len()).unwrap(),
            compressed_source
        );

        let raw_source = vec![0x41; 64 * 1024];
        let raw_path = "embeds/audio.wav";
        let deflated = zip_pack_part(&raw_source, raw_path, true).unwrap();
        assert!(deflated.len() < raw_source.len() / 4);
        let mut deflated_archive = zip::ZipArchive::new(Cursor::new(&deflated)).unwrap();
        assert_eq!(
            deflated_archive.by_index(0).unwrap().compression(),
            zip::CompressionMethod::Deflated
        );
        assert_eq!(
            zip_unpack_part(&deflated, raw_path, raw_source.len()).unwrap(),
            raw_source
        );
        assert!(zip_unpack_part(&stored, "media/wrong.png", compressed_source.len()).is_err());
        assert!(zip_unpack_part(&stored, compressed_path, compressed_source.len() - 1).is_err());
        assert!(zip_pack_part(b"x", "../escape.png", false).is_err());
    }

    #[test]
    fn structure_declares_pptx_type_first_and_drops_transient_source_id() {
        let mut deck = Deck::demo();
        deck.source_import_id = Some("temporary".into());
        let structure = structure(&deck);
        let json = serde_json::to_string(&structure).unwrap();
        assert!(json.starts_with(r#"{"app":"UniPPT","unidoc_type":"pptx","#));
        let value = serde_json::to_value(structure).unwrap();
        assert_eq!(value["unidoc_type"], "pptx");
        assert_eq!(value["manifest"]["unidoc_type"], "pptx");
        assert_eq!(value["manifest"]["opcPackage"], "pptx/package.json");
        assert_eq!(value["manifest"]["assets"], "assets/index.json");
        assert!(value["manifest"].get("presentation").is_none());
        assert_eq!(value["relationships"]["version"], 1);
        assert_eq!(
            value["relationships"]["relationships"],
            serde_json::json!([
                {
                    "source": "document/document.json",
                    "type": "native-opc-package",
                    "target": "pptx/package.json",
                    "mode": "internal"
                },
                {
                    "source": "document/document.json",
                    "type": "asset-index",
                    "target": "assets/index.json",
                    "mode": "internal"
                }
            ])
        );
        assert!(value["manifest"]["features"]
            .as_array()
            .unwrap()
            .iter()
            .any(|feature| feature == "opc-parts-v1"));
        assert!(value["manifest"]["features"]
            .as_array()
            .unwrap()
            .iter()
            .any(|feature| feature == "hybrid-br-zip"));
        assert!(!value["manifest"]["features"]
            .as_array()
            .unwrap()
            .iter()
            .any(|feature| feature == "outer-zip-v1"));
        assert_eq!(value["document"]["unidoc_type"], "pptx");
        assert!(value["document"]["deck"]["sourceImportId"].is_null());
    }

    #[test]
    fn decoder_validates_relationship_part_and_manifest_targets() {
        let bytes = encode(&Deck::demo(), &pptx_with_media(b"relationships"), "rels").unwrap();

        let mut manifest: Manifest =
            serde_json::from_slice(&part(&bytes, "manifest.json")).unwrap();
        manifest.relationships = "rels/missing.json".into();
        let missing_part = replace_part(
            &bytes,
            "manifest.json",
            serde_json::to_vec(&manifest).unwrap(),
        );
        assert!(decode(&missing_part).is_err());

        let malformed = replace_part(&bytes, "rels/relationships.json", b"{".to_vec());
        assert!(matches!(decode(&malformed), Err(UdocError::Json(_))));

        let mut relationships: RelationshipPart =
            serde_json::from_slice(&part(&bytes, "rels/relationships.json")).unwrap();
        relationships.relationships[0].target = "pptx/missing.json".into();
        let missing_target = replace_part(
            &bytes,
            "rels/relationships.json",
            serde_json::to_vec(&relationships).unwrap(),
        );
        assert!(matches!(
            decode(&missing_target),
            Err(UdocError::Invalid(message)) if message.contains("target")
        ));

        relationships.relationships.remove(0);
        relationships.relationships[0].target = "assets/index.json".into();
        let missing_required = replace_part(
            &bytes,
            "rels/relationships.json",
            serde_json::to_vec(&relationships).unwrap(),
        );
        assert!(matches!(
            decode(&missing_required),
            Err(UdocError::Invalid(message)) if message.contains("manifest.opcPackage")
        ));
    }

    #[test]
    fn decoder_accepts_legacy_ppt_type() {
        let deck = Deck::demo();
        let pptx = pptx_with_media(b"legacy native pptx proof");
        let bytes = encode_legacy_inline_with_type(&deck, &pptx, LEGACY_UNIDOC_TYPE);
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded.deck, deck);
        assert_eq!(opc_members(&decoded.presentation), opc_members(&pptx));
        let compact = decode_cached(&bytes).unwrap();
        assert!(!serde_json::to_string(&compact.deck)
            .unwrap()
            .contains("data:"));
    }

    #[test]
    fn repeated_inline_assets_are_written_once_and_round_trip() {
        let asset_bytes = vec![0x5a; 256 * 1024];
        let payload = format!("data:image/png;base64,{}", STANDARD.encode(&asset_bytes));
        let mut deck = Deck::demo();
        deck.slides[0].background_asset = Some(payload.clone());
        deck.slides[0].objects[0].asset = Some(payload.clone());
        deck.slides[0].objects[0].shape_fill_asset = Some(payload.clone());
        deck.slides[0].objects[0].style.fill = format!(r#"url("{payload}") no-repeat"#);
        let full_json_size = serde_json::to_vec(&deck).unwrap().len();
        let pptx = pptx_with_media(b"different native media");

        let encoded = encode(&deck, &pptx, "assets").unwrap();
        let directory = directory(&encoded);
        assert!(!directory
            .entries
            .iter()
            .any(|entry| entry.path.ends_with(".pptx")));
        assert!(directory
            .entries
            .iter()
            .any(|entry| entry.path == "pptx/package.json"));
        let asset_index: AssetIndex =
            serde_json::from_slice(&part(&encoded, "assets/index.json")).unwrap();
        assert_eq!(asset_index.entries.len(), 1);
        let asset_path = &asset_index.entries[0].path;
        assert_eq!(
            asset_path,
            &opc_snapshot::blob_path_for_digest(&sha256_hex(&asset_bytes))
        );
        assert_eq!(
            directory
                .entries
                .iter()
                .filter(|entry| &entry.path == asset_path)
                .count(),
            1
        );
        assert_eq!(
            directory
                .entries
                .iter()
                .find(|entry| &entry.path == asset_path)
                .unwrap()
                .codec,
            "zip"
        );
        let document = directory
            .entries
            .iter()
            .find(|entry| entry.path == "document/document.json")
            .unwrap();
        assert!(document.size * 4 < full_json_size);
        assert!(!directory
            .entries
            .iter()
            .any(|entry| entry.path.ends_with(".pptx")));

        let cached = decode_cached(&encoded).unwrap();
        assert_eq!(cached.assets.len(), 1);
        assert_eq!(
            opc_members(&cached.opc.rebuild().unwrap()),
            opc_members(&pptx)
        );
        assert!(!serde_json::to_string(&cached.deck)
            .unwrap()
            .contains("data:image/png"));
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded.deck, deck);
        assert_eq!(opc_members(&decoded.presentation), opc_members(&pptx));
    }

    #[test]
    fn cached_udoc_reopens_current_scene_and_immutable_baseline_from_snapshot() {
        let pptx = pptx_with_media(b"self-contained native part");
        let opc = opc_snapshot::explode(&pptx).unwrap();
        let cache_id = format!("pptx-{}", "b".repeat(64));
        let mut baseline = Deck::demo();
        baseline.source_import_id = Some(cache_id.clone());
        let mut current = baseline.clone();
        current.title = "edited current scene".into();
        current.slides[0].name = "edited slide".into();
        let assets = AssetCatalog::default();

        let encoded = encode_cached(&current, &baseline, &assets, &opc, "baseline").unwrap();
        let directory = directory(&encoded);
        assert!(!directory.entries.iter().any(
            |entry| entry.path == "document/presentation.pptx" || entry.path.ends_with(".pptx")
        ));
        assert!(directory
            .entries
            .iter()
            .any(|entry| entry.path == "pptx/package.json"));

        let reopened = decode_cached(&encoded).unwrap();
        assert_eq!(reopened.deck.title, "edited current scene");
        assert_eq!(reopened.deck.slides[0].name, "edited slide");
        assert_eq!(reopened.original_deck.title, baseline.title);
        assert_eq!(
            reopened.original_deck.slides[0].name,
            baseline.slides[0].name
        );
        assert_eq!(reopened.opc.digest(), opc.digest());
        assert_eq!(
            opc_members(&reopened.opc.rebuild().unwrap()),
            opc_members(&pptx)
        );
    }

    #[test]
    fn native_opc_blob_is_shared_with_the_scene_asset_index() {
        let raw: Vec<u8> = (0..256 * 1024)
            .map(|index| ((index * 73 + index / 251) & 0xff) as u8)
            .collect();
        let payload = format!("data:image/png;base64,{}", STANDARD.encode(&raw));
        let mut deck = Deck::demo();
        deck.slides[0].background_asset = Some(payload);
        let pptx = pptx_with_media(&raw);

        let encoded = encode(&deck, &pptx, "pptx-backed-assets").unwrap();
        let directory = directory(&encoded);
        assert!(!directory
            .entries
            .iter()
            .any(|entry| entry.path.ends_with(".pptx")));
        let package: OpcPackageIndex =
            serde_json::from_slice(&part(&encoded, "pptx/package.json")).unwrap();
        let native_media = package
            .entries
            .iter()
            .find(|entry| entry.name == "ppt/media/image1.png")
            .unwrap();
        let asset_index: AssetIndex =
            serde_json::from_slice(&part(&encoded, "assets/index.json")).unwrap();
        assert_eq!(asset_index.entries.len(), 1);
        assert_eq!(asset_index.entries[0].path, native_media.blob_path);
        assert_eq!(
            directory
                .entries
                .iter()
                .filter(|entry| entry.path == native_media.blob_path)
                .count(),
            1
        );
        assert!(encoded.len() < pptx.len() + raw.len() / 2);

        let cached = decode_cached(&encoded).unwrap();
        assert_eq!(cached.assets.len(), 1);
        assert!(std::sync::Arc::ptr_eq(
            &cached.assets.get(&asset_index.entries[0].id).unwrap().bytes,
            cached.opc.blobs.get(&native_media.blob_path).unwrap(),
        ));
        assert_eq!(
            opc_members(&cached.opc.rebuild().unwrap()),
            opc_members(&pptx)
        );
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded.deck, deck);
    }

    #[test]
    fn cached_compact_deck_encodes_without_materializing_document_json() {
        let payload = format!(
            "data:image/png;base64,{}",
            STANDARD.encode(vec![0x33; 512 * 1024])
        );
        let cache_id = format!("pptx-{}", "a".repeat(64));
        let mut original = Deck::demo();
        original.source_import_id = Some(cache_id.clone());
        original.slides[0].background_asset = Some(payload.clone());
        original.slides[0].objects[0].asset = Some(payload);
        let mut compact = original.clone();
        let mut catalog = AssetCatalog::default();
        externalize_deck_assets(&mut compact, &cache_id, &mut catalog).unwrap();
        let compact_size = serde_json::to_vec(&compact).unwrap().len();
        let pptx = pptx_with_media(b"compact native media");
        let opc = opc_snapshot::explode(&pptx).unwrap();

        let encoded = encode_cached(&compact, &compact, &catalog, &opc, "compact").unwrap();
        let directory = directory(&encoded);
        let document = directory
            .entries
            .iter()
            .find(|entry| entry.path == "document/document.json")
            .unwrap();
        assert!(document.size <= compact_size.saturating_mul(2) + 512);

        let reopened = decode_cached(&encoded).unwrap();
        let rebound_id = format!("pptx-{}", hex_digest(&reopened.presentation_digest));
        assert_eq!(
            reopened.deck.source_import_id.as_deref(),
            Some(rebound_id.as_str())
        );
        assert!(serde_json::to_string(&reopened.deck)
            .unwrap()
            .contains(&format!("/api/cache/{rebound_id}/asset/")));

        original.source_import_id = None;
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded.deck, original);
        assert_eq!(opc_members(&decoded.presentation), opc_members(&pptx));
    }

    #[test]
    #[ignore = "set UNIPPT_BENCH_PPTX to run the real corpus benchmark"]
    fn real_pptx_compact_udoc_benchmark() {
        let path = std::env::var("UNIPPT_BENCH_PPTX").expect("UNIPPT_BENCH_PPTX");
        let pptx = std::fs::read(path).unwrap();
        let mut deck = unippt_core::import_pptx(&pptx).unwrap();
        let cache_id = format!("pptx-{}", sha256_hex(&pptx));
        deck.source_import_id = Some(cache_id.clone());
        let mut assets = AssetCatalog::default();
        externalize_deck_assets(&mut deck, &cache_id, &mut assets).unwrap();
        let compact_json_size = serde_json::to_vec(&deck).unwrap().len();
        let opc = opc_snapshot::explode(&pptx).unwrap();

        let started = std::time::Instant::now();
        let encoded = encode_cached(&deck, &deck, &assets, &opc, "9").unwrap();
        let elapsed = started.elapsed();
        let decode_started = std::time::Instant::now();
        let reopened = decode_cached(&encoded).unwrap();
        let decode_elapsed = decode_started.elapsed();
        let directory = directory(&encoded);
        let mut codec_counts = HashMap::new();
        for entry in &directory.entries {
            *codec_counts.entry(entry.codec.as_str()).or_insert(0usize) += 1;
        }
        let declared_total: usize = directory.entries.iter().map(|entry| entry.size).sum();
        let encoded_parts: usize = directory
            .entries
            .iter()
            .map(|entry| entry.compressed_size)
            .sum();
        let legacy_outer_size = wrap_outer_zip(&encoded).unwrap().len();
        assert!(!directory
            .entries
            .iter()
            .any(|entry| entry.path.ends_with(".pptx")));
        assert!(directory
            .entries
            .iter()
            .any(|entry| entry.path == "pptx/package.json"));
        let document_size = directory
            .entries
            .iter()
            .find(|entry| entry.path == "document/document.json")
            .unwrap()
            .size;
        eprintln!(
            "pptx={} compact_json={} document_part={} unique_assets={} raw_parts={} encoded_parts={} udoc={} legacy_outer={} codecs={:?} encode_ms={} decode_ms={}",
            pptx.len(),
            compact_json_size,
            document_size,
            assets.len(),
            declared_total,
            encoded_parts,
            encoded.len(),
            legacy_outer_size,
            codec_counts,
            elapsed.as_millis(),
            decode_elapsed.as_millis(),
        );
        assert_eq!(
            opc_members(&reopened.opc.rebuild().unwrap()),
            opc_members(&pptx)
        );
        for (id, expected) in assets.iter() {
            let actual = reopened
                .assets
                .get(id)
                .unwrap_or_else(|| panic!("missing reopened asset {id}"));
            assert_eq!(actual.mime_type, expected.mime_type);
            assert_eq!(actual.bytes, expected.bytes);
        }
        assert_eq!(reopened.deck.slides.len(), deck.slides.len());
        assert!(serde_json::to_vec(&reopened.deck).unwrap().len() < 10 * 1024 * 1024);
        assert!(document_size < 10 * 1024 * 1024);
        assert!(encoded.len() < pptx.len() + 2 * 1024 * 1024);
        assert!(elapsed < std::time::Duration::from_secs(60));
        assert!(decode_elapsed < std::time::Duration::from_secs(60));
    }
}
