//! XML parsing for the `<a:clrScheme>` element in theme XML.

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::error::PptxResult;
use crate::text::font::RgbColor;
use crate::xml_util::{attr_value, local_name};

use super::ThemeColorScheme;

/// Parse the `<a:clrScheme>` from theme XML and extract the 12 color values.
///
/// Each color slot (dk1, lt1, accent1, ...) can contain either:
/// - `<a:sysClr lastClr="RRGGBB"/>` (system color with cached RGB)
/// - `<a:srgbClr val="RRGGBB"/>`
///
/// Returns `Ok(None)` if the theme XML has no color scheme element.
///
/// # Errors
///
/// Returns an error if the XML contains malformed attributes.
pub fn parse_theme_color_scheme(theme_xml: &[u8]) -> PptxResult<Option<ThemeColorScheme>> {
    let mut reader = Reader::from_reader(theme_xml);
    reader.config_mut().trim_text(true);

    let mut scheme = ThemeColorScheme::default();
    let mut current_slot: Option<String> = None;
    let mut in_clr_scheme = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let qname = e.name();
                let local = local_name(qname.as_ref());

                if local == b"clrScheme" {
                    in_clr_scheme = true;
                } else if in_clr_scheme {
                    match local {
                        b"dk1" | b"dk2" | b"lt1" | b"lt2" | b"accent1" | b"accent2"
                        | b"accent3" | b"accent4" | b"accent5" | b"accent6" | b"hlink"
                        | b"folHlink" => {
                            current_slot = Some(String::from_utf8_lossy(local).into_owned());
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::Empty(ref e)) if in_clr_scheme && current_slot.is_some() => {
                let qname = e.name();
                let local = local_name(qname.as_ref());

                let rgb = if local == b"srgbClr" {
                    attr_value(e, b"val")?.and_then(|v| RgbColor::from_hex(&v).ok())
                } else if local == b"sysClr" {
                    attr_value(e, b"lastClr")?.and_then(|v| RgbColor::from_hex(&v).ok())
                } else {
                    None
                };

                if let Some(color) = rgb {
                    if let Some(ref slot) = current_slot {
                        set_slot(&mut scheme, slot, color);
                    }
                    current_slot = None;
                }
            }
            Ok(Event::End(ref e)) => {
                let qname = e.name();
                let local = local_name(qname.as_ref());

                if local == b"clrScheme" {
                    break;
                }

                match local {
                    b"dk1" | b"dk2" | b"lt1" | b"lt2" | b"accent1" | b"accent2" | b"accent3"
                    | b"accent4" | b"accent5" | b"accent6" | b"hlink" | b"folHlink" => {
                        current_slot = None;
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => return Ok(None),
            _ => {}
        }
        buf.clear();
    }

    Ok(Some(scheme))
}

fn set_slot(scheme: &mut ThemeColorScheme, slot: &str, color: RgbColor) {
    match slot {
        "dk1" => scheme.dk1 = color,
        "dk2" => scheme.dk2 = color,
        "lt1" => scheme.lt1 = color,
        "lt2" => scheme.lt2 = color,
        "accent1" => scheme.accent1 = color,
        "accent2" => scheme.accent2 = color,
        "accent3" => scheme.accent3 = color,
        "accent4" => scheme.accent4 = color,
        "accent5" => scheme.accent5 = color,
        "accent6" => scheme.accent6 = color,
        "hlink" => scheme.hlink = color,
        "folHlink" => scheme.fol_hlink = color,
        _ => {}
    }
}
