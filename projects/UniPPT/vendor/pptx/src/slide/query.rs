//! Query and manipulation functions for slide layouts, masters, and related XML.

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::error::{PptxError, PptxResult};
use crate::opc::constants::relationship_type as RT;
use crate::opc::pack_uri::PackURI;
use crate::opc::package::OpcPackage;
use crate::xml_util::local_name;

use super::types::{SlideLayoutRef, SlideRef};

/// Remove a slide layout reference from a slide master's XML.
///
/// Finds the `<p:sldLayoutId>` element whose `r:id` matches `layout_r_id`
/// and removes it from the slide master XML. Returns the updated XML.
pub fn remove_layout_from_master_xml(master_xml: &[u8], layout_r_id: &str) -> PptxResult<Vec<u8>> {
    let xml_str = std::str::from_utf8(master_xml)?;

    let pattern = format!(r#"r:id="{layout_r_id}""#);

    let removed = xml_str.find(&pattern).and_then(|rid_pos| {
        let tag_start = xml_str[..rid_pos].rfind("<p:sldLayoutId")?;
        let rel_end = xml_str[rid_pos..].find("/>")?;
        let tag_end = rid_pos + rel_end + 2;
        let mut result = Vec::with_capacity(master_xml.len());
        result.extend_from_slice(&master_xml[..tag_start]);
        result.extend_from_slice(&master_xml[tag_end..]);
        Some(result)
    });

    Ok(removed.unwrap_or_else(|| master_xml.to_vec()))
}

/// Find which slides use a given layout.
///
/// Checks each slide's relationships for a slideLayout relationship pointing
/// to the given `layout_partname`. Returns the slide refs that use this layout.
pub fn layout_used_by_slides(
    layout_partname: &PackURI,
    slides: &[SlideRef],
    package: &OpcPackage,
) -> Vec<SlideRef> {
    slides
        .iter()
        .filter(|slide_ref| {
            package.part(&slide_ref.partname).is_some_and(|slide_part| {
                slide_part
                    .rels
                    .all_by_reltype(RT::SLIDE_LAYOUT)
                    .iter()
                    .any(|rel| {
                        rel.target_partname(slide_part.partname.base_uri())
                            .is_ok_and(|target| target.as_str() == layout_partname.as_str())
                    })
            })
        })
        .cloned()
        .collect()
}

/// Find a slide layout by name from a list of layouts.
#[must_use]
pub fn get_layout_by_name<'a>(
    layouts: &'a [SlideLayoutRef],
    name: &str,
) -> Option<&'a SlideLayoutRef> {
    layouts.iter().find(|l| l.name == name)
}

/// Extract layout rIds from a slide master's relationships.
/// Returns all relationships of type slideLayout, sorted by rId number.
pub fn extract_layout_r_ids(rels: &crate::opc::relationship::Relationships) -> Vec<String> {
    let mut r_ids: Vec<String> = rels
        .all_by_reltype(RT::SLIDE_LAYOUT)
        .into_iter()
        .map(|r| r.r_id.to_string())
        .collect();
    // Sort by numeric suffix of rId; non-numeric rIds sort first (key 0).
    // .ok() is intentional: parse failure in a sort key is not an error.
    r_ids.sort_by_key(|r_id| {
        r_id.strip_prefix("rId")
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0)
    });
    r_ids
}

/// Extract placeholder `<p:sp>` elements from a slide layout XML.
///
/// Uses a streaming `quick-xml::Reader` to find `<p:sp>` elements that
/// contain a `<p:ph>` (placeholder) element. Returns a vector of XML strings,
/// each being a complete `<p:sp>...</p:sp>` element.
/// # Errors
///
/// Returns an error if the XML is malformed.
pub fn placeholder_shapes_from_layout(layout_xml: &[u8]) -> PptxResult<Vec<String>> {
    let xml_str = std::str::from_utf8(layout_xml)?;
    let mut reader = Reader::from_str(xml_str);
    reader.config_mut().trim_text(false);

    let mut buf = Vec::new();
    let mut placeholders = Vec::new();

    // Accumulate reconstructed XML for the current <p:sp>.
    let mut sp_xml = Vec::<u8>::new();
    let mut sp_depth: u32 = 0;
    let mut has_ph = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let qn = e.name();
                let tag = local_name(qn.as_ref());
                if tag == b"sp" {
                    if sp_depth == 0 {
                        has_ph = false;
                        sp_xml.clear();
                    }
                    sp_depth += 1;
                }
                if sp_depth > 0 {
                    write_start_tag(&mut sp_xml, e);
                }
            }
            Ok(Event::Empty(ref e)) => {
                if sp_depth > 0 {
                    let qn = e.name();
                    let tag = local_name(qn.as_ref());
                    if tag == b"ph" {
                        has_ph = true;
                    }
                    write_empty_tag(&mut sp_xml, e);
                }
            }
            Ok(Event::End(ref e)) => {
                if sp_depth > 0 {
                    write_end_tag(&mut sp_xml, e);
                    let qn = e.name();
                    let tag = local_name(qn.as_ref());
                    if tag == b"sp" {
                        sp_depth -= 1;
                        if sp_depth == 0 && has_ph {
                            if let Ok(s) = String::from_utf8(std::mem::take(&mut sp_xml)) {
                                placeholders.push(s);
                            }
                        }
                    }
                }
            }
            Ok(Event::Text(ref t)) => {
                if sp_depth > 0 {
                    sp_xml.extend_from_slice(t.as_ref());
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(PptxError::Xml(e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(placeholders)
}

/// Write a start tag `<name attr="val"...>` into the buffer.
fn write_start_tag(out: &mut Vec<u8>, e: &quick_xml::events::BytesStart<'_>) {
    out.push(b'<');
    out.extend_from_slice(e.name().as_ref());
    for attr in e.attributes().flatten() {
        out.push(b' ');
        out.extend_from_slice(attr.key.as_ref());
        out.extend_from_slice(b"=\"");
        out.extend_from_slice(&attr.value);
        out.push(b'"');
    }
    out.push(b'>');
}

/// Write an empty tag `<name attr="val".../>` into the buffer.
fn write_empty_tag(out: &mut Vec<u8>, e: &quick_xml::events::BytesStart<'_>) {
    out.push(b'<');
    out.extend_from_slice(e.name().as_ref());
    for attr in e.attributes().flatten() {
        out.push(b' ');
        out.extend_from_slice(attr.key.as_ref());
        out.extend_from_slice(b"=\"");
        out.extend_from_slice(&attr.value);
        out.push(b'"');
    }
    out.extend_from_slice(b"/>");
}

/// Write an end tag `</name>` into the buffer.
fn write_end_tag(out: &mut Vec<u8>, e: &quick_xml::events::BytesEnd<'_>) {
    out.extend_from_slice(b"</");
    out.extend_from_slice(e.name().as_ref());
    out.push(b'>');
}
