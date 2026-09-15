//! XML parsing for notes slides and slide-level content.

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::error::{PptxError, PptxResult};
use crate::shapes::shapetree::ShapeTree;
use crate::xml_util::local_name;

use super::types::NotesSlide;

/// Parse a notes slide XML blob into a `NotesSlide`.
///
/// Extracts the name, shape tree, and notes text from the XML.
/// # Errors
///
/// Returns an error if the XML is malformed.
pub fn parse_notes_slide(notes_xml: &[u8]) -> PptxResult<NotesSlide> {
    parse_notes_slide_with_part_name(notes_xml, None)
}

/// Parse a notes slide XML blob into a `NotesSlide`, with an optional part name.
///
/// # Errors
///
/// Returns an error if the XML is malformed.
pub fn parse_notes_slide_with_part_name(
    notes_xml: &[u8],
    part_name: Option<String>,
) -> PptxResult<NotesSlide> {
    let name = parse_slide_name(notes_xml)?;
    let shapes = ShapeTree::from_slide_xml(notes_xml)?;
    let notes_text = parse_notes_slide_text(notes_xml)?;

    Ok(NotesSlide {
        name,
        shapes,
        notes_text,
        part_name,
    })
}

/// Extract the `name` attribute from a slide's `<p:cSld>` element.
///
/// Returns `Some(name)` if the attribute is present and non-empty, `None` otherwise.
///
/// # Errors
///
/// Returns an error if any attribute in the XML is malformed.
pub fn parse_slide_name(slide_xml: &[u8]) -> PptxResult<Option<String>> {
    let mut reader = Reader::from_reader(slide_xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => {
                let qname = e.name();
                let ln = local_name(qname.as_ref());
                if ln == b"cSld" {
                    for attr_result in e.attributes() {
                        let attr = attr_result.map_err(PptxError::XmlAttr)?;
                        if attr.key.as_ref() == b"name" {
                            return Ok(std::str::from_utf8(&attr.value)
                                .ok()
                                .filter(|s| !s.is_empty())
                                .map(std::string::ToString::to_string));
                        }
                    }
                    return Ok(None);
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(None)
}

/// Extract the text content from a notes slide's body placeholder.
///
/// Uses a streaming `quick-xml::Reader` to find the `<p:sp>` that contains
/// a `<p:ph type="body"...>` element, then extracts text from its `<p:txBody>`.
/// Paragraphs are separated by newlines.
/// # Errors
///
/// Returns an error if the XML is malformed.
pub fn parse_notes_slide_text(notes_xml: &[u8]) -> PptxResult<String> {
    let mut reader = Reader::from_reader(notes_xml);
    reader.config_mut().trim_text(false);

    let mut buf = Vec::new();

    // State: track nesting inside <p:sp> to find body placeholder and extract text.
    let mut sp_depth: u32 = 0;
    let mut has_body_ph = false;
    let mut in_tx_body = false;
    let mut in_paragraph = false;
    let mut in_t = false;
    let mut paragraphs: Vec<String> = Vec::new();
    let mut current_para = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let qn = e.name();
                let ln = local_name(qn.as_ref());
                match ln {
                    b"sp" => {
                        sp_depth += 1;
                        has_body_ph = false;
                        in_tx_body = false;
                    }
                    b"txBody" if sp_depth > 0 && has_body_ph => {
                        in_tx_body = true;
                        paragraphs.clear();
                    }
                    b"p" if in_tx_body => {
                        in_paragraph = true;
                        current_para.clear();
                    }
                    b"t" if in_paragraph => {
                        in_t = true;
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                let qn = e.name();
                let ln = local_name(qn.as_ref());
                if ln == b"ph" && sp_depth > 0 {
                    has_body_ph = is_body_placeholder(e)?;
                }
            }
            Ok(Event::Text(ref t)) if in_t => {
                if let Ok(text) = t.decode() {
                    current_para.push_str(&text);
                }
            }
            Ok(Event::End(ref e)) => {
                let qn = e.name();
                let ln = local_name(qn.as_ref());
                match ln {
                    b"sp" if sp_depth > 0 => {
                        sp_depth -= 1;
                        has_body_ph = false;
                        in_tx_body = false;
                    }
                    b"txBody" if in_tx_body => {
                        in_tx_body = false;
                        if has_body_ph {
                            return Ok(paragraphs.join("\n"));
                        }
                    }
                    b"t" => {
                        in_t = false;
                    }
                    b"p" if in_paragraph => {
                        in_paragraph = false;
                        paragraphs.push(std::mem::take(&mut current_para));
                    }
                    _ => {}
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

/// Check if a `<p:ph>` element has `type="body"`.
///
/// # Errors
///
/// Returns an error if any attribute in the element is malformed.
fn is_body_placeholder(e: &BytesStart<'_>) -> PptxResult<bool> {
    for attr_result in e.attributes() {
        let attr = attr_result.map_err(PptxError::XmlAttr)?;
        if attr.key.as_ref() == b"type" {
            return Ok(attr.value.as_ref() == b"body");
        }
    }
    Ok(false)
}
