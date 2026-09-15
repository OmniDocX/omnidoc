//! Browser playback derivatives for native PowerPoint media.
//!
//! The core scene model keeps the package asset untouched.  This server-only
//! enrichment step adds a high-quality H.264/MP4 derivative when an imported
//! AVI cannot be decoded reliably by browsers.  Failure is deliberately
//! non-fatal: native PPTX/UDOC round trips continue to use the original asset.

use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use sha2::{Digest, Sha256};
use unippt_core::{Deck, MediaData, MediaKind, SceneObject, ASSET_REFERENCE_PREFIX};

use crate::asset_transport::AssetCatalog;
use crate::opc_snapshot::OpcSnapshot;

const MAX_INPUT_BYTES: usize = 64 * 1024 * 1024;
const MAX_OUTPUT_BYTES: u64 = 64 * 1024 * 1024;
const TRANSCODE_TIMEOUT: Duration = Duration::from_secs(45);
const POLL_INTERVAL: Duration = Duration::from_millis(25);
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Default)]
pub(crate) struct DerivativeReport {
    /// Number of unique native assets transcoded in this deck.
    pub generated: usize,
    /// Number of scene media descriptors pointed at a generated derivative.
    pub assigned: usize,
    /// Deduplicated, non-fatal diagnostic messages.
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
struct DerivedMedia {
    asset: String,
    mime_type: String,
}

struct EnrichmentState<F> {
    derive: F,
    by_key: HashMap<String, Result<DerivedMedia, String>>,
    key_by_part: HashMap<String, String>,
    warned: HashSet<String>,
    report: DerivativeReport,
}

/// Enrich a compact Core import without materializing all scene assets.
/// Only an AVI that actually needs a browser derivative is copied out of the
/// unique asset catalog for ffmpeg; repeated pictures remain short refs.
pub(crate) fn enrich_playback_assets_from_catalog(
    deck: &mut Deck,
    catalog: &AssetCatalog,
) -> DerivativeReport {
    enrich_decks_with_catalog(&mut [deck], Some(catalog), transcode_h264_mp4)
}

/// Recreate portable browser derivatives for both the working scene and its
/// immutable baseline in one pass. Sharing the derivation cache avoids running
/// ffmpeg twice and keeps both Decks byte-for-byte comparable after reopen.
pub(crate) fn enrich_playback_assets_for_decks(
    decks: &mut [&mut Deck],
    catalog: &AssetCatalog,
) -> DerivativeReport {
    enrich_decks_with_catalog(decks, Some(catalog), transcode_h264_mp4)
}

#[cfg(test)]
fn enrich_with<F>(deck: &mut Deck, derive: F) -> DerivativeReport
where
    F: FnMut(&[u8]) -> Result<Vec<u8>, String>,
{
    enrich_decks_with_catalog(&mut [deck], None, derive)
}

fn enrich_decks_with_catalog<F>(
    decks: &mut [&mut Deck],
    catalog: Option<&AssetCatalog>,
    derive: F,
) -> DerivativeReport
where
    F: FnMut(&[u8]) -> Result<Vec<u8>, String>,
{
    let mut state = EnrichmentState {
        derive,
        by_key: HashMap::new(),
        key_by_part: HashMap::new(),
        warned: HashSet::new(),
        report: DerivativeReport::default(),
    };
    for deck in decks {
        for slide in &mut deck.slides {
            visit_objects(&mut slide.master_objects, catalog, &mut state);
            visit_objects(&mut slide.layout_objects, catalog, &mut state);
            visit_objects(&mut slide.objects, catalog, &mut state);
        }
    }
    state.report
}

/// Remove only server-generated AVI→MP4 playback caches before UDoc packing.
/// Native media and user-inserted assets remain untouched. The omitted bytes
/// are regenerated from the preserved native AVI when the UDoc is reopened.
pub(crate) fn strip_regenerable_playback_assets(
    decks: &mut [&mut Deck],
    catalog: &AssetCatalog,
    opc: &OpcSnapshot,
) -> usize {
    let native_blobs = opc
        .index
        .entries
        .iter()
        .map(|entry| entry.sha256.as_str())
        .collect::<HashSet<_>>();
    let mut stripped = 0usize;
    for deck in decks {
        for slide in &mut deck.slides {
            stripped += strip_objects(&mut slide.master_objects, catalog, &native_blobs);
            stripped += strip_objects(&mut slide.layout_objects, catalog, &native_blobs);
            stripped += strip_objects(&mut slide.objects, catalog, &native_blobs);
        }
    }
    stripped
}

fn strip_objects(
    objects: &mut [SceneObject],
    catalog: &AssetCatalog,
    native_blobs: &HashSet<&str>,
) -> usize {
    let mut stripped = 0usize;
    for object in objects {
        if let Some(media) = &mut object.media {
            let native_is_preserved = media
                .asset
                .as_deref()
                .and_then(asset_reference_id)
                .and_then(|id| catalog.get(id))
                .is_some_and(|asset| {
                    let digest = hex_sha256(asset.bytes.as_ref());
                    native_blobs.contains(digest.as_str())
                });
            let generated = native_is_preserved
                && requires_avi_derivative(media)
                && media
                    .playback_mime_type
                    .as_deref()
                    .is_some_and(|mime| mime.eq_ignore_ascii_case("video/mp4"))
                && media
                    .playback_asset
                    .as_deref()
                    .and_then(asset_reference_id)
                    .and_then(|id| catalog.get(id))
                    .is_some_and(|asset| {
                        let digest = hex_sha256(asset.bytes.as_ref());
                        !native_blobs.contains(digest.as_str())
                    });
            if generated {
                media.playback_asset = None;
                media.playback_mime_type = None;
                stripped += 1;
            }
        }
        stripped += strip_objects(&mut object.children, catalog, native_blobs);
    }
    stripped
}

fn asset_reference_id(value: &str) -> Option<&str> {
    let candidate = value
        .strip_prefix(ASSET_REFERENCE_PREFIX)
        .or_else(|| value.rsplit_once("/asset/").map(|(_, id)| id))?;
    (candidate.len() == 64 && candidate.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then_some(candidate)
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn visit_objects<F>(
    objects: &mut [SceneObject],
    catalog: Option<&AssetCatalog>,
    state: &mut EnrichmentState<F>,
) where
    F: FnMut(&[u8]) -> Result<Vec<u8>, String>,
{
    for object in objects {
        if let Some(media) = &mut object.media {
            enrich_media(media, catalog, state);
        }
        visit_objects(&mut object.children, catalog, state);
    }
}

fn enrich_media<F>(
    media: &mut MediaData,
    catalog: Option<&AssetCatalog>,
    state: &mut EnrichmentState<F>,
) where
    F: FnMut(&[u8]) -> Result<Vec<u8>, String>,
{
    if !requires_avi_derivative(media) || media.playback_asset.is_some() {
        return;
    }

    let known_key = media
        .source_part_name
        .as_ref()
        .and_then(|part| state.key_by_part.get(part))
        .cloned();
    let (key, input) = match known_key {
        Some(key) => (key, None),
        None => match embedded_bytes(media, catalog) {
            Ok(bytes) => {
                let key = content_key(media.source_part_name.as_deref(), &bytes);
                if let Some(part) = &media.source_part_name {
                    state.key_by_part.insert(part.clone(), key.clone());
                }
                (key, Some(bytes))
            }
            Err(error) => {
                warn_once(state, media_label(media), error);
                return;
            }
        },
    };

    if !state.by_key.contains_key(&key) {
        let result = match input {
            Some(bytes) => (state.derive)(&bytes).and_then(|output| {
                if output.is_empty() {
                    Err("ffmpeg produced an empty MP4 derivative".into())
                } else if output.len() as u64 > MAX_OUTPUT_BYTES {
                    Err(format!(
                        "MP4 derivative exceeds the {} MiB safety limit",
                        MAX_OUTPUT_BYTES / 1024 / 1024
                    ))
                } else {
                    Ok(DerivedMedia {
                        asset: format!("data:video/mp4;base64,{}", STANDARD.encode(output)),
                        mime_type: "video/mp4".into(),
                    })
                }
            }),
            None => Err("deduplication cache lost the source derivative".into()),
        };
        if result.is_ok() {
            state.report.generated += 1;
        }
        state.by_key.insert(key.clone(), result);
    }

    match state.by_key.get(&key).cloned() {
        Some(Ok(derived)) => {
            media.playback_asset = Some(derived.asset);
            media.playback_mime_type = Some(derived.mime_type);
            state.report.assigned += 1;
        }
        Some(Err(error)) => warn_once(state, media_label(media), error),
        None => warn_once(
            state,
            media_label(media),
            "missing derivative cache entry".into(),
        ),
    }
}

fn requires_avi_derivative(media: &MediaData) -> bool {
    if media.kind != MediaKind::Video {
        return false;
    }
    let mime = media
        .mime_type
        .as_deref()
        .or_else(|| media.asset.as_deref().and_then(data_uri_mime))
        .unwrap_or_default()
        .to_ascii_lowercase();
    matches!(
        mime.as_str(),
        "video/x-msvideo" | "video/avi" | "video/msvideo"
    )
}

fn embedded_bytes(media: &MediaData, catalog: Option<&AssetCatalog>) -> Result<Vec<u8>, String> {
    let asset = media
        .asset
        .as_deref()
        .ok_or_else(|| "native AVI asset is missing".to_string())?;
    if let Some(id) = asset.strip_prefix(ASSET_REFERENCE_PREFIX) {
        let catalog = catalog.ok_or_else(|| {
            "compact native AVI has no import asset catalog available".to_string()
        })?;
        let cached = catalog
            .get(id)
            .ok_or_else(|| format!("compact native AVI asset is missing: {id}"))?;
        if cached.bytes.len() > MAX_INPUT_BYTES {
            return Err(format!(
                "native AVI exceeds the {} MiB transcode safety limit",
                MAX_INPUT_BYTES / 1024 / 1024
            ));
        }
        return Ok(cached.bytes.as_ref().to_vec());
    }
    let (header, payload) = asset
        .split_once(',')
        .ok_or_else(|| "native AVI asset is not a data URI".to_string())?;
    if !header.starts_with("data:") || !header.to_ascii_lowercase().ends_with(";base64") {
        return Err("external or non-base64 AVI cannot be transcoded locally".into());
    }
    let maximum_encoded = MAX_INPUT_BYTES.saturating_mul(4) / 3 + 8;
    if payload.len() > maximum_encoded {
        return Err(format!(
            "native AVI exceeds the {} MiB transcode safety limit",
            MAX_INPUT_BYTES / 1024 / 1024
        ));
    }
    let bytes = STANDARD
        .decode(payload)
        .map_err(|error| format!("invalid native AVI data URI: {error}"))?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(format!(
            "native AVI exceeds the {} MiB transcode safety limit",
            MAX_INPUT_BYTES / 1024 / 1024
        ));
    }
    Ok(bytes)
}

fn data_uri_mime(asset: &str) -> Option<&str> {
    let header = asset.strip_prefix("data:")?.split_once(',')?.0;
    Some(header.split(';').next().unwrap_or(header))
}

fn content_key(source_part_name: Option<&str>, bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut fingerprint = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(fingerprint, "{byte:02x}");
    }
    format!(
        "{}|sha256:{fingerprint}",
        source_part_name.unwrap_or("<inline>")
    )
}

fn media_label(media: &MediaData) -> String {
    media
        .source_part_name
        .clone()
        .unwrap_or_else(|| "inline AVI".into())
}

fn warn_once<F>(state: &mut EnrichmentState<F>, label: String, error: String) {
    let warning = format!("{label}: {error}");
    if state.warned.insert(warning.clone()) {
        state.report.warnings.push(warning);
    }
}

fn transcode_h264_mp4(input: &[u8]) -> Result<Vec<u8>, String> {
    if input.len() > MAX_INPUT_BYTES {
        return Err(format!(
            "native AVI exceeds the {} MiB transcode safety limit",
            MAX_INPUT_BYTES / 1024 / 1024
        ));
    }
    let workspace = TempWorkspace::create()?;
    let input_path = workspace.path().join("source.avi");
    let output_path = workspace.path().join("playback.mp4");
    let log_path = workspace.path().join("ffmpeg.log");
    fs::write(&input_path, input).map_err(|error| format!("write AVI input: {error}"))?;
    let log = File::create(&log_path).map_err(|error| format!("create ffmpeg log: {error}"))?;
    let ffmpeg = std::env::var_os("UNIPPT_FFMPEG").unwrap_or_else(|| OsString::from("ffmpeg"));
    let mut child = Command::new(&ffmpeg)
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-y")
        .arg("-i")
        .arg(&input_path)
        .arg("-map")
        .arg("0:v:0")
        .arg("-map")
        .arg("0:a?")
        .arg("-dn")
        .arg("-c:v")
        .arg("libx264")
        .arg("-preset")
        .arg("veryfast")
        .arg("-crf")
        .arg("18")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-c:a")
        .arg("aac")
        .arg("-b:a")
        .arg("192k")
        .arg("-movflags")
        .arg("+faststart")
        .arg("-fs")
        .arg(MAX_OUTPUT_BYTES.to_string())
        .arg(&output_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::from(log))
        .spawn()
        .map_err(|error| format!("start ffmpeg ({:?}): {error}", ffmpeg))?;

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() >= TRANSCODE_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "ffmpeg exceeded the {} second transcode timeout",
                    TRANSCODE_TIMEOUT.as_secs()
                ));
            }
            Ok(None) => thread::sleep(POLL_INTERVAL),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("wait for ffmpeg: {error}"));
            }
        }
    };
    if !status.success() {
        return Err(format!(
            "ffmpeg failed with {status}: {}",
            compact_log(&log_path)
        ));
    }
    let metadata = fs::metadata(&output_path)
        .map_err(|error| format!("ffmpeg did not create an MP4 derivative: {error}"))?;
    if metadata.len() == 0 || metadata.len() > MAX_OUTPUT_BYTES {
        return Err(format!(
            "invalid MP4 derivative size: {} bytes",
            metadata.len()
        ));
    }
    fs::read(&output_path).map_err(|error| format!("read MP4 derivative: {error}"))
}

fn compact_log(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_default();
    let start = bytes.len().saturating_sub(4096);
    String::from_utf8_lossy(&bytes[start..])
        .trim()
        .replace(['\r', '\n'], " ")
}

struct TempWorkspace {
    path: PathBuf,
}

impl TempWorkspace {
    fn create() -> Result<Self, String> {
        let base = std::env::temp_dir();
        for _ in 0..16 {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = base.join(format!(
                "unippt-media-{}-{stamp}-{sequence}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(format!("create media workspace: {error}")),
            }
        }
        Err("could not allocate a unique media workspace".into())
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::io::{Cursor, Write};
    use std::sync::Arc;

    use super::*;
    use unippt_core::ImportedAsset;

    fn imported_asset(mime: &str, prefix: &str, bytes: &[u8]) -> ImportedAsset {
        let mut digest = Sha256::new();
        digest.update(prefix.as_bytes());
        digest.update(bytes);
        ImportedAsset {
            id: format!("{:x}", digest.finalize()),
            mime_type: mime.into(),
            data_uri_prefix: prefix.into(),
            bytes: Arc::from(bytes.to_vec()),
        }
    }

    fn opc_with_native_avi(bytes: &[u8]) -> OpcSnapshot {
        let cursor = Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        writer.start_file("[Content_Types].xml", options).unwrap();
        writer.write_all(b"<Types/>").unwrap();
        writer.start_file("ppt/media/media1.avi", options).unwrap();
        writer.write_all(bytes).unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        crate::opc_snapshot::explode(&bytes).unwrap()
    }

    fn avi_media() -> MediaData {
        MediaData {
            kind: MediaKind::Video,
            asset: Some("data:video/x-msvideo;base64,QUJDRA==".into()),
            mime_type: Some("video/x-msvideo".into()),
            playback_asset: None,
            playback_mime_type: None,
            source_part_name: Some("/ppt/media/media1.avi".into()),
            relationship_id: Some("rId1".into()),
            legacy_relationship_id: Some("rId2".into()),
            trim_start_ms: None,
            trim_end_ms: None,
            volume: 1.0,
            loop_playback: false,
            play_across_slides: false,
            show_when_stopped: true,
        }
    }

    fn deck_with_duplicate_avi() -> Deck {
        let mut deck = Deck::demo();
        let mut layout = deck.slides[0].objects[0].clone();
        layout.id = "layout-video".into();
        layout.media = Some(avi_media());
        let mut local = layout.clone();
        local.id = "local-video".into();
        deck.slides[0].layout_objects = vec![layout];
        deck.slides[0].objects.push(local);
        deck
    }

    #[test]
    fn keeps_native_asset_and_deduplicates_playback_derivative() {
        let calls = Cell::new(0usize);
        let mut deck = deck_with_duplicate_avi();
        let report = enrich_with(&mut deck, |bytes| {
            calls.set(calls.get() + 1);
            assert_eq!(bytes, b"ABCD");
            Ok(b"derived mp4".to_vec())
        });
        assert_eq!(calls.get(), 1);
        assert_eq!(report.generated, 1);
        assert_eq!(report.assigned, 2);
        assert!(report.warnings.is_empty());
        let first = deck.slides[0].layout_objects[0].media.as_ref().unwrap();
        let second = deck.slides[0]
            .objects
            .last()
            .unwrap()
            .media
            .as_ref()
            .unwrap();
        assert_eq!(
            first.asset.as_deref(),
            Some("data:video/x-msvideo;base64,QUJDRA==")
        );
        assert_eq!(first.playback_asset, second.playback_asset);
        assert!(first
            .playback_asset
            .as_deref()
            .is_some_and(|asset| asset.starts_with("data:video/mp4;base64,")));
        assert_eq!(first.playback_mime_type.as_deref(), Some("video/mp4"));
    }

    #[test]
    fn compact_catalog_supplies_native_media_without_materializing_the_deck() {
        let bytes: Arc<[u8]> = Arc::from(b"ABCD".to_vec());
        let prefix = "data:video/x-msvideo;base64,";
        let mut digest = Sha256::new();
        digest.update(prefix.as_bytes());
        digest.update(bytes.as_ref());
        let id = format!("{:x}", digest.finalize());
        let reference = format!("{ASSET_REFERENCE_PREFIX}{id}");
        let catalog = AssetCatalog::from_imported_assets(vec![ImportedAsset {
            id,
            mime_type: "video/x-msvideo".into(),
            data_uri_prefix: prefix.into(),
            bytes,
        }])
        .unwrap();
        let mut deck = deck_with_duplicate_avi();
        let slide = &mut deck.slides[0];
        for object in slide
            .layout_objects
            .iter_mut()
            .filter(|object| object.media.is_some())
        {
            object.media.as_mut().unwrap().asset = Some(reference.clone());
        }
        for object in slide
            .objects
            .iter_mut()
            .filter(|object| object.media.is_some())
        {
            object.media.as_mut().unwrap().asset = Some(reference.clone());
        }

        let calls = Cell::new(0usize);
        let report = enrich_decks_with_catalog(&mut [&mut deck], Some(&catalog), |input| {
            calls.set(calls.get() + 1);
            assert_eq!(input, b"ABCD");
            Ok(b"derived mp4".to_vec())
        });
        assert_eq!(calls.get(), 1);
        assert_eq!(report.generated, 1);
        assert_eq!(report.assigned, 2);
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn udoc_omits_only_regenerable_derivative_and_restores_both_decks_once() {
        let native = imported_asset("video/x-msvideo", "data:video/x-msvideo;base64,", b"ABCD");
        let derived = imported_asset("video/mp4", "data:video/mp4;base64,", b"derived mp4");
        let native_reference = format!("{ASSET_REFERENCE_PREFIX}{}", native.id);
        let derived_reference = format!("{ASSET_REFERENCE_PREFIX}{}", derived.id);
        let catalog = AssetCatalog::from_imported_assets(vec![native, derived]).unwrap();
        let opc = opc_with_native_avi(b"ABCD");

        let mut current = deck_with_duplicate_avi();
        let slide = &mut current.slides[0];
        for object in slide
            .layout_objects
            .iter_mut()
            .chain(slide.objects.iter_mut())
            .filter(|object| object.media.is_some())
        {
            let media = object.media.as_mut().unwrap();
            media.asset = Some(native_reference.clone());
            media.playback_asset = Some(derived_reference.clone());
            media.playback_mime_type = Some("video/mp4".into());
        }
        let mut baseline = current.clone();

        assert_eq!(
            strip_regenerable_playback_assets(&mut [&mut current, &mut baseline], &catalog, &opc,),
            4
        );
        for deck in [&current, &baseline] {
            for object in deck.slides[0]
                .layout_objects
                .iter()
                .chain(deck.slides[0].objects.iter())
                .filter(|object| object.media.is_some())
            {
                let media = object.media.as_ref().unwrap();
                assert_eq!(media.asset.as_deref(), Some(native_reference.as_str()));
                assert!(media.playback_asset.is_none());
                assert!(media.playback_mime_type.is_none());
            }
        }

        let calls = Cell::new(0usize);
        let report = enrich_decks_with_catalog(
            &mut [&mut current, &mut baseline],
            Some(&catalog),
            |input| {
                calls.set(calls.get() + 1);
                assert_eq!(input, b"ABCD");
                Ok(b"derived mp4".to_vec())
            },
        );
        assert_eq!(calls.get(), 1);
        assert_eq!(report.generated, 1);
        assert_eq!(report.assigned, 4);
        assert_eq!(current, baseline);
    }

    #[test]
    fn udoc_keeps_playback_asset_when_native_source_is_not_in_the_opc_snapshot() {
        let native = imported_asset(
            "video/x-msvideo",
            "data:video/x-msvideo;base64,",
            b"external AVI",
        );
        let derived = imported_asset("video/mp4", "data:video/mp4;base64,", b"user MP4");
        let native_reference = format!("{ASSET_REFERENCE_PREFIX}{}", native.id);
        let derived_reference = format!("{ASSET_REFERENCE_PREFIX}{}", derived.id);
        let catalog = AssetCatalog::from_imported_assets(vec![native, derived]).unwrap();
        let opc = opc_with_native_avi(b"different native bytes");
        let mut deck = deck_with_duplicate_avi();
        let media = deck.slides[0].layout_objects[0].media.as_mut().unwrap();
        media.asset = Some(native_reference);
        media.playback_asset = Some(derived_reference.clone());
        media.playback_mime_type = Some("video/mp4".into());

        assert_eq!(
            strip_regenerable_playback_assets(&mut [&mut deck], &catalog, &opc),
            0
        );
        assert_eq!(
            deck.slides[0].layout_objects[0]
                .media
                .as_ref()
                .unwrap()
                .playback_asset
                .as_deref(),
            Some(derived_reference.as_str())
        );
    }

    #[test]
    fn derivative_failure_is_non_fatal_and_deduplicated() {
        let mut deck = deck_with_duplicate_avi();
        let report = enrich_with(&mut deck, |_| Err("ffmpeg unavailable".into()));
        assert_eq!(report.generated, 0);
        assert_eq!(report.assigned, 0);
        assert_eq!(report.warnings.len(), 1);
        for object in deck.slides[0]
            .layout_objects
            .iter()
            .chain(deck.slides[0].objects.iter())
            .filter(|object| object.media.is_some())
        {
            let media = object.media.as_ref().unwrap();
            assert!(media.playback_asset.is_none());
            assert_eq!(
                media.asset.as_deref(),
                Some("data:video/x-msvideo;base64,QUJDRA==")
            );
        }
    }
}
