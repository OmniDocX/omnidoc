//! Helper functions for text frame parsing.

use quick_xml::events::BytesStart;

use crate::enums::text::{
    MsoTextUnderlineType, MsoVerticalAnchor, PpParagraphAlignment, TextDirection,
};
use crate::error::{PptxError, PptxResult};
use crate::text::font::Font;
use crate::text::{Paragraph, TextFrame};
use crate::units::Emu;

use crate::xml_util::attr_value;

pub(super) fn parse_body_pr_attrs(e: &BytesStart<'_>, tf: &mut TextFrame) -> PptxResult<()> {
    // wrap
    if let Some(wrap) = attr_value(e, b"wrap")? {
        tf.word_wrap = wrap != "none";
    }
    // anchor (vertical alignment)
    if let Some(anchor) = attr_value(e, b"anchor")? {
        tf.vertical_anchor = MsoVerticalAnchor::from_xml_str(&anchor);
    }
    // Preserve the exact DrawingML text-flow token.  In particular `eaVert`
    // means upright East-Asian glyphs in right-to-left vertical columns and
    // cannot be reconstructed from the text box dimensions.
    if let Some(vertical_type) = attr_value(e, b"vert")? {
        tf.vertical_type = Some(vertical_type.into_owned());
    }
    // margins – absent attribute → use OOXML default (None); present but unparseable → error
    if let Some(l) = attr_value(e, b"lIns")? {
        tf.margin_left = Some(Emu(l
            .parse::<i64>()
            .map_err(|_| PptxError::InvalidXml(format!("invalid numeric value: {l}")))?));
    }
    if let Some(t) = attr_value(e, b"tIns")? {
        tf.margin_top = Some(Emu(t
            .parse::<i64>()
            .map_err(|_| PptxError::InvalidXml(format!("invalid numeric value: {t}")))?));
    }
    if let Some(r) = attr_value(e, b"rIns")? {
        tf.margin_right = Some(Emu(r
            .parse::<i64>()
            .map_err(|_| PptxError::InvalidXml(format!("invalid numeric value: {r}")))?));
    }
    if let Some(b) = attr_value(e, b"bIns")? {
        tf.margin_bottom = Some(Emu(b
            .parse::<i64>()
            .map_err(|_| PptxError::InvalidXml(format!("invalid numeric value: {b}")))?));
    }
    // rotation
    if let Some(rot) = attr_value(e, b"rot")? {
        if let Ok(val) = rot.parse::<i64>() {
            // i64→f64: OOXML rotation values fit in 53-bit mantissa
            #[allow(clippy::cast_precision_loss)]
            {
                tf.rotation = Some(val as f64 / 60000.0);
            }
        }
    }
    Ok(())
}

pub(super) fn parse_paragraph_props(e: &BytesStart<'_>, para: &mut Paragraph) -> PptxResult<()> {
    if let Some(algn) = attr_value(e, b"algn")? {
        para.alignment = PpParagraphAlignment::from_xml_str(&algn);
    }
    if let Some(lvl) = attr_value(e, b"lvl")? {
        para.level = lvl
            .parse()
            .map_err(|_| PptxError::InvalidXml(format!("invalid numeric value: {lvl}")))?;
    }
    if let Some(rtl) = attr_value(e, b"rtl")? {
        para.text_direction = TextDirection::from_xml_attr(&rtl);
    }
    Ok(())
}

pub(super) fn parse_run_props_attrs(e: &BytesStart<'_>, font: &mut Font) -> PptxResult<()> {
    // bold
    if let Some(b) = attr_value(e, b"b")? {
        font.bold = Some(b == "1" || b == "true");
    }
    // italic
    if let Some(i) = attr_value(e, b"i")? {
        font.italic = Some(i == "1" || i == "true");
    }
    // size (in hundredths of a point); skip non-positive values
    if let Some(sz) = attr_value(e, b"sz")? {
        if let Ok(val) = sz.parse::<i64>() {
            // i64→f64: OOXML font size values fit in 53-bit mantissa
            #[allow(clippy::cast_precision_loss)]
            let pt = val as f64 / 100.0;
            if pt > 0.0 {
                font.size = Some(pt);
            }
        }
    }
    // underline
    if let Some(u) = attr_value(e, b"u")? {
        font.underline = MsoTextUnderlineType::from_xml_str(&u);
    }
    // strikethrough
    if let Some(strike) = attr_value(e, b"strike")? {
        font.strikethrough = Some(strike == "sngStrike");
    }
    // language
    if let Some(lang) = attr_value(e, b"lang")? {
        font.language_id = Some(lang.into_owned());
    }
    // baseline (super/subscript)
    if let Some(baseline) = attr_value(e, b"baseline")? {
        if let Ok(val) = baseline.parse::<i64>() {
            font.baseline_offset = Some(val);
            match val.cmp(&0) {
                std::cmp::Ordering::Greater => font.superscript = Some(true),
                std::cmp::Ordering::Less => font.subscript = Some(true),
                std::cmp::Ordering::Equal => {}
            }
        }
    }
    Ok(())
}

pub(super) const fn has_font_properties(font: &Font) -> bool {
    font.name.is_some()
        || font.east_asia_name.is_some()
        || font.complex_script_name.is_some()
        || font.symbol_name.is_some()
        || font.size.is_some()
        || font.bold.is_some()
        || font.italic.is_some()
        || font.underline.is_some()
        || font.color.is_some()
        || font.strikethrough.is_some()
        || font.subscript.is_some()
        || font.superscript.is_some()
        || font.baseline_offset.is_some()
        || font.language_id.is_some()
        || font.fill.is_some()
        || font.hyperlink.is_some()
}
