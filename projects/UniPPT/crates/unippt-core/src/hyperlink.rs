//! Loss-aware parsing for shape and picture hyperlinks.
//!
//! The vendored presentation model can emit action settings but currently
//! does not populate them while reading.  Keeping this parser at the scene
//! boundary avoids widening that dependency and, importantly, lets UniPPT
//! retain broken relationship ids instead of silently discarding them.

use std::collections::HashMap;

use pptx::opc::Relationships;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::model::{HyperlinkAction, ObjectHyperlinks};

pub(crate) type RunHyperlinks = HashMap<(u32, usize, usize), ObjectHyperlinks>;

#[derive(Default)]
struct RunShapeState {
    shape_id: Option<u32>,
    in_text_body: bool,
    next_paragraph: usize,
    paragraph: Option<usize>,
    next_run: usize,
    run: Option<usize>,
    in_run_properties: bool,
}

/// Parse `a:hlinkClick` and `a:hlinkMouseOver` from the exact DrawingML run
/// that owns them. The tuple key is `(shape id, paragraph source index, run
/// source index)`, matching the identities stored in the scene model.
pub(crate) fn parse_run_hyperlinks(
    slide_xml: &[u8],
    relationships: &Relationships,
) -> RunHyperlinks {
    let mut reader = Reader::from_reader(slide_xml);
    reader.config_mut().trim_text(false);
    let mut buffer = Vec::new();
    let mut shapes = Vec::<RunShapeState>::new();
    let mut parsed = RunHyperlinks::new();

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(element)) => {
                let local = local_name(element.name().as_ref()).to_vec();
                match local.as_slice() {
                    b"sp" => shapes.push(RunShapeState::default()),
                    b"cNvPr" => {
                        if let Some(shape) = shapes.last_mut() {
                            shape.shape_id = decoded_attr(&element, b"id", &reader)
                                .and_then(|value| value.parse().ok());
                        }
                    }
                    b"txBody" => {
                        if let Some(shape) = shapes.last_mut() {
                            shape.in_text_body = true;
                            shape.next_paragraph = 0;
                        }
                    }
                    b"p" if shapes.last().is_some_and(|shape| shape.in_text_body) => {
                        if let Some(shape) = shapes.last_mut() {
                            shape.paragraph = Some(shape.next_paragraph);
                            shape.next_paragraph += 1;
                            shape.next_run = 0;
                        }
                    }
                    b"r" | b"fld" | b"br"
                        if shapes.last().is_some_and(|shape| {
                            shape.in_text_body && shape.paragraph.is_some()
                        }) =>
                    {
                        if let Some(shape) = shapes.last_mut() {
                            shape.run = Some(shape.next_run);
                            shape.next_run += 1;
                        }
                    }
                    b"rPr" if shapes.last().is_some_and(|shape| shape.run.is_some()) => {
                        if let Some(shape) = shapes.last_mut() {
                            shape.in_run_properties = true;
                        }
                    }
                    b"hlinkClick" | b"hlinkMouseOver"
                        if shapes.last().is_some_and(|shape| shape.in_run_properties) =>
                    {
                        record_run_action(
                            &element,
                            &reader,
                            relationships,
                            shapes.last().unwrap(),
                            &mut parsed,
                        );
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(element)) => {
                let local = local_name(element.name().as_ref()).to_vec();
                match local.as_slice() {
                    b"cNvPr" => {
                        if let Some(shape) = shapes.last_mut() {
                            shape.shape_id = decoded_attr(&element, b"id", &reader)
                                .and_then(|value| value.parse().ok());
                        }
                    }
                    b"r" | b"fld" | b"br"
                        if shapes.last().is_some_and(|shape| {
                            shape.in_text_body && shape.paragraph.is_some()
                        }) =>
                    {
                        if let Some(shape) = shapes.last_mut() {
                            shape.next_run += 1;
                        }
                    }
                    b"hlinkClick" | b"hlinkMouseOver"
                        if shapes.last().is_some_and(|shape| shape.in_run_properties) =>
                    {
                        record_run_action(
                            &element,
                            &reader,
                            relationships,
                            shapes.last().unwrap(),
                            &mut parsed,
                        );
                    }
                    _ => {}
                }
            }
            Ok(Event::End(element)) => {
                let local = local_name(element.name().as_ref()).to_vec();
                match local.as_slice() {
                    b"rPr" => {
                        if let Some(shape) = shapes.last_mut() {
                            shape.in_run_properties = false;
                        }
                    }
                    b"r" | b"fld" | b"br" => {
                        if let Some(shape) = shapes.last_mut() {
                            shape.run = None;
                            shape.in_run_properties = false;
                        }
                    }
                    b"p" => {
                        if let Some(shape) = shapes.last_mut() {
                            shape.paragraph = None;
                            shape.run = None;
                        }
                    }
                    b"txBody" => {
                        if let Some(shape) = shapes.last_mut() {
                            shape.in_text_body = false;
                        }
                    }
                    b"sp" => {
                        shapes.pop();
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buffer.clear();
    }
    parsed
}

fn record_run_action(
    element: &BytesStart<'_>,
    reader: &Reader<&[u8]>,
    relationships: &Relationships,
    shape: &RunShapeState,
    parsed: &mut RunHyperlinks,
) {
    let (Some(shape_id), Some(paragraph), Some(run)) = (shape.shape_id, shape.paragraph, shape.run)
    else {
        return;
    };
    let action = parse_action(element, reader, relationships);
    let hyperlinks = parsed.entry((shape_id, paragraph, run)).or_default();
    match local_name(element.name().as_ref()) {
        b"hlinkClick" => hyperlinks.click = Some(action),
        b"hlinkMouseOver" => hyperlinks.hover = Some(action),
        _ => {}
    }
}

pub(crate) fn parse_shape_hyperlinks(
    slide_xml: &[u8],
    relationships: &Relationships,
) -> HashMap<u32, ObjectHyperlinks> {
    let mut reader = Reader::from_reader(slide_xml);
    reader.config_mut().trim_text(false);
    let mut buffer = Vec::new();
    let mut shape_stack: Vec<Option<u32>> = Vec::new();
    let mut in_nonvisual_properties = false;
    let mut parsed = HashMap::<u32, ObjectHyperlinks>::new();

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(element)) => match local_name(element.name().as_ref()) {
                b"sp" | b"pic" => shape_stack.push(None),
                b"cNvPr" => {
                    if let Some(shape_id) = shape_stack.last_mut() {
                        *shape_id = decoded_attr(&element, b"id", &reader)
                            .and_then(|value| value.parse().ok());
                    }
                    in_nonvisual_properties = true;
                }
                b"hlinkClick" | b"hlinkHover" if in_nonvisual_properties => {
                    record_action(&element, &reader, relationships, &shape_stack, &mut parsed);
                }
                _ => {}
            },
            Ok(Event::Empty(element)) => match local_name(element.name().as_ref()) {
                b"cNvPr" => {
                    if let Some(shape_id) = shape_stack.last_mut() {
                        *shape_id = decoded_attr(&element, b"id", &reader)
                            .and_then(|value| value.parse().ok());
                    }
                }
                b"hlinkClick" | b"hlinkHover" if in_nonvisual_properties => {
                    record_action(&element, &reader, relationships, &shape_stack, &mut parsed);
                }
                _ => {}
            },
            Ok(Event::End(element)) => {
                let local = local_name(element.name().as_ref()).to_vec();
                if local.as_slice() == b"cNvPr" {
                    in_nonvisual_properties = false;
                } else if matches!(local.as_slice(), b"sp" | b"pic") {
                    shape_stack.pop();
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buffer.clear();
    }
    parsed
}

fn record_action(
    element: &BytesStart<'_>,
    reader: &Reader<&[u8]>,
    relationships: &Relationships,
    shape_stack: &[Option<u32>],
    parsed: &mut HashMap<u32, ObjectHyperlinks>,
) {
    let Some(shape_id) = shape_stack.last().copied().flatten() else {
        return;
    };
    let action = parse_action(element, reader, relationships);
    let hyperlinks = parsed.entry(shape_id).or_default();
    match local_name(element.name().as_ref()) {
        b"hlinkClick" => hyperlinks.click = Some(action),
        b"hlinkHover" => hyperlinks.hover = Some(action),
        _ => {}
    }
}

fn parse_action(
    element: &BytesStart<'_>,
    reader: &Reader<&[u8]>,
    relationships: &Relationships,
) -> HyperlinkAction {
    let relationship_id = decoded_attr(element, b"id", reader);
    let relationship = relationship_id
        .as_deref()
        .and_then(|id| relationships.get(id));
    let target = relationship.map(|relationship| {
        if relationship.is_external {
            relationship.target_ref.clone()
        } else {
            relationship
                .target_partname(relationships.base_uri())
                .map_or_else(|_| relationship.target_ref.clone(), |part| part.to_string())
        }
    });
    HyperlinkAction {
        target,
        relationship_id,
        external: relationship.is_some_and(|relationship| relationship.is_external),
        action: decoded_attr(element, b"action", reader),
        tooltip: decoded_attr(element, b"tooltip", reader),
    }
}

fn decoded_attr(element: &BytesStart<'_>, wanted: &[u8], reader: &Reader<&[u8]>) -> Option<String> {
    element
        .attributes()
        .with_checks(false)
        .flatten()
        .find_map(|attribute| {
            (local_name(attribute.key.as_ref()) == wanted).then(|| {
                attribute
                    .decode_and_unescape_value(reader.decoder())
                    .map(|value| value.into_owned())
                    .unwrap_or_else(|_| String::from_utf8_lossy(attribute.value.as_ref()).into())
            })
        })
}

fn local_name(name: &[u8]) -> &[u8] {
    name.rsplit(|byte| *byte == b':').next().unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use pptx::opc::Relationships;

    use super::*;

    #[test]
    fn parses_click_and_hover_relationships_without_losing_native_identity() {
        let mut relationships = Relationships::new("/ppt/slides");
        let external = relationships.add_relationship(
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink",
            "https://example.com/a?b=1&c=2",
            true,
        );
        let internal = relationships.add_relationship(
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide",
            "slide2.xml",
            false,
        );
        let xml = format!(
            r#"<p:sld xmlns:p="p" xmlns:a="a" xmlns:r="r"><p:cSld><p:spTree>
            <p:sp><p:nvSpPr><p:cNvPr id="7" name="Link"><a:hlinkClick r:id="{external}" tooltip="A &amp; B" vendor="keep"/></p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr></p:sp>
            <p:pic><p:nvPicPr><p:cNvPr id="8" name="Picture"><a:hlinkHover r:id="{internal}" action="ppaction://hlinksldjump"/></p:cNvPr><p:cNvPicPr/><p:nvPr/></p:nvPicPr></p:pic>
            </p:spTree></p:cSld></p:sld>"#
        );

        let parsed = parse_shape_hyperlinks(xml.as_bytes(), &relationships);
        let click = parsed[&7].click.as_ref().unwrap();
        assert_eq!(click.relationship_id.as_deref(), Some(external.as_str()));
        assert_eq!(
            click.target.as_deref(),
            Some("https://example.com/a?b=1&c=2")
        );
        assert_eq!(click.tooltip.as_deref(), Some("A & B"));
        assert!(click.external);

        let hover = parsed[&8].hover.as_ref().unwrap();
        assert_eq!(hover.relationship_id.as_deref(), Some(internal.as_str()));
        assert_eq!(hover.target.as_deref(), Some("/ppt/slides/slide2.xml"));
        assert_eq!(hover.action.as_deref(), Some("ppaction://hlinksldjump"));
        assert!(!hover.external);
    }

    #[test]
    fn retains_a_broken_relationship_id_as_loss_aware_evidence() {
        let relationships = Relationships::new("/ppt/slides");
        let xml = br#"<p:sld xmlns:p="p" xmlns:a="a" xmlns:r="r"><p:cSld><p:spTree><p:sp><p:nvSpPr><p:cNvPr id="9"><a:hlinkClick r:id="rId404"/></p:cNvPr></p:nvSpPr></p:sp></p:spTree></p:cSld></p:sld>"#;

        let parsed = parse_shape_hyperlinks(xml, &relationships);
        let click = parsed[&9].click.as_ref().unwrap();
        assert_eq!(click.relationship_id.as_deref(), Some("rId404"));
        assert_eq!(click.target, None);
    }

    #[test]
    fn does_not_promote_text_run_hyperlinks_to_shape_actions() {
        let mut relationships = Relationships::new("/ppt/slides");
        let external = relationships.add_relationship(
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink",
            "https://example.com/run",
            true,
        );
        let xml = format!(
            r#"<p:sld xmlns:p="p" xmlns:a="a" xmlns:r="r"><p:cSld><p:spTree><p:sp><p:nvSpPr><p:cNvPr id="12" name="Text"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:txBody><a:p><a:r><a:rPr><a:hlinkClick r:id="{external}"/></a:rPr><a:t>linked run</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"#
        );

        let parsed = parse_shape_hyperlinks(xml.as_bytes(), &relationships);

        assert!(!parsed.contains_key(&12));
    }

    #[test]
    fn parses_run_click_and_mouse_over_at_their_exact_source_indices() {
        let mut relationships = Relationships::new("/ppt/slides");
        let click_id = relationships.add_relationship(
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink",
            "https://example.com/click?a=1&b=2",
            true,
        );
        let hover_id = relationships.add_relationship(
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink",
            "mailto:qa@example.com",
            true,
        );
        let xml = format!(
            r#"<p:sld xmlns:p="p" xmlns:a="a" xmlns:r="r"><p:cSld><p:spTree><p:sp><p:nvSpPr><p:cNvPr id="12" name="Text"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:txBody><a:p><a:r><a:rPr/><a:t>plain</a:t></a:r><a:r><a:rPr><a:hlinkClick r:id="{click_id}" tooltip="Click &amp; keep"/><a:hlinkMouseOver r:id="{hover_id}"/></a:rPr><a:t>linked</a:t></a:r></a:p><a:p><a:r><a:rPr><a:hlinkClick action="ppaction://hlinkshowjump?jump=nextslide"/></a:rPr><a:t>next</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"#
        );

        let parsed = parse_run_hyperlinks(xml.as_bytes(), &relationships);

        assert!(!parsed.contains_key(&(12, 0, 0)));
        let linked = &parsed[&(12, 0, 1)];
        assert_eq!(
            linked.click.as_ref().unwrap().target.as_deref(),
            Some("https://example.com/click?a=1&b=2")
        );
        assert_eq!(
            linked.click.as_ref().unwrap().tooltip.as_deref(),
            Some("Click & keep")
        );
        assert_eq!(
            linked.hover.as_ref().unwrap().target.as_deref(),
            Some("mailto:qa@example.com")
        );
        assert_eq!(
            parsed[&(12, 1, 0)]
                .click
                .as_ref()
                .unwrap()
                .action
                .as_deref(),
            Some("ppaction://hlinkshowjump?jump=nextslide")
        );
    }
}
