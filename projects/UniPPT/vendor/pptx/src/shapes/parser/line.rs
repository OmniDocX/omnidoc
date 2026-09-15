//! Line format parsing from OOXML XML.

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::dml::color::ColorFormat;
use crate::dml::fill::{FillFormat, SolidFill};
use crate::dml::line::LineFormat;
use crate::enums::dml::MsoLineDashStyle;
use crate::error::{PptxError, PptxResult};
use crate::units::Emu;

use crate::xml_util::{attr_value, local_name_str, read_inner_xml};

use super::color::parse_color_with_alpha_from_xml;

/// Parse line format from an `<a:ln>` element's XML bytes.
pub fn parse_line_from_xml(ln_bytes: &[u8]) -> PptxResult<Option<LineFormat>> {
    let mut reader = Reader::from_reader(ln_bytes);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut width: Option<Emu> = None;
    let mut color: Option<ColorFormat> = None;
    let mut dash_style: Option<MsoLineDashStyle> = None;
    let mut fill: Option<FillFormat> = None;

    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let qn = e.name();
                let local = local_name_str(qn.as_ref());
                match local {
                    "ln" => {
                        // .ok() is intentional: absent or malformed width
                        // means "no explicit width" (None).
                        width = attr_value(e, b"w")?
                            .and_then(|s| s.parse::<i64>().ok())
                            .map(Emu);
                    }
                    "solidFill" => {
                        let inner = read_inner_xml(&mut reader, "solidFill")
                            .map_err(|e| PptxError::InvalidXml(format!("solidFill: {e}")))?;
                        if let Some((c, opacity)) = parse_color_with_alpha_from_xml(&inner)? {
                            color = Some(c.clone());
                            fill = Some(FillFormat::Solid(SolidFill { color: c, opacity }));
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                let qn = e.name();
                let local = local_name_str(qn.as_ref());
                match local {
                    "prstDash" => {
                        dash_style =
                            attr_value(e, b"val")?.and_then(|s| MsoLineDashStyle::from_xml_str(&s));
                    }
                    "noFill" => {
                        fill = Some(FillFormat::NoFill);
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(PptxError::InvalidXml(format!("line XML error: {e}"))),
            _ => {}
        }
    }

    if width.is_some() || color.is_some() || dash_style.is_some() || fill.is_some() {
        Ok(Some(LineFormat {
            color,
            width,
            dash_style,
            fill,
            cap: None,
            join: None,
        }))
    } else {
        Ok(None)
    }
}
