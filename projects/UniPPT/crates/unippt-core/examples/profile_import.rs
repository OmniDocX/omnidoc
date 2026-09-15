use std::path::PathBuf;
use std::time::Instant;

use unippt_core::{import_pptx_compact, Deck, SceneObject, ASSET_REFERENCE_PREFIX};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("usage: profile_import SOURCE.pptx")?,
    );

    let started = Instant::now();
    let bytes = std::fs::read(&source)?;
    let read_elapsed = started.elapsed();

    let import_started = Instant::now();
    let compact = import_pptx_compact(&bytes)?;
    let import_elapsed = import_started.elapsed();
    let deck = &compact.deck;

    let json_started = Instant::now();
    let json = serde_json::to_vec(&deck)?;
    let json_elapsed = json_started.elapsed();

    let refs = AssetRefStats::from_deck(deck);
    let asset_bytes = compact
        .assets
        .iter()
        .map(|asset| asset.bytes.len())
        .sum::<usize>();
    println!("source_bytes={}", bytes.len());
    println!("slides={}", deck.slides.len());
    println!("read_ms={}", read_elapsed.as_millis());
    println!("import_ms={}", import_elapsed.as_millis());
    println!("json_ms={}", json_elapsed.as_millis());
    println!("json_bytes={}", json.len());
    println!("asset_references={}", refs.references);
    println!("asset_reference_bytes={}", refs.reference_bytes);
    println!("unique_assets={}", compact.assets.len());
    println!("unique_asset_bytes={asset_bytes}");
    Ok(())
}

#[derive(Default)]
struct AssetRefStats {
    references: usize,
    reference_bytes: usize,
}

impl AssetRefStats {
    fn from_deck(deck: &Deck) -> Self {
        let mut stats = Self::default();
        for font in &deck.fonts {
            stats.add(&font.data_uri);
        }
        for slide in &deck.slides {
            stats.add_optional(slide.background_asset.as_deref());
            stats.add_objects(&slide.master_objects);
            stats.add_objects(&slide.layout_objects);
            stats.add_objects(&slide.objects);
        }
        stats
    }

    fn add_objects(&mut self, objects: &[SceneObject]) {
        for object in objects {
            self.add_optional(object.asset.as_deref());
            self.add_optional(object.shape_fill_asset.as_deref());
            if let Some(media) = &object.media {
                self.add_optional(media.asset.as_deref());
                self.add_optional(media.playback_asset.as_deref());
            }
            self.add_objects(&object.children);
        }
    }

    fn add_optional(&mut self, value: Option<&str>) {
        if let Some(value) = value {
            self.add(value);
        }
    }

    fn add(&mut self, value: &str) {
        if value.starts_with(ASSET_REFERENCE_PREFIX) {
            self.references += 1;
            self.reference_bytes += value.len();
        }
    }
}
