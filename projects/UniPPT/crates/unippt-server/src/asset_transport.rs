use std::collections::HashMap;
use std::sync::Arc;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use sha2::{Digest, Sha256};
use unippt_core::{Deck, ImportedAsset, SceneObject, ASSET_REFERENCE_PREFIX};

#[derive(Clone, Debug)]
pub(crate) struct CachedAsset {
    pub(crate) mime_type: Arc<str>,
    pub(crate) bytes: Arc<[u8]>,
    pub(crate) browser_mime_type: Arc<str>,
    pub(crate) browser_bytes: Arc<[u8]>,
    pub(crate) data_uri_prefix: Arc<str>,
}

impl CachedAsset {
    fn data_uri(&self) -> String {
        format!("{}{}", self.data_uri_prefix, STANDARD.encode(&self.bytes))
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct AssetCatalog {
    assets: HashMap<String, CachedAsset>,
    /// Hash of the original data-URI text -> canonical raw-asset id. This
    /// avoids decoding the same repeated base64 payload hundreds of times.
    source_uris: HashMap<[u8; 32], String>,
}

impl AssetCatalog {
    /// Build the server cache catalog directly from a compact Core import.
    /// The raw resource `Arc`s are retained, so large image/font/media payloads
    /// are not copied while ownership moves from the importer into the cache.
    pub(crate) fn from_imported_assets(assets: Vec<ImportedAsset>) -> Result<Self, String> {
        let mut catalog = Self::default();
        for asset in assets {
            catalog.insert_imported_asset(asset)?;
        }
        Ok(catalog)
    }

    pub(crate) fn get(&self, id: &str) -> Option<&CachedAsset> {
        self.assets.get(id)
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.assets.len()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&str, &CachedAsset)> {
        self.assets.iter().map(|(id, asset)| (id.as_str(), asset))
    }

    pub(crate) fn insert_original_asset(
        &mut self,
        id: &str,
        mime_type: &str,
        data_uri_prefix: &str,
        bytes: Vec<u8>,
    ) -> Result<(), String> {
        self.insert_original_asset_arc(id, mime_type, data_uri_prefix, Arc::from(bytes))
    }

    fn insert_imported_asset(&mut self, asset: ImportedAsset) -> Result<(), String> {
        self.insert_original_asset_arc(
            &asset.id,
            &asset.mime_type,
            &asset.data_uri_prefix,
            asset.bytes,
        )
    }

    pub(crate) fn insert_original_asset_arc(
        &mut self,
        id: &str,
        mime_type: &str,
        data_uri_prefix: &str,
        bytes: Arc<[u8]>,
    ) -> Result<(), String> {
        let prefix_mime = mime_from_data_uri_prefix(data_uri_prefix)
            .ok_or_else(|| "asset data URI prefix is invalid".to_string())?;
        if !prefix_mime.eq_ignore_ascii_case(mime_type) {
            return Err(format!(
                "asset MIME mismatch: index declares {mime_type}, prefix declares {prefix_mime}"
            ));
        }
        let expected_id = asset_id(data_uri_prefix, bytes.as_ref());
        if id != expected_id {
            return Err(format!("asset id SHA-256 mismatch: {id}"));
        }
        if let Some(existing) = self.assets.get(id) {
            if existing.mime_type.as_ref() != mime_type
                || existing.data_uri_prefix.as_ref() != data_uri_prefix
                || existing.bytes.as_ref() != bytes.as_ref()
            {
                return Err(format!("conflicting duplicate asset: {id}"));
            }
            return Ok(());
        }
        let (browser_mime_type, browser_bytes) = browser_projection(mime_type, &bytes);
        self.assets.insert(
            id.to_string(),
            CachedAsset {
                mime_type: Arc::from(mime_type),
                bytes,
                browser_mime_type,
                browser_bytes,
                data_uri_prefix: Arc::from(data_uri_prefix),
            },
        );
        Ok(())
    }

    pub(crate) fn estimated_bytes(&self) -> usize {
        self.assets
            .values()
            .fold(0usize, |total, asset| {
                let browser_bytes = if Arc::ptr_eq(&asset.bytes, &asset.browser_bytes) {
                    0
                } else {
                    asset.browser_bytes.len()
                };
                total
                    .saturating_add(asset.bytes.len())
                    .saturating_add(browser_bytes)
                    .saturating_add(asset.mime_type.len())
                    .saturating_add(asset.browser_mime_type.len())
                    .saturating_add(asset.data_uri_prefix.len())
            })
            .saturating_add(
                self.source_uris
                    .values()
                    .map(|id| 32usize.saturating_add(id.len()))
                    .sum::<usize>(),
            )
    }
}

/// Move browser assets out of a scene and replace every repeated data URI with
/// a short, same-origin cache URL. The original bytes stay in `catalog` once.
pub(crate) fn externalize_deck_assets(
    deck: &mut Deck,
    cache_id: &str,
    catalog: &mut AssetCatalog,
) -> Result<(), String> {
    for font in &mut deck.fonts {
        externalize_string(&mut font.data_uri, cache_id, catalog)?;
    }
    for slide in &mut deck.slides {
        externalize_optional(&mut slide.background_asset, cache_id, catalog)?;
        externalize_objects(&mut slide.master_objects, cache_id, catalog)?;
        externalize_objects(&mut slide.layout_objects, cache_id, catalog)?;
        externalize_objects(&mut slide.objects, cache_id, catalog)?;
    }
    Ok(())
}

/// Convert runtime-only `/api/cache/...` URLs into package-stable,
/// content-addressed references. Portable files must never persist a server
/// cache handle that expires when the process restarts.
pub(crate) fn portableize_deck_assets(
    deck: &mut Deck,
    cache_id: &str,
    catalog: &AssetCatalog,
) -> Result<(), String> {
    for font in &mut deck.fonts {
        portableize_string(&mut font.data_uri, cache_id, catalog)?;
    }
    for slide in &mut deck.slides {
        portableize_optional(&mut slide.background_asset, cache_id, catalog)?;
        portableize_objects(&mut slide.master_objects, cache_id, catalog)?;
        portableize_objects(&mut slide.layout_objects, cache_id, catalog)?;
        portableize_objects(&mut slide.objects, cache_id, catalog)?;
    }
    Ok(())
}

/// Recreate a portable/export scene from the compact cached representation.
/// This clones the small scene first and only expands assets for the one
/// operation that needs a self-contained payload.
pub(crate) fn materialize_deck_assets(
    compact: &Deck,
    cache_id: &str,
    catalog: &AssetCatalog,
) -> Result<Deck, String> {
    let mut deck = compact.clone();
    for font in &mut deck.fonts {
        materialize_string(&mut font.data_uri, cache_id, catalog)?;
    }
    for slide in &mut deck.slides {
        materialize_optional(&mut slide.background_asset, cache_id, catalog)?;
        materialize_objects(&mut slide.master_objects, cache_id, catalog)?;
        materialize_objects(&mut slide.layout_objects, cache_id, catalog)?;
        materialize_objects(&mut slide.objects, cache_id, catalog)?;
    }
    Ok(deck)
}

/// Expand only assets that can be written into native XML during a dirty
/// differential export. Unchanged cache URLs remain short and compare equal
/// to the compact `baseline`, avoiding a full multi-hundred-megabyte scene
/// materialization.
pub(crate) fn materialize_changed_deck_assets(
    compact: &Deck,
    baseline: &Deck,
    cache_id: &str,
    catalog: &AssetCatalog,
) -> Result<Deck, String> {
    let mut deck = compact.clone();
    for slide in &mut deck.slides {
        let original_slide = baseline.slides.iter().find(|original| {
            match (&slide.source_part_name, &original.source_part_name) {
                (Some(current), Some(candidate)) => current == candidate,
                _ => slide.id == original.id,
            }
        });
        if original_slide.is_none_or(|original| slide.background_asset != original.background_asset)
        {
            materialize_optional(&mut slide.background_asset, cache_id, catalog)?;
        }
        materialize_changed_objects(
            &mut slide.objects,
            original_slide.map(|original| original.objects.as_slice()),
            cache_id,
            catalog,
        )?;
    }
    Ok(deck)
}

fn materialize_changed_objects(
    objects: &mut [SceneObject],
    baseline: Option<&[SceneObject]>,
    cache_id: &str,
    catalog: &AssetCatalog,
) -> Result<(), String> {
    for object in objects {
        let original = baseline.and_then(|objects| {
            objects.iter().find(|candidate| {
                match (object.source_shape_id, candidate.source_shape_id) {
                    (Some(current), Some(other)) => current == other,
                    _ => object.id == candidate.id,
                }
            })
        });
        let fill_changed = original.is_none_or(|original| object.style.fill != original.style.fill);
        if fill_changed || original.is_none_or(|original| object.asset != original.asset) {
            materialize_object_asset(&mut object.asset, &mut object.style.fill, cache_id, catalog)?;
        }
        if fill_changed
            || original.is_none_or(|original| object.shape_fill_asset != original.shape_fill_asset)
        {
            materialize_object_asset(
                &mut object.shape_fill_asset,
                &mut object.style.fill,
                cache_id,
                catalog,
            )?;
        }
        if let Some(media) = &mut object.media {
            let original_media = original.and_then(|original| original.media.as_ref());
            if original_media.is_none_or(|original| media.asset != original.asset) {
                materialize_optional(&mut media.asset, cache_id, catalog)?;
            }
            if original_media.is_none_or(|original| media.playback_asset != original.playback_asset)
            {
                materialize_optional(&mut media.playback_asset, cache_id, catalog)?;
            }
        }
        materialize_changed_objects(
            &mut object.children,
            original.map(|original| original.children.as_slice()),
            cache_id,
            catalog,
        )?;
    }
    Ok(())
}

/// Rewrite content-addressed asset URLs when a portable document is opened
/// under the SHA-256 id of its embedded native PPTX. No base64 data is created.
pub(crate) fn rebind_deck_asset_cache_id(
    deck: &mut Deck,
    from_cache_id: &str,
    to_cache_id: &str,
    catalog: &AssetCatalog,
) -> Result<(), String> {
    if from_cache_id == to_cache_id {
        return Ok(());
    }
    for font in &mut deck.fonts {
        rebind_string(&mut font.data_uri, from_cache_id, to_cache_id, catalog)?;
    }
    for slide in &mut deck.slides {
        rebind_optional(
            &mut slide.background_asset,
            from_cache_id,
            to_cache_id,
            catalog,
        )?;
        rebind_objects(
            &mut slide.master_objects,
            from_cache_id,
            to_cache_id,
            catalog,
        )?;
        rebind_objects(
            &mut slide.layout_objects,
            from_cache_id,
            to_cache_id,
            catalog,
        )?;
        rebind_objects(&mut slide.objects, from_cache_id, to_cache_id, catalog)?;
    }
    Ok(())
}

fn externalize_objects(
    objects: &mut [SceneObject],
    cache_id: &str,
    catalog: &mut AssetCatalog,
) -> Result<(), String> {
    for object in objects {
        externalize_object_asset(&mut object.asset, &mut object.style.fill, cache_id, catalog)?;
        externalize_object_asset(
            &mut object.shape_fill_asset,
            &mut object.style.fill,
            cache_id,
            catalog,
        )?;
        if let Some(media) = &mut object.media {
            externalize_optional(&mut media.asset, cache_id, catalog)?;
            externalize_optional(&mut media.playback_asset, cache_id, catalog)?;
        }
        externalize_objects(&mut object.children, cache_id, catalog)?;
    }
    Ok(())
}

fn portableize_objects(
    objects: &mut [SceneObject],
    cache_id: &str,
    catalog: &AssetCatalog,
) -> Result<(), String> {
    for object in objects {
        portableize_object_asset(&mut object.asset, &mut object.style.fill, cache_id, catalog)?;
        portableize_object_asset(
            &mut object.shape_fill_asset,
            &mut object.style.fill,
            cache_id,
            catalog,
        )?;
        if let Some(media) = &mut object.media {
            portableize_optional(&mut media.asset, cache_id, catalog)?;
            portableize_optional(&mut media.playback_asset, cache_id, catalog)?;
        }
        portableize_objects(&mut object.children, cache_id, catalog)?;
    }
    Ok(())
}

fn materialize_objects(
    objects: &mut [SceneObject],
    cache_id: &str,
    catalog: &AssetCatalog,
) -> Result<(), String> {
    for object in objects {
        materialize_object_asset(&mut object.asset, &mut object.style.fill, cache_id, catalog)?;
        materialize_object_asset(
            &mut object.shape_fill_asset,
            &mut object.style.fill,
            cache_id,
            catalog,
        )?;
        if let Some(media) = &mut object.media {
            materialize_optional(&mut media.asset, cache_id, catalog)?;
            materialize_optional(&mut media.playback_asset, cache_id, catalog)?;
        }
        materialize_objects(&mut object.children, cache_id, catalog)?;
    }
    Ok(())
}

fn rebind_objects(
    objects: &mut [SceneObject],
    from_cache_id: &str,
    to_cache_id: &str,
    catalog: &AssetCatalog,
) -> Result<(), String> {
    for object in objects {
        rebind_object_asset(
            &mut object.asset,
            &mut object.style.fill,
            from_cache_id,
            to_cache_id,
            catalog,
        )?;
        rebind_object_asset(
            &mut object.shape_fill_asset,
            &mut object.style.fill,
            from_cache_id,
            to_cache_id,
            catalog,
        )?;
        if let Some(media) = &mut object.media {
            rebind_optional(&mut media.asset, from_cache_id, to_cache_id, catalog)?;
            rebind_optional(
                &mut media.playback_asset,
                from_cache_id,
                to_cache_id,
                catalog,
            )?;
        }
        rebind_objects(&mut object.children, from_cache_id, to_cache_id, catalog)?;
    }
    Ok(())
}

fn externalize_optional(
    value: &mut Option<String>,
    cache_id: &str,
    catalog: &mut AssetCatalog,
) -> Result<(), String> {
    if let Some(current) = value {
        externalize_string(current, cache_id, catalog)?;
    }
    Ok(())
}

fn portableize_optional(
    value: &mut Option<String>,
    cache_id: &str,
    catalog: &AssetCatalog,
) -> Result<(), String> {
    if let Some(current) = value {
        portableize_string(current, cache_id, catalog)?;
    }
    Ok(())
}

fn portableize_object_asset(
    value: &mut Option<String>,
    style_fill: &mut String,
    cache_id: &str,
    catalog: &AssetCatalog,
) -> Result<(), String> {
    let style_reference = value
        .as_ref()
        .filter(|current| style_fill.contains(current.as_str()))
        .cloned();
    portableize_optional(value, cache_id, catalog)?;
    if let (Some(before), Some(after)) = (style_reference, value.as_deref()) {
        *style_fill = style_fill.replace(&before, after);
    }
    Ok(())
}

fn externalize_object_asset(
    value: &mut Option<String>,
    style_fill: &mut String,
    cache_id: &str,
    catalog: &mut AssetCatalog,
) -> Result<(), String> {
    let style_reference = value
        .as_ref()
        .filter(|current| style_fill.contains(current.as_str()))
        .cloned();
    externalize_optional(value, cache_id, catalog)?;
    if let (Some(before), Some(after)) = (style_reference, value.as_deref()) {
        *style_fill = style_fill.replace(&before, after);
    }
    Ok(())
}

fn materialize_optional(
    value: &mut Option<String>,
    cache_id: &str,
    catalog: &AssetCatalog,
) -> Result<(), String> {
    if let Some(current) = value {
        materialize_string(current, cache_id, catalog)?;
    }
    Ok(())
}

fn materialize_object_asset(
    value: &mut Option<String>,
    style_fill: &mut String,
    cache_id: &str,
    catalog: &AssetCatalog,
) -> Result<(), String> {
    let style_reference = value
        .as_ref()
        .filter(|current| style_fill.contains(current.as_str()))
        .cloned();
    materialize_optional(value, cache_id, catalog)?;
    if let (Some(before), Some(after)) = (style_reference, value.as_deref()) {
        *style_fill = style_fill.replace(&before, after);
    }
    Ok(())
}

fn rebind_optional(
    value: &mut Option<String>,
    from_cache_id: &str,
    to_cache_id: &str,
    catalog: &AssetCatalog,
) -> Result<(), String> {
    if let Some(current) = value {
        rebind_string(current, from_cache_id, to_cache_id, catalog)?;
    }
    Ok(())
}

fn rebind_object_asset(
    value: &mut Option<String>,
    style_fill: &mut String,
    from_cache_id: &str,
    to_cache_id: &str,
    catalog: &AssetCatalog,
) -> Result<(), String> {
    let style_reference = value
        .as_ref()
        .filter(|current| style_fill.contains(current.as_str()))
        .cloned();
    rebind_optional(value, from_cache_id, to_cache_id, catalog)?;
    if let (Some(before), Some(after)) = (style_reference, value.as_deref()) {
        *style_fill = style_fill.replace(&before, after);
    }
    Ok(())
}

fn externalize_string(
    value: &mut String,
    cache_id: &str,
    catalog: &mut AssetCatalog,
) -> Result<bool, String> {
    if value.starts_with("/api/cache/") {
        let prefix = format!("/api/cache/{cache_id}/asset/");
        let id = value.strip_prefix(&prefix).ok_or_else(|| {
            "scene contains an asset reference from another document cache".to_string()
        })?;
        if catalog.get(id).is_none() {
            return Err(format!("cached scene asset is missing: {id}"));
        }
        return Ok(false);
    }
    if let Some(id) = value.strip_prefix(ASSET_REFERENCE_PREFIX) {
        if id.len() != 64 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err("scene contains an invalid compact asset reference".into());
        }
        if catalog.get(id).is_none() {
            return Err(format!("compact scene asset is missing: {id}"));
        }
        *value = asset_url(cache_id, id);
        return Ok(true);
    }
    let source_digest: [u8; 32] = Sha256::digest(value.as_bytes()).into();
    if let Some(id) = catalog.source_uris.get(&source_digest) {
        if catalog.get(id).is_none() {
            return Err(format!("cached scene asset is missing: {id}"));
        }
        *value = asset_url(cache_id, id);
        return Ok(true);
    }
    let Some((prefix, bytes, mime_type)) = parse_base64_data_uri(value)? else {
        return Ok(false);
    };
    let (prefix, bytes, mime_type, browser_projection) =
        canonicalize_inline_asset(prefix, bytes, mime_type);
    let id = asset_id(&prefix, &bytes);
    catalog.assets.entry(id.clone()).or_insert_with(|| {
        let bytes: Arc<[u8]> = Arc::from(bytes);
        let (browser_mime_type, browser_bytes) = browser_projection
            .map(|(mime, data)| (Arc::from(mime), Arc::from(data)))
            .unwrap_or_else(|| (Arc::from(mime_type.as_str()), Arc::clone(&bytes)));
        CachedAsset {
            mime_type: Arc::from(mime_type),
            bytes,
            browser_mime_type,
            browser_bytes,
            data_uri_prefix: Arc::from(prefix),
        }
    });
    catalog.source_uris.insert(source_digest, id.clone());
    *value = asset_url(cache_id, &id);
    Ok(true)
}

fn materialize_string(
    value: &mut String,
    cache_id: &str,
    catalog: &AssetCatalog,
) -> Result<bool, String> {
    let prefix = format!("/api/cache/{cache_id}/asset/");
    let Some(id) = value.strip_prefix(&prefix) else {
        if value.starts_with("/api/cache/") && value.contains("/asset/") {
            return Err("scene contains an asset reference from another document cache".into());
        }
        return Ok(false);
    };
    let asset = catalog
        .get(id)
        .ok_or_else(|| format!("cached scene asset is missing: {id}"))?;
    *value = asset.data_uri();
    Ok(true)
}

fn portableize_string(
    value: &mut String,
    cache_id: &str,
    catalog: &AssetCatalog,
) -> Result<bool, String> {
    if let Some(id) = value.strip_prefix(ASSET_REFERENCE_PREFIX) {
        if id.len() != 64 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err("scene contains an invalid compact asset reference".into());
        }
        if catalog.get(id).is_none() {
            return Err(format!("compact scene asset is missing: {id}"));
        }
        return Ok(false);
    }
    let prefix = format!("/api/cache/{cache_id}/asset/");
    let Some(id) = value.strip_prefix(&prefix) else {
        if value.starts_with("/api/cache/") && value.contains("/asset/") {
            return Err("scene contains an asset reference from another document cache".into());
        }
        return Ok(false);
    };
    if catalog.get(id).is_none() {
        return Err(format!("cached scene asset is missing: {id}"));
    }
    *value = format!("{ASSET_REFERENCE_PREFIX}{id}");
    Ok(true)
}

fn rebind_string(
    value: &mut String,
    from_cache_id: &str,
    to_cache_id: &str,
    catalog: &AssetCatalog,
) -> Result<bool, String> {
    if let Some(id) = value.strip_prefix(ASSET_REFERENCE_PREFIX) {
        if id.len() != 64 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err("scene contains an invalid compact asset reference".into());
        }
        if catalog.get(id).is_none() {
            return Err(format!("compact scene asset is missing: {id}"));
        }
        *value = asset_url(to_cache_id, id);
        return Ok(true);
    }
    let prefix = format!("/api/cache/{from_cache_id}/asset/");
    let Some(id) = value.strip_prefix(&prefix) else {
        if value.starts_with("/api/cache/") && value.contains("/asset/") {
            return Err("scene contains an asset reference from another document cache".into());
        }
        return Ok(false);
    };
    if catalog.get(id).is_none() {
        return Err(format!("cached scene asset is missing: {id}"));
    }
    *value = asset_url(to_cache_id, id);
    Ok(true)
}

fn parse_base64_data_uri(value: &str) -> Result<Option<(String, Vec<u8>, String)>, String> {
    if !value.starts_with("data:") {
        return Ok(None);
    }
    let Some((header, payload)) = value.split_once(',') else {
        return Err("data URI has no payload separator".into());
    };
    if !header
        .split(';')
        .skip(1)
        .any(|parameter| parameter.eq_ignore_ascii_case("base64"))
    {
        return Ok(None);
    }
    let mime_type = header
        .strip_prefix("data:")
        .and_then(|rest| rest.split(';').next())
        .filter(|mime| !mime.is_empty())
        .unwrap_or("application/octet-stream")
        .to_string();
    let bytes = STANDARD
        .decode(payload)
        .map_err(|error| format!("invalid base64 data URI: {error}"))?;
    Ok(Some((format!("{header},"), bytes, mime_type)))
}

fn mime_from_data_uri_prefix(prefix: &str) -> Option<&str> {
    let header = prefix.strip_suffix(',')?;
    if !header
        .split(';')
        .skip(1)
        .any(|parameter| parameter.eq_ignore_ascii_case("base64"))
    {
        return None;
    }
    header
        .strip_prefix("data:")?
        .split(';')
        .next()
        .filter(|mime| !mime.is_empty())
}

fn asset_id(data_uri_prefix: &str, bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(data_uri_prefix.as_bytes());
    digest.update(bytes);
    hex_digest(&digest.finalize())
}

fn asset_url(cache_id: &str, asset_id: &str) -> String {
    format!("/api/cache/{cache_id}/asset/{asset_id}")
}

fn browser_projection(mime_type: &str, bytes: &Arc<[u8]>) -> (Arc<str>, Arc<[u8]>) {
    if matches!(
        mime_type.to_ascii_lowercase().as_str(),
        "image/x-emf" | "image/emf" | "application/x-emf" | "application/emf"
    ) {
        if let Ok(svg) = emf2svg::emf_to_svg_with(bytes, emf2svg::Emf2SvgOptions { lossless: true })
        {
            return (Arc::from("image/svg+xml"), Arc::from(svg.into_bytes()));
        }
    }
    (Arc::from(mime_type), Arc::clone(bytes))
}

/// Canonicalize newly inserted SVG to a native EMF payload while keeping the
/// exact SVG bytes as the browser projection. `vecmeta` writes the SVG into a
/// private EMF comment in lossless mode, so opening the resulting PPTX/UDoc
/// can recover the source SVG byte-for-byte. Existing imported PPTX assets do
/// not pass through this function and therefore retain their original OPC
/// bytes unchanged.
#[allow(clippy::type_complexity)]
fn canonicalize_inline_asset(
    prefix: String,
    bytes: Vec<u8>,
    mime_type: String,
) -> (String, Vec<u8>, String, Option<(String, Vec<u8>)>) {
    if mime_type.eq_ignore_ascii_case("image/svg+xml") {
        if let Ok(svg) = std::str::from_utf8(&bytes) {
            let options = svg2emf::EmitOptions {
                lossless: true,
                ..svg2emf::EmitOptions::default()
            };
            if let Ok(emf) = svg2emf::svg_to_emf(svg, options) {
                return (
                    "data:image/x-emf;base64,".into(),
                    emf,
                    "image/x-emf".into(),
                    Some(("image/svg+xml".into(), bytes)),
                );
            }
        }
    }
    (prefix, bytes, mime_type, None)
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserted_svg_uses_lossless_emf_native_bytes_and_exact_browser_projection() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="80"><defs><linearGradient id="g"><stop stop-color="#f00"/><stop offset="1" stop-color="#00f"/></linearGradient></defs><rect x="4" y="5" width="110" height="70" rx="8" fill="url(#g)"/></svg>"##;
        let (prefix, emf, mime, browser) = canonicalize_inline_asset(
            "data:image/svg+xml;base64,".into(),
            svg.to_vec(),
            "image/svg+xml".into(),
        );
        assert_eq!(prefix, "data:image/x-emf;base64,");
        assert_eq!(mime, "image/x-emf");
        assert!(emf.len() > svg.len());
        let (browser_mime, browser_bytes) = browser.expect("SVG keeps a browser projection");
        assert_eq!(browser_mime, "image/svg+xml");
        assert_eq!(browser_bytes, svg);
        let restored =
            emf2svg::emf_to_svg_with(&emf, emf2svg::Emf2SvgOptions { lossless: true }).unwrap();
        assert_eq!(restored.as_bytes(), svg);
        let rebuilt = svg2emf::svg_to_emf(
            &restored,
            svg2emf::EmitOptions {
                lossless: true,
                ..svg2emf::EmitOptions::default()
            },
        )
        .unwrap();
        assert_eq!(rebuilt, emf);
    }

    #[test]
    fn svg_survives_native_pptx_as_emf_and_recovers_exact_source() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="48"><path d="M2 2L62 2L32 46Z" fill="#f36" stroke="#123"/></svg>"##;
        let mut deck = Deck::demo();
        let image = &mut deck.slides[0].objects[0];
        image.kind = unippt_core::ObjectKind::Image;
        image.name = "lossless-vector.svg".into();
        image.text.clear();
        image.text_paragraphs.clear();
        image.asset = Some(format!(
            "data:image/svg+xml;base64,{}",
            STANDARD.encode(svg)
        ));
        let mut catalog = AssetCatalog::default();
        externalize_deck_assets(&mut deck, "pptx-vector", &mut catalog).unwrap();
        let native = materialize_deck_assets(&deck, "pptx-vector", &catalog).unwrap();
        let pptx = unippt_core::export_pptx(&native, None).unwrap();
        let imported = unippt_core::import_pptx_compact(&pptx).unwrap();
        let catalog = AssetCatalog::from_imported_assets(imported.assets).unwrap();
        let vector = catalog
            .iter()
            .map(|(_, asset)| asset)
            .find(|asset| asset.mime_type.as_ref() == "image/x-emf")
            .expect("exported PPTX contains the native EMF asset");
        assert_eq!(vector.browser_mime_type.as_ref(), "image/svg+xml");
        assert_eq!(vector.browser_bytes.as_ref(), svg);
    }

    #[test]
    fn native_and_playback_media_references_share_one_content_addressed_asset() {
        let payload = format!(
            "data:audio/mpeg;base64,{}",
            STANDARD.encode(vec![0x5a; 32 * 1024])
        );
        let mut deck = Deck::demo();
        deck.slides[0].objects[0].media = Some(unippt_core::MediaData {
            kind: unippt_core::MediaKind::Audio,
            asset: Some(payload.clone()),
            mime_type: Some("audio/mpeg".into()),
            playback_asset: Some(payload),
            playback_mime_type: Some("audio/mpeg".into()),
            source_part_name: None,
            relationship_id: None,
            legacy_relationship_id: None,
            trim_start_ms: None,
            trim_end_ms: None,
            volume: 1.0,
            loop_playback: false,
            play_across_slides: false,
            show_when_stopped: true,
        });
        let mut catalog = AssetCatalog::default();
        externalize_deck_assets(&mut deck, "media", &mut catalog).unwrap();
        assert_eq!(catalog.len(), 1);
        let media = deck.slides[0].objects[0].media.as_ref().unwrap();
        assert_eq!(media.asset, media.playback_asset);
        assert!(media
            .asset
            .as_deref()
            .unwrap()
            .starts_with("/api/cache/media/asset/"));
    }

    #[test]
    fn core_compact_assets_bind_without_copying_or_base64_decoding() {
        let bytes: Arc<[u8]> = Arc::from(vec![3; 128 * 1024]);
        let prefix = "data:image/png;base64,";
        let id = asset_id(prefix, bytes.as_ref());
        let reference = format!("{ASSET_REFERENCE_PREFIX}{id}");
        let imported = ImportedAsset {
            id: id.clone(),
            mime_type: "image/png".into(),
            data_uri_prefix: prefix.into(),
            bytes: Arc::clone(&bytes),
        };
        let mut catalog = AssetCatalog::from_imported_assets(vec![imported]).unwrap();
        assert!(Arc::ptr_eq(&bytes, &catalog.get(&id).unwrap().bytes));

        let mut deck = Deck::demo();
        deck.slides[0].objects[0].asset = Some(reference.clone());
        deck.slides[0].objects[0].style.fill = format!(r#"url("{reference}")"#);
        externalize_deck_assets(&mut deck, "compact", &mut catalog).unwrap();
        let compact_json = serde_json::to_string(&deck).unwrap();
        assert!(!compact_json.contains(ASSET_REFERENCE_PREFIX));
        assert!(!compact_json.contains("data:image/png"));
        assert!(compact_json.contains("/api/cache/compact/asset/"));

        let materialized = materialize_deck_assets(&deck, "compact", &catalog).unwrap();
        let expected = format!("{prefix}{}", STANDARD.encode(bytes.as_ref()));
        assert_eq!(
            materialized.slides[0].objects[0].asset.as_deref(),
            Some(expected.as_str())
        );
        assert!(materialized.slides[0].objects[0]
            .style
            .fill
            .contains(&expected));
    }

    #[test]
    fn repeated_assets_are_stored_once_and_round_trip() {
        let payload = format!(
            "data:image/png;base64,{}",
            STANDARD.encode(vec![7; 128 * 1024])
        );
        let mut deck = Deck::demo();
        deck.source_import_id = Some("pptx-test".into());
        deck.slides[0].background_asset = Some(payload.clone());
        deck.slides[0].objects[0].asset = Some(payload.clone());
        deck.slides[0].objects[0].shape_fill_asset = Some(payload.clone());
        deck.slides[0].objects[0].style.fill = format!("url(\"{payload}\") no-repeat");
        let original = deck.clone();
        let original_json = serde_json::to_vec(&deck).unwrap();
        let mut catalog = AssetCatalog::default();

        externalize_deck_assets(&mut deck, "pptx-test", &mut catalog).unwrap();
        let compact_json = serde_json::to_vec(&deck).unwrap();
        assert_eq!(catalog.len(), 1);
        assert!(compact_json.len() * 4 < original_json.len());
        assert!(deck.slides[0].objects[0]
            .style
            .fill
            .contains("/api/cache/pptx-test/asset/"));

        let restored = materialize_deck_assets(&deck, "pptx-test", &catalog).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    fn foreign_cache_asset_reference_is_rejected() {
        let mut deck = Deck::demo();
        deck.slides[0].objects[0].asset = Some("/api/cache/other/asset/0123456789abcdef".into());
        let error =
            materialize_deck_assets(&deck, "current", &AssetCatalog::default()).unwrap_err();
        assert!(error.contains("another document cache"));
    }

    #[test]
    fn compact_asset_urls_rebind_without_materializing_base64() {
        let payload = format!(
            "data:image/png;base64,{}",
            STANDARD.encode(vec![9; 64 * 1024])
        );
        let mut deck = Deck::demo();
        deck.slides[0].objects[0].asset = Some(payload.clone());
        deck.slides[0].objects[0].style.fill = format!(r#"url("{payload}")"#);
        let original = deck.clone();
        let mut catalog = AssetCatalog::default();
        externalize_deck_assets(&mut deck, "old", &mut catalog).unwrap();

        rebind_deck_asset_cache_id(&mut deck, "old", "new", &catalog).unwrap();
        let json = serde_json::to_string(&deck).unwrap();
        assert!(json.contains("/api/cache/new/asset/"));
        assert!(!json.contains("data:image/png"));
        assert_eq!(
            materialize_deck_assets(&deck, "new", &catalog).unwrap(),
            original
        );
    }

    #[test]
    fn dirty_export_expands_only_changed_assets() {
        let first = format!("data:image/png;base64,{}", STANDARD.encode(vec![1; 4096]));
        let second = format!("data:image/png;base64,{}", STANDARD.encode(vec![2; 4096]));
        let mut baseline = Deck::demo();
        baseline.slides[0].objects[0].asset = Some(first.clone());
        baseline.slides[0].objects[0].style.fill = format!(r#"url("{first}")"#);
        let mut catalog = AssetCatalog::default();
        externalize_deck_assets(&mut baseline, "pptx-test", &mut catalog).unwrap();

        let mut text_edit = baseline.clone();
        text_edit.slides[0].objects[0].text.push_str(" edited");
        let prepared =
            materialize_changed_deck_assets(&text_edit, &baseline, "pptx-test", &catalog).unwrap();
        assert!(prepared.slides[0].objects[0]
            .asset
            .as_deref()
            .unwrap()
            .starts_with("/api/cache/"));

        let mut image_edit = baseline.clone();
        image_edit.slides[0].objects[0].asset = Some(second.clone());
        image_edit.slides[0].objects[0].style.fill = format!(r#"url("{second}")"#);
        externalize_deck_assets(&mut image_edit, "pptx-test", &mut catalog).unwrap();
        let prepared =
            materialize_changed_deck_assets(&image_edit, &baseline, "pptx-test", &catalog).unwrap();
        assert_eq!(
            prepared.slides[0].objects[0].asset.as_deref(),
            Some(second.as_str())
        );
        assert!(prepared.slides[0].objects[0]
            .style
            .fill
            .contains("data:image/png"));
    }
}
