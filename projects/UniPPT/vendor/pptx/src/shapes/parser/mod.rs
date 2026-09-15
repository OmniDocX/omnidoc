//! XML-to-struct parsers for read-modify-write support.
//!
//! These functions parse OOXML shape sub-elements (text frame, fill, line)
//! back into their Rust struct representation, enabling round-tripping:
//! open .pptx -> parse shapes into structs -> modify -> save.

mod color;
mod effect;
mod fill;
mod line;
mod text_frame;
mod text_helpers;

pub use effect::{parse_shadow_from_xml, parse_soft_edge_radius_from_xml};
pub use fill::parse_fill_from_xml;
pub use line::parse_line_from_xml;
pub use text_frame::parse_text_frame_from_xml;

use quick_xml::events::Event;
use quick_xml::Reader;

use color::parse_color_from_xml;
use color::parse_color_with_alpha_from_xml;

use crate::dml::fill::FillFormat;
use crate::dml::line::LineFormat;
use crate::dml::{DuotoneColor, DuotoneColorTransform, DuotoneEffect};
use crate::enums::dml::PresetColorVal;
use crate::error::{PptxError, PptxResult};
use crate::shapes::freeform::FreeformBuilder;
use crate::xml_util::{attr_value, local_name_str, read_inner_xml};

/// Parse the two-color bitmap mapping retained below an `a:blip`.
pub fn parse_duotone_from_xml(xml: &[u8]) -> PptxResult<Option<DuotoneEffect>> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut colors = Vec::new();
    let mut current: Option<DuotoneColor> = None;
    loop {
        buffer.clear();
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(ref element)) => match local_name_str(element.name().as_ref()) {
                "srgbClr" => {
                    current = attr_value(element, b"val")?
                        .and_then(|value| crate::text::font::RgbColor::from_hex(&value).ok())
                        .map(|rgb| DuotoneColor {
                            color: crate::dml::ColorFormat::Rgb(rgb),
                            transforms: vec![],
                        });
                }
                "prstClr" => {
                    current = Some(DuotoneColor {
                        color: crate::dml::ColorFormat::Preset(crate::dml::PresetColor {
                            val: attr_value(element, b"val")?
                                .map_or(PresetColorVal::Black, |value| {
                                    PresetColorVal::from_xml_str(&value)
                                }),
                        }),
                        transforms: vec![],
                    });
                }
                _ => {}
            },
            Ok(Event::Empty(ref element)) => match local_name_str(element.name().as_ref()) {
                "srgbClr" => {
                    if let Some(rgb) = attr_value(element, b"val")?
                        .and_then(|value| crate::text::font::RgbColor::from_hex(&value).ok())
                    {
                        colors.push(DuotoneColor {
                            color: crate::dml::ColorFormat::Rgb(rgb),
                            transforms: vec![],
                        });
                    }
                }
                "prstClr" => colors.push(DuotoneColor {
                    color: crate::dml::ColorFormat::Preset(crate::dml::PresetColor {
                        val: attr_value(element, b"val")?.map_or(PresetColorVal::Black, |value| {
                            PresetColorVal::from_xml_str(&value)
                        }),
                    }),
                    transforms: vec![],
                }),
                "shade" | "satMod" => {
                    if let (Some(color), Some(value)) = (
                        current.as_mut(),
                        attr_value(element, b"val")?.and_then(|value| value.parse::<f64>().ok()),
                    ) {
                        color.transforms.push(
                            if local_name_str(element.name().as_ref()) == "shade" {
                                DuotoneColorTransform::Shade(value / 100_000.0)
                            } else {
                                DuotoneColorTransform::SaturationModulation(value / 100_000.0)
                            },
                        );
                    }
                }
                _ => {}
            },
            Ok(Event::End(ref element))
                if matches!(
                    local_name_str(element.name().as_ref()),
                    "srgbClr" | "prstClr"
                ) =>
            {
                if let Some(color) = current.take() {
                    colors.push(color);
                }
            }
            Ok(Event::Eof) => break,
            Err(error) => return Err(PptxError::Xml(error)),
            _ => {}
        }
    }
    Ok((colors.len() == 2).then(|| DuotoneEffect {
        colors,
        raw_xml: std::str::from_utf8(xml).ok().map(str::to_string),
    }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CustomPathCommand {
    Move,
    Line,
    Cubic,
}

/// Parse the first DrawingML path from an `<a:custGeom>` inside `spPr`.
///
/// PowerPoint freeforms commonly contain many subpaths in one `a:path`.
/// `FreeformBuilder` retains that ordering and can project the commands to SVG
/// without rasterising the original shape.
pub fn parse_custom_geometry_from_xml(sp_pr_bytes: &[u8]) -> PptxResult<Option<FreeformBuilder>> {
    let mut reader = Reader::from_reader(sp_pr_bytes);
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut in_custom_geometry = false;
    let mut in_path = false;
    let mut width = 0i64;
    let mut height = 0i64;
    let mut command = None;
    let mut points = Vec::<(i64, i64)>::new();
    let mut builder: Option<FreeformBuilder> = None;

    loop {
        buffer.clear();
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(ref element)) => {
                let qualified_name = element.name();
                let local = local_name_str(qualified_name.as_ref());
                match local {
                    "custGeom" => in_custom_geometry = true,
                    "path" if in_custom_geometry && !in_path => {
                        width = parse_geometry_coordinate(element, b"w")?;
                        height = parse_geometry_coordinate(element, b"h")?;
                        in_path = true;
                    }
                    "moveTo" if in_path => {
                        command = Some(CustomPathCommand::Move);
                        points.clear();
                    }
                    "lnTo" if in_path => {
                        command = Some(CustomPathCommand::Line);
                        points.clear();
                    }
                    "cubicBezTo" if in_path => {
                        command = Some(CustomPathCommand::Cubic);
                        points.clear();
                    }
                    "pt" if in_path && command.is_some() => {
                        points.push((
                            parse_geometry_coordinate(element, b"x")?,
                            parse_geometry_coordinate(element, b"y")?,
                        ));
                    }
                    "close" if in_path => {
                        if let Some(path) = builder.as_mut() {
                            path.close();
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref element)) => {
                let qualified_name = element.name();
                let local = local_name_str(qualified_name.as_ref());
                match local {
                    "pt" if in_path && command.is_some() => {
                        points.push((
                            parse_geometry_coordinate(element, b"x")?,
                            parse_geometry_coordinate(element, b"y")?,
                        ));
                    }
                    "close" if in_path => {
                        if let Some(path) = builder.as_mut() {
                            path.close();
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref element)) => {
                let qualified_name = element.name();
                let local = local_name_str(qualified_name.as_ref());
                if in_path && matches!(local, "moveTo" | "lnTo" | "cubicBezTo") {
                    append_custom_path_command(&mut builder, command, &points, width, height);
                    command = None;
                    points.clear();
                } else if local == "path" && in_path {
                    break;
                } else if local == "custGeom" {
                    in_custom_geometry = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(error) => {
                return Err(PptxError::InvalidXml(format!(
                    "custom geometry XML error: {error}"
                )))
            }
            _ => {}
        }
    }

    Ok(builder)
}

fn parse_geometry_coordinate(
    element: &quick_xml::events::BytesStart<'_>,
    name: &[u8],
) -> PptxResult<i64> {
    Ok(attr_value(element, name)?
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(0))
}

fn append_custom_path_command(
    builder: &mut Option<FreeformBuilder>,
    command: Option<CustomPathCommand>,
    points: &[(i64, i64)],
    width: i64,
    height: i64,
) {
    match (command, points) {
        (Some(CustomPathCommand::Move), [(x, y), ..]) => {
            if let Some(path) = builder.as_mut() {
                path.move_to(*x, *y);
            } else {
                *builder = Some(FreeformBuilder::new(*x, *y, width, height));
            }
        }
        (Some(CustomPathCommand::Line), [(x, y), ..]) => {
            if let Some(path) = builder.as_mut() {
                path.line_to(*x, *y);
            }
        }
        (Some(CustomPathCommand::Cubic), [first, second, end, ..]) => {
            if let Some(path) = builder.as_mut() {
                path.curve_to(first.0, first.1, second.0, second.1, end.0, end.1);
            }
        }
        _ => {}
    }
}

// ============================================================
// spPr parsing (extract fill + line from full spPr XML)
// ============================================================

/// Parse both fill and line from a `<p:spPr>` or `<a:spPr>` XML fragment.
///
/// Returns `(fill, line)`.
pub fn parse_sp_pr(sp_pr_bytes: &[u8]) -> PptxResult<(Option<FillFormat>, Option<LineFormat>)> {
    let mut reader = Reader::from_reader(sp_pr_bytes);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut fill: Option<FillFormat> = None;
    let mut line: Option<LineFormat> = None;
    let mut depth: u32 = 0;

    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                depth += 1;
                let qn = e.name();
                let local = local_name_str(qn.as_ref());
                match local {
                    "solidFill" if depth <= 2 => {
                        let inner = read_inner_xml(&mut reader, "solidFill")
                            .map_err(|e| PptxError::InvalidXml(format!("solidFill: {e}")))?;
                        if let Some((color, opacity)) = parse_color_with_alpha_from_xml(&inner)? {
                            fill = Some(FillFormat::Solid(crate::dml::fill::SolidFill {
                                color,
                                opacity,
                            }));
                        }
                    }
                    "gradFill" if depth <= 2 => {
                        let inner = read_inner_xml(&mut reader, "gradFill")
                            .map_err(|e| PptxError::InvalidXml(format!("gradFill: {e}")))?;
                        fill = fill::parse_gradient_from_inner(&inner)?;
                    }
                    "blipFill" if depth <= 2 => {
                        let inner = read_inner_xml(&mut reader, "blipFill")
                            .map_err(|e| PptxError::InvalidXml(format!("blipFill: {e}")))?;
                        fill = fill::parse_picture_from_inner(&inner)?;
                    }
                    "ln" => {
                        let w_attr = attr_value(e, b"w")?;
                        let inner = read_inner_xml(&mut reader, "ln")
                            .map_err(|e| PptxError::InvalidXml(format!("ln: {e}")))?;
                        // Reconstruct the <a:ln> element for parse_line_from_xml
                        let mut ln_xml = String::from("<a:ln");
                        if let Some(w) = w_attr {
                            ln_xml.push_str(&format!(r#" w="{w}""#));
                        }
                        ln_xml.push('>');
                        ln_xml.push_str(&String::from_utf8_lossy(&inner));
                        ln_xml.push_str("</a:ln>");
                        line = line::parse_line_from_xml(ln_xml.as_bytes())?;
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                let qn = e.name();
                let local = local_name_str(qn.as_ref());
                match local {
                    "noFill" if depth <= 1 => {
                        fill = Some(FillFormat::NoFill);
                    }
                    "grpFill" if depth <= 1 => {
                        fill = Some(FillFormat::Background);
                    }
                    _ => {}
                }
            }
            Ok(Event::End(_)) => {
                depth = depth.saturating_sub(1);
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(PptxError::InvalidXml(format!("spPr XML error: {e}"))),
            _ => {}
        }
    }

    Ok((fill, line))
}

#[cfg(test)]
mod tests;
