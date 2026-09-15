//! Presentation-level XML parsing: slide IDs, master IDs, layout names, slide size.

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::error::{PptxError, PptxResult};
use crate::units::{RelationshipId, SlideId};
use crate::xml_util::local_name;

/// Extract slide IDs (rIds) from presentation XML by parsing
/// `<p:sldIdLst>/<p:sldId>` elements.
///
/// Returns a vector of (rId, id) tuples in document order.
pub fn parse_slide_ids(presentation_xml: &[u8]) -> PptxResult<Vec<(String, SlideId)>> {
    parse_id_list(presentation_xml, b"sldIdLst", b"sldId")
}

/// Extract slide master IDs (rIds) from presentation XML by parsing
/// `<p:sldMasterIdLst>/<p:sldMasterId>` elements.
pub fn parse_slide_master_ids(presentation_xml: &[u8]) -> PptxResult<Vec<(String, SlideId)>> {
    parse_id_list(presentation_xml, b"sldMasterIdLst", b"sldMasterId")
}

/// Generic helper to parse `<list_tag>/<item_tag>` ID lists from XML.
fn parse_id_list(
    xml: &[u8],
    list_tag: &[u8],
    item_tag: &[u8],
) -> PptxResult<Vec<(String, SlideId)>> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);

    let mut ids = Vec::new();
    let mut in_list = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => {
                let qn = e.name();
                let ln = local_name(qn.as_ref());
                if ln == list_tag {
                    in_list = true;
                } else if ln == item_tag && in_list {
                    let (r_id, id) = parse_sld_id_attrs(e)?;
                    ids.push((r_id.to_string(), id));
                }
            }
            Ok(Event::End(ref e)) => {
                let qn = e.name();
                if local_name(qn.as_ref()) == list_tag {
                    in_list = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(PptxError::Xml(e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(ids)
}

/// Parse the `name` attribute from a slide layout's `<p:cSld>` element.
///
/// The `name` attribute is OOXML-optional; returns an empty string when absent.
///
/// # Errors
///
/// Returns `Err` if the XML is malformed or an attribute value is not valid UTF-8.
pub fn parse_layout_name(layout_xml: &[u8]) -> PptxResult<String> {
    let mut reader = Reader::from_reader(layout_xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => {
                let qn = e.name();
                let ln = local_name(qn.as_ref());
                if ln == b"cSld" {
                    for attr in e.attributes() {
                        let attr = attr.map_err(PptxError::XmlAttr)?;
                        if attr.key.as_ref() == b"name" {
                            let name = std::str::from_utf8(&attr.value)
                                .map_err(|e| {
                                    PptxError::InvalidXml(format!(
                                        "cSld name attribute invalid UTF-8: {e}"
                                    ))
                                })?
                                .to_string();
                            return Ok(name);
                        }
                    }
                    return Ok(String::new());
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(PptxError::Xml(e)),
            _ => {}
        }
        buf.clear();
    }
    Ok(String::new())
}

/// Parse the slide size from presentation XML.
///
/// Returns `Ok(Some((width_emu, height_emu)))` when `<p:sldSz>` is present,
/// `Ok(None)` when absent, or an error on malformed attributes.
pub fn parse_slide_size(presentation_xml: &[u8]) -> PptxResult<Option<(i64, i64)>> {
    let mut reader = Reader::from_reader(presentation_xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => {
                let qn = e.name();
                if local_name(qn.as_ref()) == b"sldSz" {
                    return parse_sld_sz_attrs(e);
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(None)
}

/// Parse cx/cy attributes from a `<p:sldSz>` element.
#[allow(clippy::similar_names)]
fn parse_sld_sz_attrs(e: &BytesStart<'_>) -> PptxResult<Option<(i64, i64)>> {
    let mut cx_val: Option<i64> = None;
    let mut cy_val: Option<i64> = None;
    for attr in e.attributes() {
        let attr = attr.map_err(PptxError::XmlAttr)?;
        match attr.key.as_ref() {
            b"cx" => {
                let s = std::str::from_utf8(&attr.value)
                    .map_err(|e| PptxError::InvalidXml(format!("sldSz cx: {e}")))?;
                cx_val = Some(
                    s.parse()
                        .map_err(|e| PptxError::InvalidXml(format!("sldSz cx parse: {e}")))?,
                );
            }
            b"cy" => {
                let s = std::str::from_utf8(&attr.value)
                    .map_err(|e| PptxError::InvalidXml(format!("sldSz cy: {e}")))?;
                cy_val = Some(
                    s.parse()
                        .map_err(|e| PptxError::InvalidXml(format!("sldSz cy parse: {e}")))?,
                );
            }
            _ => {}
        }
    }
    let cx = cx_val
        .ok_or_else(|| PptxError::InvalidXml("sldSz missing required cx attribute".into()))?;
    let cy = cy_val
        .ok_or_else(|| PptxError::InvalidXml("sldSz missing required cy attribute".into()))?;
    Ok(Some((cx, cy)))
}

/// Parse the `r:id` and `id` attributes from a `<p:sldId>` or `<p:sldMasterId>`.
fn parse_sld_id_attrs(elem: &BytesStart<'_>) -> PptxResult<(RelationshipId, SlideId)> {
    let mut r_id: Option<RelationshipId> = None;
    let mut id: Option<u32> = None;

    for attr in elem.attributes() {
        let attr = attr.map_err(PptxError::XmlAttr)?;
        let key = attr.key.as_ref();
        let local = local_name(key);

        match local {
            b"id" if key != local => {
                let value = std::str::from_utf8(&attr.value)
                    .map_err(|e| PptxError::InvalidXml(format!("invalid UTF-8: {e}")))?;
                r_id =
                    Some(RelationshipId::try_from(value).map_err(|_| {
                        PptxError::InvalidXml(format!("invalid r:id value: {value}"))
                    })?);
            }
            b"id" => {
                let s = std::str::from_utf8(&attr.value)
                    .map_err(|e| PptxError::InvalidXml(format!("sldId id: {e}")))?;
                id = Some(
                    s.parse::<u32>()
                        .map_err(|e| PptxError::InvalidXml(format!("sldId id parse: {e}")))?,
                );
            }
            _ => {}
        }
    }

    let r_id = r_id.ok_or_else(|| PptxError::InvalidXml("sldId missing r:id".to_string()))?;
    let id = id.ok_or_else(|| PptxError::InvalidXml("sldId missing id".to_string()))?;
    Ok((r_id, SlideId(id)))
}
