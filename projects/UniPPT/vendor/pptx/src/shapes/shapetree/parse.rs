//! XML parsing logic for extracting shapes from a slide's `<p:spTree>` element.

use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, Writer};

use crate::enums::shapes::{PlaceholderOrientation, PlaceholderSize, PpPlaceholderType};
use crate::error::{PptxError, PptxResult};
use crate::shapes::placeholder::PlaceholderFormat;
use crate::shapes::Shape;
use crate::units::{PlaceholderIndex, ShapeId};
use crate::xml_util::{attr_value, attr_value_ns, local_name_owned, read_inner_xml};

use super::parse_accum::{ShapeAccum, ShapeKind};
use super::xml_capture::{CaptureTarget, XmlCapture};

// --- Internal types for state machine parser ---

#[derive(Debug, PartialEq)]
enum ParseState {
    Seeking,
    InSpTree,
    InShape,
}

struct ElementCtx {
    local: String,
}

/// Process a Start or Empty element event within a shape context.
fn process_start_element(
    local: &str,
    e: &BytesStart<'_>,
    parent_local: Option<&str>,
    accum: &mut ShapeAccum,
) -> PptxResult<()> {
    match local {
        "cNvPr" => {
            accum.shape_id = ShapeId(parse_u32_attr(e, b"id")?);
            accum.name = attr_value(e, b"name")?
                .map(std::borrow::Cow::into_owned)
                .unwrap_or_default();
            if let Some(desc) = attr_value(e, b"descr")? {
                accum.description = Some(desc.into_owned());
            }
        }
        "cNvSpPr" => {
            accum.is_textbox = attr_value(e, b"txBox")?.as_deref() == Some("1");
        }
        "ph" => {
            accum.placeholder = Some(PlaceholderFormat {
                ph_type: attr_value(e, b"type")?.and_then(|c| PpPlaceholderType::from_xml_str(&c)),
                idx: PlaceholderIndex(parse_u32_attr(e, b"idx")?),
                orient: attr_value(e, b"orient")?
                    .and_then(|c| PlaceholderOrientation::from_xml_str(&c)),
                sz: attr_value(e, b"sz")?.and_then(|c| PlaceholderSize::from_xml_str(&c)),
            });
        }
        "off" if parent_local == Some("xfrm") => {
            accum.left = parse_i64_attr(e, b"x")?;
            accum.top = parse_i64_attr(e, b"y")?;
        }
        "ext" if parent_local == Some("xfrm") => {
            accum.width = parse_i64_attr(e, b"cx")?;
            accum.height = parse_i64_attr(e, b"cy")?;
        }
        "xfrm" => {
            if let Some(rot_str) = attr_value(e, b"rot")? {
                if let Ok(rot_val) = rot_str.parse::<i64>() {
                    // i64→f64: OOXML rotation values fit in 53-bit mantissa
                    #[allow(clippy::cast_precision_loss)]
                    {
                        accum.rotation = rot_val as f64 / 60000.0;
                    }
                }
            }
            accum.flip_h = attr_value(e, b"flipH")?.as_deref() == Some("1");
            accum.flip_v = attr_value(e, b"flipV")?.as_deref() == Some("1");
        }
        "prstGeom" => {
            accum.prst_geom = attr_value(e, b"prst")?.map(std::borrow::Cow::into_owned);
        }
        "txBody" => {
            accum.has_tx_body = true;
        }
        "blip" => {
            // r:embed attribute (namespaced)
            accum.image_r_id = attr_value_ns(e, b"embed")?.map(std::borrow::Cow::into_owned);
        }
        "srcRect" if parent_local == Some("blipFill") => {
            // Picture source rectangles use signed 1/100000 percentages.  These
            // values are part of the visual framing, not merely edit metadata:
            // dropping them stretches the entire bitmap into the picture box.
            accum.crop_left = parse_percentage_attr(e, b"l")?;
            accum.crop_top = parse_percentage_attr(e, b"t")?;
            accum.crop_right = parse_percentage_attr(e, b"r")?;
            accum.crop_bottom = parse_percentage_attr(e, b"b")?;
        }
        "graphicData" => {
            accum.graphic_data_uri = attr_value(e, b"uri")?.map(std::borrow::Cow::into_owned);
        }
        "relIds" => {
            // SmartArt diagram: <dgm:relIds r:dm="rIdN" .../>
            accum.smartart_r_id = attr_value_ns(e, b"dm")?.map(std::borrow::Cow::into_owned);
        }
        _ => {}
    }
    Ok(())
}

// --- Attribute parsing helpers ---

/// Parse a `u32` attribute, returning `0` when the attribute is absent or
/// non-numeric.  This is intentional: OOXML attributes such as `id`, `idx`,
/// and other numeric shape properties default to 0 when omitted.
fn parse_u32_attr(e: &BytesStart<'_>, key: &[u8]) -> PptxResult<u32> {
    Ok(attr_value(e, key)?
        .and_then(|s| s.parse().ok())
        .unwrap_or(0))
}

/// Parse an `i64` attribute, returning `0` when the attribute is absent or
/// non-numeric.  Position and extent attributes (`x`, `y`, `cx`, `cy`) in
/// OOXML default to 0 EMU when not present.
fn parse_i64_attr(e: &BytesStart<'_>, key: &[u8]) -> PptxResult<i64> {
    Ok(attr_value(e, key)?
        .and_then(|s| s.parse().ok())
        .unwrap_or(0))
}

fn parse_percentage_attr(e: &BytesStart<'_>, key: &[u8]) -> PptxResult<f64> {
    Ok(attr_value(e, key)?
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.0)
        / 100_000.0)
}

/// Parse the direct children of `p:spTree` while retaining recursive group
/// membership. PowerPoint treats every group as a single item in the parent
/// paint order; flattening it here loses both its child coordinate system and
/// its stacking context.
pub(super) fn parse_shapes_from_slide_xml(xml: &[u8]) -> PptxResult<Vec<Shape>> {
    capture_direct_shapes(xml, "spTree")?
        .iter()
        .map(|fragment| parse_shape_fragment(fragment))
        .collect()
}

fn parse_shape_fragment(fragment: &[u8]) -> PptxResult<Shape> {
    if root_local_name(fragment).as_deref() == Some("grpSp") {
        return parse_group_fragment(fragment);
    }
    let mut wrapped = br#"<p:sld xmlns:p="p" xmlns:a="a" xmlns:r="r"><p:cSld><p:spTree>"#.to_vec();
    wrapped.extend_from_slice(fragment);
    wrapped.extend_from_slice(b"</p:spTree></p:cSld></p:sld>");
    let mut parsed = parse_shapes_from_slide_xml_legacy(&wrapped)?;
    if parsed.len() != 1 {
        return Err(PptxError::InvalidXml(format!(
            "expected one shape fragment, found {}",
            parsed.len()
        )));
    }
    Ok(parsed.remove(0))
}

fn parse_group_fragment(fragment: &[u8]) -> PptxResult<Shape> {
    use crate::shapes::group::GroupShape;
    use crate::units::Emu;

    let mut reader = Reader::from_reader(fragment);
    reader.config_mut().trim_text(true);
    let mut depth = 0usize;
    let mut in_group_xfrm = false;
    let mut xfrm_depth = 0usize;
    let mut shape_id = ShapeId(0);
    let mut name = String::new();
    let mut left = 0i64;
    let mut top = 0i64;
    let mut width = 0i64;
    let mut height = 0i64;
    let mut child_left = None;
    let mut child_top = None;
    let mut child_width = None;
    let mut child_height = None;
    let mut rotation = 0.0;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref element)) => {
                let local = local_name_owned(element.name().as_ref());
                if depth == 1 && is_shape_tag(&local) {
                    break;
                }
                if local == "cNvPr" && shape_id.0 == 0 {
                    shape_id = ShapeId(parse_u32_attr(element, b"id")?);
                    name = attr_value(element, b"name")?
                        .map(std::borrow::Cow::into_owned)
                        .unwrap_or_default();
                } else if local == "xfrm" && depth == 2 {
                    in_group_xfrm = true;
                    xfrm_depth = depth;
                    if let Some(value) = attr_value(element, b"rot")? {
                        rotation = value.parse::<f64>().unwrap_or(0.0) / 60_000.0;
                    }
                } else if in_group_xfrm {
                    update_group_transform(
                        &local,
                        element,
                        &mut left,
                        &mut top,
                        &mut width,
                        &mut height,
                        &mut child_left,
                        &mut child_top,
                        &mut child_width,
                        &mut child_height,
                    )?;
                }
                depth += 1;
            }
            Ok(Event::Empty(ref element)) => {
                let local = local_name_owned(element.name().as_ref());
                if local == "cNvPr" && shape_id.0 == 0 {
                    shape_id = ShapeId(parse_u32_attr(element, b"id")?);
                    name = attr_value(element, b"name")?
                        .map(std::borrow::Cow::into_owned)
                        .unwrap_or_default();
                } else if in_group_xfrm {
                    update_group_transform(
                        &local,
                        element,
                        &mut left,
                        &mut top,
                        &mut width,
                        &mut height,
                        &mut child_left,
                        &mut child_top,
                        &mut child_width,
                        &mut child_height,
                    )?;
                }
            }
            Ok(Event::End(ref element)) => {
                depth = depth.saturating_sub(1);
                if in_group_xfrm
                    && depth == xfrm_depth
                    && local_name_owned(element.name().as_ref()) == "xfrm"
                {
                    in_group_xfrm = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(error) => return Err(PptxError::Xml(error)),
            _ => {}
        }
    }

    let (fill, line) = parse_group_shape_properties(fragment)?;
    let children = capture_direct_shapes(fragment, "grpSp")?
        .iter()
        .map(|child| parse_shape_fragment(child))
        .collect::<PptxResult<Vec<_>>>()?;
    Ok(Shape::GroupShape(Box::new(GroupShape {
        shape_id,
        name,
        left: Emu(left),
        top: Emu(top),
        width: Emu(width),
        height: Emu(height),
        rotation,
        child_left: Emu(child_left.unwrap_or(left)),
        child_top: Emu(child_top.unwrap_or(top)),
        child_width: Emu(child_width.unwrap_or(width)),
        child_height: Emu(child_height.unwrap_or(height)),
        fill,
        line,
        shapes: children,
    })))
}

fn parse_group_shape_properties(
    fragment: &[u8],
) -> PptxResult<(
    Option<crate::dml::fill::FillFormat>,
    Option<crate::dml::line::LineFormat>,
)> {
    let mut reader = Reader::from_reader(fragment);
    reader.config_mut().trim_text(false);
    let mut depth = 0usize;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref element)) => {
                let local = local_name_owned(element.name().as_ref());
                if depth == 1 && local == "grpSpPr" {
                    let inner = read_inner_xml(&mut reader, "grpSpPr")
                        .map_err(|error| PptxError::InvalidXml(format!("grpSpPr: {error}")))?;
                    let mut properties = b"<p:grpSpPr>".to_vec();
                    properties.extend_from_slice(&inner);
                    properties.extend_from_slice(b"</p:grpSpPr>");
                    return crate::shapes::parser::parse_sp_pr(&properties);
                }
                depth += 1;
            }
            Ok(Event::Empty(ref element)) => {
                if depth == 1 && local_name_owned(element.name().as_ref()) == "grpSpPr" {
                    return Ok((None, None));
                }
            }
            Ok(Event::End(_)) => depth = depth.saturating_sub(1),
            Ok(Event::Eof) => return Ok((None, None)),
            Err(error) => return Err(PptxError::Xml(error)),
            _ => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn update_group_transform(
    local: &str,
    element: &BytesStart<'_>,
    left: &mut i64,
    top: &mut i64,
    width: &mut i64,
    height: &mut i64,
    child_left: &mut Option<i64>,
    child_top: &mut Option<i64>,
    child_width: &mut Option<i64>,
    child_height: &mut Option<i64>,
) -> PptxResult<()> {
    match local {
        "off" => {
            *left = parse_i64_attr(element, b"x")?;
            *top = parse_i64_attr(element, b"y")?;
        }
        "ext" => {
            *width = parse_i64_attr(element, b"cx")?;
            *height = parse_i64_attr(element, b"cy")?;
        }
        "chOff" => {
            *child_left = Some(parse_i64_attr(element, b"x")?);
            *child_top = Some(parse_i64_attr(element, b"y")?);
        }
        "chExt" => {
            *child_width = Some(parse_i64_attr(element, b"cx")?);
            *child_height = Some(parse_i64_attr(element, b"cy")?);
        }
        _ => {}
    }
    Ok(())
}

fn is_shape_tag(local: &str) -> bool {
    matches!(local, "sp" | "pic" | "graphicFrame" | "cxnSp" | "grpSp")
}

fn root_local_name(xml: &[u8]) -> Option<String> {
    let mut reader = Reader::from_reader(xml);
    loop {
        match reader.read_event().ok()? {
            Event::Start(element) | Event::Empty(element) => {
                return Some(local_name_owned(element.name().as_ref()));
            }
            Event::Eof => return None,
            _ => {}
        }
    }
}

fn capture_direct_shapes(xml: &[u8], container: &str) -> PptxResult<Vec<Vec<u8>>> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut depth = 0usize;
    let mut container_depth = None;
    let mut alternate_content_depth = None;
    let mut choice_depth = None;
    let mut capture: Option<(Writer<Vec<u8>>, usize)> = None;
    let mut output = Vec::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(ref element)) => {
                let local = local_name_owned(element.name().as_ref());
                if let Some((writer, captured_depth)) = capture.as_mut() {
                    writer.write_event(Event::Start(element.to_owned()))?;
                    *captured_depth += 1;
                } else if container_depth.is_none() && local == container {
                    container_depth = Some(depth);
                } else if container_depth.is_some_and(|base| depth == base + 1)
                    && local == "AlternateContent"
                {
                    alternate_content_depth = Some(depth);
                } else if alternate_content_depth.is_some_and(|base| depth == base + 1)
                    && local == "Choice"
                {
                    choice_depth = Some(depth);
                } else if is_shape_tag(&local)
                    && (container_depth.is_some_and(|base| depth == base + 1)
                        || choice_depth.is_some_and(|base| depth == base + 1))
                {
                    let mut writer = Writer::new(Vec::new());
                    writer.write_event(Event::Start(element.to_owned()))?;
                    capture = Some((writer, 1));
                }
                depth += 1;
            }
            Ok(Event::Empty(ref element)) => {
                let local = local_name_owned(element.name().as_ref());
                if let Some((writer, _)) = capture.as_mut() {
                    writer.write_event(Event::Empty(element.to_owned()))?;
                } else if is_shape_tag(&local)
                    && (container_depth.is_some_and(|base| depth == base + 1)
                        || choice_depth.is_some_and(|base| depth == base + 1))
                {
                    let mut writer = Writer::new(Vec::new());
                    writer.write_event(Event::Empty(element.to_owned()))?;
                    output.push(writer.into_inner());
                }
            }
            Ok(Event::Text(ref text)) => {
                if let Some((writer, _)) = capture.as_mut() {
                    writer.write_event(Event::Text(text.to_owned()))?;
                }
            }
            Ok(Event::CData(ref text)) => {
                if let Some((writer, _)) = capture.as_mut() {
                    writer.write_event(Event::CData(text.to_owned()))?;
                }
            }
            Ok(Event::End(ref element)) => {
                depth = depth.saturating_sub(1);
                if let Some((writer, captured_depth)) = capture.as_mut() {
                    writer.write_event(Event::End(element.to_owned()))?;
                    *captured_depth = captured_depth.saturating_sub(1);
                    if *captured_depth == 0 {
                        let (writer, _) = capture.take().expect("capture exists at depth zero");
                        output.push(writer.into_inner());
                    }
                } else if container_depth == Some(depth)
                    && local_name_owned(element.name().as_ref()) == container
                {
                    break;
                } else {
                    let local = local_name_owned(element.name().as_ref());
                    if choice_depth == Some(depth) && local == "Choice" {
                        choice_depth = None;
                    }
                    if alternate_content_depth == Some(depth) && local == "AlternateContent" {
                        alternate_content_depth = None;
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(error) => return Err(PptxError::Xml(error)),
            _ => {}
        }
    }
    Ok(output)
}

/// Legacy single-shape parser used after the outer recursive collector has
/// isolated a non-group shape fragment.
#[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
fn parse_shapes_from_slide_xml_legacy(xml: &[u8]) -> PptxResult<Vec<Shape>> {
    let mut reader = Reader::from_reader(xml);
    // Text events inside a:t are copied verbatim into the captured txBody.
    // Trimming here erases whitespace-only runs (for example `<a:t> </a:t>`),
    // joining adjacent words and changing PowerPoint line wrapping.
    reader.config_mut().trim_text(false);

    let mut shapes = Vec::new();
    let mut buf = Vec::new();
    let mut state = ParseState::Seeking;

    // State-machine parser: track where we are in the XML tree
    // using a stack of element contexts.
    let mut element_stack: Vec<ElementCtx> = Vec::new();
    let mut sp_tree_depth: Option<usize> = None;

    // Accumulators for shape being built
    let mut current_shape: Option<ShapeAccum> = None;
    let mut current_shape_depth: Option<usize> = None;

    // XML capture state for sub-elements (spPr, txBody, ln)
    let mut capture: Option<XmlCapture> = None;

    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let qname = e.name();
                let local = local_name_owned(qname.as_ref());
                let stack_depth = element_stack.len();

                // If we're capturing, record this event AND still process
                if let Some(ref mut cap) = capture {
                    cap.depth += 1;
                    cap.push_start(e);
                    // Also forward to shape accumulator for basic property extraction
                    if let Some(ref mut accum) = current_shape {
                        process_start_element(
                            &local,
                            e,
                            element_stack.last().map(|context| context.local.as_str()),
                            accum,
                        )?;
                    }
                } else {
                    match &state {
                        ParseState::Seeking => {
                            if local == "spTree" {
                                sp_tree_depth = Some(stack_depth);
                                state = ParseState::InSpTree;
                            }
                        }
                        ParseState::InSpTree => {
                            // Normal shapes are direct children of spTree. Office 2010+
                            // equations are instead nested under
                            // mc:AlternateContent/mc:Choice; parse the Choice branch and
                            // deliberately skip the fallback rendering branch.
                            let is_direct_child = Some(stack_depth) == sp_tree_depth.map(|d| d + 1);
                            let is_choice_shape =
                                element_stack.iter().any(|ctx| ctx.local == "Choice")
                                    && !element_stack.iter().any(|ctx| ctx.local == "Fallback");
                            if is_direct_child || is_choice_shape {
                                let kind = match local.as_str() {
                                    "sp" => Some(ShapeKind::Sp),
                                    "pic" => Some(ShapeKind::Pic),
                                    "graphicFrame" => Some(ShapeKind::GraphicFrame),
                                    "cxnSp" => Some(ShapeKind::CxnSp),
                                    "grpSp" => Some(ShapeKind::GrpSp),
                                    _ => None,
                                };
                                if let Some(k) = kind {
                                    current_shape = Some(ShapeAccum::new(k));
                                    current_shape_depth = Some(stack_depth);
                                    state = ParseState::InShape;
                                }
                            }
                        }
                        ParseState::InShape => {
                            if let Some(ref mut accum) = current_shape {
                                process_start_element(
                                    &local,
                                    e,
                                    element_stack.last().map(|context| context.local.as_str()),
                                    accum,
                                )?;
                            }
                            // Start capturing spPr or txBody
                            match local.as_str() {
                                "spPr" => {
                                    let mut cap = XmlCapture::new(CaptureTarget::SpPr);
                                    cap.push_start_with_tag("p:spPr", e);
                                    cap.depth = 1;
                                    capture = Some(cap);
                                }
                                "txBody" => {
                                    let mut cap = XmlCapture::new(CaptureTarget::TxBody);
                                    cap.push_start_with_tag("p:txBody", e);
                                    cap.depth = 1;
                                    capture = Some(cap);
                                }
                                "duotone" => {
                                    let mut cap = XmlCapture::new(CaptureTarget::Duotone);
                                    cap.push_start(e);
                                    cap.depth = 1;
                                    capture = Some(cap);
                                }
                                _ => {}
                            }
                        }
                    }
                }

                element_stack.push(ElementCtx { local });
            }
            Ok(Event::Empty(ref e)) => {
                let qname = e.name();
                let local = local_name_owned(qname.as_ref());

                if let Some(ref mut cap) = capture {
                    cap.push_empty(e);
                    // Also forward to shape accumulator for basic property extraction
                    if let Some(ref mut accum) = current_shape {
                        process_start_element(
                            &local,
                            e,
                            element_stack.last().map(|context| context.local.as_str()),
                            accum,
                        )?;
                    }
                } else if state == ParseState::InShape {
                    if let Some(ref mut accum) = current_shape {
                        process_start_element(
                            &local,
                            e,
                            element_stack.last().map(|context| context.local.as_str()),
                            accum,
                        )?;
                    }
                }
            }
            Ok(Event::Text(ref t)) => {
                if let Some(ref mut cap) = capture {
                    cap.push_text(t.as_ref());
                }
            }
            Ok(Event::End(ref e)) => {
                if let Some(ref mut cap) = capture {
                    cap.depth -= 1;
                    if cap.depth == 0 {
                        // Close the captured element with the full QName
                        cap.push_end_raw(e.name().as_ref());
                        // Store captured XML in the accumulator.
                        // EXCEPTION(infallible): `capture` is `Some` here because we are
                        // inside the `if let Some(ref mut cap) = capture` branch and `take()`
                        // on the same binding always returns `Some`.
                        let Some(finished) = capture.take() else {
                            unreachable!("capture is always Some when depth reaches zero");
                        };
                        if let Some(ref mut accum) = current_shape {
                            match finished.target {
                                CaptureTarget::SpPr => {
                                    accum.sp_pr_xml = Some(finished.xml);
                                }
                                CaptureTarget::TxBody => {
                                    accum.tx_body_xml_bytes = Some(finished.xml);
                                }
                                CaptureTarget::Duotone => {
                                    accum.duotone_xml = Some(finished.xml);
                                }
                            }
                        }
                    } else {
                        cap.push_end_raw(e.name().as_ref());
                    }
                } else {
                    let popped = element_stack.pop();

                    if state == ParseState::InShape {
                        // Check if we're closing the shape element
                        if current_shape_depth == Some(element_stack.len()) {
                            // We've closed the shape element, whether it was a direct
                            // spTree child or the selected AlternateContent branch.
                            if let Some(accum) = current_shape.take() {
                                shapes.push(accum.into_shape());
                            }
                            current_shape_depth = None;
                            state = ParseState::InSpTree;
                        }
                    } else if state == ParseState::InSpTree {
                        if let Some(ref popped) = popped {
                            if popped.local == "spTree" {
                                state = ParseState::Seeking;
                                sp_tree_depth = None;
                            }
                        }
                    }
                    // Note: element_stack pop already done above when not capturing
                    continue;
                }
                // Pop the element stack when capturing too
                element_stack.pop();
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(PptxError::Xml(e)),
            _ => {}
        }
    }

    Ok(shapes)
}
