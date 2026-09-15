//! Corpus audit for animation fidelity.
//!
//! Reports how PowerPoint's native `p:timing` presets land in the UniPPT
//! animation model, which `presetClass`/`presetID` pairs degrade to `custom`,
//! and how much of the deck relies on features the browser runtime does not
//! model yet (text builds, acceleration curves, repeats, exotic transitions).
//!
//! usage: cargo run -p unippt-core --example audit_animations -- PPTX_OR_DIR [presetClass/presetID]
//!
//! Passing a preset selector (for example `entr/53`) switches to sample mode and
//! prints the native `p:cTn` subtree for the first few matches. The child
//! behaviors under that node are the ground truth for what PowerPoint actually
//! plays, which is more reliable than any published presetID table.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use quick_xml::events::Event;
use quick_xml::Reader;
use unippt_core::{import_pptx_compact, AnimationClass, AnimationKind};

#[derive(Default)]
struct Totals {
    files: usize,
    slides: usize,
    effects: usize,
    kinds: BTreeMap<String, usize>,
    /// `presetClass:presetID:presetSubtype` for effects that degraded to custom.
    unmapped_presets: BTreeMap<String, usize>,
    /// Every native `presetClass:presetID` seen in raw XML, mapped or not.
    native_presets: BTreeMap<String, usize>,
    transitions: BTreeMap<String, usize>,
    text_builds: BTreeMap<String, usize>,
    with_acceleration: usize,
    with_time_filter: usize,
    with_repeat: usize,
    with_auto_reverse: usize,
    with_property_tracks: usize,
    unresolved_target: usize,
    /// Effects that pair an authored fade with a `p:anim` trajectory.
    fade_with_property_tracks: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("usage: audit_animations PPTX_OR_DIRECTORY")?,
    );
    let mut files = pptx_files(&root)?;
    files.sort_by_key(|path| numeric_stem(path).unwrap_or(u32::MAX));

    if let Some(selector) = std::env::args().nth(2) {
        return sample_preset(&files, &selector);
    }

    let mut totals = Totals::default();
    for path in &files {
        match audit_file(path, &mut totals) {
            Ok(summary) => println!("{summary}"),
            Err(error) => println!("{}: ERROR {error}", path.display()),
        }
    }
    report(&totals);
    Ok(())
}

/// Prints the native `p:cTn` subtree for effects matching `presetClass/presetID`.
fn sample_preset(files: &[PathBuf], selector: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (class, id) = selector
        .split_once('/')
        .ok_or("selector must look like entr/53")?;
    let needle = format!("presetClass=\"{class}\" ");
    let id_needle = format!("presetID=\"{id}\"");
    let mut shown = 0usize;
    for path in files {
        if shown >= 4 {
            break;
        }
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(compact) = import_pptx_compact(&bytes) else {
            continue;
        };
        for (index, slide) in compact.deck.slides.iter().enumerate() {
            let Some(xml) = &slide.source_timing_xml else {
                continue;
            };
            for fragment in ctn_fragments(xml) {
                let head = fragment.split('>').next().unwrap_or_default();
                if !head.contains(&id_needle) || !head.contains(needle.trim_end()) {
                    continue;
                }
                println!(
                    "\n=== {} slide {} :: {selector} ===\n{}",
                    path.file_name().unwrap_or_default().to_string_lossy(),
                    index + 1,
                    fragment
                );
                shown += 1;
                break;
            }
            if shown >= 4 {
                break;
            }
        }
    }
    if shown == 0 {
        println!("no match for {selector}");
    }
    Ok(())
}

/// Splits a timing fragment into balanced `<p:cTn>…</p:cTn>` substrings.
fn ctn_fragments(xml: &str) -> Vec<String> {
    let mut fragments = Vec::new();
    let bytes = xml.as_bytes();
    let mut cursor = 0usize;
    while let Some(offset) = xml[cursor..].find("<p:cTn ") {
        let start = cursor + offset;
        let mut depth = 0i32;
        let mut scan = start;
        let mut end = None;
        while scan < bytes.len() {
            if xml[scan..].starts_with("<p:cTn") {
                let head_end = xml[scan..].find('>').map(|value| scan + value);
                let self_closing = head_end.is_some_and(|value| bytes[value - 1] == b'/');
                if !self_closing {
                    depth += 1;
                }
                scan = head_end.map_or(bytes.len(), |value| value + 1);
                if self_closing && depth == 0 {
                    end = Some(scan);
                    break;
                }
                continue;
            }
            if xml[scan..].starts_with("</p:cTn>") {
                depth -= 1;
                scan += "</p:cTn>".len();
                if depth == 0 {
                    end = Some(scan);
                    break;
                }
                continue;
            }
            scan += 1;
        }
        let Some(end) = end else { break };
        fragments.push(xml[start..end].to_string());
        cursor = start + "<p:cTn ".len();
    }
    fragments
}

fn audit_file(path: &Path, totals: &mut Totals) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    let compact = import_pptx_compact(&bytes).map_err(|error| error.to_string())?;
    totals.files += 1;

    let mut file_effects = 0usize;
    let mut file_custom = 0usize;
    for slide in &compact.deck.slides {
        totals.slides += 1;
        for effect in slide.animations.iter().chain(&slide.inherited_animations) {
            totals.effects += 1;
            file_effects += 1;
            let kind = format!("{:?}", effect.effect);
            *totals.kinds.entry(kind).or_default() += 1;
            if effect.effect == AnimationKind::Custom {
                file_custom += 1;
                let key = format!(
                    "class={:?} presetID={} subtype={}",
                    effect.class,
                    effect
                        .preset_id
                        .map_or_else(|| "-".to_string(), |id| id.to_string()),
                    effect
                        .preset_subtype
                        .map_or_else(|| "-".to_string(), |id| id.to_string()),
                );
                *totals.unmapped_presets.entry(key).or_default() += 1;
            }
            if effect.acceleration.is_some_and(|value| value > 0)
                || effect.deceleration.is_some_and(|value| value > 0)
            {
                totals.with_acceleration += 1;
            }
            if effect.time_filter.is_some() {
                totals.with_time_filter += 1;
            }
            if effect.repeat_count.is_some() || effect.repeat_duration_ms.is_some() {
                totals.with_repeat += 1;
            }
            if effect.auto_reverse {
                totals.with_auto_reverse += 1;
            }
            if !effect.property_animations.is_empty() {
                totals.with_property_tracks += 1;
                if effect.effect == AnimationKind::Fade
                    && matches!(
                        effect.class,
                        AnimationClass::Entrance | AnimationClass::Exit
                    )
                {
                    totals.fade_with_property_tracks += 1;
                }
            }
            if effect.target_object_id.is_none() {
                totals.unresolved_target += 1;
            }
        }
        if let Some(transition) = &slide.transition {
            *totals
                .transitions
                .entry(transition.kind.clone())
                .or_default() += 1;
        }
        if let Some(xml) = &slide.source_timing_xml {
            scan_native_timing(xml, totals);
        }
    }

    Ok(format!(
        "{:<10} slides={:<4} effects={:<5} custom={:<5} ({:.0}%)",
        path.file_name().unwrap_or_default().to_string_lossy(),
        compact.deck.slides.len(),
        file_effects,
        file_custom,
        percent(file_custom, file_effects),
    ))
}

/// Reads the raw timing fragment for signals the scene model drops entirely.
fn scan_native_timing(xml: &str, totals: &mut Totals) {
    let mut reader = Reader::from_reader(xml.as_bytes());
    reader.config_mut().trim_text(true);
    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) | Ok(Event::Empty(element)) => {
                let name = element.name();
                let local = local_name(name.as_ref());
                match local {
                    b"cTn" => {
                        let class = attribute(&element, b"presetClass");
                        let id = attribute(&element, b"presetID");
                        if let (Some(class), Some(id)) = (class, id) {
                            *totals
                                .native_presets
                                .entry(format!("{class}/{id}"))
                                .or_default() += 1;
                        }
                    }
                    b"bldP" => {
                        let build = attribute(&element, b"build").unwrap_or_else(|| "-".into());
                        let level = attribute(&element, b"bldLvl").unwrap_or_else(|| "-".into());
                        *totals
                            .text_builds
                            .entry(format!("build={build} bldLvl={level}"))
                            .or_default() += 1;
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
}

fn report(totals: &Totals) {
    println!("\n================ ANIMATION CORPUS AUDIT ================");
    println!(
        "files={} slides={} effects={}",
        totals.files, totals.slides, totals.effects
    );

    println!("\n-- mapped effect kinds --");
    let mut kinds = totals.kinds.iter().collect::<Vec<_>>();
    kinds.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
    for (kind, count) in kinds {
        println!(
            "  {kind:<12} {count:>6}  {:.1}%",
            percent(*count, totals.effects)
        );
    }

    println!("\n-- native presetClass/presetID seen in raw p:timing (top 40) --");
    let mut native = totals.native_presets.iter().collect::<Vec<_>>();
    native.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
    for (key, count) in native.iter().take(40) {
        println!("  {key:<18} {count:>6}");
    }
    println!("  (distinct preset pairs: {})", totals.native_presets.len());

    println!("\n-- effects degraded to custom, by native preset (top 40) --");
    let mut unmapped = totals.unmapped_presets.iter().collect::<Vec<_>>();
    unmapped.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
    for (key, count) in unmapped.iter().take(40) {
        println!("  {key:<46} {count:>6}");
    }
    println!(
        "  (distinct degraded signatures: {})",
        totals.unmapped_presets.len()
    );

    println!("\n-- slide transitions --");
    let mut transitions = totals.transitions.iter().collect::<Vec<_>>();
    transitions.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
    for (kind, count) in transitions {
        println!("  {kind:<18} {count:>6}");
    }

    println!("\n-- text build (p:bldP) usage, dropped by the scene model --");
    let mut builds = totals.text_builds.iter().collect::<Vec<_>>();
    builds.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
    for (key, count) in builds.iter().take(20) {
        println!("  {key:<34} {count:>6}");
    }

    println!("\n-- timing features present on imported effects --");
    println!(
        "  acceleration/deceleration {:>6}  {:.1}%",
        totals.with_acceleration,
        percent(totals.with_acceleration, totals.effects)
    );
    println!(
        "  tmFilter                  {:>6}  {:.1}%",
        totals.with_time_filter,
        percent(totals.with_time_filter, totals.effects)
    );
    println!(
        "  repeat                    {:>6}  {:.1}%",
        totals.with_repeat,
        percent(totals.with_repeat, totals.effects)
    );
    println!(
        "  autoReverse               {:>6}  {:.1}%",
        totals.with_auto_reverse,
        percent(totals.with_auto_reverse, totals.effects)
    );
    println!(
        "  p:anim property tracks    {:>6}  {:.1}%",
        totals.with_property_tracks,
        percent(totals.with_property_tracks, totals.effects)
    );
    println!(
        "  unresolved target shape   {:>6}  {:.1}%",
        totals.unresolved_target,
        percent(totals.unresolved_target, totals.effects)
    );
    println!(
        "  fade + p:anim trajectory  {:>6}  {:.1}%",
        totals.fade_with_property_tracks,
        percent(totals.fade_with_property_tracks, totals.effects)
    );
}

fn percent(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        0.0
    } else {
        (part as f64) * 100.0 / (whole as f64)
    }
}

fn attribute(element: &quick_xml::events::BytesStart<'_>, name: &[u8]) -> Option<String> {
    element.attributes().flatten().find_map(|attribute| {
        (attribute.key.as_ref() == name)
            .then(|| String::from_utf8_lossy(attribute.value.as_ref()).into_owned())
    })
}

fn local_name(name: &[u8]) -> &[u8] {
    name.rsplit(|byte| *byte == b':').next().unwrap_or(name)
}

fn pptx_files(root: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    if root.is_file() {
        return Ok(vec![root.to_path_buf()]);
    }
    Ok(std::fs::read_dir(root)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("pptx"))
        })
        .collect())
}

fn numeric_stem(path: &Path) -> Option<u32> {
    path.file_stem()?.to_str()?.parse().ok()
}
