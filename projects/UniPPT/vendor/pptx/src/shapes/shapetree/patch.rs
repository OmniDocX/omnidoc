//! Loss-aware helpers for replacing individual shape XML fragments.

use std::ops::Range;

use crate::error::{PptxError, PptxResult};
use crate::units::ShapeId;

use super::ShapeTree;

const SHAPE_TAGS: [&str; 5] = ["p:sp", "p:pic", "p:graphicFrame", "p:cxnSp", "p:grpSp"];

impl ShapeTree {
    /// Return the exact XML range occupied by a shape.
    ///
    /// When a shape is hosted by `mc:AlternateContent` (the native PowerPoint
    /// equation representation), the returned range covers the complete
    /// AlternateContent element so Choice and Fallback remain atomic.
    pub fn shape_xml_range(
        slide_xml: &[u8],
        shape_id: ShapeId,
    ) -> PptxResult<Option<Range<usize>>> {
        let xml = std::str::from_utf8(slide_xml)?;
        let mut cursor = 0;
        while let Some(relative) = xml[cursor..].find("<p:cNvPr") {
            let start = cursor + relative;
            let Some(tag_end_relative) = xml[start..].find('>') else {
                return Err(PptxError::InvalidXml(
                    "unterminated p:cNvPr element".to_string(),
                ));
            };
            let tag_end = start + tag_end_relative + 1;
            let tag = &xml[start..tag_end];
            if attribute_value(tag, "id").as_deref() == Some(shape_id.0.to_string().as_str()) {
                let Some((shape_start, shape_tag)) = nearest_open_tag(xml, start, &SHAPE_TAGS)
                else {
                    return Err(PptxError::InvalidXml(format!(
                        "shape id {} has no enclosing shape element",
                        shape_id.0
                    )));
                };
                let shape_end = element_end(xml, shape_start, shape_tag)?;

                if let Some(alternate_start) =
                    last_open_tag(xml, shape_start, "mc:AlternateContent")
                {
                    let alternate_end = element_end(xml, alternate_start, "mc:AlternateContent")?;
                    if alternate_end >= shape_end {
                        return Ok(Some(alternate_start..alternate_end));
                    }
                }
                return Ok(Some(shape_start..shape_end));
            }
            cursor = tag_end;
        }
        Ok(None)
    }

    /// Replace a shape while leaving every other byte of the slide XML intact.
    pub fn replace_shape_xml(
        slide_xml: &[u8],
        shape_id: ShapeId,
        replacement: &str,
    ) -> PptxResult<Vec<u8>> {
        let range = Self::shape_xml_range(slide_xml, shape_id)?.ok_or_else(|| {
            PptxError::InvalidXml(format!("shape id {} was not found", shape_id.0))
        })?;
        let mut output = Vec::with_capacity(slide_xml.len() - range.len() + replacement.len());
        output.extend_from_slice(&slide_xml[..range.start]);
        output.extend_from_slice(replacement.as_bytes());
        output.extend_from_slice(&slide_xml[range.end..]);
        Ok(output)
    }

    /// Remove one shape while preserving all unrelated slide markup.
    pub fn remove_shape_xml(slide_xml: &[u8], shape_id: ShapeId) -> PptxResult<Vec<u8>> {
        let Some(range) = Self::shape_xml_range(slide_xml, shape_id)? else {
            return Ok(slide_xml.to_vec());
        };
        let mut output = Vec::with_capacity(slide_xml.len() - range.len());
        output.extend_from_slice(&slide_xml[..range.start]);
        output.extend_from_slice(&slide_xml[range.end..]);
        Ok(output)
    }
}

fn attribute_value(tag: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=\"");
    let start = tag.find(&needle)? + needle.len();
    let end = tag[start..].find('"')? + start;
    Some(tag[start..end].to_string())
}

fn nearest_open_tag<'a>(xml: &str, before: usize, tags: &'a [&'a str]) -> Option<(usize, &'a str)> {
    tags.iter()
        .filter_map(|tag| last_open_tag(xml, before, tag).map(|position| (position, *tag)))
        .max_by_key(|(position, _)| *position)
}

fn last_open_tag(xml: &str, before: usize, tag: &str) -> Option<usize> {
    let needle = format!("<{tag}");
    xml[..before]
        .match_indices(&needle)
        .filter(|(position, _)| is_tag_boundary(xml, position + needle.len()))
        .map(|(position, _)| position)
        .last()
}

fn element_end(xml: &str, start: usize, tag: &str) -> PptxResult<usize> {
    let opening_end = xml[start..]
        .find('>')
        .map(|relative| start + relative + 1)
        .ok_or_else(|| PptxError::InvalidXml(format!("unterminated <{tag}> element")))?;
    if xml[..opening_end].trim_end().ends_with("/>") {
        return Ok(opening_end);
    }

    let open_needle = format!("<{tag}");
    let close_needle = format!("</{tag}>");
    let mut depth = 1_u32;
    let mut cursor = opening_end;
    while cursor < xml.len() {
        let next_open = xml[cursor..]
            .find(&open_needle)
            .map(|relative| cursor + relative)
            .filter(|position| is_tag_boundary(xml, position + open_needle.len()));
        let next_close = xml[cursor..]
            .find(&close_needle)
            .map(|relative| cursor + relative);
        match (next_open, next_close) {
            (None, Some(close)) => {
                depth -= 1;
                cursor = close + close_needle.len();
                if depth == 0 {
                    return Ok(cursor);
                }
            }
            (Some(open), Some(close)) if close < open => {
                depth -= 1;
                cursor = close + close_needle.len();
                if depth == 0 {
                    return Ok(cursor);
                }
            }
            (Some(open), _) => {
                let end = xml[open..]
                    .find('>')
                    .map(|relative| open + relative + 1)
                    .ok_or_else(|| {
                        PptxError::InvalidXml(format!("unterminated nested <{tag}> element"))
                    })?;
                if !xml[..end].trim_end().ends_with("/>") {
                    depth += 1;
                }
                cursor = end;
            }
            (None, None) => break,
        }
    }
    Err(PptxError::InvalidXml(format!(
        "missing closing </{tag}> element"
    )))
}

fn is_tag_boundary(xml: &str, position: usize) -> bool {
    xml.as_bytes()
        .get(position)
        .is_some_and(|byte| byte.is_ascii_whitespace() || matches!(byte, b'>' | b'/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_only_requested_shape() {
        let xml = br#"<p:sld><p:cSld><p:spTree><p:sp><p:nvSpPr><p:cNvPr id="2" name="A"/></p:nvSpPr></p:sp><p:sp><p:nvSpPr><p:cNvPr id="3" name="B"/></p:nvSpPr></p:sp></p:spTree></p:cSld></p:sld>"#;
        let output = ShapeTree::replace_shape_xml(
            xml,
            ShapeId(2),
            r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="Changed"/></p:nvSpPr></p:sp>"#,
        )
        .unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("Changed"));
        assert!(output.contains("name=\"B\""));
    }

    #[test]
    fn equation_range_covers_alternate_content() {
        let xml = br#"<p:sld><p:cSld><p:spTree><mc:AlternateContent><mc:Choice><p:sp><p:nvSpPr><p:cNvPr id="7" name="Equation"/></p:nvSpPr></p:sp></mc:Choice><mc:Fallback><p:sp><p:nvSpPr><p:cNvPr id="7" name="Fallback"/></p:nvSpPr></p:sp></mc:Fallback></mc:AlternateContent></p:spTree></p:cSld></p:sld>"#;
        let range = ShapeTree::shape_xml_range(xml, ShapeId(7))
            .unwrap()
            .unwrap();
        let fragment = std::str::from_utf8(&xml[range]).unwrap();
        assert!(fragment.starts_with("<mc:AlternateContent>"));
        assert!(fragment.ends_with("</mc:AlternateContent>"));
    }
}
