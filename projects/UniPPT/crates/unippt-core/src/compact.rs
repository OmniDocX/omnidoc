use std::collections::HashMap;
use std::sync::Arc;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use sha2::{Digest, Sha256};

use crate::{Deck, SceneObject};

pub const ASSET_REFERENCE_PREFIX: &str = "unippt-asset:";

/// One unique binary resource discovered while importing a PPTX package.
///
/// `bytes` is reference counted so the server can move the compact import into
/// its document cache without copying a large image, font, audio, or video
/// payload. The id is the lowercase SHA-256 of `data_uri_prefix || bytes`.
#[derive(Debug, Clone)]
pub struct ImportedAsset {
    pub id: String,
    pub mime_type: String,
    pub data_uri_prefix: String,
    pub bytes: Arc<[u8]>,
}

/// PPTX import result whose scene references binary resources by content id.
///
/// Unlike [`crate::import_pptx`], serializing `deck` does not duplicate base64
/// strings for every shape. Consumers that need a self-contained legacy deck
/// can call [`CompactImport::into_inline_deck`].
#[derive(Debug, Clone)]
pub struct CompactImport {
    pub deck: Deck,
    pub assets: Vec<ImportedAsset>,
}

impl CompactImport {
    /// Materialize every compact resource reference as a browser data URI.
    /// This preserves the historic `import_pptx` contract.
    pub fn into_inline_deck(mut self) -> Deck {
        let encoded = self
            .assets
            .into_iter()
            .map(|asset| {
                let reference = format!("{ASSET_REFERENCE_PREFIX}{}", asset.id);
                let data_uri = format!(
                    "{}{}",
                    asset.data_uri_prefix,
                    STANDARD.encode(asset.bytes.as_ref())
                );
                (reference, data_uri)
            })
            .collect::<HashMap<_, _>>();
        materialize_deck_assets(&mut self.deck, &encoded);
        self.deck
    }
}

/// Import-local, content-addressed asset collector.
///
/// The collector hashes borrowed bytes first and allocates an `Arc` only for a
/// new payload. Repeated references therefore remain small strings throughout
/// scene construction and never trigger duplicate base64 encoding.
#[derive(Debug, Default)]
pub(crate) struct AssetSink {
    by_id: HashMap<String, usize>,
    assets: Vec<ImportedAsset>,
    shared_by_allocation: HashMap<(usize, usize), Vec<SharedAssetIdentity>>,
    #[cfg(test)]
    inline: bool,
    #[cfg(test)]
    identity_hashes: usize,
}

#[derive(Debug)]
struct SharedAssetIdentity {
    mime_type: String,
    reference: String,
    /// Retain the allocation so a later Arc cannot reuse the pointer while
    /// this import-local identity entry is alive.
    bytes: Arc<[u8]>,
}

impl AssetSink {
    pub(crate) fn insert(&mut self, mime_type: &str, bytes: &[u8]) -> String {
        #[cfg(test)]
        if self.inline {
            return format!("data:{mime_type};base64,{}", STANDARD.encode(bytes));
        }
        let (id, data_uri_prefix) = self.asset_identity(mime_type, bytes);
        self.insert_new(id, mime_type, data_uri_prefix, || Arc::from(bytes))
    }

    pub(crate) fn insert_shared(&mut self, mime_type: &str, bytes: Arc<[u8]>) -> String {
        #[cfg(test)]
        if self.inline {
            return format!(
                "data:{mime_type};base64,{}",
                STANDARD.encode(bytes.as_ref())
            );
        }
        let allocation = (bytes.as_ref().as_ptr() as usize, bytes.len());
        if let Some(reference) = self
            .shared_by_allocation
            .get(&allocation)
            .and_then(|entries| {
                entries
                    .iter()
                    .find(|entry| entry.mime_type == mime_type && Arc::ptr_eq(&entry.bytes, &bytes))
            })
            .map(|entry| entry.reference.clone())
        {
            return reference;
        }

        let (id, data_uri_prefix) = self.asset_identity(mime_type, bytes.as_ref());
        let reference = self.insert_new(id, mime_type, data_uri_prefix, || Arc::clone(&bytes));
        self.shared_by_allocation
            .entry(allocation)
            .or_default()
            .push(SharedAssetIdentity {
                mime_type: mime_type.to_string(),
                reference: reference.clone(),
                bytes,
            });
        reference
    }

    pub(crate) fn insert_owned(&mut self, mime_type: &str, bytes: Vec<u8>) -> String {
        #[cfg(test)]
        if self.inline {
            return format!("data:{mime_type};base64,{}", STANDARD.encode(&bytes));
        }
        let (id, data_uri_prefix) = self.asset_identity(mime_type, &bytes);
        self.insert_new(id, mime_type, data_uri_prefix, || Arc::from(bytes))
    }

    fn asset_identity(&mut self, mime_type: &str, bytes: &[u8]) -> (String, String) {
        #[cfg(test)]
        {
            self.identity_hashes += 1;
        }
        asset_identity(mime_type, bytes)
    }

    fn insert_new(
        &mut self,
        id: String,
        mime_type: &str,
        data_uri_prefix: String,
        bytes: impl FnOnce() -> Arc<[u8]>,
    ) -> String {
        if !self.by_id.contains_key(&id) {
            let index = self.assets.len();
            self.assets.push(ImportedAsset {
                id: id.clone(),
                mime_type: mime_type.to_string(),
                data_uri_prefix,
                bytes: bytes(),
            });
            self.by_id.insert(id.clone(), index);
        }
        format!("{ASSET_REFERENCE_PREFIX}{id}")
    }

    pub(crate) fn finish(self, deck: Deck) -> CompactImport {
        CompactImport {
            deck,
            assets: self.assets,
        }
    }

    #[cfg(test)]
    pub(crate) fn inline() -> Self {
        Self {
            inline: true,
            ..Self::default()
        }
    }
}

fn asset_identity(mime_type: &str, bytes: &[u8]) -> (String, String) {
    let data_uri_prefix = format!("data:{mime_type};base64,");
    let mut digest = Sha256::new();
    digest.update(data_uri_prefix.as_bytes());
    digest.update(bytes);
    (format!("{:x}", digest.finalize()), data_uri_prefix)
}

fn materialize_deck_assets(deck: &mut Deck, assets: &HashMap<String, String>) {
    for font in &mut deck.fonts {
        materialize_value(&mut font.data_uri, assets);
    }
    for slide in &mut deck.slides {
        materialize_optional(&mut slide.background_asset, assets);
        materialize_objects(&mut slide.master_objects, assets);
        materialize_objects(&mut slide.layout_objects, assets);
        materialize_objects(&mut slide.objects, assets);
    }
}

fn materialize_objects(objects: &mut [SceneObject], assets: &HashMap<String, String>) {
    for object in objects {
        materialize_optional(&mut object.asset, assets);
        materialize_optional(&mut object.shape_fill_asset, assets);
        materialize_value(&mut object.style.fill, assets);
        if let Some(media) = &mut object.media {
            materialize_optional(&mut media.asset, assets);
            materialize_optional(&mut media.playback_asset, assets);
        }
        materialize_objects(&mut object.children, assets);
    }
}

fn materialize_optional(value: &mut Option<String>, assets: &HashMap<String, String>) {
    if let Some(value) = value {
        materialize_value(value, assets);
    }
}

fn materialize_value(value: &mut String, assets: &HashMap<String, String>) {
    let Some(first) = value.find(ASSET_REFERENCE_PREFIX) else {
        return;
    };
    if first == 0 {
        if let Some(data_uri) = assets.get(value) {
            value.clone_from(data_uri);
            return;
        }
    }

    let mut cursor = 0usize;
    let mut output = String::with_capacity(value.len());
    while let Some(relative) = value[cursor..].find(ASSET_REFERENCE_PREFIX) {
        let start = cursor + relative;
        output.push_str(&value[cursor..start]);
        let end = (start + ASSET_REFERENCE_PREFIX.len() + 64).min(value.len());
        let reference = &value[start..end];
        if let Some(data_uri) = assets.get(reference) {
            output.push_str(data_uri);
            cursor = end;
        } else {
            output.push_str(ASSET_REFERENCE_PREFIX);
            cursor = start + ASSET_REFERENCE_PREFIX.len();
        }
    }
    output.push_str(&value[cursor..]);
    *value = output;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sink_deduplicates_and_hashes_prefix_with_payload() {
        let mut sink = AssetSink::default();
        let first = sink.insert("image/png", &[1, 2, 3]);
        let duplicate = sink.insert("image/png", &[1, 2, 3]);
        let other_mime = sink.insert("image/jpeg", &[1, 2, 3]);

        assert_eq!(first, duplicate);
        assert_ne!(first, other_mime);
        assert_eq!(sink.assets.len(), 2);
        assert!(first.starts_with(ASSET_REFERENCE_PREFIX));
        assert_eq!(first.len(), ASSET_REFERENCE_PREFIX.len() + 64);
    }

    #[test]
    fn shared_arc_identity_is_hashed_only_once_per_mime() {
        let mut sink = AssetSink::default();
        let shared: Arc<[u8]> = vec![9; 1_048_576].into();

        let first = sink.insert_shared("image/png", Arc::clone(&shared));
        assert_eq!(sink.identity_hashes, 1);
        let duplicate = sink.insert_shared("image/png", Arc::clone(&shared));
        assert_eq!(first, duplicate);
        assert_eq!(sink.identity_hashes, 1);

        // Equal content in another allocation still needs one identity hash,
        // while content addressing prevents a duplicate catalog payload.
        let equal_but_distinct: Arc<[u8]> = vec![9; 1_048_576].into();
        assert_eq!(first, sink.insert_shared("image/png", equal_but_distinct));
        assert_eq!(sink.identity_hashes, 2);
        assert_eq!(sink.assets.len(), 1);

        let other_mime = sink.insert_shared("image/jpeg", Arc::clone(&shared));
        assert_ne!(first, other_mime);
        assert_eq!(sink.identity_hashes, 3);
    }

    #[test]
    fn materializes_reference_inside_css_fill() {
        let mut sink = AssetSink::default();
        let reference = sink.insert("image/png", &[1, 2, 3, 4]);
        let assets = sink
            .assets
            .into_iter()
            .map(|asset| {
                (
                    format!("{ASSET_REFERENCE_PREFIX}{}", asset.id),
                    format!(
                        "{}{}",
                        asset.data_uri_prefix,
                        STANDARD.encode(asset.bytes.as_ref())
                    ),
                )
            })
            .collect();
        let mut css = format!(r#"url("{reference}") center / cover no-repeat"#);
        materialize_value(&mut css, &assets);
        assert_eq!(
            css,
            r#"url("data:image/png;base64,AQIDBA==") center / cover no-repeat"#
        );
    }
}
