use std::collections::{HashMap, HashSet};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use pptx::chart::data::CategoryChartData;
use pptx::chart::xmlwriter::ChartXmlWriter;
use pptx::enums::chart::XlChartType;
use pptx::media::{Audio, Image, Video};
use pptx::opc::{PackURI, Part};
use pptx::slide::SlideRef;
use pptx::{Emu, Presentation, ShapeId, ShapeTree};
use thiserror::Error;

use crate::chart::ChartKind;
use crate::drawingml_text_edit::{
    make_body_pr, patch_shape_property_child, patch_shape_shadow, patch_structured_text, patch_text_frame,
    structured_text_body,
};
use crate::font_policy::primary_family;
use crate::importer::{import_pptx, ImportError};
use crate::model::{
    AnimationClass, AnimationEffect, AnimationKind, AnimationPropertyAnimation, AnimationTrigger,
    Deck, FontSlots, Frame, HyperlinkAction, MediaKind, ObjectKind, RichTextParagraph, RichTextRun,
    SceneObject, Slide, SlideTransition, TextStyle, VisualStyle,
};

const IMAGE_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";
const HYPERLINK_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink";
const SLIDE_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide";
const PACKAGE_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/package";
const OFFICE_DOCUMENT_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
const CUSTOM_XML_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/customXml";
const UNIPPT_EXTENSIONS_PART: &str = "/ppt/customXml/unippt-extensions.xml";
const LEGACY_UNIPPT_EXTENSIONS_PART: &str = "/customXml/unippt-extensions.xml";
const DRAWING_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const PRESENTATION_NS: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
const RELATIONSHIP_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const MATH_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/math";
const MC_NS: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
const A14_NS: &str = "http://schemas.microsoft.com/office/drawing/2010/main";

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("PPTX 写入失败: {0}")]
    Pptx(#[from] pptx::PptxError),
    #[error("源演示文稿重新解析失败: {0}")]
    Import(#[from] ImportError),
    #[error("演示文稿至少需要一张幻灯片")]
    EmptyDeck,
    #[error("源幻灯片被重复引用: {0}")]
    DuplicateSlideSource(String),
    #[error("找不到可用的幻灯片版式")]
    MissingLayout,
    #[error("找不到 OPC 部件: {0}")]
    MissingPart(String),
    #[error("图片 data URL 无效: {0}")]
    InvalidImage(String),
    #[error("公式对象缺少原生 OMML: {0}")]
    MissingOmml(String),
    #[error("XML 不是 UTF-8: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("DrawingML rich-text writeback failed: {0}")]
    DrawingMlText(String),
    #[error("UniPPT portable extension writeback failed: {0}")]
    PortableExtension(String),
}

/// Export an edited scene model back to a native PowerPoint package.
///
/// When `source_pptx` is supplied, the original OPC package is retained and
/// only edited slide/shape parts are patched. Unknown parts, relationships,
/// macros, media, masters and vendor extensions therefore survive unchanged.
pub fn export_pptx(deck: &Deck, source_pptx: Option<&[u8]>) -> Result<Vec<u8>, ExportError> {
    let baseline = source_pptx.map(import_pptx).transpose()?;
    export_pptx_with_baseline(deck, source_pptx, baseline.as_ref())
}

/// Export against an already parsed source scene.
///
/// Cached editor sessions use this entry point so the first dirty export does
/// not parse the source PPTX into a second `Deck`. The OPC package is still
/// opened for loss-aware XML patching, but `baseline` supplies all scene-level
/// comparisons.
pub fn export_pptx_with_baseline(
    deck: &Deck,
    source_pptx: Option<&[u8]>,
    baseline: Option<&Deck>,
) -> Result<Vec<u8>, ExportError> {
    if deck.slides.is_empty() {
        return Err(ExportError::EmptyDeck);
    }

    if let (Some(source), Some(original)) = (source_pptx, baseline) {
        if decks_equivalent_for_native(deck, original) {
            return Ok(source.to_vec());
        }
    }
    let mut presentation = source_pptx.map_or_else(Presentation::new, Presentation::from_bytes)?;
    let current_size = presentation.slide_size()?;
    if current_size != Some((deck.source_width_emu, deck.source_height_emu)) {
        presentation.set_slide_width(deck.source_width_emu)?;
        presentation.set_slide_height(deck.source_height_emu)?;
    }

    let original_refs = presentation.slides()?;
    let mut source_refs: HashMap<String, SlideRef> = original_refs
        .iter()
        .cloned()
        .map(|slide_ref| (slide_ref.partname.to_string(), slide_ref))
        .collect();

    let mut desired_sources = HashSet::new();
    for slide in &deck.slides {
        if let Some(source) = &slide.source_part_name {
            if !desired_sources.insert(source.clone()) {
                return Err(ExportError::DuplicateSlideSource(source.clone()));
            }
        }
    }

    for slide_ref in &original_refs {
        if !desired_sources.contains(slide_ref.partname.as_str()) {
            presentation.delete_slide(slide_ref)?;
            source_refs.remove(slide_ref.partname.as_str());
        }
    }

    let layout = presentation
        .slide_layouts()?
        .into_iter()
        .next()
        .ok_or(ExportError::MissingLayout)?;
    let mut desired_refs = Vec::with_capacity(deck.slides.len());
    for slide in &deck.slides {
        let slide_ref = slide
            .source_part_name
            .as_ref()
            .and_then(|source| source_refs.get(source))
            .cloned()
            .map_or_else(|| presentation.add_slide(&layout), Ok)?;
        desired_refs.push(slide_ref);
    }

    for (target_index, slide_ref) in desired_refs.iter().enumerate() {
        let current_index = presentation.slide_index(slide_ref)?;
        presentation.move_slide(current_index, target_index)?;
    }

    let baseline_slides: HashMap<&str, &Slide> = baseline
        .map(|deck| {
            deck.slides
                .iter()
                .filter_map(|slide| {
                    slide
                        .source_part_name
                        .as_deref()
                        .map(|source| (source, slide))
                })
                .collect()
        })
        .unwrap_or_default();

    for (slide, slide_ref) in deck.slides.iter().zip(&desired_refs) {
        let baseline_slide = slide
            .source_part_name
            .as_deref()
            .and_then(|source| baseline_slides.get(source).copied());
        patch_slide(&mut presentation, slide_ref, slide, baseline_slide, deck)?;
    }

    sync_portable_extensions(&mut presentation, deck)?;

    Ok(presentation.to_bytes()?)
}

/// Keep plugin/AI/dynamic HTML metadata self-contained when a UniPPT scene is
/// projected back to native PPTX. PowerPoint safely ignores this custom XML
/// part while UniPPT can restore it without depending on a sidecar file.
fn sync_portable_extensions(
    presentation: &mut Presentation,
    deck: &Deck,
) -> Result<(), ExportError> {
    let part_name = PackURI::new(UNIPPT_EXTENSIONS_PART)?;
    let legacy_part_name = PackURI::new(LEGACY_UNIPPT_EXTENSIONS_PART)?;
    // Keep the owned metadata inside the presentation subtree for consumers
    // that sandbox relationship traversal there. Migrate only UniPPT's own
    // relationship; unrelated custom XML and external links stay untouched.
    let presentation_part = presentation
        .package_mut()
        .part_by_reltype_mut(OFFICE_DOCUMENT_RELATIONSHIP)?;
    let owned_relationships: Vec<String> = presentation_part.rels.iter()
        .filter(|rel| rel.rel_type == CUSTOM_XML_RELATIONSHIP && !rel.is_external)
        .filter(|rel| rel.target_partname(presentation_part.rels.base_uri()).is_ok_and(
            |target| target == part_name || target == legacy_part_name))
        .map(|rel| rel.r_id.to_string())
        .collect();
    for id in owned_relationships {
        presentation_part.rels.remove(&id);
    }
    presentation.package_mut().remove_part(&legacy_part_name);
    if deck.extensions.is_empty() {
        presentation.package_mut().remove_part(&part_name);
        return Ok(());
    }

    let json = serde_json::to_vec(&deck.extensions)
        .map_err(|error| ExportError::PortableExtension(error.to_string()))?;
    let payload = STANDARD.encode(json);
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><unippt:extensions xmlns:unippt=\"urn:unippt:extensions:v1\" encoding=\"base64\">{payload}</unippt:extensions>"
    );
    presentation
        .package_mut()
        .put_part(Part::new(part_name, "application/xml", xml.into_bytes()));
    presentation
        .package_mut()
        .part_by_reltype_mut(OFFICE_DOCUMENT_RELATIONSHIP)?
        .rels
        .or_add(
            CUSTOM_XML_RELATIONSHIP,
            "customXml/unippt-extensions.xml",
            false,
        );
    Ok(())
}

fn decks_equivalent_for_native(left: &Deck, right: &Deck) -> bool {
    // title and source_import_id are editor/session metadata. Compare all
    // native-bearing deck fields by reference first, which is allocation-free
    // for the overwhelmingly common no-op round trip. Only a slide that differs
    // by direct PartialEq falls back to the JSON float-tolerance comparator;
    // this avoids cloning two asset-heavy Decks and materializing two complete
    // serde_json::Value trees for every export.
    if left.format != right.format
        || left.version != right.version
        || !floats_equivalent(left.width, right.width)
        || !floats_equivalent(left.height, right.height)
        || left.source_width_emu != right.source_width_emu
        || left.source_height_emu != right.source_height_emu
        || left.fonts != right.fonts
        || left.extensions != right.extensions
        || left.slides.len() != right.slides.len()
    {
        return false;
    }
    left.slides.iter().zip(&right.slides).all(|(left, right)| {
        if left == right {
            return true;
        }
        let (Ok(left), Ok(right)) = (serde_json::to_value(left), serde_json::to_value(right))
        else {
            return false;
        };
        json_values_equivalent(&left, &right)
    })
}

fn floats_equivalent(left: f64, right: f64) -> bool {
    if left == right {
        return true;
    }
    let scale = left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= scale * 1.0e-12
}

fn json_values_equivalent(left: &serde_json::Value, right: &serde_json::Value) -> bool {
    if left == right {
        return true;
    }
    match (left, right) {
        (serde_json::Value::Number(left), serde_json::Value::Number(right)) => {
            let (Some(left), Some(right)) = (left.as_f64(), right.as_f64()) else {
                return false;
            };
            let scale = left.abs().max(right.abs()).max(1.0);
            (left - right).abs() <= scale * 1.0e-12
        }
        (serde_json::Value::Array(left), serde_json::Value::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| json_values_equivalent(left, right))
        }
        (serde_json::Value::Object(left), serde_json::Value::Object(right)) => {
            left.len() == right.len()
                && left.iter().all(|(key, left)| {
                    right
                        .get(key)
                        .is_some_and(|right| json_values_equivalent(left, right))
                })
        }
        _ => false,
    }
}

fn patch_slide(
    presentation: &mut Presentation,
    slide_ref: &SlideRef,
    slide: &Slide,
    baseline: Option<&Slide>,
    deck: &Deck,
) -> Result<(), ExportError> {
    enum DeferredNative<'a> {
        Chart(&'a SceneObject),
        Media(&'a SceneObject),
    }
    let mut xml = presentation.slide_xml(slide_ref)?.to_vec();
    let baseline_objects = baseline.map_or_else(HashMap::new, |baseline_slide| {
        objects_by_source_id(&baseline_slide.objects)
    });
    let edited_objects = objects_by_source_id(&slide.objects);
    let mut object_shape_ids: HashMap<&str, u32> = flatten_objects(&slide.objects)
        .filter_map(|object| {
            object
                .source_shape_id
                .map(|shape_id| (object.id.as_str(), shape_id))
        })
        .collect();

    for source_id in baseline_objects.keys() {
        if !edited_objects.contains_key(source_id) {
            xml = ShapeTree::remove_shape_xml(&xml, ShapeId(*source_id))?;
        }
    }

    for (source_id, object) in &edited_objects {
        let Some(original) = baseline_objects.get(source_id) else {
            continue;
        };
        if !object_requires_patch(original, object) {
            continue;
        }
        let replacement = existing_object_xml(
            presentation,
            slide_ref,
            &xml,
            ShapeId(*source_id),
            original,
            object,
            deck,
        )?;
        xml = ShapeTree::replace_shape_xml(&xml, ShapeId(*source_id), &replacement)?;
    }

    let deferred_native = flatten_objects(&slide.objects)
        .filter(|object| object.source_shape_id.is_none())
        .filter_map(|object| {
            if object.kind == ObjectKind::Chart && object.chart.is_some() {
                Some(DeferredNative::Chart(object))
            } else if object.media.is_some() {
                Some(DeferredNative::Media(object))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    // Assign IDs once and insert the new fragments together. Re-parsing the
    // growing slide for every layer made dense reconstructed figures O(n²).
    let mut next_shape_id = next_shape_identifier(&xml)?;
    let mut new_fragments = String::new();
    for object in flatten_objects(&slide.objects) {
        if object.source_shape_id.is_some() {
            continue;
        }
        if (object.kind == ObjectKind::Chart && object.chart.is_some()) || object.media.is_some() {
            continue;
        }
        let next_id = ShapeId(next_shape_id);
        next_shape_id += 1;
        let fragment = object_xml(presentation, slide_ref, next_id, object, deck)?;
        new_fragments.push_str(&fragment);
        object_shape_ids.insert(object.id.as_str(), next_id.0);
    }
    if !new_fragments.is_empty() {
        xml = ShapeTree::insert_shape_xml(&xml, &new_fragments)?;
    }

    let next_native_start = next_shape_identifier(&xml)?;
    for (offset, native) in deferred_native.iter().enumerate() {
        let object = match native {
            DeferredNative::Chart(object) | DeferredNative::Media(object) => object,
        };
        object_shape_ids.insert(object.id.as_str(), next_native_start + offset as u32);
    }

    if baseline.is_none_or(|original| original.animations != slide.animations) {
        let timing = timing_xml(&slide.animations, &object_shape_ids);
        xml = replace_slide_element(xml, "p:timing", &timing, "</p:sld>");
    }
    if baseline.is_none_or(|original| original.transition != slide.transition) {
        let transition = slide
            .transition
            .as_ref()
            .map(transition_xml)
            .unwrap_or_default();
        let insertion_anchor = if std::str::from_utf8(&xml)?.contains("<p:timing") {
            "<p:timing"
        } else {
            "</p:sld>"
        };
        xml = replace_slide_element(xml, "p:transition", &transition, insertion_anchor);
    }

    *presentation.slide_xml_mut(slide_ref)? = xml;
    for native in deferred_native {
        match native {
            DeferredNative::Chart(object) => {
                add_native_chart(presentation, slide_ref, object, deck)?;
            }
            DeferredNative::Media(object) => {
                add_native_media(presentation, slide_ref, object, deck)?;
            }
        }
    }
    let background_asset_changed =
        baseline.is_none_or(|original| original.background_asset != slide.background_asset);
    if background_asset_changed {
        if let Some(asset) = slide.background_asset.as_deref() {
            set_slide_background_asset(presentation, slide_ref, asset)?;
        } else if let Some(color) = css_hex(&slide.background) {
            presentation.set_slide_background_solid(slide_ref, &color)?;
        }
    } else if slide.background_asset.is_none()
        && baseline.is_none_or(|original| original.background != slide.background)
    {
        if let Some(color) = css_hex(&slide.background) {
            presentation.set_slide_background_solid(slide_ref, &color)?;
        }
    }
    Ok(())
}

fn set_slide_background_asset(
    presentation: &mut Presentation,
    slide_ref: &SlideRef,
    asset: &str,
) -> Result<(), ExportError> {
    let (mime, bytes) = decode_data_url(asset)?;
    let image_part = presentation.add_image(&Image::from_bytes(bytes, &mime))?;
    let image_part = PackURI::new(&image_part)?;
    let target = image_part.relative_ref(slide_ref.partname.base_uri());
    let slide_part = presentation
        .package_mut()
        .part_mut(&slide_ref.partname)
        .ok_or_else(|| ExportError::MissingPart(slide_ref.partname.to_string()))?;
    let relationship_id = slide_part
        .rels
        .add_relationship(IMAGE_RELATIONSHIP, target, false);
    presentation.set_slide_background_image(slide_ref, &relationship_id)?;
    Ok(())
}

fn replace_slide_element(
    xml: Vec<u8>,
    element: &str,
    replacement: &str,
    insertion_anchor: &str,
) -> Vec<u8> {
    let Ok(source) = std::str::from_utf8(&xml) else {
        return xml;
    };
    let open = format!("<{element}");
    if let Some(start) = source.find(&open) {
        let Some(open_end_relative) = source[start..].find('>') else {
            return xml;
        };
        let open_end = start + open_end_relative + 1;
        let end = if source[start..open_end].ends_with("/>") {
            open_end
        } else {
            let close = format!("</{element}>");
            let Some(close_relative) = source[open_end..].find(&close) else {
                return xml;
            };
            open_end + close_relative + close.len()
        };
        let mut output = Vec::with_capacity(xml.len() + replacement.len());
        output.extend_from_slice(&xml[..start]);
        output.extend_from_slice(replacement.as_bytes());
        output.extend_from_slice(&xml[end..]);
        return output;
    }
    if replacement.is_empty() {
        return xml;
    }
    let Some(position) = source.find(insertion_anchor) else {
        return xml;
    };
    let mut output = Vec::with_capacity(xml.len() + replacement.len());
    output.extend_from_slice(&xml[..position]);
    output.extend_from_slice(replacement.as_bytes());
    output.extend_from_slice(&xml[position..]);
    output
}

fn timing_xml(animations: &[AnimationEffect], object_shape_ids: &HashMap<&str, u32>) -> String {
    let mut effects: Vec<&AnimationEffect> = animations.iter().collect();
    effects.sort_by_key(|effect| effect.order);
    let mut next_id = 3u32;
    let mut children = String::new();
    for effect in effects {
        let target = effect.target_shape_id.or_else(|| {
            effect
                .target_object_id
                .as_deref()
                .and_then(|id| object_shape_ids.get(id).copied())
        });
        let Some(target) = target else {
            continue;
        };
        children.push_str(&animation_effect_xml(effect, target, &mut next_id));
    }
    if children.is_empty() {
        return String::new();
    }
    format!(
        r#"<p:timing><p:tnLst><p:par><p:cTn id="1" dur="indefinite" restart="never" nodeType="tmRoot"><p:childTnLst><p:seq concurrent="1" nextAc="seek"><p:cTn id="2" dur="indefinite" nodeType="mainSeq"><p:childTnLst>{children}</p:childTnLst></p:cTn><p:prevCondLst><p:cond evt="onPrev" delay="0"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond></p:prevCondLst><p:nextCondLst><p:cond evt="onNext" delay="0"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond></p:nextCondLst></p:seq></p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>"#
    )
}

fn animation_effect_xml(effect: &AnimationEffect, target: u32, next_id: &mut u32) -> String {
    let container_id = take_timing_id(next_id);
    let preset_id = effect
        .preset_id
        .unwrap_or_else(|| default_preset_id(effect.effect));
    let preset_subtype = effect
        .preset_subtype
        .unwrap_or_else(|| default_preset_subtype(effect.effect));
    let preset_class = animation_class_xml(effect.class);
    let node_type = animation_trigger_xml(effect.trigger);
    let mut timing_attributes = String::new();
    if let Some(value) = effect.acceleration {
        timing_attributes.push_str(&format!(r#" accel="{}""#, value.min(100_000)));
    }
    if let Some(value) = effect.deceleration {
        timing_attributes.push_str(&format!(r#" decel="{}""#, value.min(100_000)));
    }
    if let Some(value) = effect.speed {
        timing_attributes.push_str(&format!(r#" spd="{value}""#));
    }
    if let Some(value) = effect.time_filter.as_deref() {
        timing_attributes.push_str(&format!(r#" tmFilter="{}""#, xml_escape(value)));
    }
    if let Some(value) = effect.repeat_count.as_deref() {
        timing_attributes.push_str(&format!(r#" repeatCount="{}""#, xml_escape(value)));
    }
    if let Some(value) = effect.repeat_duration_ms {
        timing_attributes.push_str(&format!(r#" repeatDur="{value}""#));
    }
    if effect.auto_reverse {
        timing_attributes.push_str(r#" autoRev="1""#);
    }
    let mut behaviors = String::new();

    if effect.class == AnimationClass::Entrance {
        behaviors.push_str(&visibility_behavior_xml(target, true, next_id));
    }
    for property in &effect.property_animations {
        behaviors.push_str(&property_animation_xml(
            property,
            effect.duration_ms,
            target,
            next_id,
        ));
    }
    behaviors.push_str(&primary_behavior_xml(effect, target, next_id));
    // PowerPoint pairs rotate/scale/motion presets (spin+fade, grow+fade,
    // float paths) with a companion `filter="fade"` animEffect.  Those primary
    // behaviors bind at a higher parse priority than a fade animEffect, so
    // re-emitting the fade here round-trips losslessly without downgrading the
    // effect back to a plain Fade on import.  Pure Fade already writes its own
    // fade animEffect via `primary_behavior_xml`, so it is intentionally
    // excluded to avoid a duplicate track.
    if effect.fade_filter.is_some()
        && matches!(
            effect.effect,
            AnimationKind::Spin | AnimationKind::GrowShrink | AnimationKind::MotionPath
        )
    {
        behaviors.push_str(&fade_companion_xml(effect, target, next_id));
    }
    if effect.class == AnimationClass::Exit {
        behaviors.push_str(&visibility_behavior_xml(target, false, next_id));
    }

    format!(
        r#"<p:par><p:cTn id="{container_id}" presetID="{preset_id}" presetClass="{preset_class}" presetSubtype="{preset_subtype}"{timing_attributes} fill="hold" nodeType="{node_type}"><p:stCondLst><p:cond delay="{}"/></p:stCondLst><p:childTnLst>{behaviors}</p:childTnLst></p:cTn></p:par>"#,
        effect.delay_ms
    )
}

fn property_animation_xml(
    property: &AnimationPropertyAnimation,
    fallback_duration_ms: u64,
    target: u32,
    next_id: &mut u32,
) -> String {
    if property.attributes.is_empty() {
        return String::new();
    }
    let mut attributes = String::new();
    if let Some(value) = property.calculation_mode.as_deref() {
        attributes.push_str(&format!(r#" calcmode="{}""#, xml_escape(value)));
    }
    if let Some(value) = property.value_type.as_deref() {
        attributes.push_str(&format!(r#" valueType="{}""#, xml_escape(value)));
    }
    if let Some(value) = property.from.as_deref() {
        attributes.push_str(&format!(r#" from="{}""#, xml_escape(value)));
    }
    if let Some(value) = property.to.as_deref() {
        attributes.push_str(&format!(r#" to="{}""#, xml_escape(value)));
    }
    if let Some(value) = property.by.as_deref() {
        attributes.push_str(&format!(r#" by="{}""#, xml_escape(value)));
    }
    if let Some(value) = property.bounce_end {
        attributes.push_str(&format!(
            r#" xmlns:p14="http://schemas.microsoft.com/office/powerpoint/2010/main" p14:bounceEnd="{value}""#
        ));
    }

    let id = take_timing_id(next_id);
    let duration = property.duration_ms.unwrap_or(fallback_duration_ms).max(1);
    let fill = property.fill.as_deref().unwrap_or("hold");
    let additive = property
        .additive
        .as_deref()
        .map(|value| format!(r#" additive="{}""#, xml_escape(value)))
        .unwrap_or_default();
    let attribute_names = property
        .attributes
        .iter()
        .map(|name| format!("<p:attrName>{}</p:attrName>", xml_escape(name)))
        .collect::<String>();
    let keyframes = property
        .keyframes
        .iter()
        .map(|keyframe| {
            let formula = keyframe
                .formula
                .as_deref()
                .map(|value| format!(r#" fmla="{}""#, xml_escape(value)))
                .unwrap_or_default();
            format!(
                r#"<p:tav tm="{}"{formula}><p:val><p:strVal val="{}"/></p:val></p:tav>"#,
                keyframe.time.min(100_000),
                xml_escape(&keyframe.value),
            )
        })
        .collect::<String>();
    let keyframe_list = if keyframes.is_empty() {
        String::new()
    } else {
        format!("<p:tavLst>{keyframes}</p:tavLst>")
    };
    format!(
        r#"<p:anim{attributes}><p:cBhvr{additive}><p:cTn id="{id}" dur="{duration}" fill="{}"/>{}<p:attrNameLst>{attribute_names}</p:attrNameLst></p:cBhvr>{keyframe_list}</p:anim>"#,
        xml_escape(fill),
        target_xml(target),
    )
}

fn primary_behavior_xml(effect: &AnimationEffect, target: u32, next_id: &mut u32) -> String {
    let duration = effect.duration_ms.max(1);
    let id = take_timing_id(next_id);
    let target_xml = target_xml(target);
    match effect.effect {
        AnimationKind::Appear => String::new(),
        AnimationKind::Spin => format!(
            r#"<p:animRot by="21600000"><p:cBhvr><p:cTn id="{id}" dur="{duration}" fill="hold"/>{target_xml}<p:attrNameLst><p:attrName>r</p:attrName></p:attrNameLst></p:cBhvr></p:animRot>"#
        ),
        AnimationKind::GrowShrink => format!(
            r#"<p:animScale byX="150000" byY="150000"><p:cBhvr><p:cTn id="{id}" dur="{duration}" fill="hold"/>{target_xml}<p:attrNameLst><p:attrName>ppt_w</p:attrName><p:attrName>ppt_h</p:attrName></p:attrNameLst></p:cBhvr></p:animScale>"#
        ),
        AnimationKind::MotionPath => {
            let path = effect.motion_path.as_deref().unwrap_or("M 0 0 L 0.15 0");
            format!(
                r#"<p:animMotion origin="layout" path="{}" pathEditMode="relative"><p:cBhvr><p:cTn id="{id}" dur="{duration}" fill="hold"/>{target_xml}<p:attrNameLst><p:attrName>ppt_x</p:attrName><p:attrName>ppt_y</p:attrName></p:attrNameLst></p:cBhvr></p:animMotion>"#,
                xml_escape(path)
            )
        }
        AnimationKind::Media => {
            let command = match effect.media_action.as_deref() {
                Some("pause") => "pause",
                Some("stop") => "stop",
                _ => "playFrom(0.0)",
            };
            format!(
                r#"<p:cmd type="call" cmd="{command}"><p:cBhvr><p:cTn id="{id}" dur="{duration}" fill="hold"/>{target_xml}</p:cBhvr></p:cmd>"#
            )
        }
        _ => {
            let filter = animation_filter(effect);
            let transition = if effect.class == AnimationClass::Exit {
                "out"
            } else {
                "in"
            };
            format!(
                r#"<p:animEffect transition="{transition}" filter="{}"><p:cBhvr><p:cTn id="{id}" dur="{duration}"/>{target_xml}</p:cBhvr></p:animEffect>"#,
                xml_escape(&filter)
            )
        }
    }
}

fn fade_companion_xml(effect: &AnimationEffect, target: u32, next_id: &mut u32) -> String {
    let id = take_timing_id(next_id);
    let duration = effect.duration_ms.max(1);
    let transition = match effect.fade_filter.as_deref() {
        Some("out") => "out",
        _ => "in",
    };
    format!(
        r#"<p:animEffect transition="{transition}" filter="fade"><p:cBhvr><p:cTn id="{id}" dur="{duration}"/>{}</p:cBhvr></p:animEffect>"#,
        target_xml(target),
    )
}

fn visibility_behavior_xml(target: u32, visible: bool, next_id: &mut u32) -> String {
    let id = take_timing_id(next_id);
    let value = if visible { "visible" } else { "hidden" };
    format!(
        r#"<p:set><p:cBhvr><p:cTn id="{id}" dur="1" fill="hold"/><p:tgtEl><p:spTgt spid="{target}"/></p:tgtEl><p:attrNameLst><p:attrName>style.visibility</p:attrName></p:attrNameLst></p:cBhvr><p:to><p:strVal val="{value}"/></p:to></p:set>"#
    )
}

fn target_xml(target: u32) -> String {
    format!(r#"<p:tgtEl><p:spTgt spid="{target}"/></p:tgtEl>"#)
}

fn take_timing_id(next_id: &mut u32) -> u32 {
    let id = *next_id;
    *next_id += 1;
    id
}

fn default_preset_id(kind: AnimationKind) -> u32 {
    match kind {
        AnimationKind::Appear => 1,
        AnimationKind::FlyIn => 2,
        AnimationKind::Fade => 10,
        AnimationKind::Wipe => 22,
        AnimationKind::RandomBars => 14,
        AnimationKind::Dissolve => 9,
        AnimationKind::Wheel => 21,
        AnimationKind::Circle => 6,
        AnimationKind::Split => 16,
        AnimationKind::Zoom => 23,
        AnimationKind::Spin => 8,
        AnimationKind::GrowShrink => 6,
        AnimationKind::MotionPath => 64,
        AnimationKind::Media => 1,
        AnimationKind::Custom => 10,
    }
}

fn default_preset_subtype(kind: AnimationKind) -> u32 {
    match kind {
        AnimationKind::RandomBars => 10,
        AnimationKind::Wheel => 1,
        AnimationKind::Circle => 16,
        AnimationKind::Split => 42,
        _ => 0,
    }
}

fn animation_class_xml(class: AnimationClass) -> &'static str {
    match class {
        AnimationClass::Entrance => "entr",
        AnimationClass::Emphasis => "emph",
        AnimationClass::Exit => "exit",
        AnimationClass::MotionPath => "path",
        AnimationClass::Media => "mediacall",
        AnimationClass::Custom => "entr",
    }
}

fn animation_trigger_xml(trigger: AnimationTrigger) -> &'static str {
    match trigger {
        AnimationTrigger::OnClick => "clickEffect",
        AnimationTrigger::WithPrevious => "withEffect",
        AnimationTrigger::AfterPrevious => "afterEffect",
    }
}

fn animation_filter(effect: &AnimationEffect) -> String {
    let direction = effect.direction.as_deref().unwrap_or("left");
    match effect.effect {
        AnimationKind::Fade => "fade".into(),
        AnimationKind::FlyIn => format!("slide({direction})"),
        AnimationKind::Wipe => format!("wipe({direction})"),
        AnimationKind::RandomBars => format!(
            "randombar({})",
            effect.direction.as_deref().unwrap_or("horizontal")
        ),
        AnimationKind::Dissolve => "dissolve".into(),
        AnimationKind::Wheel => format!("wheel({})", effect.direction.as_deref().unwrap_or("1")),
        AnimationKind::Circle => format!("circle({})", effect.direction.as_deref().unwrap_or("in")),
        AnimationKind::Split => format!(
            "barn({})",
            effect.direction.as_deref().unwrap_or("outHorizontal")
        ),
        AnimationKind::Zoom => "zoom(in)".into(),
        _ => "fade".into(),
    }
}

fn transition_xml(transition: &SlideTransition) -> String {
    const P14_NS: &str = "http://schemas.microsoft.com/office/powerpoint/2010/main";
    let speed = if transition.duration_ms <= 500 {
        "fast"
    } else if transition.duration_ms >= 1_500 {
        "slow"
    } else {
        "med"
    };
    let advance_click = if transition.advance_on_click {
        "1"
    } else {
        "0"
    };
    let advance_after = transition
        .advance_after_ms
        .map(|value| format!(r#" advTm="{value}""#))
        .unwrap_or_default();
    let kind = match transition.kind.as_str() {
        "blinds" | "checker" | "circle" | "comb" | "cover" | "cut" | "diamond" | "dissolve"
        | "fade" | "newsflash" | "plus" | "pull" | "push" | "random" | "randomBar" | "split"
        | "strips" | "wedge" | "wheel" | "wipe" | "zoom" => transition.kind.as_str(),
        _ => "fade",
    };
    let direction = transition
        .direction
        .as_deref()
        .map(|value| format!(r#" dir="{}""#, xml_escape(value)))
        .unwrap_or_default();
    format!(
        r#"<p:transition xmlns:p14="{P14_NS}" p14:dur="{}" spd="{speed}" advClick="{advance_click}"{advance_after}><p:{kind}{direction}/></p:transition>"#,
        transition.duration_ms
    )
}

fn object_requires_patch(original: &SceneObject, edited: &SceneObject) -> bool {
    original.name != edited.name
        || original.kind != edited.kind
        || original.frame != edited.frame
        || original.flip_h != edited.flip_h
        || original.flip_v != edited.flip_v
        || original.text != edited.text
        || original.text_paragraphs != edited.text_paragraphs
        || original.text_frame != edited.text_frame
        || original.geometry != edited.geometry
        || original.asset != edited.asset
        || original.table != edited.table
        || original.chart != edited.chart
        || original.hyperlinks != edited.hyperlinks
        || original.style != edited.style
        || original.custom_geometry != edited.custom_geometry
        || original.text_style != edited.text_style
        || formula_requires_patch(original, edited)
}

fn formula_requires_patch(original: &SceneObject, edited: &SceneObject) -> bool {
    match (&original.formula, &edited.formula) {
        (None, None) => false,
        (Some(before), Some(after)) => {
            before.display != after.display
                || before.omml != after.omml
                || (before.omml.is_none() && before.latex != after.latex)
        }
        _ => true,
    }
}

fn objects_by_source_id(objects: &[SceneObject]) -> HashMap<u32, &SceneObject> {
    flatten_objects(objects)
        .filter_map(|object| object.source_shape_id.map(|source_id| (source_id, object)))
        .collect()
}

fn next_shape_identifier(xml: &[u8]) -> Result<u32, ExportError> {
    // The shape-tree convenience API excludes the root group's ID and does
    // not visit all nested IDs. A new slide already reserves ID 1 for spTree.
    let mut reader = quick_xml::Reader::from_reader(xml);
    let mut maximum = 0_u32;
    loop {
        match reader.read_event().map_err(|error| ExportError::DrawingMlText(error.to_string()))? {
            quick_xml::events::Event::Start(element) | quick_xml::events::Event::Empty(element)
                if element.local_name().as_ref() == b"cNvPr" => {
                for attribute in element.attributes().flatten() {
                    if attribute.key.as_ref() == b"id" {
                        if let Ok(value) = std::str::from_utf8(&attribute.value).unwrap_or("").parse::<u32>() {
                            maximum = maximum.max(value);
                        }
                    }
                }
            }
            quick_xml::events::Event::Eof => break,
            _ => {}
        }
    }
    maximum.checked_add(1).ok_or_else(|| ExportError::DrawingMlText("shape IDs exhausted".into()))
}

fn flatten_objects(objects: &[SceneObject]) -> impl Iterator<Item = &SceneObject> {
    fn collect<'a>(objects: &'a [SceneObject], output: &mut Vec<&'a SceneObject>) {
        for object in objects {
            output.push(object);
            collect(&object.children, output);
        }
    }
    let mut output = Vec::new();
    collect(objects, &mut output);
    output.into_iter()
}

fn object_xml(
    presentation: &mut Presentation,
    slide_ref: &SlideRef,
    shape_id: ShapeId,
    object: &SceneObject,
    deck: &Deck,
) -> Result<String, ExportError> {
    match object.kind {
        ObjectKind::Image => picture_xml(presentation, slide_ref, shape_id, object, deck),
        ObjectKind::Math => equation_xml(shape_id, object, deck),
        ObjectKind::Table => Ok(table_object_xml(shape_id, object, deck)),
        ObjectKind::Shape | ObjectKind::Text => {
            let mut native = object.clone();
            ensure_run_hyperlink_relationships(presentation, slide_ref, &mut native)?;
            let hyperlinks = hyperlink_children_xml(presentation, slide_ref, &native)?;
            Ok(text_shape_xml(shape_id, &native, deck, &hyperlinks))
        }
        _ => {
            let hyperlinks = hyperlink_children_xml(presentation, slide_ref, object)?;
            Ok(text_shape_xml(shape_id, object, deck, &hyperlinks))
        }
    }
}

fn add_native_chart(
    presentation: &mut Presentation,
    slide_ref: &SlideRef,
    object: &SceneObject,
    deck: &Deck,
) -> Result<(), ExportError> {
    let Some(chart) = object.chart.as_ref() else {
        return Ok(());
    };
    let mut data = CategoryChartData::new();
    for category in &chart.categories {
        data.add_category(category);
    }
    for series in &chart.series {
        data.add_series_with_options(&series.name, &series.values);
    }
    let chart_type = match chart.chart_type {
        ChartKind::Line => XlChartType::Line,
        ChartKind::Pie => XlChartType::Pie,
        ChartKind::Doughnut => XlChartType::Doughnut,
        ChartKind::Area => XlChartType::Area,
        ChartKind::Bar if chart.bar_direction.as_deref() == Some("bar") => {
            XlChartType::BarClustered
        }
        ChartKind::Bar | ChartKind::Unknown => XlChartType::ColumnClustered,
    };
    let (left, top, width, height) = frame_emu(&object.frame, deck);
    presentation.add_chart_to_slide(slide_ref, &data, chart_type, left, top, width, height)?;
    Ok(())
}

fn patch_native_chart_part(
    presentation: &mut Presentation,
    object: &SceneObject,
) -> Result<(), ExportError> {
    let Some(chart) = object.chart.as_ref() else {
        return Ok(());
    };
    let Some(source_part_name) = chart.source_part_name.as_deref() else {
        return Ok(());
    };
    let part_name = PackURI::new(source_part_name)?;
    let mut data = CategoryChartData::new();
    for category in &chart.categories {
        data.add_category(category);
    }
    for series in &chart.series {
        data.add_series_with_options(&series.name, &series.values);
    }
    let chart_type = match chart.chart_type {
        ChartKind::Line => XlChartType::Line,
        ChartKind::Pie => XlChartType::Pie,
        ChartKind::Doughnut => XlChartType::Doughnut,
        ChartKind::Area => XlChartType::Area,
        ChartKind::Bar if chart.bar_direction.as_deref() == Some("bar") => {
            XlChartType::BarClustered
        }
        ChartKind::Bar | ChartKind::Unknown => XlChartType::ColumnClustered,
    };
    let chart_xml = ChartXmlWriter::write_category(&data, chart_type)?.into_bytes();
    let workbook_part = presentation.package().part(&part_name).and_then(|part| {
        part.rels
            .iter()
            .find(|relationship| relationship.rel_type == PACKAGE_RELATIONSHIP)
            .and_then(|relationship| relationship.target_partname(part.partname.base_uri()).ok())
    });
    presentation
        .package_mut()
        .part_mut(&part_name)
        .ok_or_else(|| ExportError::MissingPart(source_part_name.into()))?
        .blob = chart_xml;
    if let Some(workbook_part) = workbook_part {
        if let Some(part) = presentation.package_mut().part_mut(&workbook_part) {
            part.blob = pptx::chart::xlsx::generate_category_xlsx(&data)?;
        }
    }
    Ok(())
}

fn add_native_media(
    presentation: &mut Presentation,
    slide_ref: &SlideRef,
    object: &SceneObject,
    deck: &Deck,
) -> Result<(), ExportError> {
    let Some(media) = object.media.as_ref() else {
        return Ok(());
    };
    let asset = media
        .asset
        .as_deref()
        .ok_or_else(|| ExportError::InvalidImage(format!("{} missing media bytes", object.name)))?;
    let (media_mime, media_bytes) = decode_data_url(asset)?;
    let poster_asset = object.asset.as_deref().ok_or_else(|| {
        ExportError::InvalidImage(format!("{} missing poster image", object.name))
    })?;
    let (poster_mime, poster_bytes) = decode_data_url(poster_asset)?;
    let poster = Image::from_bytes(poster_bytes, &poster_mime);
    let (left, top, width, height) = frame_emu(&object.frame, deck);
    match media.kind {
        MediaKind::Video => presentation.add_video_to_slide(
            slide_ref,
            &Video::from_bytes(media_bytes, &media_mime),
            &poster,
            left,
            top,
            width,
            height,
        )?,
        MediaKind::Audio => presentation.add_audio_to_slide(
            slide_ref,
            &Audio::from_bytes(media_bytes, &media_mime),
            &poster,
            left,
            top,
            width,
            height,
        )?,
    }
    Ok(())
}

fn table_object_xml(shape_id: ShapeId, object: &SceneObject, deck: &Deck) -> String {
    let Some(table) = object.table.as_ref() else {
        return text_shape_xml(shape_id, object, deck, "");
    };
    let (left, top, width, height) = frame_emu(&object.frame, deck);
    let column_count = table
        .rows
        .iter()
        .map(|row| row.cells.len())
        .max()
        .unwrap_or(table.columns.len())
        .max(1);
    let column_total = table.columns.iter().sum::<f64>();
    let columns = (0..column_count)
        .map(|index| {
            let value = table.columns.get(index).copied().unwrap_or(1.0);
            let emu = if column_total > 0.0 {
                (width.0 as f64 * value / column_total).round() as i64
            } else {
                width.0 / column_count as i64
            };
            format!(r#"<a:gridCol w="{emu}"/>"#)
        })
        .collect::<String>();
    let row_total = table
        .rows
        .iter()
        .map(|row| row.height.max(0.0))
        .sum::<f64>();
    let rows = table
        .rows
        .iter()
        .map(|row| {
            let row_height = if row_total > 0.0 {
                (height.0 as f64 * row.height.max(0.0) / row_total).round() as i64
            } else {
                height.0 / table.rows.len().max(1) as i64
            };
            let cells = row.cells.iter().map(table_cell_xml).collect::<String>();
            format!(r#"<a:tr h="{row_height}">{cells}</a:tr>"#)
        })
        .collect::<String>();
    let style_id = table
        .style_id
        .as_deref()
        .unwrap_or("{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}");
    format!(
        r#"<p:graphicFrame xmlns:a="{DRAWING_NS}" xmlns:p="{PRESENTATION_NS}" xmlns:r="{RELATIONSHIP_NS}"><p:nvGraphicFramePr><p:cNvPr id="{}" name="{}"/><p:cNvGraphicFramePr><a:graphicFrameLocks noGrp="1"/></p:cNvGraphicFramePr><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr firstRow="{}" firstCol="{}" lastRow="{}" lastCol="{}" bandRow="{}" bandCol="{}"><a:tableStyleId>{}</a:tableStyleId></a:tblPr><a:tblGrid>{columns}</a:tblGrid>{rows}</a:tbl></a:graphicData></a:graphic></p:graphicFrame>"#,
        shape_id.0,
        xml_escape(&object.name),
        left.0,
        top.0,
        width.0,
        height.0,
        bool_xml(table.first_row),
        bool_xml(table.first_col),
        bool_xml(table.last_row),
        bool_xml(table.last_col),
        bool_xml(table.band_rows),
        bool_xml(table.band_cols),
        xml_escape(style_id),
    )
}

fn table_cell_xml(cell: &crate::model::TableCell) -> String {
    let grid_span = if cell.grid_span > 1 {
        format!(r#" gridSpan="{}""#, cell.grid_span)
    } else {
        String::new()
    };
    let row_span = if cell.row_span > 1 {
        format!(r#" rowSpan="{}""#, cell.row_span)
    } else {
        String::new()
    };
    let h_merge = if cell.h_merge { r#" hMerge="1""# } else { "" };
    let v_merge = if cell.v_merge { r#" vMerge="1""# } else { "" };
    let point_size = (cell.text_style.font_size * 75.0).round().max(100.0) as i64;
    let color = css_hex(&cell.text_style.color).unwrap_or_else(|| "000000".into());
    let bold = bool_xml(cell.text_style.bold);
    let italic = bool_xml(cell.text_style.italic);
    let text = xml_escape(&cell.text);
    let fill = css_hex(&cell.fill)
        .map(|value| format!(r#"<a:solidFill><a:srgbClr val="{value}"/></a:solidFill>"#))
        .unwrap_or_else(|| "<a:noFill/>".into());
    format!(
        r#"<a:tc{grid_span}{row_span}{h_merge}{v_merge}><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang="zh-CN" sz="{point_size}" b="{bold}" i="{italic}"><a:solidFill><a:srgbClr val="{color}"/></a:solidFill></a:rPr><a:t>{text}</a:t></a:r><a:endParaRPr lang="zh-CN" sz="{point_size}"/></a:p></a:txBody><a:tcPr>{fill}</a:tcPr></a:tc>"#
    )
}

const fn bool_xml(value: bool) -> &'static str {
    if value {
        "1"
    } else {
        "0"
    }
}

#[allow(clippy::too_many_arguments)]
fn existing_object_xml(
    presentation: &mut Presentation,
    slide_ref: &SlideRef,
    slide_xml: &[u8],
    shape_id: ShapeId,
    original: &SceneObject,
    edited: &SceneObject,
    deck: &Deck,
) -> Result<String, ExportError> {
    if original.kind != edited.kind {
        return object_xml(presentation, slide_ref, shape_id, edited, deck);
    }
    let mut native = edited.clone();
    if matches!(native.kind, ObjectKind::Text | ObjectKind::Shape) {
        ensure_run_hyperlink_relationships(presentation, slide_ref, &mut native)?;
    }
    let mut replacement = match native.kind {
        ObjectKind::Math => equation_xml(shape_id, &native, deck),
        ObjectKind::Image if original.asset != native.asset => {
            picture_xml(presentation, slide_ref, shape_id, &native, deck)
        }
        ObjectKind::Text | ObjectKind::Shape => {
            patch_text_shape(slide_xml, shape_id, original, &native, deck)
        }
        ObjectKind::Table if original.table != native.table => {
            Ok(table_object_xml(shape_id, &native, deck))
        }
        ObjectKind::Chart if original.chart != native.chart => {
            patch_native_chart_part(presentation, &native)?;
            patch_opaque_shape(slide_xml, shape_id, original, &native, deck)
        }
        _ => patch_opaque_shape(slide_xml, shape_id, original, &native, deck),
    }?;
    if original.hyperlinks != native.hyperlinks {
        replacement =
            patch_object_hyperlinks(presentation, slide_ref, &replacement, original, &native)?;
    }
    Ok(replacement)
}

fn text_shape_xml(
    shape_id: ShapeId,
    object: &SceneObject,
    deck: &Deck,
    hyperlinks: &str,
) -> String {
    let (left, top, width, height) = frame_emu(&object.frame, deck);
    let rotation = transform_attributes(object);
    let geometry = geometry_name(object.geometry.as_deref());
    let emu_per_px = deck.source_width_emu.max(1) as f64 / deck.width.max(1.0);
    let shape_properties = shape_properties_xml(
        object, left, top, width, height, &rotation, geometry, emu_per_px,
    );
    let text_body = if object.text_paragraphs.is_empty() {
        text_body_xml(&object.text, &object.text_style, &object.text_frame, emu_per_px)
    } else {
        structured_text_body(&object.text_paragraphs, &object.text_frame, emu_per_px)
    };
    if object.kind == ObjectKind::Connector {
        return format!(
            r#"<p:cxnSp xmlns:a="{DRAWING_NS}" xmlns:p="{PRESENTATION_NS}"><p:nvCxnSpPr><p:cNvPr id="{}" name="{}">{hyperlinks}</p:cNvPr><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr>{shape_properties}</p:cxnSp>"#,
            shape_id.0, xml_escape(&object.name)
        );
    }
    let textbox = if object.kind == ObjectKind::Text {
        r#" txBox="1""#
    } else {
        ""
    };
    format!(
        r#"<p:sp xmlns:a="{DRAWING_NS}" xmlns:p="{PRESENTATION_NS}" xmlns:r="{RELATIONSHIP_NS}"><p:nvSpPr><p:cNvPr id="{}" name="{}">{hyperlinks}</p:cNvPr><p:cNvSpPr{textbox}/><p:nvPr/></p:nvSpPr>{shape_properties}{text_body}</p:sp>"#,
        shape_id.0,
        xml_escape(&object.name),
    )
}

fn picture_xml(
    presentation: &mut Presentation,
    slide_ref: &SlideRef,
    shape_id: ShapeId,
    object: &SceneObject,
    deck: &Deck,
) -> Result<String, ExportError> {
    let asset = object
        .asset
        .as_deref()
        .ok_or_else(|| ExportError::InvalidImage("图片对象没有数据".into()))?;
    let (mime, bytes) = decode_data_url(asset)?;
    let image_part = presentation.add_image(&Image::from_bytes(bytes, &mime))?;
    let image_part = PackURI::new(&image_part)?;
    let target = image_part.relative_ref(slide_ref.partname.base_uri());
    let slide_part = presentation
        .package_mut()
        .part_mut(&slide_ref.partname)
        .ok_or_else(|| ExportError::MissingPart(slide_ref.partname.to_string()))?;
    let relationship_id = slide_part
        .rels
        .add_relationship(IMAGE_RELATIONSHIP, target, false);
    let hyperlinks = hyperlink_children_xml(presentation, slide_ref, object)?;

    let (left, top, width, height) = frame_emu(&object.frame, deck);
    let rotation = transform_attributes(object);
    let crop = &object.image_crop;
    let inset = |value: f64| (value.clamp(0.0, 1.0) * 100_000.0).round() as i64;
    let source_rect = format!(r#"<a:srcRect l="{}" t="{}" r="{}" b="{}"/>"#, inset(crop.left), inset(crop.top), inset(crop.right), inset(crop.bottom));
    Ok(format!(
        r#"<p:pic xmlns:a="{DRAWING_NS}" xmlns:p="{PRESENTATION_NS}" xmlns:r="{RELATIONSHIP_NS}"><p:nvPicPr><p:cNvPr id="{}" name="{}">{hyperlinks}</p:cNvPr><p:cNvPicPr><a:picLocks noChangeAspect="1"/></p:cNvPicPr><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="{}"/>{source_rect}<a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr><a:xfrm{rotation}><a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr></p:pic>"#,
        shape_id.0,
        xml_escape(&object.name),
        xml_escape(&relationship_id),
        left.0,
        top.0,
        width.0,
        height.0,
    ))
}

fn hyperlink_children_xml(
    presentation: &mut Presentation,
    slide_ref: &SlideRef,
    object: &SceneObject,
) -> Result<String, ExportError> {
    let mut output = String::new();
    for (element, action) in [
        ("a:hlinkClick", object.hyperlinks.click.as_ref()),
        ("a:hlinkHover", object.hyperlinks.hover.as_ref()),
    ] {
        if let Some(action) = action {
            let relationship_id = ensure_hyperlink_relationship(presentation, slide_ref, action)?;
            output.push_str(&new_hyperlink_element(
                element,
                action,
                relationship_id.as_deref(),
            ));
        }
    }
    Ok(output)
}

fn patch_object_hyperlinks(
    presentation: &mut Presentation,
    slide_ref: &SlideRef,
    fragment: &str,
    original: &SceneObject,
    edited: &SceneObject,
) -> Result<String, ExportError> {
    let mut output = fragment.to_string();
    for (element, before, after) in [
        (
            "a:hlinkClick",
            original.hyperlinks.click.as_ref(),
            edited.hyperlinks.click.as_ref(),
        ),
        (
            "a:hlinkHover",
            original.hyperlinks.hover.as_ref(),
            edited.hyperlinks.hover.as_ref(),
        ),
    ] {
        if before == after {
            continue;
        }
        let relationship_id = after
            .map(|action| ensure_hyperlink_relationship(presentation, slide_ref, action))
            .transpose()?
            .flatten();
        output = patch_hyperlink_element(&output, element, after, relationship_id.as_deref());
    }
    Ok(output)
}

fn ensure_hyperlink_relationship(
    presentation: &mut Presentation,
    slide_ref: &SlideRef,
    action: &HyperlinkAction,
) -> Result<Option<String>, ExportError> {
    let slide_part = presentation
        .package_mut()
        .part_mut(&slide_ref.partname)
        .ok_or_else(|| ExportError::MissingPart(slide_ref.partname.to_string()))?;

    if let Some(existing_id) = action.relationship_id.as_deref() {
        if let Some(existing) = slide_part.rels.get(existing_id) {
            let existing_target = if existing.is_external {
                existing.target_ref.clone()
            } else {
                existing
                    .target_partname(slide_part.rels.base_uri())
                    .map_or_else(|_| existing.target_ref.clone(), |part| part.to_string())
            };
            if action.target.as_deref().is_none_or(|target| {
                target == existing_target && action.external == existing.is_external
            }) {
                return Ok(Some(existing_id.to_string()));
            }
        } else if action.target.is_none() {
            // Broken ids are still meaningful loss-aware evidence. Keep the
            // XML reference rather than manufacturing an unrelated target.
            return Ok(Some(existing_id.to_string()));
        }
    }

    let Some(target) = action.target.as_deref() else {
        return Ok(None);
    };
    let internal_slide = !action.external
        && (target.starts_with("/ppt/slides/")
            || action
                .action
                .as_deref()
                .is_some_and(|value| value.contains("hlinksldjump")));
    let relationship_type = if internal_slide {
        SLIDE_RELATIONSHIP
    } else {
        HYPERLINK_RELATIONSHIP
    };
    let target_ref = if action.external {
        target.to_string()
    } else if target.starts_with('/') {
        PackURI::new(target)?.relative_ref(slide_ref.partname.base_uri())
    } else {
        target.to_string()
    };
    Ok(Some(slide_part.rels.or_add(
        relationship_type,
        &target_ref,
        action.external,
    )))
}

fn ensure_run_hyperlink_relationships(
    presentation: &mut Presentation,
    slide_ref: &SlideRef,
    object: &mut SceneObject,
) -> Result<(), ExportError> {
    for paragraph in &mut object.text_paragraphs {
        for run in &mut paragraph.runs {
            for action in [run.hyperlinks.click.as_mut(), run.hyperlinks.hover.as_mut()]
                .into_iter()
                .flatten()
            {
                action.relationship_id =
                    ensure_hyperlink_relationship(presentation, slide_ref, action)?;
            }
        }
    }
    Ok(())
}

fn new_hyperlink_element(
    element: &str,
    action: &HyperlinkAction,
    relationship_id: Option<&str>,
) -> String {
    let mut output = format!("<{element}");
    if let Some(relationship_id) = relationship_id {
        output.push_str(&format!(r#" r:id="{}""#, xml_escape(relationship_id)));
    }
    if let Some(action) = action.action.as_deref() {
        output.push_str(&format!(r#" action="{}""#, xml_escape(action)));
    }
    if let Some(tooltip) = action.tooltip.as_deref() {
        output.push_str(&format!(r#" tooltip="{}""#, xml_escape(tooltip)));
    }
    output.push_str("/>");
    output
}

/// Patch only the selected action element below `p:cNvPr`. Existing unknown
/// attributes and child markup survive when a target or tooltip is edited.
fn patch_hyperlink_element(
    fragment: &str,
    element: &str,
    action: Option<&HyperlinkAction>,
    relationship_id: Option<&str>,
) -> String {
    let Some(cnv_start) = fragment.find("<p:cNvPr") else {
        return fragment.to_string();
    };
    let Some(open_relative) = fragment[cnv_start..].find('>') else {
        return fragment.to_string();
    };
    let cnv_open_end = cnv_start + open_relative + 1;
    let cnv_self_closing = fragment[cnv_start..cnv_open_end].trim_end().ends_with("/>");
    let cnv_end = if cnv_self_closing {
        cnv_open_end
    } else {
        fragment[cnv_open_end..]
            .find("</p:cNvPr>")
            .map_or(fragment.len(), |relative| cnv_open_end + relative)
    };
    let needle = format!("<{element}");
    let existing_start = fragment[cnv_open_end..cnv_end]
        .find(&needle)
        .map(|relative| cnv_open_end + relative);

    if let Some(existing_start) = existing_start {
        let Some(open_end_relative) = fragment[existing_start..].find('>') else {
            return fragment.to_string();
        };
        let open_end = existing_start + open_end_relative + 1;
        let existing_end = if fragment[existing_start..open_end]
            .trim_end()
            .ends_with("/>")
        {
            open_end
        } else {
            let close = format!("</{element}>");
            fragment[open_end..cnv_end]
                .find(&close)
                .map_or(open_end, |relative| open_end + relative + close.len())
        };
        let Some(action) = action else {
            return format!(
                "{}{}",
                &fragment[..existing_start],
                &fragment[existing_end..]
            );
        };
        let mut native = fragment[existing_start..existing_end].to_string();
        native = set_or_remove_attribute(&native, element, "r:id", relationship_id);
        native = set_or_remove_attribute(&native, element, "action", action.action.as_deref());
        native = set_or_remove_attribute(&native, element, "tooltip", action.tooltip.as_deref());
        return format!(
            "{}{}{}",
            &fragment[..existing_start],
            native,
            &fragment[existing_end..]
        );
    }

    let Some(action) = action else {
        return fragment.to_string();
    };
    let native = new_hyperlink_element(element, action, relationship_id);
    if cnv_self_closing {
        let slash = cnv_open_end - 2;
        return format!(
            "{}>{native}</p:cNvPr>{}",
            &fragment[..slash],
            &fragment[cnv_open_end..]
        );
    }
    format!("{}{}{}", &fragment[..cnv_end], native, &fragment[cnv_end..])
}

fn set_or_remove_attribute(
    xml: &str,
    element: &str,
    attribute: &str,
    value: Option<&str>,
) -> String {
    let needle = format!("<{element}");
    let Some(start) = xml.find(&needle) else {
        return xml.to_string();
    };
    let Some(relative_end) = xml[start..].find('>') else {
        return xml.to_string();
    };
    let end = start + relative_end;
    let tag = &xml[start..end];
    let attribute_needle = format!(" {attribute}=\"");
    let updated = match (tag.find(&attribute_needle), value) {
        (Some(attribute_start), Some(value)) => {
            let value_start = attribute_start + attribute_needle.len();
            let Some(value_end_relative) = tag[value_start..].find('"') else {
                return xml.to_string();
            };
            let value_end = value_start + value_end_relative;
            format!(
                "{}{}{}",
                &tag[..value_start],
                xml_escape(value),
                &tag[value_end..]
            )
        }
        (Some(attribute_start), None) => {
            let value_start = attribute_start + attribute_needle.len();
            let Some(value_end_relative) = tag[value_start..].find('"') else {
                return xml.to_string();
            };
            let attribute_end = value_start + value_end_relative + 1;
            format!("{}{}", &tag[..attribute_start], &tag[attribute_end..])
        }
        (None, Some(value)) => {
            let insertion = tag
                .rfind('/')
                .filter(|index| tag[index + 1..].trim().is_empty())
                .unwrap_or(tag.len());
            format!(
                r#"{} {attribute}="{}"{}"#,
                &tag[..insertion],
                xml_escape(value),
                &tag[insertion..]
            )
        }
        (None, None) => tag.to_string(),
    };
    format!("{}{}{}", &xml[..start], updated, &xml[end..])
}

fn equation_xml(
    shape_id: ShapeId,
    object: &SceneObject,
    deck: &Deck,
) -> Result<String, ExportError> {
    let formula = object
        .formula
        .as_ref()
        .ok_or_else(|| ExportError::MissingOmml(object.id.clone()))?;
    let omml = formula
        .omml
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ExportError::MissingOmml(object.id.clone()))?;
    let omml = style_omml_runs(&normalize_omml(omml), &object.text_style);

    let choice_shape = equation_choice_shape(shape_id, object, deck, &omml);
    let mut fallback = object.clone();
    fallback.kind = ObjectKind::Text;
    fallback.text = formula.latex.clone();
    fallback.formula = None;
    let fallback_shape = text_shape_xml(shape_id, &fallback, deck, "");
    Ok(format!(
        r#"<mc:AlternateContent xmlns:mc="{MC_NS}"><mc:Choice xmlns:a14="{A14_NS}" Requires="a14">{choice_shape}</mc:Choice><mc:Fallback>{fallback_shape}</mc:Fallback></mc:AlternateContent>"#
    ))
}

fn equation_choice_shape(
    shape_id: ShapeId,
    object: &SceneObject,
    deck: &Deck,
    omml: &str,
) -> String {
    let (left, top, width, height) = frame_emu(&object.frame, deck);
    let rotation = transform_attributes(object);
    let emu_per_px = deck.source_width_emu.max(1) as f64 / deck.width.max(1.0);
    let shape_properties = shape_properties_xml(
        object, left, top, width, height, &rotation, "rect", emu_per_px,
    );
    let end_properties = math_run_properties_xml(&object.text_style, "a:endParaRPr");
    format!(
        r#"<p:sp xmlns:a="{DRAWING_NS}" xmlns:p="{PRESENTATION_NS}" xmlns:r="{RELATIONSHIP_NS}"><p:nvSpPr><p:cNvPr id="{}" name="{}"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>{shape_properties}<p:txBody><a:bodyPr wrap="square"/><a:lstStyle/><a:p><a:pPr algn="{}"/><a14:m>{omml}</a14:m>{end_properties}</a:p></p:txBody></p:sp>"#,
        shape_id.0,
        xml_escape(&object.name),
        alignment(&object.text_style.align),
    )
}

/// PowerPoint renders the `mc:Choice` OMML branch, not the DrawingML fallback.
/// Math properties (`m:rPr`) cannot carry Office text size or colour, so every
/// math run also needs its host DrawingML `a:rPr` sibling.
fn style_omml_runs(omml: &str, style: &TextStyle) -> String {
    let properties = math_run_properties_xml(style, "a:rPr");
    let mut output = omml.to_string();
    let mut search_from = 0usize;

    while let Some(relative_start) = output[search_from..].find("<m:r") {
        let run_start = search_from + relative_start;
        let after_name = output.as_bytes().get(run_start + 4).copied();
        if !matches!(
            after_name,
            Some(b'>') | Some(b' ') | Some(b'\t') | Some(b'\r') | Some(b'\n')
        ) {
            search_from = run_start + 4;
            continue;
        }
        let Some(open_end_relative) = output[run_start..].find('>') else {
            break;
        };
        let content_start = run_start + open_end_relative + 1;
        let Some(close_relative) = output[content_start..].find("</m:r>") else {
            break;
        };
        let close_start = content_start + close_relative;
        let run_content = &output[content_start..close_start];

        if let Some(property_relative) = run_content.find("<a:rPr") {
            let property_start = content_start + property_relative;
            let Some(tag_end_relative) = output[property_start..close_start].find('>') else {
                break;
            };
            let tag_end = property_start + tag_end_relative + 1;
            let property_end = if output[property_start..tag_end].trim_end().ends_with("/>") {
                tag_end
            } else if let Some(end_relative) = output[tag_end..close_start].find("</a:rPr>") {
                tag_end + end_relative + "</a:rPr>".len()
            } else {
                break;
            };
            output.replace_range(property_start..property_end, &properties);
            search_from = property_start + properties.len();
        } else {
            let insertion = leading_math_run_properties_end(&output, content_start, close_start);
            output.insert_str(insertion, &properties);
            search_from = insertion + properties.len();
        }
    }
    output
}

fn leading_math_run_properties_end(xml: &str, content_start: usize, close_start: usize) -> usize {
    let content = &xml[content_start..close_start];
    let trimmed = content.trim_start();
    let leading_whitespace = content.len() - trimmed.len();
    if !trimmed.starts_with("<m:rPr") {
        return content_start + leading_whitespace;
    }
    let property_start = content_start + leading_whitespace;
    let Some(tag_end_relative) = xml[property_start..close_start].find('>') else {
        return property_start;
    };
    let tag_end = property_start + tag_end_relative + 1;
    if xml[property_start..tag_end].trim_end().ends_with("/>") {
        tag_end
    } else {
        xml[tag_end..close_start]
            .find("</m:rPr>")
            .map_or(property_start, |end_relative| {
                tag_end + end_relative + "</m:rPr>".len()
            })
    }
}

fn math_run_properties_xml(style: &TextStyle, tag: &str) -> String {
    let font_size = (style.font_size.max(1.0) * 75.0).round() as i64;
    let color = css_hex(&style.color).unwrap_or_else(|| "172033".into());
    let native_fonts = text_style_font_slots(style);
    let language = native_fonts
        .language_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("zh-CN");
    let font_nodes = font_slots_xml(&native_fonts);
    format!(
        "<{tag} lang=\"{}\" sz=\"{font_size}\" b=\"{}\" i=\"{}\"><a:solidFill><a:srgbClr val=\"{color}\"/></a:solidFill>{font_nodes}</{tag}>",
        xml_escape(language),
        bool_xml(style.bold),
        bool_xml(style.italic),
    )
}

#[allow(clippy::too_many_arguments)]
fn shape_properties_xml(
    object: &SceneObject,
    left: Emu,
    top: Emu,
    width: Emu,
    height: Emu,
    rotation: &str,
    geometry: &str,
    emu_per_px: f64,
) -> String {
    let fill = fill_xml(&object.style);
    let line = line_xml(&object.style, emu_per_px);
    let shadow = shadow_xml(&object.style, emu_per_px);
    let geometry = geometry_xml(object, geometry);
    format!(
        r#"<p:spPr><a:xfrm{rotation}><a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/></a:xfrm>{geometry}{fill}{line}{shadow}</p:spPr>"#,
        left.0, top.0, width.0, height.0,
    )
}

fn geometry_xml(object: &SceneObject, preset: &str) -> String {
    object
        .custom_geometry
        .as_ref()
        .and_then(custom_geometry_xml)
        .unwrap_or_else(|| format!(r#"<a:prstGeom prst="{preset}"><a:avLst/></a:prstGeom>"#))
}

/// Convert UniPPT's HTML/SVG path projection back to native DrawingML.  The
/// editable scene intentionally uses an absolute M/L/C/Z subset, which is
/// compact, AI-friendly and lossless for PowerPoint freeform ink paths.
fn custom_geometry_xml(geometry: &crate::model::CustomGeometry) -> Option<String> {
    let source = geometry
        .path_data
        .replace(',', " ")
        .replace(['M', 'm'], " M ")
        .replace(['L', 'l'], " L ")
        .replace(['C', 'c'], " C ")
        .replace(['Z', 'z'], " Z ");
    let tokens: Vec<_> = source.split_whitespace().collect();
    let number = |value: &str| value.parse::<f64>().ok().map(|value| value.round() as i64);
    let mut index = 0usize;
    let mut commands = String::new();
    let mut moved = false;
    while index < tokens.len() {
        match tokens[index] {
            "M" | "L" => {
                let command = tokens[index];
                let x = number(tokens.get(index + 1).copied()?)?;
                let y = number(tokens.get(index + 2).copied()?)?;
                let tag = if command == "M" { "moveTo" } else { "lnTo" };
                commands.push_str(&format!(r#"<a:{tag}><a:pt x="{x}" y="{y}"/></a:{tag}>"#));
                moved |= command == "M";
                index += 3;
            }
            "C" => {
                let values = (1..=6)
                    .map(|offset| number(tokens.get(index + offset).copied()?))
                    .collect::<Option<Vec<_>>>()?;
                commands.push_str(&format!(
                    r#"<a:cubicBezTo><a:pt x="{}" y="{}"/><a:pt x="{}" y="{}"/><a:pt x="{}" y="{}"/></a:cubicBezTo>"#,
                    values[0], values[1], values[2], values[3], values[4], values[5]
                ));
                index += 7;
            }
            "Z" => {
                commands.push_str("<a:close/>");
                index += 1;
            }
            _ => return None,
        }
    }
    if !moved || geometry.width <= 0 || geometry.height <= 0 {
        return None;
    }
    Some(format!(
        r#"<a:custGeom><a:avLst/><a:gdLst/><a:ahLst/><a:cxnLst/><a:rect l="l" t="t" r="r" b="b"/><a:pathLst><a:path w="{}" h="{}">{commands}</a:path></a:pathLst></a:custGeom>"#,
        geometry.width, geometry.height
    ))
}

fn text_body_xml(text: &str, style: &TextStyle, frame: &crate::model::TextFrameStyle, emu_per_px: f64) -> String {
    let font_size = (style.font_size.max(1.0) * 75.0).round() as i64;
    let bold = if style.bold { "1" } else { "0" };
    let italic = if style.italic { "1" } else { "0" };
    let color = css_hex(&style.color).unwrap_or_else(|| "172033".into());
    let native_fonts = text_style_font_slots(style);
    let language = native_fonts
        .language_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("zh-CN");
    let font_nodes = font_slots_xml(&native_fonts);
    let mut paragraphs = String::new();
    for line in text.split('\n') {
        paragraphs.push_str(&format!(
            r#"<a:p><a:pPr algn="{}"/><a:r><a:rPr lang="{}" sz="{font_size}" b="{bold}" i="{italic}"><a:solidFill><a:srgbClr val="{color}"/></a:solidFill>{}</a:rPr><a:t>{}</a:t></a:r><a:endParaRPr lang="{}" sz="{font_size}"/></a:p>"#,
            alignment(&style.align),
            xml_escape(language),
            font_nodes,
            xml_escape(line),
            xml_escape(language),
        ));
    }
    let body_pr = make_body_pr(frame, emu_per_px);
    format!(r#"<p:txBody>{body_pr}<a:lstStyle/>{paragraphs}</p:txBody>"#)
}

fn text_style_font_slots(style: &TextStyle) -> FontSlots {
    if !style.native_fonts.is_empty() {
        return style.native_fonts.clone();
    }
    let typeface = style
        .native_font_family
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map_or_else(|| primary_family(&style.font_family), str::to_string);
    FontSlots::unified(typeface)
}

fn font_slots_xml(slots: &FontSlots) -> String {
    [
        ("latin", slots.latin.as_deref()),
        ("ea", slots.east_asia.as_deref()),
        ("cs", slots.complex_script.as_deref()),
        ("sym", slots.symbol.as_deref()),
    ]
    .into_iter()
    .filter_map(|(tag, value)| {
        value
            .filter(|value| !value.trim().is_empty())
            .map(|value| format!("<a:{tag} typeface=\"{}\"/>", xml_escape(value)))
    })
    .collect()
}

fn patch_opaque_shape(
    slide_xml: &[u8],
    shape_id: ShapeId,
    original: &SceneObject,
    edited: &SceneObject,
    deck: &Deck,
) -> Result<String, ExportError> {
    let range = ShapeTree::shape_xml_range(slide_xml, shape_id)?
        .ok_or_else(|| ExportError::MissingPart(format!("shape {}", shape_id.0)))?;
    let mut fragment = std::str::from_utf8(&slide_xml[range])?.to_string();
    if original.name != edited.name {
        fragment = replace_first_attribute(&fragment, "p:cNvPr", "name", &edited.name);
    }
    if original.frame != edited.frame
        || original.flip_h != edited.flip_h
        || original.flip_v != edited.flip_v
    {
        fragment = patch_transform(fragment, edited, deck);
    }
    Ok(fragment)
}

fn patch_text_shape(
    slide_xml: &[u8],
    shape_id: ShapeId,
    original: &SceneObject,
    edited: &SceneObject,
    deck: &Deck,
) -> Result<String, ExportError> {
    let range = ShapeTree::shape_xml_range(slide_xml, shape_id)?
        .ok_or_else(|| ExportError::MissingPart(format!("shape {}", shape_id.0)))?;
    let mut fragment = std::str::from_utf8(&slide_xml[range])?.to_string();
    if original.name != edited.name {
        fragment = replace_first_attribute(&fragment, "p:cNvPr", "name", &edited.name);
    }
    if original.frame != edited.frame
        || original.flip_h != edited.flip_h
        || original.flip_v != edited.flip_v
    {
        fragment = patch_transform(fragment, edited, deck);
    }
    if !fragment.contains("<p:spPr")
        && (original.style != edited.style
            || original.geometry != edited.geometry
            || original.custom_geometry != edited.custom_geometry)
    {
        let (left, top, width, height) = frame_emu(&edited.frame, deck);
        let rotation = transform_attributes(edited);
        let properties = shape_properties_xml(
            edited,
            left,
            top,
            width,
            height,
            &rotation,
            geometry_name(edited.geometry.as_deref()),
            deck.source_width_emu.max(1) as f64 / deck.width.max(1.0),
        );
        fragment = replace_or_insert_element(&fragment, "p:spPr", &properties, "</p:nvSpPr>");
    } else {
        if original.geometry != edited.geometry
            || original.custom_geometry != edited.custom_geometry
        {
            let geometry = geometry_xml(edited, geometry_name(edited.geometry.as_deref()));
            fragment = patch_shape_property_child(
                &fragment,
                &["prstGeom", "custGeom"],
                &geometry,
                &[
                    "noFill",
                    "solidFill",
                    "gradFill",
                    "pattFill",
                    "blipFill",
                    "grpFill",
                    "ln",
                    "effectLst",
                    "extLst",
                ],
            )
            .map_err(ExportError::DrawingMlText)?;
        }
        if original.style.fill != edited.style.fill
            || original.style.gradient != edited.style.gradient
            || original.style.opacity != edited.style.opacity
        {
            fragment = patch_shape_property_child(
                &fragment,
                &[
                    "noFill",
                    "solidFill",
                    "gradFill",
                    "pattFill",
                    "blipFill",
                    "grpFill",
                ],
                &fill_xml(&edited.style),
                &["ln", "effectLst", "scene3d", "sp3d", "extLst"],
            )
            .map_err(ExportError::DrawingMlText)?;
        }
        if original.style.stroke != edited.style.stroke
            || original.style.stroke_width != edited.style.stroke_width
            || original.style.stroke_dash != edited.style.stroke_dash
        {
            let emu_per_px = deck.source_width_emu.max(1) as f64 / deck.width.max(1.0);
            fragment = patch_shape_property_child(
                &fragment,
                &["ln"],
                &line_xml(&edited.style, emu_per_px),
                &["effectLst", "scene3d", "sp3d", "extLst"],
            )
            .map_err(ExportError::DrawingMlText)?;
        }
        if original.style.shadow != edited.style.shadow {
            let emu_per_px = deck.source_width_emu.max(1) as f64 / deck.width.max(1.0);
            let effect = shadow_effect_xml(&edited.style, emu_per_px);
            fragment =
                patch_shape_shadow(&fragment, (!effect.is_empty()).then_some(effect.as_str()))
                    .map_err(ExportError::DrawingMlText)?;
        }
    }
    let emu_per_px = deck.source_width_emu.max(1) as f64 / deck.width.max(1.0);
    if !edited.text_paragraphs.is_empty() {
        let effective_paragraphs = effective_text_paragraphs(original, edited);
        if fragment.contains("<p:txBody") {
            if original.text_paragraphs != effective_paragraphs {
                fragment = patch_structured_text(
                    &fragment,
                    &original.text_paragraphs,
                    &effective_paragraphs,
                )
                .map_err(ExportError::DrawingMlText)?;
            }
        } else {
            let body = structured_text_body(&effective_paragraphs, &edited.text_frame, emu_per_px);
            fragment = replace_or_insert_element(&fragment, "p:txBody", &body, "</p:spPr>");
        }
    } else if original.text != edited.text
        || original.text_style != edited.text_style
        || original.text_paragraphs != edited.text_paragraphs
    {
        // Legacy/plain UDOC objects still use the uniform fallback. Imported
        // structured text never takes this path, so native run markup remains
        // available for differential edits.
        let body = text_body_xml(&edited.text, &edited.text_style, &edited.text_frame, emu_per_px);
        fragment = replace_or_insert_element(&fragment, "p:txBody", &body, "</p:spPr>");
    }
    if original.text_frame != edited.text_frame {
        if !fragment.contains("<p:txBody") {
            let body = text_body_xml(&edited.text, &edited.text_style, &edited.text_frame, emu_per_px);
            fragment = replace_or_insert_element(&fragment, "p:txBody", &body, "</p:spPr>");
        }
        fragment = patch_text_frame(
            &fragment,
            &original.text_frame,
            &edited.text_frame,
            emu_per_px,
        )
        .map_err(ExportError::DrawingMlText)?;
    }
    Ok(fragment)
}

/// Older editor builds changed only the plain `text` mirror, while imported
/// objects already carried structured paragraphs.  Preserve the source
/// identity and style of the corresponding paragraph/first run so that this
/// compatibility path still uses the differential native writer.
fn effective_text_paragraphs(
    original: &SceneObject,
    edited: &SceneObject,
) -> Vec<RichTextParagraph> {
    let mut paragraphs = edited.text_paragraphs.clone();
    if original.text != edited.text && original.text_paragraphs == edited.text_paragraphs {
        paragraphs = edited
            .text
            .split('\n')
            .enumerate()
            .map(|(index, line)| {
                let mut paragraph = original
                    .text_paragraphs
                    .get(index)
                    .cloned()
                    .unwrap_or_else(|| new_paragraph(&edited.text_style));
                if let Some(mut run) = paragraph.runs.first().cloned() {
                    run.text = line.trim_end_matches('\r').to_string();
                    paragraph.runs = vec![run];
                } else {
                    paragraph.runs = vec![new_run(line.trim_end_matches('\r'), &edited.text_style)];
                }
                paragraph
            })
            .collect();
    }
    if original.text_style != edited.text_style {
        for paragraph in &mut paragraphs {
            if original.text_style.align != edited.text_style.align {
                paragraph.align.clone_from(&edited.text_style.align);
            }
            for run in &mut paragraph.runs {
                if original.text_style.font_family != edited.text_style.font_family {
                    run.font_family.clone_from(&edited.text_style.font_family);
                }
                if original.text_style.native_fonts != edited.text_style.native_fonts {
                    run.native_fonts.clone_from(&edited.text_style.native_fonts);
                } else if original.text_style.native_font_family
                    != edited.text_style.native_font_family
                {
                    run.native_font_family
                        .clone_from(&edited.text_style.native_font_family);
                    let typeface = edited
                        .text_style
                        .native_font_family
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map_or_else(
                            || primary_family(&edited.text_style.font_family),
                            str::to_string,
                        );
                    let language_id = run.native_fonts.language_id.clone();
                    run.native_fonts = FontSlots::unified(typeface);
                    run.native_fonts.language_id = language_id;
                }
                if original.text_style.font_size != edited.text_style.font_size {
                    run.font_size = edited.text_style.font_size;
                }
                if original.text_style.color != edited.text_style.color {
                    run.color.clone_from(&edited.text_style.color);
                }
                if original.text_style.bold != edited.text_style.bold {
                    run.bold = edited.text_style.bold;
                }
                if original.text_style.italic != edited.text_style.italic {
                    run.italic = edited.text_style.italic;
                }
            }
        }
    }
    paragraphs
}

fn new_paragraph(style: &TextStyle) -> RichTextParagraph {
    RichTextParagraph {
        source_index: None,
        runs: Vec::new(),
        align: style.align.clone(),
        level: 0,
        bullet: None,
        line_spacing: None,
        space_before: None,
        space_after: None,
    }
}

fn new_run(text: &str, style: &TextStyle) -> RichTextRun {
    RichTextRun {
        source_index: None,
        text: text.to_string(),
        font_family: style.font_family.clone(),
        native_font_family: style.native_font_family.clone(),
        native_fonts: style.native_fonts.clone(),
        font_size: style.font_size,
        color: style.color.clone(),
        gradient: None,
        bold: style.bold,
        italic: style.italic,
        underline: false,
        underline_style: None,
        strikethrough: false,
        baseline: "normal".into(),
        baseline_offset: None,
        alpha: 1.0,
        hyperlinks: crate::model::ObjectHyperlinks::default(),
    }
}

fn patch_transform(mut fragment: String, object: &SceneObject, deck: &Deck) -> String {
    let frame = &object.frame;
    let (left, top, width, height) = frame_emu(frame, deck);
    if !fragment.contains("<a:xfrm") && !fragment.contains("<p:xfrm") {
        let attributes = transform_attributes(object);
        let transform = format!(
            r#"<a:xfrm{attributes}><a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/></a:xfrm>"#,
            left.0, top.0, width.0, height.0
        );
        fragment = insert_shape_property_child(&fragment, &transform);
        return fragment;
    }
    fragment = replace_first_attribute(&fragment, "a:off", "x", &left.0.to_string());
    fragment = replace_first_attribute(&fragment, "a:off", "y", &top.0.to_string());
    fragment = replace_first_attribute(&fragment, "a:ext", "cx", &width.0.to_string());
    fragment = replace_first_attribute(&fragment, "a:ext", "cy", &height.0.to_string());
    let rotation = (frame.rotation * 60_000.0).round() as i64;
    fragment = replace_first_attribute(&fragment, "a:xfrm", "rot", &rotation.to_string());
    fragment = replace_first_attribute(&fragment, "p:xfrm", "rot", &rotation.to_string());
    for (attribute, value) in [
        ("flipH", if object.flip_h { "1" } else { "0" }),
        ("flipV", if object.flip_v { "1" } else { "0" }),
    ] {
        fragment = replace_first_attribute(&fragment, "a:xfrm", attribute, value);
        fragment = replace_first_attribute(&fragment, "p:xfrm", attribute, value);
    }
    fragment
}

fn insert_shape_property_child(fragment: &str, child: &str) -> String {
    for element in ["p:spPr", "p:grpSpPr"] {
        let needle = format!("<{element}");
        let Some(start) = fragment.find(&needle) else {
            continue;
        };
        let Some(relative_end) = fragment[start..].find('>') else {
            return fragment.to_string();
        };
        let open_end = start + relative_end + 1;
        if fragment[start..open_end].ends_with("/>") {
            let slash = open_end - 2;
            return format!(
                "{}>{child}</{element}>{}",
                &fragment[..slash],
                &fragment[open_end..]
            );
        }
        return format!(
            "{}{}{}",
            &fragment[..open_end],
            child,
            &fragment[open_end..]
        );
    }
    fragment.to_string()
}

fn replace_or_insert_element(xml: &str, element: &str, replacement: &str, anchor: &str) -> String {
    let open = format!("<{element}");
    if let Some(start) = xml.find(&open) {
        let Some(open_end_relative) = xml[start..].find('>') else {
            return xml.to_string();
        };
        let open_end = start + open_end_relative + 1;
        let end = if xml[..open_end].trim_end().ends_with("/>") {
            open_end
        } else {
            let close = format!("</{element}>");
            let Some(close_relative) = xml[open_end..].find(&close) else {
                return xml.to_string();
            };
            open_end + close_relative + close.len()
        };
        return format!("{}{}{}", &xml[..start], replacement, &xml[end..]);
    }
    if let Some(position) = xml.find(anchor) {
        let insert_at = position + anchor.len();
        return format!("{}{}{}", &xml[..insert_at], replacement, &xml[insert_at..]);
    }
    xml.to_string()
}

fn replace_first_attribute(xml: &str, element: &str, attribute: &str, value: &str) -> String {
    let needle = format!("<{element}");
    let Some(start) = xml.find(&needle) else {
        return xml.to_string();
    };
    let Some(relative_end) = xml[start..].find('>') else {
        return xml.to_string();
    };
    let end = start + relative_end;
    let tag = &xml[start..end];
    let attr_needle = format!("{attribute}=\"");
    let updated_tag = if let Some(attr_start) = tag.find(&attr_needle) {
        let value_start = attr_start + attr_needle.len();
        if let Some(value_end_relative) = tag[value_start..].find('"') {
            let value_end = value_start + value_end_relative;
            format!(
                "{}{}{}",
                &tag[..value_start],
                xml_escape(value),
                &tag[value_end..]
            )
        } else {
            tag.to_string()
        }
    } else {
        format!("{tag} {attribute}=\"{}\"", xml_escape(value))
    };
    format!("{}{}{}", &xml[..start], updated_tag, &xml[end..])
}

fn frame_emu(frame: &Frame, deck: &Deck) -> (Emu, Emu, Emu, Emu) {
    let scale = deck.source_width_emu.max(1) as f64 / deck.width.max(1.0);
    let to_emu = |value: f64| Emu((value * scale).round() as i64);
    (
        to_emu(frame.x),
        to_emu(frame.y),
        to_emu(frame.width.max(1.0)),
        to_emu(frame.height.max(1.0)),
    )
}

fn transform_attributes(object: &SceneObject) -> String {
    let mut attributes = String::new();
    if object.frame.rotation.abs() >= f64::EPSILON {
        attributes.push_str(&format!(
            r#" rot="{}""#,
            (object.frame.rotation * 60_000.0).round() as i64
        ));
    }
    if object.flip_h {
        attributes.push_str(r#" flipH="1""#);
    }
    if object.flip_v {
        attributes.push_str(r#" flipV="1""#);
    }
    attributes
}

fn fill_xml(style: &VisualStyle) -> String {
    if style.fill.trim_start().starts_with("linear-gradient") {
        if let Some(gradient) = style
            .gradient
            .as_ref()
            .filter(|gradient| !gradient.stops.is_empty())
        {
            let raw_angle = ((gradient.angle - 90.0).rem_euclid(360.0) * 60_000.0).round() as i64;
            let stops = gradient
                .stops
                .iter()
                .filter_map(|stop| {
                    let color = css_hex(&stop.color)?;
                    let position = (stop.position.clamp(0.0, 1.0) * 100_000.0).round() as i64;
                    let alpha = (stop.opacity.clamp(0.0, 1.0) * 100_000.0).round() as i64;
                    Some(format!(
                        r#"<a:gs pos="{position}"><a:srgbClr val="{color}"><a:alpha val="{alpha}"/></a:srgbClr></a:gs>"#
                    ))
                })
                .collect::<String>();
            if !stops.is_empty() {
                return format!(
                    r#"<a:gradFill rotWithShape="1"><a:gsLst>{stops}</a:gsLst><a:lin ang="{raw_angle}" scaled="1"/></a:gradFill>"#
                );
            }
        }
    }
    css_hex(&style.fill).map_or_else(
        || "<a:noFill/>".into(),
        |color| {
            let alpha = (style.opacity.clamp(0.0, 1.0) * 100_000.0).round() as i64;
            format!(
                r#"<a:solidFill><a:srgbClr val="{color}"><a:alpha val="{alpha}"/></a:srgbClr></a:solidFill>"#
            )
        },
    )
}

fn line_xml(style: &VisualStyle, emu_per_px: f64) -> String {
    css_hex(&style.stroke).map_or_else(
        || "<a:ln><a:noFill/></a:ln>".into(),
        |color| {
            let width = (style.stroke_width.max(0.25) * emu_per_px).round() as i64;
            let dash = style
                .stroke_dash
                .as_deref()
                .filter(|value| !value.is_empty() && *value != "solid")
                .map(|value| format!(r#"<a:prstDash val="{}"/>"#, xml_escape(value)))
                .unwrap_or_default();
            format!(
                r#"<a:ln w="{width}"><a:solidFill><a:srgbClr val="{color}"/></a:solidFill>{dash}</a:ln>"#
            )
        },
    )
}

fn shadow_xml(style: &VisualStyle, emu_per_px: f64) -> String {
    let effect = shadow_effect_xml(style, emu_per_px);
    if effect.is_empty() {
        String::new()
    } else {
        format!("<a:effectLst>{effect}</a:effectLst>")
    }
}

fn shadow_effect_xml(style: &VisualStyle, emu_per_px: f64) -> String {
    let Some(shadow) = &style.shadow else {
        return String::new();
    };
    let color = css_hex(&shadow.color).unwrap_or_else(|| "000000".into());
    let blur = (shadow.blur.max(0.0) * emu_per_px).round() as i64;
    let distance_px = shadow.offset_x.hypot(shadow.offset_y);
    let distance = (distance_px * emu_per_px).round() as i64;
    let direction = if distance_px <= f64::EPSILON {
        0
    } else {
        (shadow
            .offset_y
            .atan2(shadow.offset_x)
            .to_degrees()
            .rem_euclid(360.0)
            * 60_000.0)
            .round() as i64
    };
    let alpha = (shadow.opacity.clamp(0.0, 1.0) * 100_000.0).round() as i64;
    let tag = if shadow.inset {
        "innerShdw"
    } else {
        "outerShdw"
    };
    format!(
        r#"<a:{tag} blurRad="{blur}" dist="{distance}" dir="{direction}" rotWithShape="0"><a:srgbClr val="{color}"><a:alpha val="{alpha}"/></a:srgbClr></a:{tag}>"#
    )
}

fn css_hex(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    for index in 0..bytes.len().saturating_sub(6) {
        if bytes[index] == b'#'
            && bytes[index + 1..index + 7]
                .iter()
                .all(u8::is_ascii_hexdigit)
        {
            return Some(value[index + 1..index + 7].to_ascii_uppercase());
        }
    }
    None
}

fn geometry_name(value: Option<&str>) -> &str {
    match value.unwrap_or_default().to_ascii_lowercase().as_str() {
        "line" | "straightconnector1" => "line",
        "roundrect" | "roundedrectangle" => "roundRect",
        "ellipse" | "oval" => "ellipse",
        "diamond" => "diamond",
        "triangle" | "isoscelestriangle" => "triangle",
        "hexagon" => "hexagon",
        "cloud" => "cloud",
        _ => "rect",
    }
}

fn alignment(value: &str) -> &str {
    match value {
        "center" => "ctr",
        "right" => "r",
        "justify" => "just",
        "justifyLow" => "justLow",
        "distributed" => "dist",
        "thaiDistributed" => "thaiDist",
        _ => "l",
    }
}

fn decode_data_url(value: &str) -> Result<(String, Vec<u8>), ExportError> {
    let (header, encoded) = value
        .split_once(',')
        .ok_or_else(|| ExportError::InvalidImage("缺少 data URL 分隔符".into()))?;
    let mime = header
        .strip_prefix("data:")
        .and_then(|header| header.strip_suffix(";base64"))
        .filter(|mime| mime.contains('/'))
        .ok_or_else(|| ExportError::InvalidImage("只支持 base64 data URL".into()))?;
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|error| ExportError::InvalidImage(error.to_string()))?;
    Ok((mime.to_string(), bytes))
}

fn normalize_omml(value: &str) -> String {
    let trimmed = value.trim();
    let mut normalized = if trimmed.starts_with("<m:oMathPara") {
        trimmed.to_string()
    } else {
        format!(r#"<m:oMathPara xmlns:m="{MATH_NS}">{trimmed}</m:oMathPara>"#)
    };
    if !normalized.contains("xmlns:m=") {
        if let Some(end) = normalized.find('>') {
            normalized.insert_str(end, &format!(r#" xmlns:m="{MATH_NS}""#));
        }
    }
    normalized
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use pptx::opc::{PackURI, Part};

    use super::*;
    use crate::model::{
        AnimationClass, AnimationEffect, AnimationKind, AnimationPropertyKeyframe,
        AnimationTrigger, FormulaData, GradientFillStyle, GradientStopStyle, HyperlinkAction,
        ObjectHyperlinks, ObjectKind, SceneObject, ShadowStyle, SlideTransition, TextStyle,
        VisualStyle,
    };

    fn fixture() -> Vec<u8> {
        include_bytes!("../tests/fixtures/test1.pptx").to_vec()
    }

    #[test]
    fn no_op_export_returns_the_exact_source_package() {
        let source = fixture();
        let mut deck = import_pptx(&source).unwrap();
        deck.title = "renamed editor document".into();
        deck.source_import_id = Some("opaque-session-handle".into());

        let output = export_pptx(&deck, Some(&source)).unwrap();

        assert_eq!(output, source);
    }

    #[test]
    fn no_op_export_tolerates_json_float_rounding_noise() {
        let source = fixture();
        let mut deck = import_pptx(&source).unwrap();
        deck.slides[0].objects[0].frame.x += 1.0e-13;

        let output = export_pptx(&deck, Some(&source)).unwrap();

        assert_eq!(output, source);
    }

    #[test]
    fn explicit_cached_baseline_supports_dirty_export() {
        let source = fixture();
        let baseline = import_pptx(&source).unwrap();
        let mut edited = baseline.clone();
        edited.slides[0]
            .objects
            .iter_mut()
            .find(|object| !object.text.is_empty())
            .unwrap()
            .text = "cached baseline edit".into();

        let output = export_pptx_with_baseline(&edited, Some(&source), Some(&baseline)).unwrap();
        let restored = import_pptx(&output).unwrap();
        assert!(restored.slides[0]
            .objects
            .iter()
            .any(|object| object.text == "cached baseline edit"));
    }

    #[test]
    fn portable_extensions_round_trip_through_native_custom_xml() {
        let source = fixture();
        let baseline = import_pptx(&source).unwrap();
        let mut edited = baseline.clone();
        edited.extensions.insert(
            "org.unippt.dynamic".into(),
            serde_json::json!({
                "objects": {
                    "dynamic-1": {
                        "objectName": "UniPPT Dynamic · SVG",
                        "kind": "svg",
                        "source": "<svg xmlns=\"http://www.w3.org/2000/svg\"><circle r=\"8\"/></svg>"
                    }
                }
            }),
        );

        let output = export_pptx_with_baseline(&edited, Some(&source), Some(&baseline)).unwrap();
        let restored = import_pptx(&output).unwrap();
        assert_eq!(restored.extensions, edited.extensions);

        let package = Presentation::from_bytes(&output).unwrap();
        let extension_part = PackURI::new(UNIPPT_EXTENSIONS_PART).unwrap();
        assert!(package.package().part(&extension_part).is_some());
        assert!(package
            .package()
            .part_by_reltype(OFFICE_DOCUMENT_RELATIONSHIP)
            .unwrap()
            .rels
            .iter()
            .any(|relationship| relationship.rel_type == CUSTOM_XML_RELATIONSHIP));
    }

    #[test]
    fn portable_extensions_migrate_legacy_location_without_losing_other_parts() {
        let mut package = Presentation::from_bytes(&fixture()).unwrap();
        let legacy = PackURI::new(LEGACY_UNIPPT_EXTENSIONS_PART).unwrap();
        let unrelated = PackURI::new("/customXml/third-party.xml").unwrap();
        let payload = STANDARD.encode(br#"{"org.unippt.test":{"preserve":true}}"#);
        package.package_mut().put_part(Part::new(legacy.clone(), "application/xml",
            format!("<?xml version=\"1.0\"?><unippt:extensions xmlns:unippt=\"urn:unippt:extensions:v1\" encoding=\"base64\">{payload}</unippt:extensions>").into_bytes()));
        package.package_mut().put_part(Part::new(unrelated.clone(), "application/xml", b"<third-party/>".to_vec()));
        let rels = &mut package.package_mut().part_by_reltype_mut(OFFICE_DOCUMENT_RELATIONSHIP).unwrap().rels;
        rels.or_add(CUSTOM_XML_RELATIONSHIP, "../customXml/unippt-extensions.xml", false);
        rels.or_add(CUSTOM_XML_RELATIONSHIP, "../customXml/third-party.xml", false);
        let source = package.to_bytes().unwrap();
        let baseline = import_pptx(&source).unwrap();
        assert_eq!(baseline.extensions["org.unippt.test"]["preserve"], true);
        assert_eq!(export_pptx_with_baseline(&baseline, Some(&source), Some(&baseline)).unwrap(), source);
        let mut edited = baseline.clone();
        edited.extensions.insert("org.unippt.new".into(), serde_json::json!({"value": 2}));
        let output = export_pptx_with_baseline(&edited, Some(&source), Some(&baseline)).unwrap();
        assert_eq!(import_pptx(&output).unwrap().extensions, edited.extensions);
        let migrated = Presentation::from_bytes(&output).unwrap();
        assert!(migrated.package().part(&legacy).is_none());
        assert_eq!(migrated.package().part(&unrelated).unwrap().blob, b"<third-party/>");
        let rels = &migrated.package().part_by_reltype(OFFICE_DOCUMENT_RELATIONSHIP).unwrap().rels;
        assert!(rels.iter().any(|r| r.target_ref == "customXml/unippt-extensions.xml"));
        assert!(!rels.iter().any(|r| r.target_ref == "../customXml/unippt-extensions.xml"));
        assert!(rels.iter().any(|r| r.target_ref == "../customXml/third-party.xml"));

        let mut cleared = edited.clone();
        cleared.extensions.clear();
        let cleared_bytes = export_pptx_with_baseline(&cleared, Some(&output), Some(&edited)).unwrap();
        let cleared_package = Presentation::from_bytes(&cleared_bytes).unwrap();
        assert!(import_pptx(&cleared_bytes).unwrap().extensions.is_empty());
        assert!(cleared_package.package().part(&PackURI::new(UNIPPT_EXTENSIONS_PART).unwrap()).is_none());
        let rels = &cleared_package.package().part_by_reltype(OFFICE_DOCUMENT_RELATIONSHIP).unwrap().rels;
        assert!(!rels.iter().any(|r| r.target_ref.ends_with("unippt-extensions.xml")));
        assert!(cleared_package.package().part(&unrelated).is_some());
    }

    #[test]
    fn native_text_without_runs_keeps_rotation_and_text_frame() {
        let deck = import_pptx(&fixture()).unwrap();
        let mut object = deck.slides[0].objects[0].clone();
        object.kind = ObjectKind::Text;
        object.text = "Step 1: Prior Generation".into();
        object.text_paragraphs.clear();
        object.frame.rotation = -90.0;
        object.text_frame.word_wrap = false;
        object.text_frame.margin_left = 0.0;
        object.text_frame.margin_right = 0.0;
        object.text_frame.vertical_type = "vert270".into();
        let xml = text_shape_xml(ShapeId(901), &object, &deck, "");
        assert!(xml.contains(r#"rot="-5400000""#));
        assert!(xml.contains(r#"wrap="none""#));
        assert!(xml.contains(r#"vert="vert270""#));
        assert!(xml.contains(r#"lIns="0" rIns="0""#));
    }

    #[test]
    fn new_picture_preserves_all_four_source_crop_insets() {
        let source = fixture();
        let deck = import_pptx(&source).unwrap();
        let mut presentation = Presentation::from_bytes(&source).unwrap();
        let slide = presentation.slides().unwrap()[0].clone();
        let mut object = deck.slides[0].objects[0].clone();
        object.kind = ObjectKind::Image;
        object.asset = Some("data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Y9ZbKkAAAAASUVORK5CYII=".into());
        object.image_crop.left = 0.1;
        object.image_crop.top = 0.2;
        object.image_crop.right = 0.3;
        object.image_crop.bottom = 0.4;
        let xml = picture_xml(&mut presentation, &slide, ShapeId(903), &object, &deck).unwrap();
        assert!(xml.contains(r#"<a:srcRect l="10000" t="20000" r="30000" b="40000"/>"#));
    }

    #[test]
    fn new_shape_ids_reserve_root_and_nested_group_identifiers() {
        assert_eq!(next_shape_identifier(br#"<p:spTree><p:nvGrpSpPr><p:cNvPr id="1"/></p:nvGrpSpPr></p:spTree>"#).unwrap(), 2);
        assert_eq!(next_shape_identifier(br#"<p:spTree><p:cNvPr id="2"/><p:grpSp><p:cNvPr id="99"/></p:grpSp></p:spTree>"#).unwrap(), 100);
    }

    #[test]
    fn new_connectors_export_as_native_connector_shapes() {
        let deck = import_pptx(&fixture()).unwrap();
        let mut object = deck.slides[0].objects[0].clone();
        object.kind = ObjectKind::Connector;
        object.geometry = Some("line".into());
        object.custom_geometry = None;
        object.text.clear();
        object.text_paragraphs.clear();
        let xml = text_shape_xml(ShapeId(902), &object, &deck, "");
        assert!(xml.starts_with("<p:cxnSp "));
        assert!(xml.contains("<p:cNvCxnSpPr/>"));
        assert!(xml.contains(r#"prst="line""#));
        assert!(!xml.contains("<p:txBody"));
    }

    #[test]
    fn image_background_exports_as_native_slide_background_not_a_movable_shape() {
        let source = fixture();
        let baseline = import_pptx(&source).unwrap();
        let original_object_count = baseline.slides[0].objects.len();
        let mut edited = baseline.clone();
        edited.slides[0].background_asset = Some(
            "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Y9ZbKkAAAAASUVORK5CYII=".into(),
        );

        let output = export_pptx_with_baseline(&edited, Some(&source), Some(&baseline)).unwrap();
        let restored = import_pptx(&output).unwrap();
        assert!(restored.slides[0]
            .background_asset
            .as_deref()
            .is_some_and(|asset| asset.starts_with("data:image/png;base64,")));
        assert_eq!(restored.slides[0].objects.len(), original_object_count);

        let reopened = Presentation::from_bytes(&output).unwrap();
        let slide = &reopened.slides().unwrap()[0];
        let xml = std::str::from_utf8(reopened.slide_xml(slide).unwrap()).unwrap();
        assert!(xml.contains("<p:bg><p:bgPr><a:blipFill>"));
        assert!(xml.contains("<a:blip r:embed=\"rId"));
    }

    #[test]
    fn patches_text_and_preserves_unknown_parts() {
        let mut source = Presentation::from_bytes(&fixture()).unwrap();
        let custom_uri = PackURI::new("/customXml/unippt-proof.xml").unwrap();
        source.package_mut().put_part(Part::new(
            custom_uri.clone(),
            "application/xml",
            b"<proof keep=\"exact\"/>".to_vec(),
        ));
        let source = source.to_bytes().unwrap();
        let mut deck = import_pptx(&source).unwrap();
        let text = deck.slides[0]
            .objects
            .iter_mut()
            .find(|object| !object.text.is_empty())
            .unwrap();
        text.text = "UniPPT 原生回写".into();
        text.frame.x += 20.0;

        let output = export_pptx(&deck, Some(&source)).unwrap();
        let reopened = Presentation::from_bytes(&output).unwrap();
        assert_eq!(
            reopened.package().part(&custom_uri).unwrap().blob,
            b"<proof keep=\"exact\"/>"
        );
        let imported = import_pptx(&output).unwrap();
        assert!(imported.slides[0]
            .objects
            .iter()
            .any(|object| object.text.contains("UniPPT 原生回写")));
    }

    #[test]
    fn inserts_native_powerpoint_equation() {
        let source = fixture();
        let mut deck = import_pptx(&source).unwrap();
        deck.slides[0].objects.push(SceneObject {
            id: "new-equation".into(),
            source_shape_id: None,
            name: "公式".into(),
            kind: ObjectKind::Math,
            frame: Frame {
                x: 220.0,
                y: 260.0,
                width: 600.0,
                height: 120.0,
                rotation: 0.0,
            },
            flip_h: false,
            flip_v: false,
            text: "E=mc^2".into(),
            text_paragraphs: vec![],
            text_frame: crate::model::TextFrameStyle::default(),
            geometry: None,
            custom_geometry: None,
            asset: None,
            image_crop: crate::model::ImageCrop::default(),
            image_fill_rect: crate::model::ImageFillRect::default(),
            image_effects: crate::model::ImageEffects::default(),
            shape_fill_asset: None,
            formula: Some(FormulaData {
                latex: "E=mc^2".into(),
                omml: Some(format!(
                    r#"<m:oMathPara xmlns:m="{MATH_NS}"><m:oMath><m:r><m:t>E=mc</m:t></m:r><m:sSup><m:e><m:r><m:t>c</m:t></m:r></m:e><m:sup><m:r><m:t>2</m:t></m:r></m:sup></m:sSup></m:oMath></m:oMathPara>"#
                )),
                display: true,
            }),
            media: None,
            table: None,
            chart: None,
            hyperlinks: crate::model::ObjectHyperlinks::default(),
            style: VisualStyle::default(),
            text_style: TextStyle {
                font_family: "Cambria Math".into(),
                native_font_family: Some("Cambria Math".into()),
                native_fonts: FontSlots::unified("Cambria Math"),
                font_size: 42.0,
                color: "#ffffff".into(),
                bold: false,
                italic: false,
                align: "center".into(),
            },
            children: vec![],
        });

        let output = export_pptx(&deck, Some(&source)).unwrap();
        let reopened = Presentation::from_bytes(&output).unwrap();
        let slide_xml =
            std::str::from_utf8(reopened.slide_xml(&reopened.slides().unwrap()[0]).unwrap())
                .unwrap();
        assert!(slide_xml.contains("<mc:AlternateContent"));
        assert!(slide_xml.contains("<a14:m>"));
        assert!(slide_xml.contains("<m:oMathPara"));
        let choice = slide_xml.split("<mc:Fallback>").next().unwrap();
        assert!(choice.contains(r#"<a:rPr lang="zh-CN" sz="3150" b="0" i="0">"#));
        assert!(choice.contains(r#"<a:srgbClr val="FFFFFF"/>"#));
        assert!(choice.contains(r#"<a:latin typeface="Cambria Math"/>"#));
        assert!(choice.contains(r#"<a:endParaRPr lang="zh-CN" sz="3150""#));

        let imported = import_pptx(&output).unwrap();
        assert!(imported.slides[0]
            .objects
            .iter()
            .any(|object| object.kind == ObjectKind::Math));
    }

    #[test]
    fn writes_editable_native_animation_timeline() {
        let source = fixture();
        let mut deck = import_pptx(&source).unwrap();
        let target = deck.slides[0]
            .objects
            .iter()
            .find(|object| object.source_shape_id.is_some())
            .unwrap()
            .clone();
        deck.slides[0].animations.push(AnimationEffect {
            id: "anim-test".into(),
            source_timing_id: None,
            target_object_id: Some(target.id),
            target_shape_id: target.source_shape_id,
            effect: AnimationKind::Fade,
            class: AnimationClass::Entrance,
            trigger: AnimationTrigger::OnClick,
            duration_ms: 720,
            delay_ms: 80,
            acceleration: Some(40_000),
            deceleration: Some(40_000),
            speed: Some(-100_000),
            time_filter: Some("0,0; .5,1; 1,1".into()),
            repeat_count: Some("2000".into()),
            repeat_duration_ms: Some(2_880),
            auto_reverse: true,
            order: 0,
            preset_id: None,
            preset_subtype: None,
            direction: None,
            motion_path: None,
            fade_filter: None,
            property_animations: vec![AnimationPropertyAnimation {
                attributes: vec!["ppt_x".into()],
                calculation_mode: Some("lin".into()),
                value_type: Some("num".into()),
                additive: Some("base".into()),
                bounce_end: Some(67_000),
                from: None,
                to: None,
                by: None,
                duration_ms: Some(720),
                fill: Some("hold".into()),
                keyframes: vec![
                    AnimationPropertyKeyframe {
                        time: 0,
                        value: "0-#ppt_w/2".into(),
                        formula: None,
                    },
                    AnimationPropertyKeyframe {
                        time: 100_000,
                        value: "#ppt_x".into(),
                        formula: None,
                    },
                ],
            }],
            media_action: None,
        });

        let output = export_pptx(&deck, Some(&source)).unwrap();
        let reopened = Presentation::from_bytes(&output).unwrap();
        let slide_xml =
            std::str::from_utf8(reopened.slide_xml(&reopened.slides().unwrap()[0]).unwrap())
                .unwrap();
        assert!(slide_xml.contains("<p:timing>"));
        assert!(slide_xml.contains("presetClass=\"entr\""));
        assert!(slide_xml.contains("dur=\"720\""));
        assert!(slide_xml.contains("accel=\"40000\""));
        assert!(slide_xml.contains("decel=\"40000\""));
        assert!(slide_xml.contains("spd=\"-100000\""));
        assert!(slide_xml.contains("tmFilter=\"0,0; .5,1; 1,1\""));
        assert!(slide_xml.contains("repeatCount=\"2000\""));
        assert!(slide_xml.contains("repeatDur=\"2880\""));
        assert!(slide_xml.contains("autoRev=\"1\""));
        assert!(slide_xml.contains("<p:attrName>ppt_x</p:attrName>"));
        assert!(slide_xml.contains("p14:bounceEnd=\"67000\""));
        assert!(slide_xml.contains("val=\"0-#ppt_w/2\""));

        let imported = import_pptx(&output).unwrap();
        assert_eq!(imported.slides[0].animations.len(), 1);
        let effect = &imported.slides[0].animations[0];
        assert_eq!(effect.effect, AnimationKind::Fade);
        assert_eq!(effect.fade_filter.as_deref(), Some("in"));
        assert_eq!(effect.duration_ms, 720);
        assert_eq!(effect.delay_ms, 80);
        assert_eq!(effect.acceleration, Some(40_000));
        assert_eq!(effect.deceleration, Some(40_000));
        assert_eq!(effect.speed, Some(-100_000));
        assert_eq!(effect.time_filter.as_deref(), Some("0,0; .5,1; 1,1"));
        assert_eq!(effect.repeat_count.as_deref(), Some("2000"));
        assert_eq!(effect.repeat_duration_ms, Some(2_880));
        assert!(effect.auto_reverse);
        assert_eq!(effect.target_shape_id, target.source_shape_id);
        assert_eq!(effect.property_animations.len(), 1);
        assert_eq!(effect.property_animations[0].attributes, ["ppt_x"]);
        assert_eq!(
            effect.property_animations[0].keyframes[0].value,
            "0-#ppt_w/2"
        );
    }

    #[test]
    fn writes_native_powerpoint_object_transition_presets() {
        let cases = [
            (AnimationKind::RandomBars, 14, 10, "randombar(horizontal)"),
            (AnimationKind::Dissolve, 9, 0, "dissolve"),
            (AnimationKind::Wheel, 21, 1, "wheel(1)"),
            (AnimationKind::Circle, 6, 16, "circle(in)"),
            (AnimationKind::Split, 16, 42, "barn(outHorizontal)"),
        ];

        for (kind, preset_id, preset_subtype, filter) in cases {
            let effect = AnimationEffect {
                id: format!("anim-{preset_id}"),
                source_timing_id: None,
                target_object_id: Some("shape-42".into()),
                target_shape_id: Some(42),
                effect: kind,
                class: AnimationClass::Entrance,
                trigger: AnimationTrigger::AfterPrevious,
                duration_ms: 500,
                delay_ms: 0,
                acceleration: None,
                deceleration: None,
                speed: None,
                time_filter: None,
                repeat_count: None,
                repeat_duration_ms: None,
                auto_reverse: false,
                order: 0,
                preset_id: None,
                preset_subtype: None,
                direction: None,
                motion_path: None,
                fade_filter: None,
                property_animations: vec![],
                media_action: None,
            };
            let mut next_id = 3;
            let xml = animation_effect_xml(&effect, 42, &mut next_id);
            assert!(xml.contains(&format!(r#"presetID="{preset_id}""#)));
            assert!(xml.contains(&format!(r#"presetSubtype="{preset_subtype}""#)));
            assert!(xml.contains(&format!(r#"filter="{filter}""#)));
        }
    }

    #[test]
    fn serializes_powerpoint_transition_controls() {
        let transition = SlideTransition {
            kind: "push".into(),
            duration_ms: 1_750,
            advance_on_click: false,
            advance_after_ms: Some(4_250),
            direction: Some("l".into()),
        };
        let xml = transition_xml(&transition);

        assert_eq!(
            xml,
            r#"<p:transition xmlns:p14="http://schemas.microsoft.com/office/powerpoint/2010/main" p14:dur="1750" spd="slow" advClick="0" advTm="4250"><p:push dir="l"/></p:transition>"#
        );

        let source = fixture();
        let mut deck = import_pptx(&source).unwrap();
        deck.slides[0].transition = Some(transition.clone());
        let output = export_pptx(&deck, Some(&source)).unwrap();
        let imported = import_pptx(&output).unwrap();
        let reopened = imported.slides[0].transition.as_ref().unwrap();
        assert_eq!(reopened.kind, transition.kind);
        assert_eq!(reopened.duration_ms, 1_750);
        assert!(!reopened.advance_on_click);
        assert_eq!(reopened.advance_after_ms, Some(4_250));
        assert_eq!(reopened.direction.as_deref(), Some("l"));
    }

    #[test]
    fn serializes_structured_gradient_dash_and_shadow() {
        let style = VisualStyle {
            fill: "linear-gradient(90deg, #FF0000, #0000FF)".into(),
            gradient: Some(GradientFillStyle {
                angle: 90.0,
                stops: vec![
                    GradientStopStyle {
                        position: 0.0,
                        color: "#FF0000".into(),
                        opacity: 1.0,
                    },
                    GradientStopStyle {
                        position: 1.0,
                        color: "#0000FF".into(),
                        opacity: 0.5,
                    },
                ],
            }),
            stroke: "#112233".into(),
            stroke_width: 2.0,
            stroke_dash: Some("dashDot".into()),
            opacity: 1.0,
            shadow: Some(ShadowStyle {
                color: "#000000".into(),
                opacity: 0.4,
                offset_x: 3.0,
                offset_y: 4.0,
                blur: 6.0,
                inset: false,
            }),
        };

        let fill = fill_xml(&style);
        assert!(fill.contains("<a:gradFill"));
        assert!(fill.contains("pos=\"100000\""));
        assert!(fill.contains("<a:alpha val=\"50000\"/>"));
        assert!(fill.contains("ang=\"0\""));
        let line = line_xml(&style, 9_525.0);
        assert!(line.contains("w=\"19050\""));
        assert!(line.contains("<a:prstDash val=\"dashDot\"/>"));
        let shadow = shadow_xml(&style, 9_525.0);
        assert!(shadow.contains("<a:outerShdw"));
        assert!(shadow.contains("dist=\"47625\""));
        assert!(shadow.contains("<a:alpha val=\"40000\"/>"));
    }

    #[test]
    fn inserts_missing_placeholder_transform_and_flip_overrides() {
        let deck = Deck::demo();
        let mut object = deck.slides[0].objects[0].clone();
        object.frame = Frame {
            x: 10.0,
            y: 20.0,
            width: 300.0,
            height: 140.0,
            rotation: 12.0,
        };
        object.flip_h = true;
        let source = r#"<p:sp><p:nvSpPr/><p:spPr><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:solidFill/></p:spPr><p:txBody/></p:sp>"#;
        let output = patch_transform(source.to_string(), &object, &deck);

        assert!(output.contains("<a:xfrm rot=\"720000\" flipH=\"1\">"));
        assert!(output.contains("<a:off x=\"95250\" y=\"190500\"/>"));
        assert!(output.find("<a:xfrm").unwrap() < output.find("<a:prstGeom").unwrap());
        assert!(output.contains("<a:solidFill/>"));
    }

    #[test]
    fn edits_shape_hyperlink_and_preserves_unknown_xml_and_relationships() {
        let mut presentation = Presentation::from_bytes(&fixture()).unwrap();
        let slide_ref = presentation.slides().unwrap()[0].clone();
        let slide_part = presentation
            .package_mut()
            .part_mut(&slide_ref.partname)
            .unwrap();
        slide_part
            .rels
            .add_relationship("urn:vendor:proof", "https://vendor.invalid/proof", true);
        let source = presentation.to_bytes().unwrap();
        let mut deck = import_pptx(&source).unwrap();
        let source_shape_id = {
            let object = deck.slides[0]
                .objects
                .iter_mut()
                .find(|object| object.source_shape_id.is_some())
                .unwrap();
            object.hyperlinks = ObjectHyperlinks {
                click: Some(HyperlinkAction {
                    target: Some("https://example.com/edited?a=1&b=2".into()),
                    relationship_id: None,
                    external: true,
                    action: None,
                    tooltip: Some("Open example".into()),
                }),
                hover: None,
            };
            object.source_shape_id
        };

        let output = export_pptx(&deck, Some(&source)).unwrap();
        let reopened = Presentation::from_bytes(&output).unwrap();
        let reopened_ref = &reopened.slides().unwrap()[0];
        let relationships = &reopened
            .package()
            .part(&reopened_ref.partname)
            .unwrap()
            .rels;
        assert!(relationships.iter().any(|relationship| {
            relationship.rel_type == "urn:vendor:proof"
                && relationship.target_ref == "https://vendor.invalid/proof"
        }));
        assert!(relationships.iter().any(|relationship| {
            relationship.rel_type == HYPERLINK_RELATIONSHIP
                && relationship.target_ref == "https://example.com/edited?a=1&b=2"
                && relationship.is_external
        }));

        let imported = import_pptx(&output).unwrap();
        let imported = imported.slides[0]
            .objects
            .iter()
            .find(|candidate| candidate.source_shape_id == source_shape_id)
            .unwrap();
        let click = imported.hyperlinks.click.as_ref().unwrap();
        assert_eq!(
            click.target.as_deref(),
            Some("https://example.com/edited?a=1&b=2")
        );
        assert_eq!(click.tooltip.as_deref(), Some("Open example"));
    }

    #[test]
    fn differential_hyperlink_patch_keeps_vendor_attributes_and_children() {
        let fragment = r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="Link"><a:hlinkClick r:id="rId4" tooltip="Before" vendor:flag="keep"><a:extLst><a:ext uri="vendor"/></a:extLst></a:hlinkClick><a:extLst><a:ext uri="shape"/></a:extLst></p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/></p:sp>"#;
        let action = HyperlinkAction {
            target: Some("https://example.com".into()),
            relationship_id: Some("rId9".into()),
            external: true,
            action: Some("ppaction://custom".into()),
            tooltip: Some("After".into()),
        };

        let output = patch_hyperlink_element(fragment, "a:hlinkClick", Some(&action), Some("rId9"));

        assert!(output.contains("r:id=\"rId9\""));
        assert!(output.contains("tooltip=\"After\""));
        assert!(output.contains("action=\"ppaction://custom\""));
        assert!(output.contains("vendor:flag=\"keep\""));
        assert!(output.contains("<a:ext uri=\"vendor\"/>"));
        assert!(output.contains("<a:ext uri=\"shape\"/>"));
    }

    #[test]
    fn differential_hyperlink_patch_inserts_attributes_before_self_closing_slash() {
        let fragment = r#"<p:sp><p:nvSpPr><p:cNvPr id="2"><a:hlinkClick vendor:flag="keep"/></p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/></p:sp>"#;
        let action = HyperlinkAction {
            target: None,
            relationship_id: None,
            external: false,
            action: Some("ppaction://hlinkshowjump?jump=nextslide".into()),
            tooltip: Some("Next".into()),
        };

        let output = patch_hyperlink_element(fragment, "a:hlinkClick", Some(&action), None);

        assert!(output.contains(r#"vendor:flag="keep" action="ppaction://hlinkshowjump?jump=nextslide" tooltip="Next"/>"#));
        assert!(!output.contains("/ action="));
    }

    #[test]
    fn run_hyperlink_round_trip_stays_on_the_run_and_keeps_unknown_xml() {
        let mut presentation = Presentation::from_bytes(&fixture()).unwrap();
        let slide_ref = presentation.slides().unwrap()[0].clone();
        let relationship_id = presentation
            .package_mut()
            .part_mut(&slide_ref.partname)
            .unwrap()
            .rels
            .add_relationship(
                HYPERLINK_RELATIONSHIP,
                "https://example.com/before?a=1&b=2",
                true,
            );
        let slide_xml = presentation.slide_xml(&slide_ref).unwrap().to_vec();
        let shape_id = ShapeTree::from_slide_xml(&slide_xml)
            .unwrap()
            .max_shape_id()
            .0
            + 1;
        let fragment = format!(
            r#"<p:sp xmlns:a="{DRAWING_NS}" xmlns:p="{PRESENTATION_NS}" xmlns:r="{RELATIONSHIP_NS}" xmlns:v="urn:unippt:test"><p:nvSpPr><p:cNvPr id="{shape_id}" name="Run Link"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="500000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang="en-US" sz="1800"><a:hlinkClick r:id="{relationship_id}" tooltip="Before" v:flag="keep"><v:payload exact="yes"/></a:hlinkClick></a:rPr><a:t>linked run</a:t></a:r><a:endParaRPr/></a:p></p:txBody></p:sp>"#
        );
        *presentation.slide_xml_mut(&slide_ref).unwrap() =
            ShapeTree::insert_shape_xml(&slide_xml, &fragment).unwrap();
        let source = presentation.to_bytes().unwrap();

        let mut deck = import_pptx(&source).unwrap();
        let object = deck.slides[0]
            .objects
            .iter_mut()
            .find(|object| object.text.contains("linked run"))
            .unwrap();
        assert!(object.hyperlinks.click.is_none());
        let run = &mut object.text_paragraphs[0].runs[0];
        assert_eq!(
            run.hyperlinks.click.as_ref().unwrap().target.as_deref(),
            Some("https://example.com/before?a=1&b=2")
        );
        run.hyperlinks.click.as_mut().unwrap().target =
            Some("https://example.com/after?a=3&b=4".into());
        run.hyperlinks.click.as_mut().unwrap().tooltip = Some("After".into());
        run.hyperlinks.hover = Some(HyperlinkAction {
            target: Some("mailto:qa@example.com".into()),
            relationship_id: None,
            external: true,
            action: None,
            tooltip: Some("Mail QA".into()),
        });

        let output = export_pptx(&deck, Some(&source)).unwrap();
        let reopened = Presentation::from_bytes(&output).unwrap();
        let output_xml = std::str::from_utf8(reopened.slide_xml(&slide_ref).unwrap()).unwrap();
        assert!(output_xml.contains(r#"v:flag="keep""#));
        assert!(output_xml.contains(r#"<v:payload exact="yes"/>"#));
        assert!(output_xml.contains("<a:hlinkMouseOver"));

        let imported = import_pptx(&output).unwrap();
        let object = imported.slides[0]
            .objects
            .iter()
            .find(|object| object.text.contains("linked run"))
            .unwrap();
        assert!(object.hyperlinks.click.is_none());
        let run = &object.text_paragraphs[0].runs[0];
        assert_eq!(
            run.hyperlinks.click.as_ref().unwrap().target.as_deref(),
            Some("https://example.com/after?a=3&b=4")
        );
        assert_eq!(
            run.hyperlinks.hover.as_ref().unwrap().target.as_deref(),
            Some("mailto:qa@example.com")
        );
    }

    #[test]
    fn html_first_custom_geometry_serializes_as_native_drawingml() {
        let geometry = crate::model::CustomGeometry {
            width: 320,
            height: 180,
            path_data: "M 10 20 L 100 80 C 120 90 140 100 160 120 Z".into(),
        };
        let xml = custom_geometry_xml(&geometry).unwrap();
        assert!(xml.contains(r#"<a:path w="320" h="180">"#));
        assert!(xml.contains(r#"<a:moveTo><a:pt x="10" y="20"/></a:moveTo>"#));
        assert!(xml.contains(r#"<a:lnTo><a:pt x="100" y="80"/></a:lnTo>"#));
        assert!(xml.contains("<a:cubicBezTo>"));
        assert!(xml.contains("<a:close/>"));
    }

    #[test]
    fn newly_drawn_ink_round_trips_as_editable_native_freeform() {
        let mut deck = Deck::demo();
        let mut ink = deck.slides[0].objects[0].clone();
        ink.id = "ink-roundtrip".into();
        ink.source_shape_id = None;
        ink.name = "墨迹·钢笔".into();
        ink.kind = ObjectKind::Shape;
        ink.text.clear();
        ink.text_paragraphs.clear();
        ink.geometry = None;
        ink.custom_geometry = Some(crate::model::CustomGeometry {
            width: 320,
            height: 180,
            path_data: "M 10 20 L 100 80 L 300 150".into(),
        });
        ink.style.fill = "transparent".into();
        ink.style.stroke = "#202124".into();
        ink.style.stroke_width = 4.0;
        deck.slides[0].objects.push(ink);

        let output = export_pptx(&deck, None).unwrap();
        let imported = import_pptx(&output).unwrap();
        let ink = imported.slides[0]
            .objects
            .iter()
            .find(|object| object.name == "墨迹·钢笔")
            .unwrap();
        assert_eq!(ink.custom_geometry.as_ref().unwrap().width, 320);
        assert!(ink
            .custom_geometry
            .as_ref()
            .unwrap()
            .path_data
            .contains("L 300 150"));
        assert_eq!(ink.style.stroke.to_ascii_lowercase(), "#202124");
    }
}
