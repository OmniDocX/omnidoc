use std::collections::VecDeque;
use std::sync::Arc;

use serde::Serialize;
use sha2::{Digest, Sha256};
use unippt_core::Deck;

use crate::asset_transport::AssetCatalog;
use crate::opc_snapshot::OpcSnapshot;

const DEFAULT_MAX_DOCUMENTS: usize = 4;
const DEFAULT_MAX_MIB: usize = 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ArtifactKind {
    Pptx,
    UdocJson,
    Udoc,
    Html,
    Video,
    VideoFast,
}

#[derive(Clone)]
pub(crate) struct CachedDocument {
    pub(crate) id: String,
    /// Authoritative native package representation. The cache deliberately
    /// does not retain a second complete PPTX byte stream.
    pub(crate) opc: Arc<OpcSnapshot>,
    pub(crate) deck: Arc<Deck>,
    /// Immutable compact scene imported from the source PPTX. Dirty exports
    /// use it as their differential baseline instead of reparsing `source`.
    pub(crate) original_deck: Arc<Deck>,
    pub(crate) deck_json: Arc<[u8]>,
    pub(crate) assets: Arc<AssetCatalog>,
    pub(crate) revision: u64,
    pub(crate) dirty: bool,
}

struct Entry {
    document: CachedDocument,
    original_deck: Arc<Deck>,
    original_assets: Arc<AssetCatalog>,
    digest: [u8; 32],
    base_bytes: usize,
    original_base_bytes: usize,
    revision: u64,
    dirty: bool,
    pptx: Option<Arc<[u8]>>,
    udoc_json: Option<Arc<[u8]>>,
    udoc: Option<Arc<[u8]>>,
    html: Option<Arc<[u8]>>,
    video: Option<Arc<[u8]>>,
    video_fast: Option<Arc<[u8]>>,
}

impl Entry {
    fn snapshot(&self) -> CachedDocument {
        let mut document = self.document.clone();
        document.revision = self.revision;
        document.dirty = self.dirty;
        document
    }

    fn estimated_bytes(&self) -> usize {
        self.base_bytes
            .saturating_add(self.pptx.as_ref().map_or(0, |bytes| bytes.len()))
            .saturating_add(self.udoc_json.as_ref().map_or(0, |bytes| bytes.len()))
            .saturating_add(self.udoc.as_ref().map_or(0, |bytes| bytes.len()))
            .saturating_add(self.html.as_ref().map_or(0, |bytes| bytes.len()))
            .saturating_add(self.video.as_ref().map_or(0, |bytes| bytes.len()))
            .saturating_add(self.video_fast.as_ref().map_or(0, |bytes| bytes.len()))
    }

    fn restore_original(&mut self) {
        if !self.dirty && self.revision == 0 {
            return;
        }
        self.document.deck = Arc::clone(&self.original_deck);
        self.document.assets = Arc::clone(&self.original_assets);
        self.base_bytes = self.original_base_bytes;
        self.revision = 0;
        self.dirty = false;
        self.pptx = None;
        self.udoc_json = None;
        self.udoc = None;
        self.html = None;
        self.video = None;
        self.video_fast = None;
    }

    fn artifact(&self, kind: ArtifactKind) -> Option<Arc<[u8]>> {
        match kind {
            ArtifactKind::Pptx => self.pptx.clone(),
            ArtifactKind::UdocJson => self.udoc_json.clone(),
            ArtifactKind::Udoc => self.udoc.clone(),
            ArtifactKind::Html => self.html.clone(),
            ArtifactKind::Video => self.video.clone(),
            ArtifactKind::VideoFast => self.video_fast.clone(),
        }
    }

    fn replace_artifact(&mut self, kind: ArtifactKind, bytes: Arc<[u8]>) {
        match kind {
            ArtifactKind::Pptx => self.pptx = Some(bytes),
            ArtifactKind::UdocJson => self.udoc_json = Some(bytes),
            ArtifactKind::Udoc => self.udoc = Some(bytes),
            ArtifactKind::Html => self.html = Some(bytes),
            ArtifactKind::Video => self.video = Some(bytes),
            ArtifactKind::VideoFast => self.video_fast = Some(bytes),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CacheStats {
    pub(crate) documents: usize,
    pub(crate) estimated_bytes: usize,
    pub(crate) max_documents: usize,
    pub(crate) max_bytes: usize,
}

/// Bounded LRU cache for an imported presentation and its expensive derived
/// representations. The server wraps this value in `Arc<Mutex<_>>`; values
/// returned to request handlers are `Arc`s so eviction never invalidates an
/// in-flight response.
pub(crate) struct DocumentCache {
    entries: VecDeque<Entry>,
    max_documents: usize,
    max_bytes: usize,
}

impl DocumentCache {
    pub(crate) fn from_env() -> Self {
        let max_documents = std::env::var("UNIPPT_CACHE_MAX_DOCUMENTS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_MAX_DOCUMENTS);
        let max_mib = std::env::var("UNIPPT_CACHE_MAX_MIB")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_MAX_MIB);
        Self::new(
            max_documents,
            max_mib.saturating_mul(1024).saturating_mul(1024),
        )
    }

    pub(crate) fn new(max_documents: usize, max_bytes: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            max_documents: max_documents.max(1),
            max_bytes: max_bytes.max(1),
        }
    }

    pub(crate) fn digest(source: &[u8]) -> [u8; 32] {
        Sha256::digest(source).into()
    }

    pub(crate) fn id_for_digest(digest: &[u8; 32]) -> String {
        let mut id = String::with_capacity(69);
        id.push_str("pptx-");
        for byte in digest {
            use std::fmt::Write;
            let _ = write!(id, "{byte:02x}");
        }
        id
    }

    pub(crate) fn get_by_digest(&mut self, digest: &[u8; 32]) -> Option<CachedDocument> {
        let index = self
            .entries
            .iter()
            .position(|entry| &entry.digest == digest)?;
        let mut entry = self.entries.remove(index)?;
        // Reopening the same source is an explicit request for the imported
        // source snapshot, not for a previous browser editing session.
        entry.restore_original();
        let document = entry.snapshot();
        self.entries.push_back(entry);
        Some(document)
    }

    pub(crate) fn get(&mut self, id: &str) -> Option<CachedDocument> {
        let index = self
            .entries
            .iter()
            .position(|entry| entry.document.id == id)?;
        let entry = self.entries.remove(index)?;
        let document = entry.snapshot();
        self.entries.push_back(entry);
        Some(document)
    }

    pub(crate) fn insert(
        &mut self,
        digest: [u8; 32],
        opc: Arc<OpcSnapshot>,
        deck: Arc<Deck>,
        original_deck: Arc<Deck>,
        deck_json: Arc<[u8]>,
        assets: Arc<AssetCatalog>,
    ) -> Result<CachedDocument, String> {
        let id = Self::id_for_digest(&digest);
        let base_bytes = opc
            .estimated_bytes()
            // `Deck` owns essentially the same strings as its JSON projection.
            // Count that projection once more to conservatively account for it.
            .saturating_add(deck_json.len().saturating_mul(2))
            .saturating_add(assets.estimated_bytes());
        if base_bytes > self.max_bytes {
            return Err(format!(
                "document needs approximately {base_bytes} cache bytes, limit is {}",
                self.max_bytes
            ));
        }

        if let Some(index) = self
            .entries
            .iter()
            .position(|entry| entry.document.id == id)
        {
            self.entries.remove(index);
        }
        while self.entries.len() >= self.max_documents
            || self.estimated_bytes().saturating_add(base_bytes) > self.max_bytes
        {
            if self.entries.pop_front().is_none() {
                break;
            }
        }

        let dirty = deck.as_ref() != original_deck.as_ref();
        let document = CachedDocument {
            id,
            opc,
            original_deck: Arc::clone(&original_deck),
            deck,
            deck_json,
            assets,
            revision: 0,
            dirty,
        };
        self.entries.push_back(Entry {
            original_deck,
            original_assets: Arc::clone(&document.assets),
            document: document.clone(),
            digest,
            base_bytes,
            original_base_bytes: base_bytes,
            revision: 0,
            dirty,
            pptx: None,
            udoc_json: None,
            udoc: None,
            html: None,
            video: None,
            video_fast: None,
        });
        Ok(document)
    }

    /// Replaces the working scene after the browser sends one dirty revision.
    /// The original import JSON remains untouched so importing the same source
    /// again still restores the source snapshot rather than an edited session.
    pub(crate) fn update_deck(
        &mut self,
        id: &str,
        client_revision: u64,
        deck: Arc<Deck>,
        estimated_deck_bytes: usize,
        assets: Arc<AssetCatalog>,
    ) -> Result<CachedDocument, String> {
        let Some(index) = self
            .entries
            .iter()
            .position(|entry| entry.document.id == id)
        else {
            return Err("cached document expired".into());
        };
        let mut entry = self
            .entries
            .remove(index)
            .expect("entry index remains valid");
        if client_revision == entry.revision {
            let document = entry.snapshot();
            self.entries.push_back(entry);
            return Ok(document);
        }
        if client_revision < entry.revision {
            let current_revision = entry.revision;
            self.entries.push_back(entry);
            return Err(format!(
                "stale browser revision {client_revision}; current revision is {current_revision}"
            ));
        }
        let base_bytes = entry
            .document
            .opc
            .estimated_bytes()
            .saturating_add(entry.document.deck_json.len())
            .saturating_add(estimated_deck_bytes)
            .saturating_add(assets.estimated_bytes());
        if base_bytes > self.max_bytes {
            self.entries.insert(index, entry);
            return Err(format!(
                "edited document needs approximately {base_bytes} cache bytes, limit is {}",
                self.max_bytes
            ));
        }

        while self.estimated_bytes().saturating_add(base_bytes) > self.max_bytes {
            if self.entries.pop_front().is_none() {
                break;
            }
        }
        entry.document.deck = deck;
        entry.document.assets = assets;
        entry.base_bytes = base_bytes;
        entry.revision = client_revision;
        entry.dirty = true;
        entry.pptx = None;
        entry.udoc_json = None;
        entry.udoc = None;
        entry.html = None;
        entry.video = None;
        entry.video_fast = None;
        let document = entry.snapshot();
        self.entries.push_back(entry);
        Ok(document)
    }

    pub(crate) fn artifact(
        &mut self,
        id: &str,
        revision: u64,
        kind: ArtifactKind,
    ) -> Option<Arc<[u8]>> {
        let index = self
            .entries
            .iter()
            .position(|entry| entry.document.id == id)?;
        let entry = self.entries.remove(index)?;
        let artifact = (entry.revision == revision)
            .then(|| entry.artifact(kind))
            .flatten();
        self.entries.push_back(entry);
        artifact
    }

    /// Stores a lazy artifact when it fits. Other least-recently-used
    /// documents may be evicted, but the document that owns the artifact is
    /// always protected. An oversized artifact is returned to the caller but
    /// deliberately not retained.
    pub(crate) fn store_artifact(
        &mut self,
        id: &str,
        revision: u64,
        kind: ArtifactKind,
        bytes: Arc<[u8]>,
    ) -> bool {
        let Some(index) = self
            .entries
            .iter()
            .position(|entry| entry.document.id == id)
        else {
            return false;
        };
        if self.entries[index].revision != revision {
            return false;
        }
        let old_size = self.entries[index]
            .artifact(kind)
            .as_ref()
            .map_or(0, |old| old.len());
        let required = self
            .estimated_bytes()
            .saturating_sub(old_size)
            .saturating_add(bytes.len());

        if required > self.max_bytes {
            // The inner budget check can stop evicting early, so a plain loop
            // with `let-else` reads better than `while let` here.
            #[allow(clippy::while_let_loop)]
            loop {
                let Some(eviction) = self
                    .entries
                    .iter()
                    .position(|entry| entry.document.id != id)
                else {
                    break;
                };
                self.entries.remove(eviction);
                let current_old = self
                    .entries
                    .iter()
                    .find(|entry| entry.document.id == id)
                    .and_then(|entry| entry.artifact(kind))
                    .as_ref()
                    .map_or(0, |old| old.len());
                if self
                    .estimated_bytes()
                    .saturating_sub(current_old)
                    .saturating_add(bytes.len())
                    <= self.max_bytes
                {
                    break;
                }
            }
        }

        let Some(index) = self
            .entries
            .iter()
            .position(|entry| entry.document.id == id)
        else {
            return false;
        };
        let old_size = self.entries[index]
            .artifact(kind)
            .as_ref()
            .map_or(0, |old| old.len());
        if self
            .estimated_bytes()
            .saturating_sub(old_size)
            .saturating_add(bytes.len())
            > self.max_bytes
        {
            return false;
        }
        let mut entry = self
            .entries
            .remove(index)
            .expect("entry index remains valid");
        entry.replace_artifact(kind, bytes);
        self.entries.push_back(entry);
        true
    }

    pub(crate) fn stats(&self) -> CacheStats {
        CacheStats {
            documents: self.entries.len(),
            estimated_bytes: self.estimated_bytes(),
            max_documents: self.max_documents,
            max_bytes: self.max_bytes,
        }
    }

    fn estimated_bytes(&self) -> usize {
        self.entries.iter().fold(0usize, |total, entry| {
            total.saturating_add(entry.estimated_bytes())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use std::sync::{Arc, Mutex};
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    fn snapshot(marker: &[u8]) -> Arc<OpcSnapshot> {
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        writer
            .start_file("ppt/presentation.xml", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(marker).unwrap();
        let pptx = writer.finish().unwrap().into_inner();
        Arc::new(crate::opc_snapshot::explode(&pptx).unwrap())
    }

    fn insert(cache: &mut DocumentCache, source: &[u8]) -> CachedDocument {
        let digest = DocumentCache::digest(source);
        let mut deck = Deck::demo();
        deck.source_import_id = Some(DocumentCache::id_for_digest(&digest));
        let json = serde_json::to_vec(&deck).unwrap();
        cache
            .insert(
                digest,
                snapshot(source),
                Arc::new(deck.clone()),
                Arc::new(deck),
                Arc::from(json),
                Arc::new(AssetCatalog::default()),
            )
            .unwrap()
    }

    #[test]
    fn content_digest_reuses_import_and_lru_evicts_oldest() {
        let mut cache = DocumentCache::new(2, 1024 * 1024);
        let first = insert(&mut cache, b"first");
        let second = insert(&mut cache, b"second");
        assert_eq!(
            cache
                .get_by_digest(&DocumentCache::digest(b"first"))
                .unwrap()
                .id,
            first.id
        );
        let third = insert(&mut cache, b"third");
        assert!(cache.get(&second.id).is_none());
        assert!(cache.get(&first.id).is_some());
        assert!(cache.get(&third.id).is_some());
    }

    #[test]
    fn artifacts_are_reused_and_never_break_the_byte_limit() {
        let mut cache = DocumentCache::new(2, 32 * 1024);
        let document = insert(&mut cache, b"pptx");
        let artifact: Arc<[u8]> = Arc::from(vec![7; 512]);
        assert!(cache.store_artifact(
            &document.id,
            document.revision,
            ArtifactKind::UdocJson,
            artifact.clone()
        ));
        assert!(Arc::ptr_eq(
            &cache
                .artifact(&document.id, document.revision, ArtifactKind::UdocJson)
                .unwrap(),
            &artifact
        ));
        assert!(cache.stats().estimated_bytes <= cache.stats().max_bytes);
        let oversized: Arc<[u8]> = Arc::from(vec![0; 64 * 1024]);
        assert!(!cache.store_artifact(
            &document.id,
            document.revision,
            ArtifactKind::Html,
            oversized
        ));
        assert!(cache
            .artifact(&document.id, document.revision, ArtifactKind::Html)
            .is_none());
    }

    #[test]
    fn cache_is_safe_to_share_across_request_threads() {
        let cache = Arc::new(Mutex::new(DocumentCache::new(2, 1024 * 1024)));
        let document = insert(&mut cache.lock().unwrap(), b"threaded");
        let mut workers = Vec::new();
        for _ in 0..8 {
            let cache = Arc::clone(&cache);
            let id = document.id.clone();
            workers.push(std::thread::spawn(move || {
                assert_eq!(cache.lock().unwrap().get(&id).unwrap().id, id);
            }));
        }
        for worker in workers {
            worker.join().unwrap();
        }
    }

    #[test]
    fn dirty_sync_invalidates_artifacts_and_rejects_stale_builds() {
        let mut cache = DocumentCache::new(2, 1024 * 1024);
        let original = insert(&mut cache, b"editable");
        assert!(cache.store_artifact(
            &original.id,
            original.revision,
            ArtifactKind::Pptx,
            Arc::from(b"old".to_vec())
        ));
        let mut edited = (*original.deck).clone();
        edited.title = "edited".into();
        let current = cache
            .update_deck(
                &original.id,
                1,
                Arc::new(edited),
                4096,
                Arc::new(AssetCatalog::default()),
            )
            .unwrap();
        assert!(current.dirty);
        assert_eq!(current.revision, 1);
        assert!(cache
            .artifact(&current.id, current.revision, ArtifactKind::Pptx)
            .is_none());
        assert!(!cache.store_artifact(
            &current.id,
            original.revision,
            ArtifactKind::Pptx,
            Arc::from(b"stale".to_vec())
        ));
    }

    #[test]
    fn repeating_a_client_revision_preserves_all_derived_artifacts() {
        let mut cache = DocumentCache::new(2, 1024 * 1024);
        let original = insert(&mut cache, b"idempotent");
        let mut edited = (*original.deck).clone();
        edited.title = "first synchronized edit".into();
        let current = cache
            .update_deck(
                &original.id,
                7,
                Arc::new(edited),
                4096,
                Arc::new(AssetCatalog::default()),
            )
            .unwrap();
        let html: Arc<[u8]> = Arc::from(b"cached html".to_vec());
        let udoc: Arc<[u8]> = Arc::from(b"cached udoc".to_vec());
        assert!(cache.store_artifact(
            &current.id,
            current.revision,
            ArtifactKind::Html,
            Arc::clone(&html),
        ));
        assert!(cache.store_artifact(
            &current.id,
            current.revision,
            ArtifactKind::Udoc,
            Arc::clone(&udoc),
        ));

        let mut conflicting_duplicate = (*original.deck).clone();
        conflicting_duplicate.title = "must be ignored for the same revision".into();
        let duplicate = cache
            .update_deck(
                &original.id,
                7,
                Arc::new(conflicting_duplicate),
                usize::MAX,
                Arc::new(AssetCatalog::default()),
            )
            .unwrap();
        assert_eq!(duplicate.revision, 7);
        assert_eq!(duplicate.deck.title, "first synchronized edit");
        assert!(Arc::ptr_eq(
            &cache
                .artifact(&duplicate.id, duplicate.revision, ArtifactKind::Html)
                .unwrap(),
            &html,
        ));
        assert!(Arc::ptr_eq(
            &cache
                .artifact(&duplicate.id, duplicate.revision, ArtifactKind::Udoc)
                .unwrap(),
            &udoc,
        ));
    }

    #[test]
    fn importing_the_same_digest_after_edits_restores_the_source_snapshot() {
        let mut cache = DocumentCache::new(2, 1024 * 1024);
        let original = insert(&mut cache, b"reopen-source");
        let original_title = original.deck.title.clone();
        let mut edited = (*original.deck).clone();
        edited.title = "edited".into();
        let dirty = cache
            .update_deck(
                &original.id,
                1,
                Arc::new(edited),
                4096,
                Arc::new(AssetCatalog::default()),
            )
            .unwrap();
        assert!(cache.store_artifact(
            &dirty.id,
            dirty.revision,
            ArtifactKind::Pptx,
            Arc::from(b"edited pptx".to_vec()),
        ));

        let reopened = cache
            .get_by_digest(&DocumentCache::digest(b"reopen-source"))
            .unwrap();
        assert_eq!(reopened.revision, 0);
        assert!(!reopened.dirty);
        assert_eq!(reopened.deck.title, original_title);
        assert!(cache
            .artifact(&reopened.id, reopened.revision, ArtifactKind::Pptx)
            .is_none());
    }

    #[test]
    fn persisted_current_scene_different_from_baseline_reopens_dirty() {
        let mut cache = DocumentCache::new(2, 1024 * 1024);
        let digest = DocumentCache::digest(b"persisted-current");
        let mut baseline = Deck::demo();
        baseline.title = "baseline".into();
        let mut current = baseline.clone();
        current.title = "persisted edit".into();
        let json = Arc::from(serde_json::to_vec(&current).unwrap());
        let reopened = cache
            .insert(
                digest,
                snapshot(b"persisted-current"),
                Arc::new(current),
                Arc::new(baseline),
                json,
                Arc::new(AssetCatalog::default()),
            )
            .unwrap();
        assert!(reopened.dirty);
        assert_eq!(reopened.deck.title, "persisted edit");
        assert_eq!(reopened.original_deck.title, "baseline");
    }
}
