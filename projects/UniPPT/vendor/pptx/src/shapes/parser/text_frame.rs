//! Text frame parsing from OOXML XML.

use std::mem;

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::dml::color::ColorFormat;
use crate::dml::fill::FillFormat;
use crate::enums::dml::MsoThemeColorIndex;
use crate::enums::text::MsoAutoSize;
use crate::error::PptxResult;
use crate::text::font::{Font, RgbColor};
use crate::text::{Paragraph, Run, TextFrame};

use crate::xml_util::{attr_value, local_name_str, read_inner_xml};

use crate::dml::color::ThemeColor;

use super::text_helpers::{
    has_font_properties, parse_body_pr_attrs, parse_paragraph_props, parse_run_props_attrs,
};

/// Parse a `<p:txBody>` (or `<a:txBody>`) XML fragment into a `TextFrame`.
///
/// # Errors
///
/// Returns an error if the XML is malformed or contains invalid attributes.
#[allow(clippy::too_many_lines)]
pub fn parse_text_frame_from_xml(tx_body_bytes: &[u8]) -> PptxResult<Option<TextFrame>> {
    let mut reader = Reader::from_reader(tx_body_bytes);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();

    let mut tf = TextFrame::new();
    tf.paragraphs.clear();

    let mut in_body_pr = false;
    let mut in_paragraph = false;
    let mut in_run = false;
    let mut in_run_props = false;
    let mut in_para_props = false;
    let mut in_t = false;
    let mut in_end_para_rpr = false;
    let mut paragraph_spacing_context: Option<&'static str> = None;

    let mut current_para = Paragraph::new();
    let mut current_run = Run::new();
    let mut current_font = Font::new();
    let mut para_font = Font::new();
    let mut found_body_pr = false;

    // Track nested depth for collecting color within rPr
    let mut rpr_solid_fill_depth: Option<u32> = None;
    let mut para_rpr_solid_fill_depth: Option<u32> = None;
    let mut depth: u32 = 0;

    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                depth += 1;
                let qn = e.name();
                let local = local_name_str(qn.as_ref());
                match local {
                    "bodyPr" => {
                        found_body_pr = true;
                        in_body_pr = true;
                        parse_body_pr_attrs(e, &mut tf)?;
                    }
                    "normAutofit" if in_body_pr => {
                        parse_norm_autofit(e, &mut tf)?;
                    }
                    "spAutoFit" if in_body_pr => {
                        tf.auto_size = MsoAutoSize::ShapeToFitText;
                    }
                    "p" if !in_body_pr => {
                        in_paragraph = true;
                        current_para = Paragraph::new();
                    }
                    "pPr" if in_paragraph => {
                        in_para_props = true;
                        parse_paragraph_props(e, &mut current_para)?;
                    }
                    "lnSpc" if in_para_props => paragraph_spacing_context = Some("line"),
                    "spcBef" if in_para_props => paragraph_spacing_context = Some("before"),
                    "spcAft" if in_para_props => paragraph_spacing_context = Some("after"),
                    "defRPr" if in_para_props => {
                        para_font = Font::new();
                        parse_run_props_attrs(e, &mut para_font)?;
                        in_end_para_rpr = false;
                    }
                    "r" if in_paragraph => {
                        in_run = true;
                        current_run = Run::new();
                        current_font = Font::new();
                    }
                    "rPr" if in_run => {
                        in_run_props = true;
                        parse_run_props_attrs(e, &mut current_font)?;
                    }
                    "endParaRPr" if in_paragraph => {
                        in_end_para_rpr = true;
                        para_font = Font::new();
                        parse_run_props_attrs(e, &mut para_font)?;
                    }
                    "solidFill" if in_run_props => {
                        rpr_solid_fill_depth = Some(depth);
                    }
                    "solidFill" if in_end_para_rpr || in_para_props => {
                        para_rpr_solid_fill_depth = Some(depth);
                    }
                    "srgbClr" if rpr_solid_fill_depth.is_some() => {
                        if let Some(val) = attr_value(e, b"val")? {
                            if let Ok(rgb) = RgbColor::from_hex(&val) {
                                current_font.color = Some(rgb);
                            }
                        }
                    }
                    "srgbClr" if para_rpr_solid_fill_depth.is_some() => {
                        if let Some(val) = attr_value(e, b"val")? {
                            if let Ok(rgb) = RgbColor::from_hex(&val) {
                                para_font.color = Some(rgb);
                            }
                        }
                    }
                    "schemeClr" if rpr_solid_fill_depth.is_some() => {
                        current_font.fill = parse_scheme_font_fill(&mut reader, e)?;
                        // The helper consumes the matching schemeClr end tag.
                        depth = depth.saturating_sub(1);
                    }
                    "schemeClr" if para_rpr_solid_fill_depth.is_some() => {
                        para_font.fill = parse_scheme_font_fill(&mut reader, e)?;
                        depth = depth.saturating_sub(1);
                    }
                    "gradFill" if in_run_props => {
                        let inner = read_inner_xml(&mut reader, "gradFill")?;
                        current_font.fill = super::fill::parse_gradient_from_inner(&inner)?;
                        // `read_inner_xml` consumes the matching end event, so
                        // keep the parser's explicit depth in sync.
                        depth = depth.saturating_sub(1);
                    }
                    "gradFill" if in_end_para_rpr || in_para_props => {
                        let inner = read_inner_xml(&mut reader, "gradFill")?;
                        para_font.fill = super::fill::parse_gradient_from_inner(&inner)?;
                        depth = depth.saturating_sub(1);
                    }
                    "t" if in_run => {
                        in_t = true;
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                let qn = e.name();
                let local = local_name_str(qn.as_ref());
                if in_para_props {
                    if let Some(value) =
                        attr_value(e, b"val")?.and_then(|value| value.parse::<f64>().ok())
                    {
                        match (paragraph_spacing_context, local) {
                            (Some("line"), "spcPct") => {
                                current_para.line_spacing = Some(value / 100_000.0);
                            }
                            (Some("before"), "spcPts") => {
                                current_para.space_before = Some(value / 100.0);
                            }
                            (Some("after"), "spcPts") => {
                                current_para.space_after = Some(value / 100.0);
                            }
                            _ => {}
                        }
                    }
                }
                handle_empty_element(
                    local,
                    e,
                    &mut tf,
                    &mut found_body_pr,
                    in_body_pr,
                    in_paragraph,
                    in_run,
                    in_run_props,
                    in_end_para_rpr,
                    in_para_props,
                    rpr_solid_fill_depth,
                    para_rpr_solid_fill_depth,
                    &mut current_font,
                    &mut para_font,
                    &mut current_para,
                )?;
            }
            Ok(Event::Text(ref t)) if in_t => {
                if let Ok(text) = t.decode() {
                    current_run.set_text(&text);
                }
            }
            Ok(Event::End(ref e)) => {
                if matches!(
                    local_name_str(e.name().as_ref()),
                    "lnSpc" | "spcBef" | "spcAft"
                ) {
                    paragraph_spacing_context = None;
                }
                handle_end_element(
                    e,
                    &mut in_body_pr,
                    &mut in_paragraph,
                    &mut in_para_props,
                    &mut in_run,
                    &mut in_run_props,
                    &mut in_end_para_rpr,
                    &mut in_t,
                    &mut rpr_solid_fill_depth,
                    &mut para_rpr_solid_fill_depth,
                    depth,
                    &mut tf,
                    &mut current_para,
                    &mut current_run,
                    &mut current_font,
                    &mut para_font,
                );
                depth = depth.saturating_sub(1);
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    if !found_body_pr && tf.paragraphs().is_empty() {
        return Ok(None);
    }

    Ok(Some(tf))
}

/// Parse `<normAutofit>` element for font scale.
fn parse_norm_autofit(e: &quick_xml::events::BytesStart<'_>, tf: &mut TextFrame) -> PptxResult<()> {
    tf.auto_size = MsoAutoSize::TextToFitShape;
    if let Some(scale) = attr_value(e, b"fontScale")? {
        if let Ok(val) = scale.parse::<i64>() {
            // i64→f64: OOXML font scale values fit in 53-bit mantissa
            #[allow(clippy::cast_precision_loss)]
            {
                tf.font_scale = Some(val as f64 / 1000.0);
            }
        }
    }
    Ok(())
}

/// Parse a non-empty `<a:schemeClr>` used by a text solid fill. Text colors
/// commonly carry `lumMod`/`lumOff` children (for example bg1 at 50% becomes
/// the grey labels used by PowerPoint). The old text parser only handled an
/// empty schemeClr and silently fell back to the editor's default navy.
fn parse_scheme_font_fill(
    reader: &mut Reader<&[u8]>,
    element: &quick_xml::events::BytesStart<'_>,
) -> PptxResult<Option<FillFormat>> {
    let theme_color = attr_value(element, b"val")?
        .as_deref()
        .and_then(MsoThemeColorIndex::from_xml_str);
    let brightness = parse_scheme_brightness(reader)?;
    Ok(theme_color.map(|theme_color| {
        FillFormat::solid(ColorFormat::Theme(ThemeColor {
            theme_color,
            brightness,
        }))
    }))
}

fn parse_scheme_brightness(reader: &mut Reader<&[u8]>) -> PptxResult<Option<f64>> {
    let mut buf = Vec::new();
    let mut depth = 1u32;
    let mut lum_mod = None;
    let mut lum_off = None;
    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref element)) => {
                depth += 1;
                parse_luminance_transform(element, &mut lum_mod, &mut lum_off)?;
            }
            Ok(Event::Empty(ref element)) => {
                parse_luminance_transform(element, &mut lum_mod, &mut lum_off)?;
            }
            Ok(Event::End(_)) => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(error) => return Err(error.into()),
            _ => {}
        }
    }
    Ok(match (lum_mod, lum_off) {
        (Some(_), Some(offset)) if offset > 0.0 => Some(offset),
        (Some(modifier), None) if modifier < 1.0 => Some(-(1.0 - modifier)),
        _ => None,
    })
}

fn parse_luminance_transform(
    element: &quick_xml::events::BytesStart<'_>,
    lum_mod: &mut Option<f64>,
    lum_off: &mut Option<f64>,
) -> PptxResult<()> {
    let qn = element.name();
    let value = attr_value(element, b"val")?
        .and_then(|value| value.parse::<f64>().ok())
        .map(|value| value / 100_000.0);
    match local_name_str(qn.as_ref()) {
        "lumMod" => *lum_mod = value,
        "lumOff" => *lum_off = value,
        _ => {}
    }
    Ok(())
}

/// Handle an empty XML element within the text frame parser.
#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
fn handle_empty_element(
    local: &str,
    e: &quick_xml::events::BytesStart<'_>,
    tf: &mut TextFrame,
    found_body_pr: &mut bool,
    in_body_pr: bool,
    in_paragraph: bool,
    in_run: bool,
    in_run_props: bool,
    in_end_para_rpr: bool,
    in_para_props: bool,
    rpr_solid_fill_depth: Option<u32>,
    para_rpr_solid_fill_depth: Option<u32>,
    current_font: &mut Font,
    para_font: &mut Font,
    current_para: &mut Paragraph,
) -> PptxResult<()> {
    match local {
        "bodyPr" => {
            *found_body_pr = true;
            parse_body_pr_attrs(e, tf)?;
        }
        "normAutofit" if in_body_pr => {
            parse_norm_autofit(e, tf)?;
        }
        "spAutoFit" if in_body_pr => {
            tf.auto_size = MsoAutoSize::ShapeToFitText;
        }
        "pPr" if in_paragraph => {
            parse_paragraph_props(e, current_para)?;
        }
        "rPr" if in_run => {
            parse_run_props_attrs(e, current_font)?;
        }
        "endParaRPr" if in_paragraph => {
            *para_font = Font::new();
            parse_run_props_attrs(e, para_font)?;
        }
        "srgbClr" if rpr_solid_fill_depth.is_some() => {
            if let Some(val) = attr_value(e, b"val")? {
                if let Ok(rgb) = RgbColor::from_hex(&val) {
                    current_font.color = Some(rgb);
                }
            }
        }
        "schemeClr" if rpr_solid_fill_depth.is_some() => {
            if let Some(val) = attr_value(e, b"val")? {
                if let Some(tc) = MsoThemeColorIndex::from_xml_str(&val) {
                    current_font.fill = Some(FillFormat::solid(ColorFormat::Theme(ThemeColor {
                        theme_color: tc,
                        brightness: None,
                    })));
                }
            }
        }
        "schemeClr" if para_rpr_solid_fill_depth.is_some() => {
            if let Some(val) = attr_value(e, b"val")? {
                if let Some(tc) = MsoThemeColorIndex::from_xml_str(&val) {
                    para_font.fill = Some(FillFormat::solid(ColorFormat::Theme(ThemeColor {
                        theme_color: tc,
                        brightness: None,
                    })));
                }
            }
        }
        "srgbClr" if para_rpr_solid_fill_depth.is_some() => {
            if let Some(val) = attr_value(e, b"val")? {
                if let Ok(rgb) = RgbColor::from_hex(&val) {
                    para_font.color = Some(rgb);
                }
            }
        }
        "latin" if in_run_props => {
            if let Some(typeface) = attr_value(e, b"typeface")? {
                current_font.name = Some(typeface.into_owned());
            }
        }
        "latin" if in_end_para_rpr || in_para_props => {
            if let Some(typeface) = attr_value(e, b"typeface")? {
                para_font.name = Some(typeface.into_owned());
            }
        }
        "ea" if in_run_props => {
            if let Some(typeface) = attr_value(e, b"typeface")? {
                current_font.east_asia_name = Some(typeface.into_owned());
            }
        }
        "ea" if in_end_para_rpr || in_para_props => {
            if let Some(typeface) = attr_value(e, b"typeface")? {
                para_font.east_asia_name = Some(typeface.into_owned());
            }
        }
        "cs" if in_run_props => {
            if let Some(typeface) = attr_value(e, b"typeface")? {
                current_font.complex_script_name = Some(typeface.into_owned());
            }
        }
        "cs" if in_end_para_rpr || in_para_props => {
            if let Some(typeface) = attr_value(e, b"typeface")? {
                para_font.complex_script_name = Some(typeface.into_owned());
            }
        }
        "sym" if in_run_props => {
            if let Some(typeface) = attr_value(e, b"typeface")? {
                current_font.symbol_name = Some(typeface.into_owned());
            }
        }
        "sym" if in_end_para_rpr || in_para_props => {
            if let Some(typeface) = attr_value(e, b"typeface")? {
                para_font.symbol_name = Some(typeface.into_owned());
            }
        }
        "br" if in_paragraph => {
            let mut br_run = Run::new();
            br_run.is_line_break = true;
            current_para.runs.push(br_run);
        }
        _ => {}
    }
    Ok(())
}

/// Handle an end XML element within the text frame parser.
#[allow(clippy::too_many_arguments)]
fn handle_end_element(
    e: &quick_xml::events::BytesEnd<'_>,
    in_body_pr: &mut bool,
    in_paragraph: &mut bool,
    in_para_props: &mut bool,
    in_run: &mut bool,
    in_run_props: &mut bool,
    in_end_para_rpr: &mut bool,
    in_t: &mut bool,
    rpr_solid_fill_depth: &mut Option<u32>,
    para_rpr_solid_fill_depth: &mut Option<u32>,
    depth: u32,
    tf: &mut TextFrame,
    current_para: &mut Paragraph,
    current_run: &mut Run,
    current_font: &mut Font,
    para_font: &mut Font,
) {
    let qn = e.name();
    let local = local_name_str(qn.as_ref());
    match local {
        "bodyPr" => {
            *in_body_pr = false;
        }
        "p" => {
            if *in_paragraph {
                if has_font_properties(para_font) {
                    current_para.font = Some(mem::take(para_font));
                } else {
                    *para_font = Font::new();
                }
                tf.paragraphs.push(mem::take(current_para));
                *in_paragraph = false;
            }
        }
        "pPr" => {
            *in_para_props = false;
        }
        "defRPr" if *in_para_props => {
            if has_font_properties(para_font) {
                current_para.font = Some(para_font.clone());
            }
        }
        "r" => {
            if *in_run {
                if has_font_properties(current_font) {
                    *current_run.font_mut() = mem::take(current_font);
                } else {
                    *current_font = Font::new();
                }
                current_para.runs.push(mem::take(current_run));
                *in_run = false;
                *in_run_props = false;
            }
        }
        "rPr" => {
            *in_run_props = false;
        }
        "endParaRPr" => {
            *in_end_para_rpr = false;
        }
        "solidFill" => {
            if *rpr_solid_fill_depth == Some(depth) {
                *rpr_solid_fill_depth = None;
            }
            if *para_rpr_solid_fill_depth == Some(depth) {
                *para_rpr_solid_fill_depth = None;
            }
        }
        "t" => {
            *in_t = false;
        }
        _ => {}
    }
}
