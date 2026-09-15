use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use pptx::dml::color::ColorFormat;
use pptx::dml::effect::{
    DuotoneColor, DuotoneColorTransform, DuotoneEffect, ShadowFormat, ShadowType,
};
use pptx::dml::fill::FillFormat;
use pptx::dml::line::LineFormat;
use pptx::enums::shapes::PpPlaceholderType;
use pptx::opc::pack_uri::PackURI;
use pptx::presentation::SceneAssetCache;
use pptx::shapes::{AutoShape, Shape, ShapeTree};
use pptx::text::{BulletFormat, Font, Paragraph, TextFrame};
use pptx::theme::ThemeColorScheme;
use pptx::Presentation;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use quick_xml::Writer;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

use crate::animation::parse_slide_timing;
use crate::chart::{resolve_slide_charts, ChartData};
use crate::compact::{AssetSink, CompactImport};
use crate::font_policy::{primary_family, FontPolicy};
use crate::hyperlink::{parse_run_hyperlinks, parse_shape_hyperlinks, RunHyperlinks};
use crate::media::parse_slide_media_with_assets;
use crate::model::{
    AnimationEffect, Deck, DuotoneStyle, FontSlots, FormulaData, Frame, GradientFillStyle,
    GradientStopStyle, ImageCrop, ImageFillRect, ObjectKind, RichTextParagraph, RichTextRun,
    SceneObject, ShadowStyle, Slide, TableCell, TableCellBorder, TableCellBorders, TableData,
    TableRow, TextFrameStyle, TextStyle, VisualStyle,
};

const CANVAS_WIDTH: f64 = 1280.0;
const RT_SLIDE_LAYOUT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout";
const RT_SLIDE_MASTER: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster";
const RT_THEME: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme";
const DEFAULT_WIDTH_EMU: i64 = 12_192_000;
const DEFAULT_HEIGHT_EMU: i64 = 6_858_000;
const UNIPPT_EXTENSIONS_PART: &str = "/ppt/customXml/unippt-extensions.xml";
const LEGACY_UNIPPT_EXTENSIONS_PART: &str = "/customXml/unippt-extensions.xml";

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("PPTX 解析失败: {0}")]
    Pptx(#[from] pptx::PptxError),
    #[error("PPTX 中没有幻灯片")]
    EmptyDeck,
}

/// Convert a PPTX byte stream into UniPPT's stable browser scene model.
pub fn import_pptx(bytes: &[u8]) -> Result<Deck, ImportError> {
    Ok(import_pptx_compact(bytes)?.into_inline_deck())
}

/// Convert a PPTX byte stream into a compact scene plus unique binary assets.
///
/// Resource-bearing fields use deterministic `unippt-asset:<sha256>` refs.
/// This is the preferred server import path because repeated pictures do not
/// allocate or encode another base64 string for every shape occurrence.
pub fn import_pptx_compact(bytes: &[u8]) -> Result<CompactImport, ImportError> {
    let presentation = Presentation::from_bytes(bytes)?;
    let slide_refs = presentation.slides()?;
    if slide_refs.is_empty() {
        return Err(ImportError::EmptyDeck);
    }

    let (source_width_emu, source_height_emu) = presentation
        .slide_size()?
        .unwrap_or((DEFAULT_WIDTH_EMU, DEFAULT_HEIGHT_EMU));
    let scale = CANVAS_WIDTH / source_width_emu.max(1) as f64;
    let canvas_height = source_height_emu.max(1) as f64 * scale;
    let mut asset_sink = AssetSink::default();
    let mut scene_asset_cache = SceneAssetCache::new();
    let mut shape_tree_cache = HashMap::<String, Arc<ShapeTree>>::new();
    let fonts = crate::embedded_fonts::extract_with_assets(&presentation, &mut asset_sink);
    let fallback_theme = ResolvedSlideTheme {
        colors: presentation.theme_colors()?.unwrap_or_default(),
        fonts: FontPolicy::from_presentation(&presentation, &fonts),
    };
    let mut theme_cache = HashMap::new();

    let mut slides = Vec::with_capacity(slide_refs.len());
    for (slide_index, slide_ref) in slide_refs.iter().enumerate() {
        let slide_theme = resolved_theme_for_slide(
            &presentation,
            slide_ref,
            &fonts,
            &fallback_theme,
            &mut theme_cache,
        )?;
        let theme = &slide_theme.colors;
        let font_policy = &slide_theme.fonts;
        let tree = cached_part_shape_tree(
            &presentation,
            &slide_ref.partname,
            &mut scene_asset_cache,
            &mut shape_tree_cache,
        )?;
        let (layout_tree, master_tree) = placeholder_inheritance_trees(
            &presentation,
            slide_ref,
            &mut scene_asset_cache,
            &mut shape_tree_cache,
        )?;
        let name = presentation
            .slide_name(slide_ref)?
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| format!("幻灯片 {}", slide_index + 1));
        let notes = presentation
            .notes_slide_text(slide_ref)?
            .unwrap_or_default();
        let slide_xml = presentation.slide_xml(slide_ref)?;
        let mut tables = parse_slide_tables_with_font_policy(slide_xml, scale, theme, font_policy);
        let mut charts = resolve_slide_charts(&presentation, &slide_ref.partname, slide_xml, theme);
        let (hyperlinks, run_hyperlinks, media) = presentation
            .package()
            .part(&slide_ref.partname)
            .map(|part| {
                (
                    parse_shape_hyperlinks(slide_xml, &part.rels),
                    parse_run_hyperlinks(slide_xml, &part.rels),
                    parse_slide_media_with_assets(
                        slide_xml,
                        part,
                        presentation.package(),
                        &mut asset_sink,
                    ),
                )
            })
            .unwrap_or_default();
        let mut objects = tree
            .shapes
            .iter()
            .enumerate()
            .map(|(z, shape)| {
                let resolved = resolve_slide_placeholder(
                    shape,
                    layout_tree.as_deref(),
                    master_tree.as_deref(),
                );
                convert_shape_with_font_policy(
                    &resolved,
                    slide_index,
                    z,
                    scale,
                    theme,
                    font_policy,
                    &mut asset_sink,
                )
            })
            .collect::<Vec<_>>();
        apply_shape_hyperlinks(&mut objects, &hyperlinks);
        apply_run_hyperlinks(&mut objects, &run_hyperlinks);
        apply_shape_media(&mut objects, &media);
        apply_slide_tables(&mut objects, &mut tables);
        apply_slide_charts(&mut objects, &mut charts);
        let timing = parse_slide_timing(slide_xml, &objects);
        let inherited = inherited_scene_layers(
            &presentation,
            slide_ref,
            slide_index,
            scale,
            theme,
            font_policy,
            slide_xml,
            &mut asset_sink,
            &mut scene_asset_cache,
            &mut shape_tree_cache,
        )?;
        let (background, background_asset) =
            resolve_slide_background(&presentation, &slide_ref.partname, theme, &mut asset_sink);
        slides.push(Slide {
            id: format!("slide-{}", slide_index + 1),
            source_part_name: Some(slide_ref.partname.to_string()),
            name,
            background,
            background_asset,
            notes,
            master_objects: inherited.master_objects,
            layout_objects: inherited.layout_objects,
            objects,
            inherited_animations: inherited.animations,
            animations: timing.animations,
            transition: timing.transition,
            source_timing_xml: timing.source_timing_xml,
            source_transition_xml: timing.source_transition_xml,
        });
    }

    let extensions = read_portable_extensions(&presentation);
    let deck = Deck {
        format: "unippt".into(),
        version: 1,
        title: "导入的演示文稿".into(),
        width: CANVAS_WIDTH,
        height: canvas_height,
        source_width_emu,
        source_height_emu,
        source_import_id: None,
        fonts,
        extensions,
        slides,
    };
    Ok(asset_sink.finish(deck))
}

fn read_portable_extensions(
    presentation: &Presentation,
) -> std::collections::BTreeMap<String, serde_json::Value> {
    // Read both locations; old saved decks remain importable and byte-identical
    // no-op exports are still handled before the exporter migration runs.
    let Some(part) = [UNIPPT_EXTENSIONS_PART, LEGACY_UNIPPT_EXTENSIONS_PART]
        .iter()
        .filter_map(|name| PackURI::new(*name).ok())
        .find_map(|name| presentation.package().part(&name)) else {
        return Default::default();
    };
    let Ok(xml) = std::str::from_utf8(&part.blob) else {
        return Default::default();
    };
    let Some(body_start) = xml.find('>').map(|index| index + 1) else {
        return Default::default();
    };
    let Some(root_start) = xml[body_start..]
        .find('>')
        .map(|index| body_start + index + 1)
    else {
        return Default::default();
    };
    let Some(root_end) = xml.rfind("</unippt:extensions>") else {
        return Default::default();
    };
    if root_start > root_end {
        return Default::default();
    }
    STANDARD
        .decode(xml[root_start..root_end].trim())
        .ok()
        .and_then(|json| serde_json::from_slice(&json).ok())
        .unwrap_or_default()
}

#[derive(Debug, Clone)]
struct ResolvedSlideTheme {
    colors: ThemeColorScheme,
    fonts: FontPolicy,
}

/// Resolve the theme selected by one slide's layout/master chain.
///
/// A PPTX is allowed to contain several masters, each with its own theme. The
/// upstream convenience APIs intentionally return the first master's theme,
/// which is useful as a presentation-level default but is incorrect for later
/// masters. Cache by theme part name because many slides commonly share one
/// master and parsing both color/font schemes is otherwise repeated work.
fn resolved_theme_for_slide(
    presentation: &Presentation,
    slide_ref: &pptx::slide::SlideRef,
    embedded_fonts: &[crate::model::EmbeddedFont],
    fallback: &ResolvedSlideTheme,
    cache: &mut HashMap<String, ResolvedSlideTheme>,
) -> Result<ResolvedSlideTheme, pptx::PptxError> {
    let Some(theme_part_name) = theme_part_for_slide(presentation, slide_ref)? else {
        return Ok(fallback.clone());
    };
    let cache_key = theme_part_name.to_string();
    if let Some(theme) = cache.get(&cache_key) {
        return Ok(theme.clone());
    }
    let Some(theme_part) = presentation.package().part(&theme_part_name) else {
        return Ok(fallback.clone());
    };
    let resolved = resolved_theme_from_xml(&theme_part.blob, embedded_fonts)?;
    cache.insert(cache_key, resolved.clone());
    Ok(resolved)
}

fn resolved_theme_from_xml(
    theme_xml: &[u8],
    embedded_fonts: &[crate::model::EmbeddedFont],
) -> Result<ResolvedSlideTheme, pptx::PptxError> {
    Ok(ResolvedSlideTheme {
        colors: pptx::theme::parse_theme_color_scheme(theme_xml)?.unwrap_or_default(),
        fonts: FontPolicy::from_theme_xml(theme_xml, embedded_fonts),
    })
}

fn theme_part_for_slide(
    presentation: &Presentation,
    slide_ref: &pptx::slide::SlideRef,
) -> Result<Option<PackURI>, pptx::PptxError> {
    let Some(layout_ref) = presentation.slide_layout_for(slide_ref)? else {
        return Ok(None);
    };
    let Some(master_ref) = presentation.slide_master_for_layout(&layout_ref)? else {
        return Ok(None);
    };
    Ok(theme_part_for_master(presentation, &master_ref.partname))
}

fn theme_part_for_master(
    presentation: &Presentation,
    master_part_name: &PackURI,
) -> Option<PackURI> {
    let master_part = presentation.package().part(master_part_name)?;
    master_part
        .rels
        .all_by_reltype(RT_THEME)
        .first()
        .and_then(|relationship| {
            relationship
                .target_partname(master_part.partname.base_uri())
                .ok()
        })
}

fn apply_shape_media(
    objects: &mut [SceneObject],
    media: &std::collections::HashMap<u32, crate::model::MediaData>,
) {
    for object in objects {
        if let Some(native) = object
            .source_shape_id
            .and_then(|shape_id| media.get(&shape_id))
        {
            object.media = Some(native.clone());
        }
        apply_shape_media(&mut object.children, media);
    }
}

fn apply_shape_hyperlinks(
    objects: &mut [SceneObject],
    hyperlinks: &std::collections::HashMap<u32, crate::model::ObjectHyperlinks>,
) {
    for object in objects {
        if let Some(shape_id) = object.source_shape_id {
            if let Some(actions) = hyperlinks.get(&shape_id) {
                object.hyperlinks.clone_from(actions);
            }
        }
        apply_shape_hyperlinks(&mut object.children, hyperlinks);
    }
}

fn apply_run_hyperlinks(objects: &mut [SceneObject], hyperlinks: &RunHyperlinks) {
    for object in objects {
        if let Some(shape_id) = object.source_shape_id {
            for paragraph in &mut object.text_paragraphs {
                let Some(paragraph_index) = paragraph.source_index else {
                    continue;
                };
                for run in &mut paragraph.runs {
                    let Some(run_index) = run.source_index else {
                        continue;
                    };
                    if let Some(actions) = hyperlinks.get(&(shape_id, paragraph_index, run_index)) {
                        run.hyperlinks.clone_from(actions);
                    }
                }
            }
        }
        apply_run_hyperlinks(&mut object.children, hyperlinks);
    }
}

fn apply_slide_tables(objects: &mut [SceneObject], tables: &mut HashMap<u32, TableData>) {
    for object in objects {
        if object.kind == ObjectKind::Table {
            object.table = object
                .source_shape_id
                .and_then(|shape_id| tables.remove(&shape_id));
        }
        apply_slide_tables(&mut object.children, tables);
    }
}

fn apply_slide_charts(objects: &mut [SceneObject], charts: &mut HashMap<u32, ChartData>) {
    for object in objects {
        if object.kind == ObjectKind::Chart {
            object.chart = object
                .source_shape_id
                .and_then(|shape_id| charts.remove(&shape_id));
        }
        apply_slide_charts(&mut object.children, charts);
    }
}

#[derive(Debug, Default)]
struct InheritedSceneLayers {
    master_objects: Vec<SceneObject>,
    layout_objects: Vec<SceneObject>,
    animations: Vec<AnimationEffect>,
}

/// Project visible master/layout furniture into separate scene layers.
///
/// A slide relationship id is local to its OPC part, so inherited parts must
/// be parsed and hydrated against their own relationships.  Keeping the
/// resulting objects outside `Slide.objects` also prevents source shape-id
/// collisions during slide-local editing and native writeback.
#[allow(clippy::too_many_arguments)]
fn inherited_scene_layers(
    presentation: &Presentation,
    slide_ref: &pptx::slide::SlideRef,
    slide_index: usize,
    scale: f64,
    theme: &ThemeColorScheme,
    font_policy: &FontPolicy,
    slide_xml: &[u8],
    asset_sink: &mut AssetSink,
    scene_asset_cache: &mut SceneAssetCache,
    shape_tree_cache: &mut HashMap<String, Arc<ShapeTree>>,
) -> Result<InheritedSceneLayers, pptx::PptxError> {
    let Some(layout_ref) = presentation.slide_layout_for(slide_ref)? else {
        return Ok(InheritedSceneLayers::default());
    };
    let mut layers = InheritedSceneLayers::default();
    let (layout_objects, mut layout_animations) = inherited_part_scene(
        presentation,
        &layout_ref.partname,
        slide_index,
        scale,
        theme,
        font_policy,
        "layout",
        asset_sink,
        scene_asset_cache,
        shape_tree_cache,
    )?;
    layers.layout_objects = layout_objects;
    layers.animations.append(&mut layout_animations);

    let layout_shows_master = presentation
        .package()
        .part(&layout_ref.partname)
        .is_none_or(|part| show_master_shapes(&part.blob));
    if show_master_shapes(slide_xml) && layout_shows_master {
        if let Some(master_ref) = presentation.slide_master_for_layout(&layout_ref)? {
            let (master_objects, mut master_animations) = inherited_part_scene(
                presentation,
                &master_ref.partname,
                slide_index,
                scale,
                theme,
                font_policy,
                "master",
                asset_sink,
                scene_asset_cache,
                shape_tree_cache,
            )?;
            layers.master_objects = master_objects;
            // Master effects precede layout effects in the inherited paint and
            // timing order.
            master_animations.append(&mut layers.animations);
            layers.animations = master_animations;
        }
    }
    for (order, animation) in layers.animations.iter_mut().enumerate() {
        animation.order = order as u32;
    }
    Ok(layers)
}

#[allow(clippy::too_many_arguments)]
fn inherited_part_scene(
    presentation: &Presentation,
    part_name: &PackURI,
    slide_index: usize,
    scale: f64,
    theme: &ThemeColorScheme,
    font_policy: &FontPolicy,
    layer_name: &str,
    asset_sink: &mut AssetSink,
    scene_asset_cache: &mut SceneAssetCache,
    shape_tree_cache: &mut HashMap<String, Arc<ShapeTree>>,
) -> Result<(Vec<SceneObject>, Vec<AnimationEffect>), pptx::PptxError> {
    let Some(part) = presentation.package().part(part_name) else {
        return Ok((Vec::new(), Vec::new()));
    };
    let tree =
        cached_part_shape_tree(presentation, part_name, scene_asset_cache, shape_tree_cache)?;
    let mut objects = tree
        .shapes
        .iter()
        .filter(|shape| !shape.is_placeholder())
        .enumerate()
        .map(|(z, shape)| {
            convert_shape_with_font_policy(
                shape,
                slide_index,
                z,
                scale,
                theme,
                font_policy,
                asset_sink,
            )
        })
        .collect::<Vec<_>>();
    namespace_object_ids(&mut objects, layer_name);

    let hyperlinks = parse_shape_hyperlinks(&part.blob, &part.rels);
    let run_hyperlinks = parse_run_hyperlinks(&part.blob, &part.rels);
    let media = parse_slide_media_with_assets(&part.blob, part, presentation.package(), asset_sink);
    let mut tables = parse_slide_tables_with_font_policy(&part.blob, scale, theme, font_policy);
    let mut charts = resolve_slide_charts(presentation, part_name, &part.blob, theme);
    apply_shape_hyperlinks(&mut objects, &hyperlinks);
    apply_run_hyperlinks(&mut objects, &run_hyperlinks);
    apply_shape_media(&mut objects, &media);
    apply_slide_tables(&mut objects, &mut tables);
    apply_slide_charts(&mut objects, &mut charts);

    let mut animations = parse_slide_timing(&part.blob, &objects).animations;
    for animation in &mut animations {
        animation.id = format!("{layer_name}-{}", animation.id);
    }
    Ok((objects, animations))
}

fn namespace_object_ids(objects: &mut [SceneObject], layer_name: &str) {
    for object in objects {
        object.id = format!("{layer_name}-{}", object.id);
        namespace_object_ids(&mut object.children, layer_name);
    }
}

fn show_master_shapes(xml: &[u8]) -> bool {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) | Ok(Event::Empty(element)) => {
                let qualified_name = element.name();
                let local = local_name(qualified_name.as_ref());
                if matches!(local, b"sld" | b"sldLayout") {
                    return !matches!(
                        xml_attr(&element, b"showMasterSp").as_deref(),
                        Some("0" | "false")
                    );
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    true
}

/// Read layout/master trees used as style inheritance sources for slide-local
/// placeholders. Their visible furniture is projected separately by
/// `inherited_scene_layers`.
#[allow(clippy::type_complexity)]
fn placeholder_inheritance_trees(
    presentation: &Presentation,
    slide_ref: &pptx::slide::SlideRef,
    scene_asset_cache: &mut SceneAssetCache,
    shape_tree_cache: &mut HashMap<String, Arc<ShapeTree>>,
) -> Result<(Option<Arc<ShapeTree>>, Option<Arc<ShapeTree>>), pptx::PptxError> {
    let Some(layout_ref) = presentation.slide_layout_for(slide_ref)? else {
        return Ok((None, None));
    };
    let layout_tree = presentation
        .package()
        .part(&layout_ref.partname)
        .map(|_| {
            cached_part_shape_tree(
                presentation,
                &layout_ref.partname,
                scene_asset_cache,
                shape_tree_cache,
            )
        })
        .transpose()?;
    let master_tree = presentation
        .slide_master_for_layout(&layout_ref)?
        .and_then(|master_ref| {
            presentation
                .package()
                .part(&master_ref.partname)
                .map(|_| master_ref.partname)
        })
        .map(|part_name| {
            cached_part_shape_tree(
                presentation,
                &part_name,
                scene_asset_cache,
                shape_tree_cache,
            )
        })
        .transpose()?;
    Ok((layout_tree, master_tree))
}

fn cached_part_shape_tree(
    presentation: &Presentation,
    part_name: &PackURI,
    scene_asset_cache: &mut SceneAssetCache,
    shape_tree_cache: &mut HashMap<String, Arc<ShapeTree>>,
) -> Result<Arc<ShapeTree>, pptx::PptxError> {
    let cache_key = part_name.to_string();
    if let Some(tree) = shape_tree_cache.get(&cache_key) {
        return Ok(Arc::clone(tree));
    }
    let tree =
        Arc::new(presentation.part_shape_tree_with_asset_cache(part_name, scene_asset_cache)?);
    shape_tree_cache.insert(cache_key, Arc::clone(&tree));
    Ok(tree)
}

fn resolve_slide_placeholder(
    slide_shape: &Shape,
    layout_tree: Option<&ShapeTree>,
    master_tree: Option<&ShapeTree>,
) -> Shape {
    let mut resolved = slide_shape.clone();
    if slide_shape.placeholder().is_none() {
        return resolved;
    }

    let layout_shape = layout_tree.and_then(|tree| matching_placeholder(slide_shape, &tree.shapes));
    let master_key = layout_shape.unwrap_or(slide_shape);
    let master_shape = master_tree.and_then(|tree| matching_placeholder(master_key, &tree.shapes));

    if !shape_has_usable_frame(&resolved) {
        if let Some(source) = layout_shape
            .filter(|shape| shape_has_usable_frame(shape))
            .or_else(|| master_shape.filter(|shape| shape_has_usable_frame(shape)))
        {
            copy_shape_frame(&mut resolved, source);
        }
    }

    let layout_auto = layout_shape.and_then(auto_shape);
    let master_auto = master_shape.and_then(auto_shape);
    if let Shape::AutoShape(target) = &mut resolved {
        inherit_auto_shape(target, layout_auto, master_auto);
    }
    resolved
}

fn matching_placeholder<'a>(shape: &Shape, candidates: &'a [Shape]) -> Option<&'a Shape> {
    let placeholder = shape.placeholder()?;
    if placeholder.idx.0 > 0 {
        if let Some(candidate) = candidates.iter().find(|candidate| {
            candidate
                .placeholder()
                .is_some_and(|value| value.idx == placeholder.idx)
        }) {
            return Some(candidate);
        }
    }
    candidates
        .iter()
        .find(|candidate| {
            candidate.placeholder().is_some_and(|value| {
                placeholder_types_compatible(placeholder.ph_type, value.ph_type)
            })
        })
        .or_else(|| {
            candidates.iter().find(|candidate| {
                candidate
                    .placeholder()
                    .is_some_and(|value| value.idx == placeholder.idx)
            })
        })
}

fn placeholder_types_compatible(
    left: Option<PpPlaceholderType>,
    right: Option<PpPlaceholderType>,
) -> bool {
    if left == right {
        return true;
    }
    let is_title = |value| {
        matches!(
            value,
            Some(
                PpPlaceholderType::Title
                    | PpPlaceholderType::CenterTitle
                    | PpPlaceholderType::VerticalTitle
            )
        )
    };
    let is_body = |value| {
        matches!(
            value,
            None | Some(
                PpPlaceholderType::Body
                    | PpPlaceholderType::Object
                    | PpPlaceholderType::Subtitle
                    | PpPlaceholderType::VerticalBody
                    | PpPlaceholderType::VerticalObject
            )
        )
    };
    (is_title(left) && is_title(right)) || (is_body(left) && is_body(right))
}

fn shape_has_usable_frame(shape: &Shape) -> bool {
    shape.width().0 > 0 && shape.height().0 > 0
}

fn copy_shape_frame(target: &mut Shape, source: &Shape) {
    let left = source.left();
    let top = source.top();
    let width = source.width();
    let height = source.height();
    let rotation = source.rotation();
    match target {
        Shape::AutoShape(shape) => {
            shape.left = left;
            shape.top = top;
            shape.width = width;
            shape.height = height;
            shape.rotation = rotation;
        }
        Shape::Picture(shape) => {
            shape.left = left;
            shape.top = top;
            shape.width = width;
            shape.height = height;
            shape.rotation = rotation;
        }
        Shape::GraphicFrame(shape) => {
            shape.left = left;
            shape.top = top;
            shape.width = width;
            shape.height = height;
            shape.rotation = rotation;
        }
        _ => {}
    }
}

fn auto_shape(shape: &Shape) -> Option<&AutoShape> {
    match shape {
        Shape::AutoShape(shape) => Some(shape),
        _ => None,
    }
}

fn inherit_auto_shape(
    target: &mut AutoShape,
    layout: Option<&AutoShape>,
    master: Option<&AutoShape>,
) {
    if target.fill.is_none() {
        target.fill = layout
            .and_then(|shape| shape.fill.clone())
            .or_else(|| master.and_then(|shape| shape.fill.clone()));
    }
    if target.line.is_none() {
        target.line = layout
            .and_then(|shape| shape.line.clone())
            .or_else(|| master.and_then(|shape| shape.line.clone()));
    }
    if target.shadow.is_none() {
        target.shadow = layout
            .and_then(|shape| shape.shadow.clone())
            .or_else(|| master.and_then(|shape| shape.shadow.clone()));
    }
    if target.prst_geom.is_none() {
        target.prst_geom = layout
            .and_then(|shape| shape.prst_geom.clone())
            .or_else(|| master.and_then(|shape| shape.prst_geom.clone()));
    }

    let local_xml = target.tx_body_xml.clone();
    inherit_text_frame(&mut target.text_frame, local_xml.as_deref(), layout, master);
}

fn inherit_text_frame(
    target: &mut Option<TextFrame>,
    local_xml: Option<&[u8]>,
    layout: Option<&AutoShape>,
    master: Option<&AutoShape>,
) {
    if target.is_none()
        && layout.and_then(AutoShape::text_frame).is_none()
        && master.and_then(AutoShape::text_frame).is_none()
    {
        return;
    }
    let frame = target.get_or_insert_with(TextFrame::new);

    inherit_body_property(
        local_xml,
        b"wrap",
        layout,
        master,
        |source| Some(source.word_wrap),
        |value| frame.word_wrap = value,
    );
    inherit_body_property(
        local_xml,
        b"lIns",
        layout,
        master,
        |source| source.margin_left,
        |value| frame.margin_left = Some(value),
    );
    inherit_body_property(
        local_xml,
        b"rIns",
        layout,
        master,
        |source| source.margin_right,
        |value| frame.margin_right = Some(value),
    );
    inherit_body_property(
        local_xml,
        b"tIns",
        layout,
        master,
        |source| source.margin_top,
        |value| frame.margin_top = Some(value),
    );
    inherit_body_property(
        local_xml,
        b"bIns",
        layout,
        master,
        |source| source.margin_bottom,
        |value| frame.margin_bottom = Some(value),
    );
    inherit_body_property(
        local_xml,
        b"anchor",
        layout,
        master,
        |source| source.vertical_anchor,
        |value| frame.vertical_anchor = Some(value),
    );
    inherit_body_property(
        local_xml,
        b"vert",
        layout,
        master,
        |source| source.vertical_type.clone(),
        |value| frame.vertical_type = Some(value),
    );
    inherit_body_property(
        local_xml,
        b"rot",
        layout,
        master,
        |source| source.rotation,
        |value| frame.rotation = Some(value),
    );

    if !text_body_has_autofit(local_xml) {
        if let Some(source) = [layout, master].into_iter().flatten().find(|shape| {
            text_body_has_autofit(shape.tx_body_xml.as_deref()) && shape.text_frame().is_some()
        }) {
            if let Some(source_frame) = source.text_frame() {
                frame.auto_size = source_frame.auto_size;
                frame.font_scale = source_frame.font_scale;
            }
        }
    }

    let layout_frame = layout.and_then(AutoShape::text_frame);
    let master_frame = master.and_then(AutoShape::text_frame);
    for paragraph in frame.paragraphs_mut() {
        let layout_paragraph = layout_frame
            .and_then(|source| paragraph_for_level(source, paragraph.level))
            .cloned();
        let master_paragraph = master_frame
            .and_then(|source| paragraph_for_level(source, paragraph.level))
            .cloned();
        inherit_paragraph(
            paragraph,
            layout_paragraph.as_ref(),
            master_paragraph.as_ref(),
        );
    }
}

fn inherit_body_property<T: Clone>(
    local_xml: Option<&[u8]>,
    attribute: &[u8],
    layout: Option<&AutoShape>,
    master: Option<&AutoShape>,
    get: impl Fn(&TextFrame) -> Option<T>,
    set: impl FnOnce(T),
) {
    if text_body_has_body_pr_attribute(local_xml, attribute) {
        return;
    }
    for source in [layout, master].into_iter().flatten() {
        if !text_body_has_body_pr_attribute(source.tx_body_xml.as_deref(), attribute) {
            continue;
        }
        if let Some(value) = source.text_frame().and_then(&get) {
            set(value);
            return;
        }
    }
}

fn text_body_has_body_pr_attribute(xml: Option<&[u8]>, wanted: &[u8]) -> bool {
    let Some(xml) = xml else {
        return false;
    };
    let mut reader = Reader::from_reader(xml);
    loop {
        match reader.read_event() {
            Ok(Event::Start(element) | Event::Empty(element))
                if local_name(element.name().as_ref()) == b"bodyPr" =>
            {
                return element
                    .attributes()
                    .with_checks(false)
                    .flatten()
                    .any(|attribute| local_name(attribute.key.as_ref()) == wanted);
            }
            Ok(Event::Eof) | Err(_) => return false,
            _ => {}
        }
    }
}

fn text_body_has_autofit(xml: Option<&[u8]>) -> bool {
    let Some(xml) = xml else {
        return false;
    };
    let mut reader = Reader::from_reader(xml);
    loop {
        match reader.read_event() {
            Ok(Event::Start(element) | Event::Empty(element)) => {
                if matches!(
                    local_name(element.name().as_ref()),
                    b"normAutofit" | b"spAutoFit" | b"noAutofit"
                ) {
                    return true;
                }
            }
            Ok(Event::Eof) | Err(_) => return false,
            _ => {}
        }
    }
}

fn paragraph_for_level(frame: &TextFrame, level: u8) -> Option<&Paragraph> {
    frame
        .paragraphs()
        .iter()
        .find(|paragraph| paragraph.level == level)
        .or_else(|| frame.paragraphs().first())
}

fn inherit_paragraph(
    target: &mut Paragraph,
    layout: Option<&Paragraph>,
    master: Option<&Paragraph>,
) {
    if target.alignment.is_none() {
        target.alignment = layout
            .and_then(|paragraph| paragraph.alignment)
            .or_else(|| master.and_then(|paragraph| paragraph.alignment));
    }
    if target.space_before.is_none() {
        target.space_before = layout
            .and_then(|paragraph| paragraph.space_before)
            .or_else(|| master.and_then(|paragraph| paragraph.space_before));
    }
    if target.space_after.is_none() {
        target.space_after = layout
            .and_then(|paragraph| paragraph.space_after)
            .or_else(|| master.and_then(|paragraph| paragraph.space_after));
    }
    if target.line_spacing.is_none() {
        target.line_spacing = layout
            .and_then(|paragraph| paragraph.line_spacing)
            .or_else(|| master.and_then(|paragraph| paragraph.line_spacing));
    }
    if target.bullet.is_none() {
        target.bullet = layout
            .and_then(|paragraph| paragraph.bullet.clone())
            .or_else(|| master.and_then(|paragraph| paragraph.bullet.clone()));
    }
    if target.bullet_color.is_none() {
        target.bullet_color = layout
            .and_then(|paragraph| paragraph.bullet_color.clone())
            .or_else(|| master.and_then(|paragraph| paragraph.bullet_color.clone()));
    }
    if target.bullet_font.is_none() {
        target.bullet_font = layout
            .and_then(|paragraph| paragraph.bullet_font.clone())
            .or_else(|| master.and_then(|paragraph| paragraph.bullet_font.clone()));
    }
    if target.bullet_size_pct.is_none() {
        target.bullet_size_pct = layout
            .and_then(|paragraph| paragraph.bullet_size_pct)
            .or_else(|| master.and_then(|paragraph| paragraph.bullet_size_pct));
    }
    if target.bullet_size_pts.is_none() {
        target.bullet_size_pts = layout
            .and_then(|paragraph| paragraph.bullet_size_pts)
            .or_else(|| master.and_then(|paragraph| paragraph.bullet_size_pts));
    }
    if target.text_direction.is_none() {
        target.text_direction = layout
            .and_then(|paragraph| paragraph.text_direction)
            .or_else(|| master.and_then(|paragraph| paragraph.text_direction));
    }

    let layout_font = layout.and_then(paragraph_style_font);
    let master_font = master.and_then(paragraph_style_font);
    let mut inherited = target.font.clone().unwrap_or_default();
    inherit_font(&mut inherited, layout_font.as_ref(), master_font.as_ref());
    if font_has_properties(&inherited) {
        target.font = Some(inherited);
    }
}

fn paragraph_style_font(paragraph: &Paragraph) -> Option<Font> {
    let mut font = paragraph
        .runs()
        .first()
        .map(|run| run.font().clone())
        .unwrap_or_default();
    inherit_font(&mut font, paragraph.font.as_ref(), None);
    font_has_properties(&font).then_some(font)
}

fn inherit_font(target: &mut Font, layout: Option<&Font>, master: Option<&Font>) {
    macro_rules! inherit {
        ($field:ident) => {
            if target.$field.is_none() {
                target.$field = layout
                    .and_then(|font| font.$field.clone())
                    .or_else(|| master.and_then(|font| font.$field.clone()));
            }
        };
    }
    inherit!(name);
    inherit!(east_asia_name);
    inherit!(complex_script_name);
    inherit!(symbol_name);
    inherit!(size);
    inherit!(bold);
    inherit!(italic);
    inherit!(underline);
    inherit!(color);
    inherit!(strikethrough);
    inherit!(subscript);
    inherit!(superscript);
    inherit!(language_id);
    inherit!(fill);
    inherit!(hyperlink);
}

fn font_has_properties(font: &Font) -> bool {
    font.name.is_some()
        || font.east_asia_name.is_some()
        || font.complex_script_name.is_some()
        || font.symbol_name.is_some()
        || font.size.is_some()
        || font.bold.is_some()
        || font.italic.is_some()
        || font.underline.is_some()
        || font.color.is_some()
        || font.strikethrough.is_some()
        || font.subscript.is_some()
        || font.superscript.is_some()
        || font.language_id.is_some()
        || font.fill.is_some()
        || font.hyperlink.is_some()
}

#[derive(Debug, Default, PartialEq, Eq)]
struct BackgroundSpec {
    color: Option<String>,
    image_relationship_id: Option<String>,
}

fn resolve_slide_background(
    presentation: &Presentation,
    slide_part_name: &PackURI,
    theme: &ThemeColorScheme,
    asset_sink: &mut AssetSink,
) -> (String, Option<String>) {
    let mut part_name = slide_part_name.clone();
    for next_relationship in [Some(RT_SLIDE_LAYOUT), Some(RT_SLIDE_MASTER), None] {
        let Some(part) = presentation.package().part(&part_name) else {
            break;
        };
        if let Some(spec) = parse_background_spec(&part.blob, theme) {
            let asset = spec.image_relationship_id.and_then(|relationship_id| {
                let target = part.related_partname(&relationship_id).ok()?;
                let image = presentation.package().part(&target)?;
                Some(asset_sink.insert(&image.content_type, &image.blob))
            });
            return (spec.color.unwrap_or_else(|| "#ffffff".into()), asset);
        }
        let Some(relationship_type) = next_relationship else {
            break;
        };
        let Ok(relationship) = part.rels.by_reltype(relationship_type) else {
            break;
        };
        let Ok(next_part_name) = relationship.target_partname(part.partname.base_uri()) else {
            break;
        };
        part_name = next_part_name;
    }
    ("#ffffff".into(), None)
}

fn parse_background_spec(xml: &[u8], theme: &ThemeColorScheme) -> Option<BackgroundSpec> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut in_background = false;
    let mut spec = BackgroundSpec::default();
    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                let local = local_name(element.name().as_ref()).to_vec();
                if local == b"bg" {
                    in_background = true;
                } else if in_background {
                    update_background_spec(&mut spec, &local, &element, theme);
                }
            }
            Ok(Event::Empty(element)) if in_background => {
                update_background_spec(
                    &mut spec,
                    local_name(element.name().as_ref()),
                    &element,
                    theme,
                );
            }
            Ok(Event::End(element)) if local_name(element.name().as_ref()) == b"bg" => {
                return Some(spec);
            }
            Ok(Event::Eof) | Err(_) => return None,
            _ => {}
        }
    }
}

fn update_background_spec(
    spec: &mut BackgroundSpec,
    local: &[u8],
    element: &BytesStart<'_>,
    theme: &ThemeColorScheme,
) {
    match local {
        b"blip" => spec.image_relationship_id = xml_attr(element, b"embed"),
        b"srgbClr" => {
            spec.color = xml_attr(element, b"val").map(|value| format!("#{value}"));
        }
        b"sysClr" => {
            spec.color = xml_attr(element, b"lastClr").map(|value| format!("#{value}"));
        }
        b"schemeClr" => {
            spec.color = xml_attr(element, b"val")
                .and_then(|name| theme.by_name(&name))
                .map(|color| color.to_string());
        }
        _ => {}
    }
}

fn xml_attr(element: &BytesStart<'_>, wanted: &[u8]) -> Option<String> {
    element
        .attributes()
        .with_checks(false)
        .flatten()
        .find_map(|attribute| {
            (local_name(attribute.key.as_ref()) == wanted)
                .then(|| String::from_utf8_lossy(attribute.value.as_ref()).into_owned())
        })
}

fn local_name(name: &[u8]) -> &[u8] {
    name.rsplit(|byte| *byte == b':').next().unwrap_or(name)
}

/// Parse native DrawingML tables directly from the slide XML.  The vendor
/// shape tree intentionally exposes only `has_table`; keeping this projection
/// here lets the editor render real content without replacing the source
/// `p:graphicFrame` or any unknown extension markup.
#[cfg(test)]
fn parse_slide_tables(xml: &[u8], scale: f64, theme: &ThemeColorScheme) -> HashMap<u32, TableData> {
    parse_slide_tables_with_font_policy(xml, scale, theme, &FontPolicy::default())
}

fn parse_slide_tables_with_font_policy(
    xml: &[u8],
    scale: f64,
    theme: &ThemeColorScheme,
    font_policy: &FontPolicy,
) -> HashMap<u32, TableData> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut capture: Option<(Writer<Vec<u8>>, usize)> = None;
    let mut tables = HashMap::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                if let Some((writer, depth)) = capture.as_mut() {
                    let _ = writer.write_event(Event::Start(element.to_owned()));
                    *depth += 1;
                } else if local_name(element.name().as_ref()) == b"graphicFrame" {
                    let mut writer = Writer::new(Vec::new());
                    let _ = writer.write_event(Event::Start(element.to_owned()));
                    capture = Some((writer, 1));
                }
            }
            Ok(Event::Empty(element)) => {
                if let Some((writer, _)) = capture.as_mut() {
                    let _ = writer.write_event(Event::Empty(element.to_owned()));
                }
            }
            Ok(Event::Text(text)) => {
                if let Some((writer, _)) = capture.as_mut() {
                    let _ = writer.write_event(Event::Text(text.to_owned()));
                }
            }
            Ok(Event::CData(text)) => {
                if let Some((writer, _)) = capture.as_mut() {
                    let _ = writer.write_event(Event::CData(text.to_owned()));
                }
            }
            Ok(Event::Comment(text)) => {
                if let Some((writer, _)) = capture.as_mut() {
                    let _ = writer.write_event(Event::Comment(text.to_owned()));
                }
            }
            Ok(Event::End(element)) => {
                if let Some((writer, depth)) = capture.as_mut() {
                    let _ = writer.write_event(Event::End(element.to_owned()));
                    *depth = depth.saturating_sub(1);
                    if *depth == 0 {
                        if let Some((writer, _)) = capture.take() {
                            if let Some((shape_id, table)) = parse_table_graphic_frame(
                                &writer.into_inner(),
                                scale,
                                theme,
                                font_policy,
                            ) {
                                tables.insert(shape_id, table);
                            }
                        }
                    }
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    tables
}

fn parse_table_graphic_frame(
    xml: &[u8],
    scale: f64,
    theme: &ThemeColorScheme,
    font_policy: &FontPolicy,
) -> Option<(u32, TableData)> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut shape_id = None;
    let mut table = TableData {
        columns: Vec::new(),
        rows: Vec::new(),
        first_row: false,
        first_col: false,
        last_row: false,
        last_col: false,
        band_rows: false,
        band_cols: false,
        style_id: None,
    };
    let mut in_table = false;
    let mut reading_style_id = false;
    let mut style_id = String::new();
    let mut row: Option<TableRow> = None;
    let mut cell_capture: Option<(Writer<Vec<u8>>, usize)> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                if let Some((writer, depth)) = cell_capture.as_mut() {
                    let _ = writer.write_event(Event::Start(element.to_owned()));
                    *depth += 1;
                    continue;
                }
                let element_name = element.name();
                let local = local_name(element_name.as_ref());
                match local {
                    b"cNvPr" if shape_id.is_none() => {
                        shape_id = xml_attr(&element, b"id").and_then(|value| value.parse().ok());
                    }
                    b"tbl" => in_table = true,
                    b"tblPr" if in_table => apply_table_flags(&mut table, &element),
                    b"tableStyleId" | b"tblStyleId" if in_table => {
                        reading_style_id = true;
                        style_id.clear();
                    }
                    b"gridCol" if in_table => {
                        if let Some(width) = xml_i64(&element, b"w") {
                            table.columns.push((width as f64 * scale).max(0.0));
                        }
                    }
                    b"tr" if in_table => {
                        row = Some(TableRow {
                            height: xml_i64(&element, b"h")
                                .map_or(0.0, |height| height as f64 * scale),
                            cells: Vec::new(),
                        });
                    }
                    b"tc" if in_table && row.is_some() => {
                        let mut writer = Writer::new(Vec::new());
                        let _ = writer.write_event(Event::Start(element.to_owned()));
                        cell_capture = Some((writer, 1));
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(element)) => {
                if let Some((writer, _)) = cell_capture.as_mut() {
                    let _ = writer.write_event(Event::Empty(element.to_owned()));
                    continue;
                }
                let element_name = element.name();
                let local = local_name(element_name.as_ref());
                match local {
                    b"cNvPr" if shape_id.is_none() => {
                        shape_id = xml_attr(&element, b"id").and_then(|value| value.parse().ok());
                    }
                    b"tblPr" if in_table => apply_table_flags(&mut table, &element),
                    b"gridCol" if in_table => {
                        if let Some(width) = xml_i64(&element, b"w") {
                            table.columns.push((width as f64 * scale).max(0.0));
                        }
                    }
                    b"tc" if in_table && row.is_some() => {
                        if let Some(current) = row.as_mut() {
                            current.cells.push(empty_table_cell());
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(text)) => {
                if let Some((writer, _)) = cell_capture.as_mut() {
                    let _ = writer.write_event(Event::Text(text.to_owned()));
                } else if reading_style_id {
                    if let Ok(value) = text.decode() {
                        style_id.push_str(&value);
                    }
                }
            }
            Ok(Event::CData(text)) => {
                if let Some((writer, _)) = cell_capture.as_mut() {
                    let _ = writer.write_event(Event::CData(text.to_owned()));
                }
            }
            Ok(Event::End(element)) => {
                if let Some((writer, depth)) = cell_capture.as_mut() {
                    let _ = writer.write_event(Event::End(element.to_owned()));
                    *depth = depth.saturating_sub(1);
                    if *depth == 0 {
                        if let Some((writer, _)) = cell_capture.take() {
                            if let Some(current) = row.as_mut() {
                                current.cells.push(parse_table_cell(
                                    &writer.into_inner(),
                                    scale,
                                    theme,
                                    font_policy,
                                ));
                            }
                        }
                    }
                    continue;
                }
                match local_name(element.name().as_ref()) {
                    b"tableStyleId" | b"tblStyleId" if reading_style_id => {
                        reading_style_id = false;
                        if !style_id.trim().is_empty() {
                            table.style_id = Some(style_id.trim().to_string());
                        }
                    }
                    b"tr" if in_table => {
                        if let Some(row) = row.take() {
                            table.rows.push(row);
                        }
                    }
                    b"tbl" => in_table = false,
                    _ => {}
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    if table.rows.is_empty() {
        return None;
    }
    if table.columns.is_empty() {
        let columns = table
            .rows
            .iter()
            .map(|row| row.cells.len())
            .max()
            .unwrap_or(1)
            .max(1);
        table.columns = vec![1.0; columns];
    }
    apply_table_style_fallback(&mut table, theme);
    shape_id.map(|shape_id| (shape_id, table))
}

fn apply_table_flags(table: &mut TableData, element: &BytesStart<'_>) {
    table.first_row = xml_bool(element, b"firstRow");
    table.first_col = xml_bool(element, b"firstCol");
    table.last_row = xml_bool(element, b"lastRow");
    table.last_col = xml_bool(element, b"lastCol");
    table.band_rows = xml_bool(element, b"bandRow");
    table.band_cols = xml_bool(element, b"bandCol");
}

fn parse_table_cell(
    xml: &[u8],
    scale: f64,
    theme: &ThemeColorScheme,
    font_policy: &FontPolicy,
) -> TableCell {
    let mut cell = empty_table_cell();
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    loop {
        match reader.read_event() {
            Ok(Event::Start(element) | Event::Empty(element))
                if local_name(element.name().as_ref()) == b"tc" =>
            {
                cell.grid_span = xml_u32(&element, b"gridSpan").unwrap_or(1).max(1);
                cell.row_span = xml_u32(&element, b"rowSpan").unwrap_or(1).max(1);
                cell.h_merge = xml_bool(&element, b"hMerge");
                cell.v_merge = xml_bool(&element, b"vMerge");
                break;
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    if let Some(tx_body) = capture_xml_fragment(xml, b"txBody") {
        if let Ok(Some(frame)) = pptx::shapes::parser::parse_text_frame_from_xml(&tx_body) {
            cell.text = frame.text();
            cell.text_style = text_style_with_font_policy(&frame, theme, font_policy);
            cell.text_paragraphs =
                rich_text_with_font_policy(&frame, &cell.text_style, theme, font_policy);
            cell.text_frame = text_frame_style(&frame, scale);
        }
    }
    let (fill, borders) = parse_table_cell_format(xml, scale, theme);
    cell.fill = fill;
    cell.borders = borders;
    cell
}

fn empty_table_cell() -> TableCell {
    TableCell {
        text: String::new(),
        text_paragraphs: Vec::new(),
        text_frame: TextFrameStyle::default(),
        text_style: TextStyle::default(),
        fill: "transparent".into(),
        borders: TableCellBorders::default(),
        grid_span: 1,
        row_span: 1,
        h_merge: false,
        v_merge: false,
    }
}

fn capture_xml_fragment(xml: &[u8], wanted: &[u8]) -> Option<Vec<u8>> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut capture: Option<(Writer<Vec<u8>>, usize)> = None;
    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                if let Some((writer, depth)) = capture.as_mut() {
                    writer.write_event(Event::Start(element.to_owned())).ok()?;
                    *depth += 1;
                } else if local_name(element.name().as_ref()) == wanted {
                    let mut writer = Writer::new(Vec::new());
                    writer.write_event(Event::Start(element.to_owned())).ok()?;
                    capture = Some((writer, 1));
                }
            }
            Ok(Event::Empty(element)) => {
                if let Some((writer, _)) = capture.as_mut() {
                    writer.write_event(Event::Empty(element.to_owned())).ok()?;
                } else if local_name(element.name().as_ref()) == wanted {
                    let mut writer = Writer::new(Vec::new());
                    writer.write_event(Event::Empty(element.to_owned())).ok()?;
                    return Some(writer.into_inner());
                }
            }
            Ok(Event::Text(text)) => {
                if let Some((writer, _)) = capture.as_mut() {
                    writer.write_event(Event::Text(text.to_owned())).ok()?;
                }
            }
            Ok(Event::CData(text)) => {
                if let Some((writer, _)) = capture.as_mut() {
                    writer.write_event(Event::CData(text.to_owned())).ok()?;
                }
            }
            Ok(Event::End(element)) => {
                if let Some((writer, depth)) = capture.as_mut() {
                    writer.write_event(Event::End(element.to_owned())).ok()?;
                    *depth = depth.saturating_sub(1);
                    if *depth == 0 {
                        let (writer, _) = capture.take()?;
                        return Some(writer.into_inner());
                    }
                }
            }
            Ok(Event::Eof) | Err(_) => return None,
            _ => {}
        }
    }
}

fn parse_table_cell_format(
    xml: &[u8],
    scale: f64,
    theme: &ThemeColorScheme,
) -> (String, TableCellBorders) {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut stack = Vec::<Vec<u8>>::new();
    let mut fill = "transparent".to_string();
    let mut borders = TableCellBorders::default();

    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                apply_cell_format_element(&element, &stack, scale, theme, &mut fill, &mut borders);
                stack.push(local_name(element.name().as_ref()).to_vec());
            }
            Ok(Event::Empty(element)) => {
                apply_cell_format_element(&element, &stack, scale, theme, &mut fill, &mut borders)
            }
            Ok(Event::End(_)) => {
                stack.pop();
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    (fill, borders)
}

fn apply_cell_format_element(
    element: &BytesStart<'_>,
    stack: &[Vec<u8>],
    scale: f64,
    theme: &ThemeColorScheme,
    fill: &mut String,
    borders: &mut TableCellBorders,
) {
    if !stack.iter().any(|value| value.as_slice() == b"tcPr") {
        return;
    }
    let element_name = element.name();
    let local = local_name(element_name.as_ref());
    if matches!(local, b"lnL" | b"lnR" | b"lnT" | b"lnB") {
        let width = xml_i64(element, b"w").map_or(1.0, |value| (value as f64 * scale).max(0.5));
        *table_border_mut(borders, local) = Some(TableCellBorder {
            color: "#D9D9D9".into(),
            width,
            dash: None,
        });
        return;
    }
    let active_border = stack
        .iter()
        .rev()
        .find(|value| matches!(value.as_slice(), b"lnL" | b"lnR" | b"lnT" | b"lnB"))
        .map(Vec::as_slice);
    if local == b"prstDash" {
        if let (Some(side), Some(value)) = (active_border, xml_attr(element, b"val")) {
            if let Some(border) = table_border_mut(borders, side).as_mut() {
                border.dash = Some(value);
            }
        }
        return;
    }
    if matches!(local, b"srgbClr" | b"schemeClr" | b"sysClr")
        && stack.iter().any(|value| value.as_slice() == b"solidFill")
    {
        if let Some(color) = table_xml_color(element, theme) {
            if let Some(side) = active_border {
                if let Some(border) = table_border_mut(borders, side).as_mut() {
                    border.color = color;
                }
            } else {
                *fill = color;
            }
        }
    }
}

fn table_border_mut<'a>(
    borders: &'a mut TableCellBorders,
    side: &[u8],
) -> &'a mut Option<TableCellBorder> {
    match side {
        b"lnL" => &mut borders.left,
        b"lnR" => &mut borders.right,
        b"lnT" => &mut borders.top,
        _ => &mut borders.bottom,
    }
}

fn table_xml_color(element: &BytesStart<'_>, theme: &ThemeColorScheme) -> Option<String> {
    match local_name(element.name().as_ref()) {
        b"srgbClr" => xml_attr(element, b"val").map(|value| format!("#{value}")),
        b"sysClr" => xml_attr(element, b"lastClr").map(|value| format!("#{value}")),
        b"schemeClr" => xml_attr(element, b"val")
            .and_then(|name| theme.by_name(&name))
            .map(|value| value.to_string()),
        _ => None,
    }
}

fn apply_table_style_fallback(table: &mut TableData, theme: &ThemeColorScheme) {
    let accent = theme
        .by_name("accent1")
        .map(|value| value.to_string())
        .unwrap_or_else(|| "#4472C4".into());
    let band = theme
        .by_name("accent1")
        .map(|value| apply_brightness(value.r, value.g, value.b, Some(0.84)))
        .unwrap_or_else(|| "#D9E2F3".into());
    let styled = table.style_id.is_some();
    let row_count = table.rows.len();
    for (row_index, row) in table.rows.iter_mut().enumerate() {
        for (column_index, cell) in row.cells.iter_mut().enumerate() {
            let is_header = table.first_row && row_index == 0;
            let is_footer = table.last_row && row_index + 1 == row_count;
            let is_first_column = table.first_col && column_index == 0;
            let is_last_column = table.last_col && column_index + 1 == table.columns.len();
            if styled && cell.fill == "transparent" {
                if is_header || is_footer {
                    cell.fill.clone_from(&accent);
                    set_table_cell_text(cell, "#FFFFFF", true);
                } else if (table.band_rows && row_index % 2 == usize::from(table.first_row))
                    || (table.band_cols && column_index % 2 == 1)
                {
                    cell.fill.clone_from(&band);
                } else {
                    cell.fill = "#FFFFFF".into();
                }
                if is_first_column || is_last_column {
                    cell.text_style.bold = true;
                    for paragraph in &mut cell.text_paragraphs {
                        for run in &mut paragraph.runs {
                            run.bold = true;
                        }
                    }
                }
            }
            let border = || TableCellBorder {
                color: "#D9D9D9".into(),
                width: 1.0,
                dash: None,
            };
            cell.borders.left.get_or_insert_with(border);
            cell.borders.right.get_or_insert_with(border);
            cell.borders.top.get_or_insert_with(border);
            cell.borders.bottom.get_or_insert_with(border);
        }
    }
}

fn set_table_cell_text(cell: &mut TableCell, color: &str, bold: bool) {
    cell.text_style.color = color.into();
    cell.text_style.bold = bold;
    for paragraph in &mut cell.text_paragraphs {
        for run in &mut paragraph.runs {
            run.color = color.into();
            run.bold = bold;
        }
    }
}

fn xml_bool(element: &BytesStart<'_>, wanted: &[u8]) -> bool {
    xml_attr(element, wanted)
        .is_some_and(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "on"))
}

fn xml_i64(element: &BytesStart<'_>, wanted: &[u8]) -> Option<i64> {
    xml_attr(element, wanted)?.parse().ok()
}

fn xml_u32(element: &BytesStart<'_>, wanted: &[u8]) -> Option<u32> {
    xml_attr(element, wanted)?.parse().ok()
}

#[cfg(test)]
fn convert_shape(
    shape: &Shape,
    slide_index: usize,
    z: usize,
    scale: f64,
    theme: &ThemeColorScheme,
) -> SceneObject {
    convert_shape_with_font_policy(
        shape,
        slide_index,
        z,
        scale,
        theme,
        &FontPolicy::default(),
        &mut AssetSink::inline(),
    )
}

fn convert_shape_with_font_policy(
    shape: &Shape,
    slide_index: usize,
    z: usize,
    scale: f64,
    theme: &ThemeColorScheme,
    font_policy: &FontPolicy,
    asset_sink: &mut AssetSink,
) -> SceneObject {
    convert_shape_with_group_fill(
        shape,
        slide_index,
        z,
        scale,
        theme,
        font_policy,
        None,
        None,
        asset_sink,
    )
}

#[allow(clippy::too_many_arguments)]
fn convert_shape_with_group_fill(
    shape: &Shape,
    slide_index: usize,
    z: usize,
    scale: f64,
    theme: &ThemeColorScheme,
    font_policy: &FontPolicy,
    inherited_group_fill: Option<&FillFormat>,
    frame_override: Option<Frame>,
    asset_sink: &mut AssetSink,
) -> SceneObject {
    let shape_id = shape.shape_id().0;
    let mut object = SceneObject {
        id: format!("shape-{}-{shape_id}-{z}", slide_index + 1),
        source_shape_id: Some(shape_id),
        name: shape.name().to_string(),
        kind: ObjectKind::Unknown,
        frame: frame_override.unwrap_or(Frame {
            x: shape.left().0 as f64 * scale,
            y: shape.top().0 as f64 * scale,
            width: shape.width().0 as f64 * scale,
            height: shape.height().0 as f64 * scale,
            rotation: shape.rotation(),
        }),
        flip_h: false,
        flip_v: false,
        text: String::new(),
        text_paragraphs: vec![],
        text_frame: TextFrameStyle::default(),
        geometry: None,
        custom_geometry: None,
        asset: None,
        image_crop: ImageCrop::default(),
        image_fill_rect: ImageFillRect::default(),
        image_effects: crate::model::ImageEffects::default(),
        shape_fill_asset: None,
        formula: None,
        media: None,
        hyperlinks: crate::model::ObjectHyperlinks::default(),
        table: None,
        chart: None,
        style: VisualStyle::default(),
        text_style: TextStyle::default(),
        children: vec![],
    };

    match shape {
        Shape::AutoShape(auto_shape) => apply_auto_shape(
            &mut object,
            auto_shape,
            scale,
            theme,
            font_policy,
            inherited_group_fill,
            asset_sink,
        ),
        Shape::Picture(picture) => {
            object.kind = ObjectKind::Image;
            object.flip_h = picture.flip_h;
            object.flip_v = picture.flip_v;
            object.image_effects.soft_edge_radius = picture
                .soft_edge_radius
                .map_or(0.0, |radius| radius.0 as f64 * scale);
            object.image_crop = ImageCrop {
                left: picture.crop_left.clamp(0.0, 1.0),
                top: picture.crop_top.clamp(0.0, 1.0),
                right: picture.crop_right.clamp(0.0, 1.0),
                bottom: picture.crop_bottom.clamp(0.0, 1.0),
            };
            object.image_effects.duotone = picture
                .duotone
                .as_ref()
                .and_then(|effect| duotone_style(effect, theme));
            object.geometry = picture
                .auto_shape_type
                .as_ref()
                .map(|value| format!("{value:?}"));
            if let (Some(data), Some(mime)) = (&picture.image_data, &picture.image_content_type) {
                object.asset = Some(asset_sink.insert_shared(mime, data.clone()));
            }
            object.style = line_style(picture.line.as_ref(), scale, theme);
            object.style.shadow = picture
                .shadow
                .as_ref()
                .map(|shadow| shadow_style(shadow, scale, theme));
        }
        Shape::GraphicFrame(frame) => {
            object.kind = if frame.has_table {
                ObjectKind::Table
            } else if frame.has_chart {
                ObjectKind::Chart
            } else if frame.has_smartart() {
                ObjectKind::SmartArt
            } else {
                ObjectKind::Unknown
            };
        }
        Shape::GroupShape(group) => {
            object.kind = ObjectKind::Group;
            let group_fill = match group.fill.as_ref() {
                Some(FillFormat::Background) | None => inherited_group_fill,
                Some(fill) => Some(fill),
            };
            let child_width = group.child_width.0;
            let child_height = group.child_height.0;
            let child_scale_x = if child_width == 0 {
                scale
            } else {
                object.frame.width / child_width as f64
            };
            let child_scale_y = if child_height == 0 {
                scale
            } else {
                object.frame.height / child_height as f64
            };
            object.children = group
                .shapes
                .iter()
                .enumerate()
                .map(|(child_z, child)| {
                    let child_frame = Frame {
                        x: (child.left().0 - group.child_left.0) as f64 * child_scale_x,
                        y: (child.top().0 - group.child_top.0) as f64 * child_scale_y,
                        width: child.width().0 as f64 * child_scale_x,
                        height: child.height().0 as f64 * child_scale_y,
                        rotation: child.rotation(),
                    };
                    convert_shape_with_group_fill(
                        child,
                        slide_index,
                        z * 1_000 + child_z,
                        // Group child coordinates live in the group's arbitrary
                        // chOff/chExt space, but DrawingML style measurements
                        // (text insets, line widths, shadows, soft edges, ...)
                        // remain EMUs.  Applying the child-coordinate ratio to
                        // those values can inflate a normal 9.6 px text inset
                        // into thousands of pixels and make the text disappear.
                        // Keep the document EMU -> CSS scale for styles while
                        // `child_frame` handles the group geometry transform.
                        scale,
                        theme,
                        font_policy,
                        group_fill,
                        Some(child_frame),
                        asset_sink,
                    )
                })
                .collect();
        }
        Shape::Connector(connector) => {
            object.kind = ObjectKind::Connector;
            object.geometry = connector
                .prst_geom
                .as_ref()
                .map(|value| format!("{value:?}"));
            object.style = line_style(connector.line.as_ref(), scale, theme);
        }
        Shape::OleObject(_) => object.kind = ObjectKind::Ole,
        _ => object.kind = ObjectKind::Unknown,
    }
    object
}

fn apply_auto_shape(
    object: &mut SceneObject,
    shape: &AutoShape,
    scale: f64,
    theme: &ThemeColorScheme,
    font_policy: &FontPolicy,
    inherited_group_fill: Option<&FillFormat>,
    asset_sink: &mut AssetSink,
) {
    object.text = shape.text_frame().map(TextFrame::text).unwrap_or_default();
    object.kind = if shape.is_textbox || (!object.text.is_empty() && shape.fill.is_none()) {
        ObjectKind::Text
    } else {
        ObjectKind::Shape
    };
    object.geometry = shape.prst_geom.as_ref().map(|value| format!("{value:?}"));
    object.custom_geometry =
        shape
            .custom_geometry
            .as_ref()
            .map(|geometry| crate::model::CustomGeometry {
                width: geometry.width(),
                height: geometry.height(),
                path_data: geometry.to_svg_path_data(),
            });
    let effective_fill = match shape.fill.as_ref() {
        Some(FillFormat::Background) => inherited_group_fill,
        fill => fill,
    };
    object.style = shape_style(
        effective_fill,
        shape.line.as_ref(),
        shape.shadow.as_ref(),
        scale,
        theme,
    );
    if let Some(FillFormat::Picture(fill)) = effective_fill {
        if let Some(crop) = fill.source_rect {
            object.image_crop = ImageCrop {
                left: (crop.left as f64 / 100_000.0).clamp(0.0, 1.0),
                top: (crop.top as f64 / 100_000.0).clamp(0.0, 1.0),
                right: (crop.right as f64 / 100_000.0).clamp(0.0, 1.0),
                bottom: (crop.bottom as f64 / 100_000.0).clamp(0.0, 1.0),
            };
        }
        if let Some(rect) = fill.fill_rect {
            object.image_fill_rect = ImageFillRect {
                left: rect.left as f64 / 100_000.0,
                top: rect.top as f64 / 100_000.0,
                right: rect.right as f64 / 100_000.0,
                bottom: rect.bottom as f64 / 100_000.0,
            };
        }
        if let (Some(data), Some(mime)) = (&fill.image_data, &fill.image_content_type) {
            let asset = asset_sink.insert_shared(mime, data.clone());
            object.asset = Some(asset.clone());
            object.shape_fill_asset = Some(asset.clone());
            object.style.fill = if fill.tile {
                format!(r#"url("{asset}") repeat"#)
            } else {
                format!(r#"url("{asset}") center / 100% 100% no-repeat"#)
            };
        }
    }
    if let Some(frame) = shape.text_frame() {
        object.text_style = text_style_with_font_policy(frame, theme, font_policy);
        object.text_paragraphs =
            rich_text_with_font_policy(frame, &object.text_style, theme, font_policy);
        object.text_frame = text_frame_style(frame, scale);
    }
    if let Some((omml, display)) = shape.tx_body_xml.as_deref().and_then(extract_omml_fragment) {
        object.kind = ObjectKind::Math;
        object.formula = Some(FormulaData {
            latex: String::new(),
            omml: Some(omml),
            display,
        });
    }
}

fn extract_omml_fragment(xml: &[u8]) -> Option<(String, bool)> {
    let source = std::str::from_utf8(xml).ok()?;
    for (tag, display) in [("m:oMathPara", true), ("m:oMath", false)] {
        let open = format!("<{tag}");
        let Some(start) = source.find(&open) else {
            continue;
        };
        let after_open = &source[start..];
        let Some(open_end) = after_open.find('>') else {
            continue;
        };
        if after_open[..=open_end].trim_end().ends_with("/>") {
            return Some((after_open[..=open_end].to_string(), display));
        }
        let close = format!("</{tag}>");
        let Some(end_start) = after_open.find(&close) else {
            continue;
        };
        let end = end_start + close.len();
        return Some((after_open[..end].to_string(), display));
    }
    None
}

fn native_font_family(
    run: Option<&Font>,
    paragraph: Option<&Font>,
    resolved_stack: &str,
) -> Option<String> {
    let raw = run
        .and_then(|font| {
            font.east_asia_name
                .as_deref()
                .or(font.name.as_deref())
                .or(font.complex_script_name.as_deref())
        })
        .or_else(|| {
            paragraph.and_then(|font| {
                font.east_asia_name
                    .as_deref()
                    .or(font.name.as_deref())
                    .or(font.complex_script_name.as_deref())
            })
        })
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.starts_with('+'));
    Some(raw.map_or_else(|| primary_family(resolved_stack), str::to_string))
}

fn native_font_slots(run: Option<&Font>, paragraph: Option<&Font>) -> FontSlots {
    FontSlots {
        latin: run
            .and_then(|font| font.name.clone())
            .or_else(|| paragraph.and_then(|font| font.name.clone())),
        east_asia: run
            .and_then(|font| font.east_asia_name.clone())
            .or_else(|| paragraph.and_then(|font| font.east_asia_name.clone())),
        complex_script: run
            .and_then(|font| font.complex_script_name.clone())
            .or_else(|| paragraph.and_then(|font| font.complex_script_name.clone())),
        symbol: run
            .and_then(|font| font.symbol_name.clone())
            .or_else(|| paragraph.and_then(|font| font.symbol_name.clone())),
        language_id: run
            .and_then(|font| font.language_id.clone())
            .or_else(|| paragraph.and_then(|font| font.language_id.clone())),
    }
}

fn text_style_with_font_policy(
    frame: &TextFrame,
    theme: &ThemeColorScheme,
    font_policy: &FontPolicy,
) -> TextStyle {
    let mut style = TextStyle::default();
    if let Some(paragraph) = frame.paragraphs().first() {
        style.align = paragraph
            .alignment
            .map(|alignment| match alignment.to_xml_str() {
                "ctr" => "center",
                "r" => "right",
                "just" => "justify",
                "justLow" => "justifyLow",
                "dist" => "distributed",
                "thaiDist" => "thaiDistributed",
                _ => "left",
            })
            .unwrap_or("left")
            .to_string();
        let run_font = paragraph.runs().first().map(|run| run.font());
        let paragraph_font = paragraph.font.as_ref();
        let sample_text =
            paragraph
                .runs()
                .first()
                .map_or("", |run| if run.is_line_break { "\n" } else { run.text() });
        let fallback_family = style.font_family.clone();
        style.font_family =
            font_policy.css_stack(run_font, paragraph_font, sample_text, &fallback_family);
        style.native_font_family = native_font_family(run_font, paragraph_font, &style.font_family);
        style.native_fonts = native_font_slots(run_font, paragraph_font);
        if run_font.is_some() || paragraph_font.is_some() {
            if let Some(size) = run_font
                .and_then(|font| font.size)
                .or_else(|| paragraph_font.and_then(|font| font.size))
            {
                style.font_size = size * 96.0 / 72.0;
            }
            if let Some(color) = run_font
                .and_then(|font| font_color(font, theme))
                .or_else(|| paragraph_font.and_then(|font| font_color(font, theme)))
            {
                style.color = color;
            }
            style.bold = run_font
                .and_then(|font| font.bold)
                .or_else(|| paragraph_font.and_then(|font| font.bold))
                .unwrap_or(false);
            style.italic = run_font
                .and_then(|font| font.italic)
                .or_else(|| paragraph_font.and_then(|font| font.italic))
                .unwrap_or(false);
        }
    }
    style
}

#[cfg(test)]
fn rich_text(
    frame: &TextFrame,
    fallback: &TextStyle,
    theme: &ThemeColorScheme,
) -> Vec<RichTextParagraph> {
    rich_text_with_font_policy(frame, fallback, theme, &FontPolicy::default())
}

fn rich_text_with_font_policy(
    frame: &TextFrame,
    fallback: &TextStyle,
    theme: &ThemeColorScheme,
    font_policy: &FontPolicy,
) -> Vec<RichTextParagraph> {
    let font_scale = frame.font_scale.unwrap_or(100.0) / 100.0;
    frame
        .paragraphs()
        .iter()
        .enumerate()
        .map(|(paragraph_index, paragraph)| {
            let paragraph_font = paragraph.font.as_ref();
            let runs = paragraph
                .runs()
                .iter()
                .enumerate()
                .map(|(run_index, run)| {
                    let font = run.font();
                    let text: String = if run.is_line_break {
                        "\n".into()
                    } else {
                        run.text().into()
                    };
                    let font_family = font_policy.css_stack(
                        Some(font),
                        paragraph_font,
                        &text,
                        &fallback.font_family,
                    );
                    let native_font_family =
                        native_font_family(Some(font), paragraph_font, &font_family);
                    let native_fonts = native_font_slots(Some(font), paragraph_font);
                    let font_size = font
                        .size
                        .or_else(|| paragraph_font.and_then(|value| value.size))
                        .map_or(fallback.font_size, |points| points * 96.0 / 72.0)
                        * font_scale;
                    let color = font_color(font, theme)
                        .or_else(|| paragraph_font.and_then(|font| font_color(font, theme)))
                        .unwrap_or_else(|| fallback.color.clone());
                    let underline_style = font
                        .underline
                        .or_else(|| paragraph_font.and_then(|value| value.underline))
                        .map(|value| value.to_xml_str().to_string());
                    RichTextRun {
                        source_index: Some(run_index),
                        text,
                        font_family,
                        native_font_family,
                        native_fonts,
                        font_size,
                        color,
                        gradient: font_gradient(font, theme).or_else(|| {
                            paragraph_font.and_then(|value| font_gradient(value, theme))
                        }),
                        bold: font
                            .bold
                            .or_else(|| paragraph_font.and_then(|value| value.bold))
                            .unwrap_or(fallback.bold),
                        italic: font
                            .italic
                            .or_else(|| paragraph_font.and_then(|value| value.italic))
                            .unwrap_or(fallback.italic),
                        underline: underline_style
                            .as_deref()
                            .is_some_and(|value| value != "none"),
                        underline_style,
                        // The current pptx vendor model retains the effective
                        // colour but does not expose DrawingML alpha transforms.
                        // Keep the loss-aware field at its neutral value until
                        // the parser can provide the source transform.
                        alpha: 1.0,
                        strikethrough: font
                            .strikethrough
                            .or_else(|| paragraph_font.and_then(|value| value.strikethrough))
                            .unwrap_or(false),
                        baseline: if font.superscript == Some(true) {
                            "super".into()
                        } else if font.subscript == Some(true) {
                            "sub".into()
                        } else {
                            "normal".into()
                        },
                        hyperlinks: crate::model::ObjectHyperlinks::default(),
                        baseline_offset: font.baseline_offset
                            .or_else(|| paragraph_font.and_then(|f| f.baseline_offset))
                            .filter(|v| !matches!(*v, 0 | -25000 | 30000))
                            .map(|v| v as f64 / 1000.0),
                    }
                })
                .collect();
            let bullet = match paragraph.bullet.as_ref() {
                Some(BulletFormat::Character(value)) => Some(value.to_string()),
                Some(BulletFormat::AutoNumbered(_)) => Some("1.".into()),
                Some(BulletFormat::Picture(_)) => Some("•".into()),
                Some(BulletFormat::None) | None => None,
                Some(_) => Some("•".into()),
            };
            RichTextParagraph {
                source_index: Some(paragraph_index),
                runs,
                align: paragraph_alignment(paragraph.alignment),
                level: paragraph.level,
                bullet,
                line_spacing: paragraph.line_spacing,
                space_before: paragraph.space_before.map(|value| value * 96.0 / 72.0),
                space_after: paragraph.space_after.map(|value| value * 96.0 / 72.0),
            }
        })
        .collect()
}

fn text_frame_style(frame: &TextFrame, scale: f64) -> TextFrameStyle {
    let vertical_align = match frame.vertical_anchor.map(|value| value.to_xml_str()) {
        Some("t") => "top",
        Some("b") => "bottom",
        _ => "center",
    };
    TextFrameStyle {
        margin_left: frame
            .margin_left
            .map_or(0.0, |value| value.0 as f64 * scale),
        margin_right: frame
            .margin_right
            .map_or(0.0, |value| value.0 as f64 * scale),
        margin_top: frame.margin_top.map_or(0.0, |value| value.0 as f64 * scale),
        margin_bottom: frame
            .margin_bottom
            .map_or(0.0, |value| value.0 as f64 * scale),
        vertical_align: vertical_align.into(),
        vertical_type: frame
            .vertical_type
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "horz".into()),
        word_wrap: frame.word_wrap,
        auto_size: match frame.auto_size.to_xml_str() {
            "normAutofit" => "textToFitShape",
            "spAutoFit" => "shapeToFitText",
            _ => "none",
        }
        .into(),
    }
}

fn paragraph_alignment(alignment: Option<pptx::enums::text::PpParagraphAlignment>) -> String {
    alignment
        .map(|value| match value.to_xml_str() {
            "ctr" => "center",
            "r" => "right",
            "just" => "justify",
            "justLow" => "justifyLow",
            "dist" => "distributed",
            "thaiDist" => "thaiDistributed",
            _ => "left",
        })
        .unwrap_or("left")
        .into()
}

fn font_color(font: &Font, theme: &ThemeColorScheme) -> Option<String> {
    font.color.map(|value| value.to_string()).or_else(|| {
        font.fill.as_ref().and_then(|fill| match fill {
            FillFormat::Solid(solid) => css_color(&solid.color, theme),
            _ => None,
        })
    })
}

fn font_gradient(font: &Font, theme: &ThemeColorScheme) -> Option<GradientFillStyle> {
    let FillFormat::Gradient(gradient) = font.fill.as_ref()? else {
        return None;
    };
    let stops = gradient
        .stops
        .iter()
        .filter_map(|stop| {
            css_color(&stop.color, theme).map(|color| GradientStopStyle {
                position: stop.position.clamp(0.0, 1.0),
                color,
                opacity: stop.opacity.clamp(0.0, 1.0),
            })
        })
        .collect::<Vec<_>>();
    (!stops.is_empty()).then(|| GradientFillStyle {
        angle: (90.0 - gradient.angle.unwrap_or(0.0)).rem_euclid(360.0),
        stops,
    })
}

fn shape_style(
    fill: Option<&FillFormat>,
    line: Option<&LineFormat>,
    shadow: Option<&ShadowFormat>,
    scale: f64,
    theme: &ThemeColorScheme,
) -> VisualStyle {
    let mut style = line_style(line, scale, theme);
    style.fill = match fill {
        Some(FillFormat::NoFill) | None => "transparent".into(),
        Some(FillFormat::Solid(solid)) => {
            let color = css_color(&solid.color, theme).unwrap_or_else(|| "#dbe5ff".into());
            css_color_with_alpha(&color, solid.opacity)
        }
        Some(FillFormat::Gradient(gradient)) => {
            let css_angle = (90.0 - gradient.angle.unwrap_or(0.0)).rem_euclid(360.0);
            let stops = gradient
                .stops
                .iter()
                .filter_map(|stop| {
                    css_color(&stop.color, theme).map(|color| GradientStopStyle {
                        position: stop.position.clamp(0.0, 1.0),
                        color,
                        opacity: stop.opacity.clamp(0.0, 1.0),
                    })
                })
                .collect::<Vec<_>>();
            style.gradient = Some(GradientFillStyle {
                angle: css_angle,
                stops: stops.clone(),
            });
            if stops.is_empty() {
                "transparent".into()
            } else {
                let stop_css = stops
                    .iter()
                    .map(|stop| {
                        format!(
                            "{} {:.3}%",
                            css_color_with_alpha(&stop.color, stop.opacity),
                            stop.position * 100.0
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("linear-gradient({css_angle:.3}deg, {stop_css})")
            }
        }
        Some(FillFormat::Pattern(pattern)) => {
            let foreground = pattern
                .fore_color
                .as_ref()
                .and_then(|color| css_color(color, theme))
                .unwrap_or_else(|| "#7f8ca5".into());
            let background = pattern
                .back_color
                .as_ref()
                .and_then(|color| css_color(color, theme))
                .unwrap_or_else(|| "#ffffff".into());
            format!(
                "repeating-linear-gradient(45deg, {foreground} 0, {foreground} 3px, {background} 3px, {background} 6px)"
            )
        }
        Some(FillFormat::Picture(_)) | Some(FillFormat::Background) => "transparent".into(),
        Some(_) => "transparent".into(),
    };
    style.shadow = shadow.map(|shadow| shadow_style(shadow, scale, theme));
    style
}

fn line_style(line: Option<&LineFormat>, scale: f64, theme: &ThemeColorScheme) -> VisualStyle {
    let mut style = VisualStyle::default();
    if let Some(line) = line {
        if matches!(line.fill, Some(FillFormat::NoFill)) {
            style.stroke = "transparent".into();
            style.stroke_width = 0.0;
            style.stroke_dash = None;
            return style;
        }
        style.stroke = line
            .color
            .as_ref()
            .and_then(|color| css_color(color, theme))
            .unwrap_or_else(|| "#52617c".into());
        style.stroke_width = line
            .width
            .map_or(1.0, |width| (width.0 as f64 * scale).max(1.0));
        style.stroke_dash = line.dash_style.map(|dash| dash.to_xml_str().to_string());
    }
    style
}

fn shadow_style(shadow: &ShadowFormat, scale: f64, theme: &ThemeColorScheme) -> ShadowStyle {
    let distance = shadow.distance.map_or(0.0, |value| value.0 as f64 * scale);
    let angle = shadow.direction.unwrap_or(0.0).to_radians();
    ShadowStyle {
        color: shadow
            .color
            .as_ref()
            .and_then(|color| css_color(color, theme))
            .unwrap_or_else(|| "#000000".into()),
        opacity: shadow.opacity.unwrap_or(0.35).clamp(0.0, 1.0),
        offset_x: distance * angle.cos(),
        offset_y: distance * angle.sin(),
        blur: shadow
            .blur_radius
            .map_or(0.0, |value| value.0 as f64 * scale),
        inset: matches!(shadow.shadow_type, ShadowType::Inner),
    }
}

fn css_color(color: &ColorFormat, theme: &ThemeColorScheme) -> Option<String> {
    match color {
        ColorFormat::Rgb(rgb) => Some(rgb.to_string()),
        ColorFormat::System(system) => system.last_color.as_ref().map(|value| format!("#{value}")),
        ColorFormat::Theme(value) => theme
            .by_name(value.theme_color.to_xml_str())
            .map(|rgb| apply_brightness(rgb.r, rgb.g, rgb.b, value.brightness)),
        ColorFormat::Hsl(value) => Some(format!(
            "hsl({:.3}deg {:.3}% {:.3}%)",
            value.hue, value.saturation, value.luminance
        )),
        ColorFormat::Preset(value) => match value.val.to_xml_str() {
            "white" => Some("#FFFFFF".into()),
            "black" => Some("#000000".into()),
            _ => None,
        },
        _ => None,
    }
}

fn duotone_style(effect: &DuotoneEffect, theme: &ThemeColorScheme) -> Option<DuotoneStyle> {
    let mut colors = effect
        .colors
        .iter()
        .filter_map(|color| duotone_color(color, theme));
    Some(DuotoneStyle {
        shadow_color: colors.next()?,
        highlight_color: colors.next()?,
    })
}

fn duotone_color(color: &DuotoneColor, theme: &ThemeColorScheme) -> Option<String> {
    let css = css_color(&color.color, theme)?;
    let hex = css.strip_prefix('#').filter(|value| value.len() == 6)?;
    let mut rgb = [
        f64::from(u8::from_str_radix(&hex[0..2], 16).ok()?) / 255.0,
        f64::from(u8::from_str_radix(&hex[2..4], 16).ok()?) / 255.0,
        f64::from(u8::from_str_radix(&hex[4..6], 16).ok()?) / 255.0,
    ];
    for transform in &color.transforms {
        match transform {
            DuotoneColorTransform::Shade(value) => rgb
                .iter_mut()
                .for_each(|channel| *channel *= value.clamp(0.0, 1.0)),
            DuotoneColorTransform::SaturationModulation(value) => {
                let max = rgb.iter().copied().fold(0.0, f64::max);
                let min = rgb.iter().copied().fold(1.0, f64::min);
                let lightness = (max + min) / 2.0;
                let saturation = if (max - min).abs() < f64::EPSILON {
                    0.0
                } else {
                    (max - min) / (1.0 - (2.0 * lightness - 1.0).abs())
                };
                let gray = lightness;
                let factor = (saturation * value).clamp(0.0, 1.0) / saturation.max(f64::EPSILON);
                rgb.iter_mut().for_each(|channel| {
                    *channel = (gray + (*channel - gray) * factor).clamp(0.0, 1.0)
                });
            }
        }
    }
    Some(format!(
        "#{:02X}{:02X}{:02X}",
        (rgb[0] * 255.0).round() as u8,
        (rgb[1] * 255.0).round() as u8,
        (rgb[2] * 255.0).round() as u8
    ))
}

fn apply_brightness(red: u8, green: u8, blue: u8, brightness: Option<f64>) -> String {
    let brightness = brightness.unwrap_or(0.0).clamp(-1.0, 1.0);
    let channel = |value: u8| {
        let value = f64::from(value);
        let adjusted = if brightness >= 0.0 {
            value + (255.0 - value) * brightness
        } else {
            value * (1.0 + brightness)
        };
        adjusted.round().clamp(0.0, 255.0) as u8
    };
    format!(
        "#{:02X}{:02X}{:02X}",
        channel(red),
        channel(green),
        channel(blue)
    )
}

fn css_color_with_alpha(color: &str, alpha: f64) -> String {
    if alpha >= 0.999 {
        return color.to_string();
    }
    let Some(hex) = color.strip_prefix('#').filter(|value| value.len() == 6) else {
        return color.to_string();
    };
    let red = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let green = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let blue = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    format!("rgba({red}, {green}, {blue}, {:.4})", alpha.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn themed_xml(accent1: &str, minor_latin: &str) -> Vec<u8> {
        format!(
            r#"<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
              <a:themeElements>
                <a:clrScheme name="Test">
                  <a:dk1><a:srgbClr val="000000"/></a:dk1>
                  <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
                  <a:dk2><a:srgbClr val="111111"/></a:dk2>
                  <a:lt2><a:srgbClr val="EEEEEE"/></a:lt2>
                  <a:accent1><a:srgbClr val="{accent1}"/></a:accent1>
                  <a:accent2><a:srgbClr val="222222"/></a:accent2>
                  <a:accent3><a:srgbClr val="333333"/></a:accent3>
                  <a:accent4><a:srgbClr val="444444"/></a:accent4>
                  <a:accent5><a:srgbClr val="555555"/></a:accent5>
                  <a:accent6><a:srgbClr val="666666"/></a:accent6>
                  <a:hlink><a:srgbClr val="0000FF"/></a:hlink>
                  <a:folHlink><a:srgbClr val="800080"/></a:folHlink>
                </a:clrScheme>
                <a:fontScheme name="Test">
                  <a:majorFont><a:latin typeface="Heading Test"/><a:ea typeface=""/><a:cs typeface=""/></a:majorFont>
                  <a:minorFont><a:latin typeface="{minor_latin}"/><a:ea typeface=""/><a:cs typeface=""/></a:minorFont>
                </a:fontScheme>
              </a:themeElements>
            </a:theme>"#
        )
        .into_bytes()
    }

    #[test]
    fn parses_color_and_font_policy_from_each_concrete_theme() {
        let first = resolved_theme_from_xml(&themed_xml("123456", "Body One"), &[]).unwrap();
        let second = resolved_theme_from_xml(&themed_xml("ABCDEF", "Body Two"), &[]).unwrap();

        assert_eq!(first.colors.accent1.to_string(), "#123456");
        assert_eq!(second.colors.accent1.to_string(), "#ABCDEF");
        let first_stack = first.fonts.css_stack(None, None, "Latin", "sans-serif");
        let second_stack = second.fonts.css_stack(None, None, "Latin", "sans-serif");
        assert!(first_stack.starts_with("'Body One'"), "{first_stack}");
        assert!(second_stack.starts_with("'Body Two'"), "{second_stack}");
    }

    #[test]
    fn paragraph_alignment_preserves_distribution_modes() {
        use pptx::enums::text::PpParagraphAlignment;

        assert_eq!(
            paragraph_alignment(Some(PpParagraphAlignment::Distribute)),
            "distributed"
        );
        assert_eq!(
            paragraph_alignment(Some(PpParagraphAlignment::ThaiDistribute)),
            "thaiDistributed"
        );
        assert_eq!(
            paragraph_alignment(Some(PpParagraphAlignment::JustifyLow)),
            "justifyLow"
        );
        assert_eq!(
            paragraph_alignment(Some(PpParagraphAlignment::Justify)),
            "justify"
        );
    }

    #[test]
    fn text_frame_projection_preserves_east_asian_vertical_mode() {
        let mut frame = TextFrame::new();
        frame.vertical_type = Some("eaVert".into());
        let style = text_frame_style(&frame, 1.0);
        assert_eq!(style.vertical_type, "eaVert");
    }

    #[test]
    fn explicit_no_fill_line_stays_invisible() {
        let line = LineFormat {
            width: Some(pptx::units::Emu(38_100)),
            fill: Some(FillFormat::NoFill),
            ..LineFormat::default()
        };
        let style = line_style(Some(&line), 1.0, &ThemeColorScheme::default());
        assert_eq!(style.stroke, "transparent");
        assert_eq!(style.stroke_width, 0.0);
        assert_eq!(style.stroke_dash, None);
    }

    #[test]
    fn demo_serializes_and_round_trips() {
        let demo = Deck::demo();
        let json = serde_json::to_string(&demo).unwrap();
        let restored: Deck = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, demo);
    }

    #[test]
    fn direct_rgb_becomes_css_hex() {
        let theme = ThemeColorScheme::default();
        assert_eq!(
            css_color(&ColorFormat::rgb(10, 32, 255), &theme).as_deref(),
            Some("#0A20FF")
        );
    }

    #[test]
    fn rich_text_tracks_source_identity_and_underline_style() {
        let mut frame = TextFrame::new();
        let first_paragraph = &mut frame.paragraphs_mut()[0];
        let first_run = first_paragraph.add_run();
        first_run.set_text("First");
        first_run.font_mut().underline =
            Some(pptx::enums::text::MsoTextUnderlineType::WavyDoubleLine);
        let second_run = first_paragraph.add_run();
        second_run.set_text(" run");

        let second_paragraph = frame.add_paragraph();
        second_paragraph.add_run().set_text("Second");

        let paragraphs = rich_text(&frame, &TextStyle::default(), &ThemeColorScheme::default());
        assert_eq!(paragraphs[0].source_index, Some(0));
        assert_eq!(paragraphs[1].source_index, Some(1));
        assert_eq!(paragraphs[0].runs[0].source_index, Some(0));
        assert_eq!(paragraphs[0].runs[1].source_index, Some(1));
        assert!(paragraphs[0].runs[0].underline);
        assert_eq!(
            paragraphs[0].runs[0].underline_style.as_deref(),
            Some("wavyDbl")
        );
        assert_eq!(paragraphs[0].runs[0].alpha, 1.0);
    }

    #[test]
    fn legacy_rich_text_json_uses_loss_aware_defaults() {
        let run: RichTextRun = serde_json::from_str(
            r##"{"text":"legacy","fontFamily":"Arial","fontSize":16.0,"color":"#112233","bold":false,"italic":false}"##,
        )
        .unwrap();

        assert_eq!(run.source_index, None);
        assert_eq!(run.underline_style, None);
        assert_eq!(run.alpha, 1.0);
    }

    #[test]
    fn nonstandard_script_offsets_survive_vendor_xml_and_scene_projection() {
        let mut frame = TextFrame::new();
        let run = frame.paragraphs_mut()[0].add_run();
        run.set_text("0");
        run.font_mut().subscript = Some(true);
        run.font_mut().baseline_offset = Some(-47500);
        let paragraphs = rich_text(&frame, &TextStyle::default(), &ThemeColorScheme::default());
        assert_eq!(paragraphs[0].runs[0].baseline, "sub");
        assert_eq!(paragraphs[0].runs[0].baseline_offset, Some(-47.5));
    }

    #[test]
    fn extracts_display_and_inline_omml() {
        let display = br#"<p:txBody><m:oMathPara><m:oMath><m:r><m:t>x</m:t></m:r></m:oMath></m:oMathPara></p:txBody>"#;
        let (xml, is_display) = extract_omml_fragment(display).unwrap();
        assert!(is_display);
        assert!(xml.starts_with("<m:oMathPara>"));

        let inline = br#"<p:txBody><m:oMath><m:r><m:t>y</m:t></m:r></m:oMath></p:txBody>"#;
        let (xml, is_display) = extract_omml_fragment(inline).unwrap();
        assert!(!is_display);
        assert!(xml.starts_with("<m:oMath>"));
    }

    #[test]
    fn parses_picture_and_solid_slide_backgrounds() {
        let theme = ThemeColorScheme::default();
        let picture = br#"<p:sld xmlns:p="p" xmlns:a="a" xmlns:r="r"><p:cSld><p:bg><p:bgPr><a:blipFill><a:blip r:embed="rId4"/></a:blipFill></p:bgPr></p:bg></p:cSld></p:sld>"#;
        assert_eq!(
            parse_background_spec(picture, &theme),
            Some(BackgroundSpec {
                color: None,
                image_relationship_id: Some("rId4".into()),
            })
        );

        let solid = br#"<p:sld xmlns:p="p" xmlns:a="a"><p:cSld><p:bg><p:bgPr><a:solidFill><a:srgbClr val="F2E4DF"/></a:solidFill></p:bgPr></p:bg></p:cSld></p:sld>"#;
        assert_eq!(
            parse_background_spec(solid, &theme),
            Some(BackgroundSpec {
                color: Some("#F2E4DF".into()),
                image_relationship_id: None,
            })
        );

        let themed = br#"<p:sld xmlns:p="p" xmlns:a="a"><p:cSld><p:bg><p:bgPr><a:solidFill><a:schemeClr val="accent2"/></a:solidFill></p:bgPr></p:bg></p:cSld></p:sld>"#;
        assert_eq!(
            parse_background_spec(themed, &theme),
            Some(BackgroundSpec {
                color: Some(theme.accent2.to_string()),
                image_relationship_id: None,
            })
        );
    }

    #[test]
    fn native_gradient_and_shadow_become_browser_styles() {
        use pptx::dml::effect::ShadowFormat;
        use pptx::dml::fill::{GradientFill, GradientStop};
        use pptx::units::Emu;

        let fill = FillFormat::Gradient(GradientFill {
            stops: vec![
                GradientStop::new(0.0, ColorFormat::rgb(255, 0, 0)).unwrap(),
                GradientStop::new(1.0, ColorFormat::rgb(0, 0, 255)).unwrap(),
            ],
            angle: Some(45.0),
        });
        let mut shadow =
            ShadowFormat::outer(ColorFormat::rgb(0, 0, 0), Emu(25_400), Emu(12_700), 90.0);
        shadow.opacity = Some(0.5);
        let style = shape_style(
            Some(&fill),
            None,
            Some(&shadow),
            1.0 / 9_525.0,
            &ThemeColorScheme::default(),
        );

        assert!(style.fill.starts_with("linear-gradient(45.000deg"));
        assert_eq!(style.gradient.as_ref().unwrap().stops.len(), 2);
        let rendered_shadow = style.shadow.unwrap();
        assert!((rendered_shadow.offset_y - 1.333_333).abs() < 0.001);
        assert_eq!(rendered_shadow.opacity, 0.5);
    }

    #[test]
    fn shape_fill_alpha_is_projected_without_fading_text_or_stroke() {
        let theme = ThemeColorScheme::default();
        let fill = FillFormat::solid_with_opacity(ColorFormat::rgb(17, 34, 51), 0.7);
        let style = shape_style(Some(&fill), None, None, 1.0, &theme);

        assert_eq!(style.fill, "rgba(17, 34, 51, 0.7000)");
        assert_eq!(style.opacity, 1.0);
        assert_eq!(style.stroke, "transparent");
    }

    #[test]
    fn gradient_stop_alpha_is_projected_to_css_and_structured_style() {
        use pptx::dml::fill::{GradientFill, GradientStop};

        let theme = ThemeColorScheme::default();
        let fill = FillFormat::Gradient(GradientFill {
            stops: vec![
                GradientStop::with_opacity(0.25, ColorFormat::rgb(17, 34, 51), 0.42).unwrap(),
            ],
            angle: Some(0.0),
        });
        let style = shape_style(Some(&fill), None, None, 1.0, &theme);

        assert!(style.fill.contains("rgba(17, 34, 51, 0.4200) 25.000%"));
        assert_eq!(style.gradient.unwrap().stops[0].opacity, 0.42);
        assert_eq!(style.opacity, 1.0);
    }

    #[test]
    fn auto_shape_picture_fill_becomes_browser_background() {
        use pptx::dml::fill::{PictureFill, PictureSourceRect};
        use pptx::units::{Emu, RelationshipId, ShapeId};

        let mut auto_shape = AutoShape::new(
            ShapeId(12),
            "Picture fill",
            Emu(0),
            Emu(0),
            Emu(914_400),
            Emu(914_400),
        );
        auto_shape.fill = Some(FillFormat::Picture(PictureFill {
            image_r_id: RelationshipId::try_from("rId7").unwrap(),
            source_rect: Some(PictureSourceRect {
                left: 1000,
                top: 2000,
                right: 3000,
                bottom: 4000,
            }),
            fill_rect: Some(PictureSourceRect {
                left: -15048,
                top: -15048,
                right: -15048,
                bottom: -15048,
            }),
            stretch: true,
            tile: false,
            image_data: Some(vec![1, 2, 3, 4].into()),
            image_content_type: Some("image/png".into()),
        }));

        let object = convert_shape(
            &Shape::AutoShape(Box::new(auto_shape)),
            0,
            0,
            1.0,
            &ThemeColorScheme::default(),
        );
        let expected = "data:image/png;base64,AQIDBA==";
        assert_eq!(object.kind, ObjectKind::Shape);
        assert_eq!(object.asset.as_deref(), Some(expected));
        assert_eq!(object.shape_fill_asset.as_deref(), Some(expected));
        assert_eq!(
            object.style.fill,
            format!(r#"url("{expected}") center / 100% 100% no-repeat"#)
        );
        assert!((object.image_fill_rect.left + 0.15048).abs() < f64::EPSILON);
        assert!((object.image_fill_rect.top + 0.15048).abs() < f64::EPSILON);
        assert!((object.image_fill_rect.right + 0.15048).abs() < f64::EPSILON);
        assert!((object.image_fill_rect.bottom + 0.15048).abs() < f64::EPSILON);
    }

    #[test]
    fn picture_source_crop_projects_to_browser_scene() {
        let xml = br#"<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
        <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
        <p:pic><p:nvPicPr><p:cNvPr id="4" name="Cropped picture"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr>
        <p:blipFill><a:blip r:embed="rId2"/><a:srcRect l="18887" t="9703" r="16572" b="34374"/><a:stretch><a:fillRect/></a:stretch></p:blipFill>
        <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr></p:pic>
        </p:spTree></p:cSld></p:sld>"#;
        let tree = ShapeTree::from_slide_xml(xml).unwrap();
        let object = convert_shape(&tree.shapes[0], 0, 0, 1.0, &ThemeColorScheme::default());

        assert_eq!(object.kind, ObjectKind::Image);
        assert!((object.image_crop.left - 0.18887).abs() < f64::EPSILON);
        assert!((object.image_crop.top - 0.09703).abs() < f64::EPSILON);
        assert!((object.image_crop.right - 0.16572).abs() < f64::EPSILON);
        assert!((object.image_crop.bottom - 0.34374).abs() < f64::EPSILON);
    }

    #[test]
    fn legacy_scene_json_defaults_shape_fill_asset() {
        let mut value = serde_json::to_value(Deck::demo()).unwrap();
        value["slides"][0]["objects"][0]
            .as_object_mut()
            .unwrap()
            .remove("shapeFillAsset");
        let restored: Deck = serde_json::from_value(value).unwrap();
        assert!(restored.slides[0].objects[0].shape_fill_asset.is_none());
    }

    #[test]
    fn group_fill_resolves_nearest_ancestor_recursively() {
        let tree = placeholder_tree(
            r#"<p:grpSp><p:nvGrpSpPr><p:cNvPr id="2" name="Outer"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
            <p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1000" cy="1000"/><a:chOff x="0" y="0"/><a:chExt cx="1000" cy="1000"/></a:xfrm><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></p:grpSpPr>
            <p:sp><p:nvSpPr><p:cNvPr id="3" name="Outer Child"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100" cy="100"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:grpFill/></p:spPr></p:sp>
            <p:grpSp><p:nvGrpSpPr><p:cNvPr id="4" name="Inner"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
            <p:grpSpPr><a:xfrm><a:off x="100" y="100"/><a:ext cx="500" cy="500"/><a:chOff x="0" y="0"/><a:chExt cx="500" cy="500"/></a:xfrm><a:solidFill><a:srgbClr val="0000FF"/></a:solidFill></p:grpSpPr>
            <p:sp><p:nvSpPr><p:cNvPr id="5" name="Inner Child"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100" cy="100"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:grpFill/></p:spPr></p:sp>
            </p:grpSp></p:grpSp>"#,
        );

        let outer = convert_shape(&tree.shapes[0], 0, 0, 1.0, &ThemeColorScheme::default());
        assert_eq!(outer.children[0].style.fill, "#FF0000");
        assert_eq!(outer.children[1].children[0].style.fill, "#0000FF");
    }

    #[test]
    fn nested_group_frames_compose_parent_and_child_coordinate_spaces() {
        let tree = placeholder_tree(
            r#"<p:grpSp><p:nvGrpSpPr><p:cNvPr id="2" name="Outer"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
            <p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="10000" cy="5000"/><a:chOff x="0" y="0"/><a:chExt cx="100" cy="50"/></a:xfrm></p:grpSpPr>
            <p:grpSp><p:nvGrpSpPr><p:cNvPr id="3" name="Inverse inner"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
            <p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100" cy="50"/><a:chOff x="0" y="0"/><a:chExt cx="10000" cy="5000"/></a:xfrm></p:grpSpPr>
            <p:sp><p:nvSpPr><p:cNvPr id="4" name="Leaf"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="2500" y="1000"/><a:ext cx="2500" cy="2000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr></p:sp>
            </p:grpSp></p:grpSp>"#,
        );

        let outer = convert_shape(&tree.shapes[0], 0, 0, 1.0, &ThemeColorScheme::default());
        let inner = &outer.children[0];
        let leaf = &inner.children[0];
        assert_eq!(inner.frame.width, 10_000.0);
        assert_eq!(inner.frame.height, 5_000.0);
        assert_eq!(leaf.frame.x, 2_500.0);
        assert_eq!(leaf.frame.y, 1_000.0);
        assert_eq!(leaf.frame.width, 2_500.0);
        assert_eq!(leaf.frame.height, 2_000.0);
    }

    #[test]
    fn group_coordinate_scale_does_not_scale_text_insets() {
        let tree = placeholder_tree(
            r#"<p:grpSp><p:nvGrpSpPr><p:cNvPr id="2" name="Scaled group"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
            <p:grpSpPr><a:xfrm><a:off x="1420495" y="1098550"/><a:ext cx="4293870" cy="4902835"/><a:chOff x="3303" y="2793"/><a:chExt cx="6762" cy="7721"/></a:xfrm></p:grpSpPr>
            <p:sp><p:nvSpPr><p:cNvPr id="3" name="Visible text"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
            <p:spPr><a:xfrm><a:off x="4409" y="2867"/><a:ext cx="4268" cy="4966"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/></p:spPr>
            <p:txBody><a:bodyPr wrap="none"><a:spAutoFit/></a:bodyPr><a:lstStyle/><a:p><a:r><a:rPr sz="19900"/><a:t>Visible</a:t></a:r></a:p></p:txBody>
            </p:sp></p:grpSp>"#,
        );

        let scale = 1.0 / 9_525.0;
        let group = convert_shape(&tree.shapes[0], 0, 0, scale, &ThemeColorScheme::default());
        let text = &group.children[0];
        assert!((group.frame.width - 450.8).abs() < 0.01);
        assert!((text.frame.width - 284.53).abs() < 0.01);
        assert!((text.text_frame.margin_left - 9.6).abs() < 0.01);
        assert!((text.text_frame.margin_right - 9.6).abs() < 0.01);
        assert!((text.text_frame.margin_top - 4.8).abs() < 0.01);
        assert!((text.text_frame.margin_bottom - 4.8).abs() < 0.01);
    }

    fn placeholder_tree(shape: &str) -> ShapeTree {
        let xml = format!(
            r#"<p:sld xmlns:p="p" xmlns:a="a"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>{shape}</p:spTree></p:cSld></p:sld>"#
        );
        ShapeTree::from_slide_xml(xml.as_bytes()).unwrap()
    }

    #[test]
    fn slide_placeholder_inherits_frame_and_missing_native_styles() {
        let slide = placeholder_tree(
            r#"<p:sp><p:nvSpPr><p:cNvPr id="42" name="Slide Content"/><p:cNvSpPr/><p:nvPr><p:ph idx="1"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr wrap="square"/><a:lstStyle/><a:p><a:r><a:rPr b="0"/><a:t>Slide text</a:t></a:r></a:p></p:txBody></p:sp>"#,
        );
        let layout = placeholder_tree(
            r#"<p:sp><p:nvSpPr><p:cNvPr id="7" name="Layout Body"/><p:cNvSpPr/><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr><p:spPr><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></p:spPr><p:txBody><a:bodyPr anchor="t"/><a:lstStyle/><a:p><a:pPr algn="ctr"><a:defRPr sz="3000" i="1"><a:latin typeface="Layout Font"/></a:defRPr></a:pPr></a:p></p:txBody></p:sp>"#,
        );
        let master = placeholder_tree(
            r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="Master Body"/><p:cNvSpPr/><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x="100" y="200"/><a:ext cx="3000" cy="2000"/></a:xfrm><a:solidFill><a:srgbClr val="0000FF"/></a:solidFill><a:ln w="12700"><a:solidFill><a:srgbClr val="112233"/></a:solidFill></a:ln><a:effectLst><a:outerShdw blurRad="25400" dist="12700" dir="5400000"><a:srgbClr val="000000"><a:alpha val="50000"/></a:srgbClr></a:outerShdw></a:effectLst></p:spPr><p:txBody><a:bodyPr wrap="none" lIns="111" anchor="b"/><a:lstStyle/><a:p><a:pPr algn="r"><a:defRPr sz="2400" b="1"><a:solidFill><a:srgbClr val="00AA00"/></a:solidFill></a:defRPr></a:pPr></a:p></p:txBody></p:sp>"#,
        );

        let resolved = resolve_slide_placeholder(&slide.shapes[0], Some(&layout), Some(&master));
        assert_eq!(resolved.shape_id().0, 42);
        assert_eq!(resolved.left().0, 100);
        assert_eq!(resolved.top().0, 200);
        assert_eq!(resolved.width().0, 3000);
        assert_eq!(resolved.height().0, 2000);

        let Shape::AutoShape(shape) = &resolved else {
            panic!("expected auto shape");
        };
        assert!(matches!(shape.fill, Some(FillFormat::Solid(_))));
        assert!(shape.line.is_some());
        assert!(shape.shadow.is_some());
        let frame = shape.text_frame().unwrap();
        assert!(frame.word_wrap, "slide-local wrap must win");
        assert_eq!(frame.margin_left, Some(pptx::units::Emu(111)));
        assert_eq!(
            frame.vertical_anchor,
            Some(pptx::enums::text::MsoVerticalAnchor::Top)
        );
        let paragraph = &frame.paragraphs()[0];
        assert_eq!(
            paragraph.alignment,
            Some(pptx::enums::text::PpParagraphAlignment::Center)
        );
        let inherited_font = paragraph.font.as_ref().unwrap();
        assert_eq!(inherited_font.size, Some(30.0));
        assert_eq!(inherited_font.italic, Some(true));
        assert_eq!(inherited_font.bold, Some(true));

        let object = convert_shape(&resolved, 0, 0, 1.0, &ThemeColorScheme::default());
        assert_eq!(object.source_shape_id, Some(42));
        assert_eq!(object.style.fill, "#FF0000");
        assert_eq!(object.style.stroke, "#112233");
        assert!(object.style.shadow.is_some());
        assert_eq!(object.text, "Slide text");
        assert!(!object.text_style.bold, "slide-local b=0 must win");
        assert!(object.text_style.italic);
        assert_eq!(object.text_style.font_size, 40.0);
        assert_eq!(object.text_paragraphs[0].runs[0].color, "#00AA00");
    }

    #[test]
    fn imports_mixed_script_font_slots_and_language_without_flattening() {
        let slide = placeholder_tree(
            r#"<p:sp><p:nvSpPr><p:cNvPr id="42" name="Mixed script"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1000" cy="1000"/></a:xfrm></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang="zh-CN"><a:latin typeface="Calibri"/><a:ea typeface="Microsoft YaHei"/><a:cs typeface="Arial"/><a:sym typeface="Wingdings"/></a:rPr><a:t>ABC中文</a:t></a:r><a:r><a:rPr lang="ar-SA"><a:latin typeface="Aptos"/><a:ea typeface="等线"/><a:cs typeface="Traditional Arabic"/><a:sym typeface="Symbol"/></a:rPr><a:t> العربية</a:t></a:r></a:p></p:txBody></p:sp>"#,
        );

        let object = convert_shape(&slide.shapes[0], 0, 0, 1.0, &ThemeColorScheme::default());
        let first = &object.text_paragraphs[0].runs[0];
        assert_eq!(first.native_fonts.latin.as_deref(), Some("Calibri"));
        assert_eq!(
            first.native_fonts.east_asia.as_deref(),
            Some("Microsoft YaHei")
        );
        assert_eq!(first.native_fonts.complex_script.as_deref(), Some("Arial"));
        assert_eq!(first.native_fonts.symbol.as_deref(), Some("Wingdings"));
        assert_eq!(first.native_fonts.language_id.as_deref(), Some("zh-CN"));

        let second = &object.text_paragraphs[0].runs[1];
        assert_eq!(second.native_fonts.latin.as_deref(), Some("Aptos"));
        assert_eq!(second.native_fonts.east_asia.as_deref(), Some("等线"));
        assert_eq!(
            second.native_fonts.complex_script.as_deref(),
            Some("Traditional Arabic")
        );
        assert_eq!(second.native_fonts.symbol.as_deref(), Some("Symbol"));
        assert_eq!(second.native_fonts.language_id.as_deref(), Some("ar-SA"));
    }

    #[test]
    fn master_placeholder_type_fallback_handles_different_indices() {
        let slide = placeholder_tree(
            r#"<p:sp><p:nvSpPr><p:cNvPr id="19" name="Slide Footer"/><p:cNvSpPr/><p:nvPr><p:ph type="ftr" idx="10"/></p:nvPr></p:nvSpPr><p:spPr/></p:sp>"#,
        );
        let layout = placeholder_tree(
            r#"<p:sp><p:nvSpPr><p:cNvPr id="8" name="Layout Footer"/><p:cNvSpPr/><p:nvPr><p:ph type="ftr" idx="10"/></p:nvPr></p:nvSpPr><p:spPr/></p:sp>"#,
        );
        let master = placeholder_tree(
            r#"<p:sp><p:nvSpPr><p:cNvPr id="5" name="Master Footer"/><p:cNvSpPr/><p:nvPr><p:ph type="ftr" idx="3"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x="77" y="88"/><a:ext cx="900" cy="100"/></a:xfrm></p:spPr></p:sp>"#,
        );

        let resolved = resolve_slide_placeholder(&slide.shapes[0], Some(&layout), Some(&master));
        assert_eq!(resolved.shape_id().0, 19);
        assert_eq!(resolved.left().0, 77);
        assert_eq!(resolved.top().0, 88);
        assert_eq!(resolved.width().0, 900);
        assert_eq!(resolved.height().0, 100);
    }

    #[test]
    fn parses_native_table_grid_rich_text_and_merge_topology() {
        let xml = br#"<p:sld xmlns:p="p" xmlns:a="a"><p:cSld><p:spTree>
          <p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="17" name="Sales Table"/></p:nvGraphicFramePr>
            <p:xfrm><a:off x="0" y="0"/><a:ext cx="3000000" cy="1200000"/></p:xfrm>
            <a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl>
              <a:tblPr firstRow="1" bandRow="1"><a:tableStyleId>{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}</a:tableStyleId></a:tblPr>
              <a:tblGrid><a:gridCol w="1000000"/><a:gridCol w="2000000"/></a:tblGrid>
              <a:tr h="400000">
                <a:tc gridSpan="2"><a:txBody><a:bodyPr anchor="ctr"/><a:lstStyle/><a:p><a:r><a:rPr sz="1800" b="1"/><a:t>Quarterly sales</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>
                <a:tc hMerge="1"><a:txBody><a:bodyPr/><a:lstStyle/><a:p/></a:txBody><a:tcPr/></a:tc>
              </a:tr>
              <a:tr h="800000">
                <a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Q1</a:t></a:r></a:p></a:txBody><a:tcPr><a:solidFill><a:srgbClr val="FFF2CC"/></a:solidFill><a:lnL w="12700"><a:solidFill><a:srgbClr val="AA3300"/></a:solidFill></a:lnL></a:tcPr></a:tc>
                <a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>128</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>
              </a:tr>
            </a:tbl></a:graphicData></a:graphic>
          </p:graphicFrame>
        </p:spTree></p:cSld></p:sld>"#;

        let tables = parse_slide_tables(xml, 0.001, &ThemeColorScheme::default());
        let table = tables.get(&17).expect("table identity must match cNvPr id");
        assert_eq!(table.columns, vec![1000.0, 2000.0]);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].height, 400.0);
        assert_eq!(table.rows[0].cells[0].text, "Quarterly sales");
        assert_eq!(table.rows[0].cells[0].grid_span, 2);
        assert!(table.rows[0].cells[1].h_merge);
        assert_eq!(table.rows[1].cells[0].fill, "#FFF2CC");
        assert_eq!(
            table.rows[1].cells[0].borders.left.as_ref().unwrap().color,
            "#AA3300"
        );
        assert!(table.first_row && table.band_rows);
        assert!(table
            .style_id
            .as_deref()
            .is_some_and(|id| id.contains("5C22544A")));
    }

    #[test]
    fn table_scene_geometry_survives_grid_extension_elements() {
        let xml = br#"<p:sld xmlns:p="p" xmlns:a="a"><p:cSld><p:spTree>
          <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>
          <p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="8" name="Table 7"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
            <p:xfrm><a:off x="263526" y="1566332"/><a:ext cx="11642924" cy="2995862"/></p:xfrm>
            <a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr/>
              <a:tblGrid><a:gridCol w="11642924"><a:extLst><a:ext uri="{vendor-column-id}"><a:colId val="20000"/></a:ext></a:extLst></a:gridCol></a:tblGrid>
              <a:tr h="2995862"><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>value</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:extLst><a:ext uri="{vendor-row-id}"><a:rowId val="10000"/></a:ext></a:extLst></a:tr>
            </a:tbl></a:graphicData></a:graphic>
          </p:graphicFrame>
        </p:spTree></p:cSld></p:sld>"#;
        let scale = 0.001;
        let tree = ShapeTree::from_slide_xml(xml).unwrap();
        let mut objects = tree
            .shapes
            .iter()
            .enumerate()
            .map(|(z, shape)| convert_shape(shape, 0, z, scale, &ThemeColorScheme::default()))
            .collect::<Vec<_>>();
        let mut tables = parse_slide_tables(xml, scale, &ThemeColorScheme::default());
        apply_slide_tables(&mut objects, &mut tables);

        let object = &objects[0];
        assert_eq!(object.source_shape_id, Some(8));
        assert!((object.frame.x - 263.526).abs() < 0.001);
        assert!((object.frame.y - 1566.332).abs() < 0.001);
        assert!((object.frame.width - 11642.924).abs() < 0.001);
        assert!((object.frame.height - 2995.862).abs() < 0.001);
        assert_eq!(
            object.table.as_ref().unwrap().rows[0].cells[0].text,
            "value"
        );
    }

    #[test]
    fn table_projection_ignores_other_graphic_frames_and_keeps_source_identity() {
        let xml = br#"<p:sld xmlns:p="p" xmlns:a="a" xmlns:c="c" xmlns:r="r"><p:cSld><p:spTree>
          <p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="5" name="Chart"/></p:nvGraphicFramePr><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart r:id="rId1"/></a:graphicData></a:graphic></p:graphicFrame>
          <p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="42" name="Table"/></p:nvGraphicFramePr><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr/><a:tblGrid><a:gridCol w="1"/></a:tblGrid><a:tr h="1"><a:tc rowSpan="2"><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>A</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc></a:tr><a:tr h="1"><a:tc vMerge="1"><a:txBody><a:bodyPr/><a:lstStyle/><a:p/></a:txBody><a:tcPr/></a:tc></a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame>
        </p:spTree></p:cSld></p:sld>"#;

        let tables = parse_slide_tables(xml, 1.0, &ThemeColorScheme::default());
        assert_eq!(tables.len(), 1);
        let table = tables
            .get(&42)
            .expect("table remains keyed by graphicFrame shape id");
        assert_eq!(table.rows[0].cells[0].row_span, 2);
        assert!(table.rows[1].cells[0].v_merge);
        assert!(!tables.contains_key(&5));
    }

    #[test]
    fn recursively_binds_grouped_table_without_changing_object_order() {
        let demo = Deck::demo();
        let mut before = demo.slides[0].objects[0].clone();
        before.id = "before".into();
        let mut table_object = demo.slides[0].objects[0].clone();
        table_object.id = "nested-table".into();
        table_object.kind = ObjectKind::Table;
        table_object.source_shape_id = Some(42);
        table_object.table = None;
        let mut group = demo.slides[0].objects[0].clone();
        group.id = "group".into();
        group.kind = ObjectKind::Group;
        group.children = vec![table_object];
        let mut after = demo.slides[0].objects[0].clone();
        after.id = "after".into();
        let mut objects = vec![before, group, after];

        let data = TableData {
            columns: vec![1.0],
            rows: vec![TableRow {
                height: 1.0,
                cells: vec![empty_table_cell()],
            }],
            first_row: false,
            first_col: false,
            last_row: false,
            last_col: false,
            band_rows: false,
            band_cols: false,
            style_id: None,
        };
        let mut tables = HashMap::from([(42, data.clone())]);
        apply_slide_tables(&mut objects, &mut tables);

        assert_eq!(
            objects
                .iter()
                .map(|object| object.id.as_str())
                .collect::<Vec<_>>(),
            vec!["before", "group", "after"]
        );
        assert_eq!(objects[1].children[0].table.as_ref(), Some(&data));
        assert!(tables.is_empty());
    }
}
