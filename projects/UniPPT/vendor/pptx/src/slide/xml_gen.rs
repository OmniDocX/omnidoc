//! Slide XML generation and manipulation functions.

use crate::error::{PptxError, PptxResult};
use crate::units::SlideId;

/// Build a minimal new slide XML blob.
///
/// This produces a valid `<p:sld>` element with an empty shape tree.
#[must_use]
pub fn new_slide_xml() -> Vec<u8> {
    let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr></p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>"#;
    xml.as_bytes().to_vec()
}

/// Build a minimal new notes slide XML blob.
#[must_use]
pub fn new_notes_slide_xml() -> Vec<u8> {
    let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:notes xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr><p:sp><p:nvSpPr><p:cNvPr id="2" name="Slide Image Placeholder 1"/><p:cNvSpPr><a:spLocks noGrp="1" noRot="1" noChangeAspect="1"/></p:cNvSpPr><p:nvPr><p:ph type="sldImg"/></p:nvPr></p:nvSpPr><p:spPr/></p:sp><p:sp><p:nvSpPr><p:cNvPr id="3" name="Notes Placeholder 2"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr lang="en-US"/></a:p></p:txBody></p:sp></p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:notes>"#;
    xml.as_bytes().to_vec()
}

/// Build a minimal notes master XML blob.
#[must_use]
pub fn new_notes_master_xml() -> Vec<u8> {
    let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:notesMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr></p:spTree></p:cSld><p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/></p:notesMaster>"#;
    xml.as_bytes().to_vec()
}

/// Add a `<p:sldId>` entry to the presentation XML's `<p:sldIdLst>`.
///
/// If no `<p:sldIdLst>` exists, one is created. Returns the updated XML.
pub fn add_slide_id_to_presentation_xml(
    presentation_xml: &[u8],
    r_id: impl AsRef<str>,
    slide_id: SlideId,
) -> PptxResult<Vec<u8>> {
    let xml_str = std::str::from_utf8(presentation_xml)?;
    let r_id = r_id.as_ref();

    let new_entry = format!(r#"<p:sldId id="{slide_id}" r:id="{r_id}"/>"#);

    // Try to insert before the closing </p:sldIdLst> tag
    if let Some(pos) = xml_str.find("</p:sldIdLst>") {
        let mut result = Vec::with_capacity(presentation_xml.len() + new_entry.len());
        result.extend_from_slice(&presentation_xml[..pos]);
        result.extend_from_slice(new_entry.as_bytes());
        result.extend_from_slice(&presentation_xml[pos..]);
        return Ok(result);
    }

    // If there's an empty <p:sldIdLst/>, replace it
    if let Some(pos) = xml_str.find("<p:sldIdLst/>") {
        let mut result = Vec::with_capacity(presentation_xml.len() + new_entry.len() + 30);
        result.extend_from_slice(&presentation_xml[..pos]);
        result.extend_from_slice(b"<p:sldIdLst>");
        result.extend_from_slice(new_entry.as_bytes());
        result.extend_from_slice(b"</p:sldIdLst>");
        result.extend_from_slice(&presentation_xml[pos + "<p:sldIdLst/>".len()..]);
        return Ok(result);
    }

    // If there's no sldIdLst at all, insert one after the opening sldMasterIdLst block.
    // Look for </p:sldMasterIdLst> and insert after it.
    if let Some(pos) = xml_str.find("</p:sldMasterIdLst>") {
        let insert_pos = pos + "</p:sldMasterIdLst>".len();
        let mut result = Vec::with_capacity(presentation_xml.len() + new_entry.len() + 30);
        result.extend_from_slice(&presentation_xml[..insert_pos]);
        result.extend_from_slice(b"<p:sldIdLst>");
        result.extend_from_slice(new_entry.as_bytes());
        result.extend_from_slice(b"</p:sldIdLst>");
        result.extend_from_slice(&presentation_xml[insert_pos..]);
        return Ok(result);
    }

    Err(PptxError::InvalidXml(
        "could not find insertion point for sldIdLst in presentation XML".to_string(),
    ))
}

/// Compute the next available slide ID from existing slide IDs.
/// Slide IDs in OOXML start at 256 and increment.
pub fn next_slide_id(existing_ids: &[(String, SlideId)]) -> SlideId {
    // 255 is the correct fallback: OOXML slide IDs start at 256, so when
    // no slides exist yet, max+1 = 256.
    let max = existing_ids.iter().map(|(_, id)| id.0).max().unwrap_or(255);
    SlideId(max + 1)
}

/// Remove a `<p:sldId>` entry from the presentation XML by its r:id.
///
/// Returns the updated XML. If the entry is not found, the XML is returned unchanged.
pub fn remove_slide_id_from_presentation_xml(
    presentation_xml: &[u8],
    r_id: impl AsRef<str>,
) -> PptxResult<Vec<u8>> {
    let xml_str = std::str::from_utf8(presentation_xml)?;
    let r_id = r_id.as_ref();

    // Find and remove the <p:sldId ... r:id="rIdN"/> entry
    // Match patterns like: <p:sldId id="256" r:id="rId7"/>
    let pattern = format!(r#"r:id="{r_id}""#);

    // Find the sldId element containing our r:id and remove it.
    let removed = xml_str.find(&pattern).and_then(|rid_pos| {
        let tag_start = xml_str[..rid_pos].rfind("<p:sldId")?;
        let rel_end = xml_str[rid_pos..].find("/>")?;
        let tag_end = rid_pos + rel_end + 2;
        let mut v = Vec::with_capacity(presentation_xml.len());
        v.extend_from_slice(&presentation_xml[..tag_start]);
        v.extend_from_slice(&presentation_xml[tag_end..]);
        Some(v)
    });
    let modified = removed.is_some();
    let result_bytes = removed.unwrap_or_else(|| presentation_xml.to_vec());

    // If sldIdLst is now empty, replace <p:sldIdLst></p:sldIdLst> with <p:sldIdLst/>
    if modified {
        let result_str = std::str::from_utf8(&result_bytes)?;
        if let Some(pos) = result_str.find("<p:sldIdLst></p:sldIdLst>") {
            let empty_tag = b"<p:sldIdLst/>";
            let old_tag = b"<p:sldIdLst></p:sldIdLst>";
            let mut v = Vec::with_capacity(result_bytes.len());
            v.extend_from_slice(&result_bytes[..pos]);
            v.extend_from_slice(empty_tag);
            v.extend_from_slice(&result_bytes[pos + old_tag.len()..]);
            return Ok(v);
        }
    }

    Ok(result_bytes)
}

/// Reorder slides in the presentation XML by moving a slide from one position to another.
///
/// `from_index` and `to_index` are 0-based indices within the `<p:sldIdLst>`.
/// Returns the updated XML bytes.
pub fn reorder_slide_in_presentation_xml(
    presentation_xml: &[u8],
    from_index: usize,
    to_index: usize,
) -> PptxResult<Vec<u8>> {
    let xml_str = std::str::from_utf8(presentation_xml)?;

    // Extract the sldIdLst content
    let lst_start_tag = "<p:sldIdLst>";
    let lst_end_tag = "</p:sldIdLst>";

    let lst_start = xml_str.find(lst_start_tag).ok_or_else(|| {
        PptxError::InvalidXml("no sldIdLst found in presentation XML".to_string())
    })?;
    let lst_end = xml_str.find(lst_end_tag).ok_or_else(|| {
        PptxError::InvalidXml("no closing sldIdLst found in presentation XML".to_string())
    })?;

    let content_start = lst_start + lst_start_tag.len();
    let inner = &xml_str[content_start..lst_end];

    // Parse individual <p:sldId .../> entries
    let mut entries: Vec<&str> = Vec::new();
    let mut search_start = 0;
    while let Some(tag_start) = inner[search_start..].find("<p:sldId") {
        let abs_start = search_start + tag_start;
        let rest = &inner[abs_start..];
        if let Some(tag_end) = rest.find("/>") {
            let entry = &inner[abs_start..abs_start + tag_end + 2];
            entries.push(entry);
            search_start = abs_start + tag_end + 2;
        } else {
            break;
        }
    }

    if from_index >= entries.len() || to_index >= entries.len() {
        return Err(PptxError::InvalidXml(format!(
            "slide reorder indices out of range: from={}, to={}, count={}",
            from_index,
            to_index,
            entries.len()
        )));
    }

    // Reorder
    let entry = entries.remove(from_index);
    entries.insert(to_index, entry);

    // Rebuild directly into Vec<u8>
    let new_inner_len: usize = entries.iter().map(|e| e.len()).sum();
    let mut result = Vec::with_capacity(presentation_xml.len() + new_inner_len);
    result.extend_from_slice(&presentation_xml[..content_start]);
    for e in &entries {
        result.extend_from_slice(e.as_bytes());
    }
    result.extend_from_slice(&presentation_xml[lst_end..]);

    Ok(result)
}

/// Update the `<p:sldSz>` element in presentation XML with new cx/cy values.
///
/// Returns the updated XML bytes.
pub fn set_slide_size_in_xml(presentation_xml: &[u8], cx: i64, cy: i64) -> PptxResult<Vec<u8>> {
    let xml_str = std::str::from_utf8(presentation_xml)?;

    // Find <p:sldSz ... /> or <p:sldSz ...>
    // We need to replace the cx and cy attribute values
    if let Some(sld_sz_pos) = xml_str.find("<p:sldSz") {
        let after = &xml_str[sld_sz_pos..];
        // Find end of the sldSz element
        let end_pos = after
            .find("/>")
            .or_else(|| after.find('>'))
            .ok_or_else(|| PptxError::InvalidXml("malformed sldSz element".to_string()))?;
        let elem_str = &after[..end_pos
            + if after.as_bytes().get(end_pos + 1) == Some(&b'>') {
                2
            } else {
                1
            }];

        // Build new element preserving the type attribute if present
        let type_attr = elem_str.find("type=\"").and_then(|type_pos| {
            let rest = &elem_str[type_pos + 6..];
            rest.find('"').map(|end| &rest[..end])
        });

        let new_elem = type_attr.map_or_else(
            || format!(r#"<p:sldSz cx="{cx}" cy="{cy}"/>"#),
            |t| format!(r#"<p:sldSz cx="{cx}" cy="{cy}" type="{t}"/>"#),
        );

        // Find exact end position of the sldSz element
        let abs_end = sld_sz_pos + end_pos;
        let abs_end = if presentation_xml.get(abs_end) == Some(&b'/') {
            abs_end + 2 // skip />
        } else {
            abs_end + 1 // skip >
        };

        let mut result = Vec::with_capacity(presentation_xml.len() + new_elem.len());
        result.extend_from_slice(&presentation_xml[..sld_sz_pos]);
        result.extend_from_slice(new_elem.as_bytes());
        result.extend_from_slice(&presentation_xml[abs_end..]);
        Ok(result)
    } else {
        Ok(presentation_xml.to_vec())
    }
}
