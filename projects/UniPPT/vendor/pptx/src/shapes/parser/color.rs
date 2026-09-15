//! Color parsing from OOXML XML.

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::dml::color::{ColorFormat, HslColor, PresetColor, SystemColor, ThemeColor};
use crate::enums::dml::{MsoThemeColorIndex, PresetColorVal, SystemColorVal};
use crate::error::PptxResult;
use crate::text::font::RgbColor;
use crate::xml_util::{attr_value, local_name_str};

/// Parse a color element from within a parent element's children.
///
/// Looks for `<a:srgbClr>`, `<a:schemeClr>`, `<a:sysClr>`, `<a:hslClr>`, `<a:prstClr>`.
///
/// # Errors
///
/// Returns an error if the XML contains malformed attributes.
pub(super) fn parse_color_from_xml(xml: &[u8]) -> PptxResult<Option<ColorFormat>> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => {
                let qn = e.name();
                let local = local_name_str(qn.as_ref());
                match local {
                    "srgbClr" => {
                        if let Some(val) = attr_value(e, b"val")? {
                            if let Ok(rgb) = RgbColor::from_hex(&val) {
                                return Ok(Some(ColorFormat::Rgb(rgb)));
                            }
                        }
                    }
                    "schemeClr" => {
                        if let Some(val) = attr_value(e, b"val")? {
                            if let Some(tc) = MsoThemeColorIndex::from_xml_str(&val) {
                                // Check for brightness modifiers in children
                                let brightness =
                                    parse_theme_brightness_from_remaining(&mut reader)?;
                                // Use validated constructor; skip if brightness out of range
                                if let Ok(theme) = ThemeColor::new(tc, brightness) {
                                    return Ok(Some(ColorFormat::Theme(theme)));
                                }
                            }
                        }
                    }
                    "sysClr" => {
                        let val = attr_value(e, b"val")?.map_or(SystemColorVal::WindowText, |c| {
                            SystemColorVal::from_xml_str(&c)
                        });
                        let last_color =
                            attr_value(e, b"lastClr")?.map(std::borrow::Cow::into_owned);
                        return Ok(Some(ColorFormat::System(SystemColor { val, last_color })));
                    }
                    "hslClr" => {
                        // .ok() is intentional: malformed values default to 0.0;
                        // the subsequent HslColor::new validates the final range.
                        // i64→f64: OOXML values fit in 53-bit mantissa
                        #[allow(clippy::cast_precision_loss)]
                        let hue = attr_value(e, b"hue")?
                            .and_then(|s| s.parse::<i64>().ok())
                            .map_or(0.0, |v| v as f64 / 60000.0);
                        #[allow(clippy::cast_precision_loss)]
                        let sat = attr_value(e, b"sat")?
                            .and_then(|s| s.parse::<i64>().ok())
                            .map_or(0.0, |v| v as f64 / 1000.0);
                        #[allow(clippy::cast_precision_loss)]
                        let lum = attr_value(e, b"lum")?
                            .and_then(|s| s.parse::<i64>().ok())
                            .map_or(0.0, |v| v as f64 / 1000.0);
                        if let Ok(hsl) = HslColor::new(hue, sat, lum) {
                            return Ok(Some(ColorFormat::Hsl(hsl)));
                        }
                    }
                    "prstClr" => {
                        let val = attr_value(e, b"val")?
                            .map_or(PresetColorVal::Black, |c| PresetColorVal::from_xml_str(&c));
                        return Ok(Some(ColorFormat::Preset(PresetColor { val })));
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    Ok(None)
}

/// Parse the first color and its direct DrawingML `<a:alpha>` transform.
///
/// Keeping this separate preserves the long-standing `parse_color_from_xml`
/// API for consumers that only need chroma while allowing fills to retain
/// their independent opacity.
pub(super) fn parse_color_with_alpha_from_xml(
    xml: &[u8],
) -> PptxResult<Option<(ColorFormat, f64)>> {
    let Some(color) = parse_color_from_xml(xml)? else {
        return Ok(None);
    };

    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut alpha = 1.0;
    loop {
        buffer.clear();
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(ref element) | Event::Empty(ref element))
                if local_name_str(element.name().as_ref()) == "alpha" =>
            {
                #[allow(clippy::cast_precision_loss)]
                if let Some(value) =
                    attr_value(element, b"val")?.and_then(|value| value.parse::<i64>().ok())
                {
                    alpha = (value as f64 / 100_000.0).clamp(0.0, 1.0);
                }
                break;
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    Ok(Some((color, alpha)))
}

/// After reading a `<a:schemeClr>` Start element, read its children to find
/// lumMod/lumOff brightness modifiers.
///
/// `.ok()` on parse is intentional: malformed modifier values are treated as
/// absent, causing the function to return `None` (no brightness override).
///
/// # Errors
///
/// Returns an error if the XML contains malformed attributes.
fn parse_theme_brightness_from_remaining(reader: &mut Reader<&[u8]>) -> PptxResult<Option<f64>> {
    let mut buf = Vec::new();
    let mut lum_mod: Option<f64> = None;
    let mut lum_off: Option<f64> = None;
    let mut depth = 1u32;

    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                depth += 1;
                let qn = e.name();
                let local = local_name_str(qn.as_ref());
                match local {
                    // i64→f64: OOXML modifier values fit in 53-bit mantissa
                    "lumMod" => {
                        #[allow(clippy::cast_precision_loss)]
                        {
                            lum_mod = attr_value(e, b"val")?
                                .and_then(|s| s.parse::<i64>().ok())
                                .map(|v| v as f64 / 100_000.0);
                        }
                    }
                    "lumOff" => {
                        #[allow(clippy::cast_precision_loss)]
                        {
                            lum_off = attr_value(e, b"val")?
                                .and_then(|s| s.parse::<i64>().ok())
                                .map(|v| v as f64 / 100_000.0);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                let qn = e.name();
                let local = local_name_str(qn.as_ref());
                match local {
                    // i64→f64: OOXML modifier values fit in 53-bit mantissa
                    "lumMod" => {
                        #[allow(clippy::cast_precision_loss)]
                        {
                            lum_mod = attr_value(e, b"val")?
                                .and_then(|s| s.parse::<i64>().ok())
                                .map(|v| v as f64 / 100_000.0);
                        }
                    }
                    "lumOff" => {
                        #[allow(clippy::cast_precision_loss)]
                        {
                            lum_off = attr_value(e, b"val")?
                                .and_then(|s| s.parse::<i64>().ok())
                                .map(|v| v as f64 / 100_000.0);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(_)) => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    // Reconstruct brightness from lumMod/lumOff
    Ok(match (lum_mod, lum_off) {
        (Some(_mod_val), Some(off_val)) if off_val > 0.0 => {
            // Tint: brightness = lumOff
            Some(off_val)
        }
        (Some(mod_val), None) if mod_val < 1.0 => {
            // Shade: brightness = -(1.0 - lumMod)
            Some(-(1.0 - mod_val))
        }
        _ => None,
    })
}
