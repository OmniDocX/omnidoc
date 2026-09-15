//! Fill format parsing from OOXML XML.

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::dml::fill::{
    FillFormat, GradientFill, GradientStop, PictureFill, PictureSourceRect, SolidFill,
};
use crate::error::{PptxError, PptxResult};
use crate::units::RelationshipId;

use crate::xml_util::{attr_value, attr_value_ns, local_name_str, read_inner_xml};

use super::color::parse_color_with_alpha_from_xml;

/// Parse fill format from `<p:spPr>` or similar XML containing fill elements.
///
/// Detects `<a:solidFill>`, `<a:noFill/>`, `<a:gradFill>`, `<a:blipFill>`,
/// and `<a:grpFill/>`.
pub fn parse_fill_from_xml(sp_pr_bytes: &[u8]) -> PptxResult<Option<FillFormat>> {
    let mut reader = Reader::from_reader(sp_pr_bytes);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let qn = e.name();
                let local = local_name_str(qn.as_ref());
                match local {
                    "solidFill" => {
                        // Read inner content to find the color
                        let inner = read_inner_xml(&mut reader, "solidFill")
                            .map_err(|e| PptxError::InvalidXml(format!("solidFill: {e}")))?;
                        let (color, opacity) = match parse_color_with_alpha_from_xml(&inner)? {
                            Some(value) => value,
                            None => return Ok(None),
                        };
                        return Ok(Some(FillFormat::Solid(SolidFill { color, opacity })));
                    }
                    "gradFill" => {
                        let inner = read_inner_xml(&mut reader, "gradFill")
                            .map_err(|e| PptxError::InvalidXml(format!("gradFill: {e}")))?;
                        return parse_gradient_from_inner(&inner);
                    }
                    "blipFill" => {
                        let inner = read_inner_xml(&mut reader, "blipFill")
                            .map_err(|e| PptxError::InvalidXml(format!("blipFill: {e}")))?;
                        return parse_picture_from_inner(&inner);
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                let qn = e.name();
                let local = local_name_str(qn.as_ref());
                match local {
                    "noFill" => return Ok(Some(FillFormat::NoFill)),
                    "grpFill" => return Ok(Some(FillFormat::Background)),
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(PptxError::InvalidXml(format!("fill XML error: {e}"))),
            _ => {}
        }
    }
    Ok(None)
}

pub(super) fn parse_picture_from_inner(xml: &[u8]) -> PptxResult<Option<FillFormat>> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut image_r_id = None;
    let mut source_rect = None;
    let mut fill_rect = None;
    let mut stretch = false;
    let mut tile = false;

    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let qn = e.name();
                match local_name_str(qn.as_ref()) {
                    "blip" => {
                        image_r_id = attr_value_ns(e, b"embed")?.map(|value| value.into_owned());
                    }
                    "srcRect" => {
                        source_rect = Some(PictureSourceRect {
                            left: picture_rect_value(e, b"l")?,
                            top: picture_rect_value(e, b"t")?,
                            right: picture_rect_value(e, b"r")?,
                            bottom: picture_rect_value(e, b"b")?,
                        });
                    }
                    "fillRect" => {
                        fill_rect = Some(PictureSourceRect {
                            left: picture_rect_value(e, b"l")?,
                            top: picture_rect_value(e, b"t")?,
                            right: picture_rect_value(e, b"r")?,
                            bottom: picture_rect_value(e, b"b")?,
                        });
                    }
                    "stretch" => stretch = true,
                    "tile" => tile = true,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(PptxError::InvalidXml(format!(
                    "picture fill XML error: {e}"
                )))
            }
            _ => {}
        }
    }

    let Some(image_r_id) = image_r_id else {
        return Ok(None);
    };
    Ok(Some(FillFormat::Picture(PictureFill {
        image_r_id: RelationshipId::try_from(image_r_id)?,
        source_rect,
        fill_rect,
        stretch,
        tile,
        image_data: None,
        image_content_type: None,
    })))
}

fn picture_rect_value(element: &quick_xml::events::BytesStart<'_>, name: &[u8]) -> PptxResult<i64> {
    Ok(attr_value(element, name)?
        .and_then(|value| value.parse().ok())
        .unwrap_or(0))
}

pub(super) fn parse_gradient_from_inner(xml: &[u8]) -> PptxResult<Option<FillFormat>> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut stops = Vec::new();
    let mut angle: Option<f64> = None;

    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let qn = e.name();
                let local = local_name_str(qn.as_ref());
                if local == "gs" {
                    // .ok() is intentional: malformed position defaults to 0.0
                    // i64→f64: OOXML position values fit in 53-bit mantissa
                    #[allow(clippy::cast_precision_loss)]
                    let pos = attr_value(e, b"pos")?
                        .and_then(|s| s.parse::<i64>().ok())
                        .map_or(0.0, |v| v as f64 / 100_000.0);
                    let inner = read_inner_xml(&mut reader, "gs")
                        .map_err(|e| PptxError::InvalidXml(format!("gs: {e}")))?;
                    if let Some((color, opacity)) = parse_color_with_alpha_from_xml(&inner)? {
                        if let Ok(stop) = GradientStop::with_opacity(pos, color, opacity) {
                            stops.push(stop);
                        }
                    }
                }
            }
            Ok(Event::Empty(ref e)) => {
                let qn = e.name();
                let local = local_name_str(qn.as_ref());
                if local == "lin" {
                    // .ok() is intentional: malformed angle defaults to 0
                    let raw_angle = attr_value(e, b"ang")?
                        .and_then(|s| s.parse::<i64>().ok())
                        .unwrap_or(0);
                    // Convert from OOXML clockwise 60000ths to API counter-clockwise degrees
                    // i64→f64: OOXML angle values fit in 53-bit mantissa
                    #[allow(clippy::cast_precision_loss)]
                    let cw_degrees = raw_angle as f64 / 60000.0;
                    let api_angle = if cw_degrees == 0.0 {
                        0.0
                    } else {
                        360.0 - cw_degrees
                    };
                    angle = Some(api_angle);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(PptxError::InvalidXml(format!("gradient XML error: {e}"))),
            _ => {}
        }
    }

    if stops.is_empty() {
        return Ok(None);
    }

    Ok(Some(FillFormat::Gradient(GradientFill { stops, angle })))
}
