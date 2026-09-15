//! DrawingML effect parsing used by the read/modify/write shape path.
//!
//! The writer has supported shadows for a long time, but the package parser
//! previously discarded them.  Keeping the structured value here lets
//! downstream renderers reproduce the native PowerPoint appearance while the
//! untouched source XML remains available to loss-aware exporters.

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::dml::effect::{ShadowFormat, ShadowType};
use crate::error::{PptxError, PptxResult};
use crate::units::Emu;
use crate::xml_util::{attr_value, local_name_str, read_inner_xml};

use super::parse_color_from_xml;

/// Parse the first native outer/inner shadow from a shape-property fragment.
pub fn parse_shadow_from_xml(sp_pr_bytes: &[u8]) -> PptxResult<Option<ShadowFormat>> {
    let mut reader = Reader::from_reader(sp_pr_bytes);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref element)) => {
                let qualified_name = element.name();
                let name = local_name_str(qualified_name.as_ref());
                if name == "outerShdw" || name == "innerShdw" {
                    let shadow_type = if name == "innerShdw" {
                        ShadowType::Inner
                    } else {
                        ShadowType::Outer
                    };
                    let blur_radius = emu_attr(element, b"blurRad")?;
                    let distance = emu_attr(element, b"dist")?;
                    let direction = angle_attr(element, b"dir")?;
                    let inner = read_inner_xml(&mut reader, name)
                        .map_err(|error| PptxError::InvalidXml(format!("{name}: {error}")))?;
                    let color = parse_color_from_xml(&inner)?;
                    let opacity = parse_alpha(&inner)?;
                    return Ok(Some(ShadowFormat {
                        shadow_type,
                        color,
                        blur_radius,
                        distance,
                        direction,
                        opacity,
                    }));
                }
            }
            Ok(Event::Eof) => break,
            Err(error) => return Err(PptxError::InvalidXml(format!("effect XML error: {error}"))),
            _ => {}
        }
    }

    Ok(None)
}

/// Parse the native DrawingML soft-edge radius from a shape-property fragment.
///
/// The value is stored in EMU by `<a:softEdge rad="..."/>`. Keeping it as an
/// exact integer prevents a read/project/write cycle from introducing unit
/// drift before the scene layer converts it to CSS pixels.
pub fn parse_soft_edge_radius_from_xml(sp_pr_bytes: &[u8]) -> PptxResult<Option<Emu>> {
    let mut reader = Reader::from_reader(sp_pr_bytes);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref element) | Event::Empty(ref element))
                if local_name_str(element.name().as_ref()) == "softEdge" =>
            {
                return emu_attr(element, b"rad");
            }
            Ok(Event::Eof) => break,
            Err(error) => {
                return Err(PptxError::InvalidXml(format!(
                    "soft-edge XML error: {error}"
                )))
            }
            _ => {}
        }
    }

    Ok(None)
}

fn emu_attr(element: &BytesStart<'_>, name: &[u8]) -> PptxResult<Option<Emu>> {
    Ok(attr_value(element, name)?
        .and_then(|value| value.parse::<i64>().ok())
        .map(Emu))
}

fn angle_attr(element: &BytesStart<'_>, name: &[u8]) -> PptxResult<Option<f64>> {
    Ok(attr_value(element, name)?
        .and_then(|value| value.parse::<i64>().ok())
        .map(|value| value as f64 / 60_000.0))
}

fn parse_alpha(xml: &[u8]) -> PptxResult<Option<f64>> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref element) | Event::Empty(ref element))
                if local_name_str(element.name().as_ref()) == "alpha" =>
            {
                return Ok(attr_value(element, b"val")?
                    .and_then(|value| value.parse::<i64>().ok())
                    .map(|value| (value as f64 / 100_000.0).clamp(0.0, 1.0)));
            }
            Ok(Event::Eof) => break,
            Err(error) => {
                return Err(PptxError::InvalidXml(format!(
                    "shadow alpha XML error: {error}"
                )))
            }
            _ => {}
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dml::color::ColorFormat;

    #[test]
    fn parses_outer_shadow_without_discarding_alpha() {
        let xml = br#"<p:spPr xmlns:p="p" xmlns:a="a"><a:effectLst><a:outerShdw blurRad="50800" dist="38100" dir="16200000"><a:srgbClr val="102030"><a:alpha val="42000"/></a:srgbClr></a:outerShdw><a:glow rad="5000"/></a:effectLst></p:spPr>"#;
        let shadow = parse_shadow_from_xml(xml).unwrap().unwrap();
        assert_eq!(shadow.shadow_type, ShadowType::Outer);
        assert_eq!(shadow.blur_radius, Some(Emu(50_800)));
        assert_eq!(shadow.distance, Some(Emu(38_100)));
        assert_eq!(shadow.direction, Some(270.0));
        assert_eq!(shadow.opacity, Some(0.42));
        assert!(matches!(shadow.color, Some(ColorFormat::Rgb(_))));
    }

    #[test]
    fn parses_soft_edge_radius_without_rounding() {
        let xml = br#"<p:spPr xmlns:p="p" xmlns:a="a"><a:effectLst><a:softEdge rad="635000"/></a:effectLst></p:spPr>"#;
        assert_eq!(
            parse_soft_edge_radius_from_xml(xml).unwrap(),
            Some(Emu(635_000))
        );
    }
}
