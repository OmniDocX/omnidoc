//! Slide background manipulation functions.

use crate::dml::fill::GradientFill;
use crate::error::{PptxError, PptxResult};
use crate::xml_util::{xml_escape, WriteXml};

/// Set a solid color background on slide XML.
///
/// Inserts a `<p:bg>` element with a solid fill inside `<p:cSld>`.
/// The color should be a 6-char hex RGB string (e.g. "FF0000" for red).
pub(crate) fn set_slide_background_solid(slide_xml: &mut Vec<u8>, color: &str) -> PptxResult<()> {
    let xml_str = std::str::from_utf8(slide_xml)?;

    let bg_xml = format!(
        r#"<p:bg><p:bgPr><a:solidFill><a:srgbClr val="{}"/></a:solidFill><a:effectLst/></p:bgPr></p:bg>"#,
        xml_escape(color)
    );

    // Insert after <p:cSld> (or <p:cSld name="...">)
    let insert_at = if let Some(pos) = xml_str.find("<p:cSld>") {
        pos + "<p:cSld>".len()
    } else if let Some(pos) = xml_str.find("<p:cSld ") {
        let rest = &xml_str[pos..];
        if let Some(close) = rest.find('>') {
            pos + close + 1
        } else {
            return Err(PptxError::InvalidXml("malformed cSld element".to_string()));
        }
    } else {
        return Err(PptxError::InvalidXml(
            "no cSld element found in slide XML".to_string(),
        ));
    };

    let mut result = Vec::with_capacity(slide_xml.len() + bg_xml.len());
    result.extend_from_slice(&slide_xml[..insert_at]);
    result.extend_from_slice(bg_xml.as_bytes());
    result.extend_from_slice(&slide_xml[insert_at..]);
    *slide_xml = result;
    Ok(())
}

/// Set a gradient background on slide XML.
///
/// Inserts a `<p:bg>` element with a gradient fill inside `<p:cSld>`.
/// Uses the provided `GradientFill` to generate the gradient XML.
pub(crate) fn set_slide_background_gradient(
    slide_xml: &mut Vec<u8>,
    gradient: &GradientFill,
) -> PptxResult<()> {
    let xml_str = std::str::from_utf8(slide_xml)?;
    let xml_str = xml_str.to_owned();

    // Remove any existing <p:bg> element first
    let xml_str = remove_bg_element(&xml_str);

    let fill = crate::dml::fill::FillFormat::Gradient(gradient.clone());
    let bg_xml = format!(
        "<p:bg><p:bgPr>{}<a:effectLst/></p:bgPr></p:bg>",
        fill.to_xml_string()
    );

    let result = insert_bg_after_csld(&xml_str, &bg_xml)?;
    *slide_xml = result.into_bytes();
    Ok(())
}

/// Set an image background on slide XML.
///
/// Inserts a `<p:bg>` element with a blipFill inside `<p:cSld>`.
/// The `image_r_id` is the relationship ID referencing the image part.
pub(crate) fn set_slide_background_image(
    slide_xml: &mut Vec<u8>,
    image_r_id: &str,
) -> PptxResult<()> {
    let xml_str = std::str::from_utf8(slide_xml)?;
    let xml_str = xml_str.to_owned();

    // Remove any existing <p:bg> element first
    let xml_str = remove_bg_element(&xml_str);

    let bg_xml = format!(
        r#"<p:bg><p:bgPr><a:blipFill><a:blip r:embed="{}"/><a:stretch><a:fillRect/></a:stretch></a:blipFill><a:effectLst/></p:bgPr></p:bg>"#,
        xml_escape(image_r_id)
    );

    let result = insert_bg_after_csld(&xml_str, &bg_xml)?;
    *slide_xml = result.into_bytes();
    Ok(())
}

/// Set whether the slide follows the master slide background.
///
/// When `follow` is `true`, any existing `<p:bg>` element is removed so the
/// slide inherits the master's background. When `false`, this is a no-op
/// (the slide keeps its explicit background, or has none).
pub(crate) fn set_follow_master_background(
    slide_xml: &mut Vec<u8>,
    follow: bool,
) -> PptxResult<()> {
    if follow {
        let xml_str = std::str::from_utf8(slide_xml)?.to_owned();
        let result = remove_bg_element(&xml_str);
        *slide_xml = result.into_bytes();
    }
    Ok(())
}

/// Helper: remove the `<p:bg>...</p:bg>` element from slide XML.
pub(super) fn remove_bg_element(xml_str: &str) -> String {
    xml_str
        .find("<p:bg>")
        .and_then(|bg_start| {
            let bg_end = bg_start + xml_str[bg_start..].find("</p:bg>")? + "</p:bg>".len();
            let mut result = String::with_capacity(xml_str.len() - (bg_end - bg_start));
            result.push_str(&xml_str[..bg_start]);
            result.push_str(&xml_str[bg_end..]);
            Some(result)
        })
        .unwrap_or_else(|| xml_str.to_string())
}

/// Helper: insert a `<p:bg>` element right after the opening `<p:cSld>` tag.
fn insert_bg_after_csld(xml_str: &str, bg_xml: &str) -> PptxResult<String> {
    let insert_pos = if let Some(pos) = xml_str.find("<p:cSld>") {
        pos + "<p:cSld>".len()
    } else if let Some(pos) = xml_str.find("<p:cSld ") {
        let rest = &xml_str[pos..];
        if let Some(close) = rest.find('>') {
            pos + close + 1
        } else {
            return Err(PptxError::InvalidXml("malformed cSld element".to_string()));
        }
    } else {
        return Err(PptxError::InvalidXml(
            "no cSld element found in slide XML".to_string(),
        ));
    };

    let mut result = String::with_capacity(xml_str.len() + bg_xml.len());
    result.push_str(&xml_str[..insert_pos]);
    result.push_str(bg_xml);
    result.push_str(&xml_str[insert_pos..]);
    Ok(result)
}
