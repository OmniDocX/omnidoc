//! HTML rendering helpers for slides and shapes.

use std::fmt::{self, Write};

use super::utils::{base64_encode, emu_to_px, html_escape};
use crate::shapes::{Shape, ShapeTree};

pub(super) fn write_doc_header(w: &mut String, slide_w: f64, slide_h: f64) -> fmt::Result {
    w.write_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n")?;
    w.write_str("<meta charset=\"utf-8\">\n")?;
    w.write_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n")?;
    w.write_str("<title>Presentation</title>\n")?;
    w.write_str("<style>\n")?;
    w.write_str(
        "body { font-family: Calibri, Arial, sans-serif; background: #f0f0f0; margin: 20px; }\n",
    )?;
    writeln!(
        w,
        ".slide {{ position: relative; width: {slide_w:.0}px; height: {slide_h:.0}px; \
         background: #fff; margin: 20px auto; overflow: hidden; \
         box-shadow: 0 2px 8px rgba(0,0,0,0.2); }}"
    )?;
    w.write_str(
        ".slide-number { position: absolute; bottom: 8px; right: 12px; \
         font-size: 12px; color: #999; }\n",
    )?;
    w.write_str(".shape { position: absolute; box-sizing: border-box; overflow: hidden; }\n")?;
    w.write_str("table.pptx-table { border-collapse: collapse; width: 100%; height: 100%; }\n")?;
    w.write_str(
        "table.pptx-table td, table.pptx-table th { border: 1px solid #bbb; padding: 4px 6px; \
         vertical-align: top; font-size: 14px; }\n",
    )?;
    w.write_str("img.shape-img { width: 100%; height: 100%; object-fit: contain; }\n")?;
    w.write_str("</style>\n")?;
    w.write_str("</head>\n<body>\n")
}

pub(super) fn write_doc_footer(w: &mut String) -> fmt::Result {
    w.write_str("</body>\n</html>\n")
}

pub(super) fn write_slide(
    w: &mut String,
    index: usize,
    _name: &str,
    tree: &ShapeTree,
    slide_w: f64,
    slide_h: f64,
) -> fmt::Result {
    writeln!(
        w,
        "<div class=\"slide\" style=\"width:{slide_w:.0}px;height:{slide_h:.0}px;\">"
    )?;

    for shape in tree.iter() {
        write_shape(w, shape)?;
    }

    // Slide number
    writeln!(w, "<div class=\"slide-number\">{}</div>", index + 1)?;
    w.write_str("</div>\n")
}

fn write_shape(w: &mut String, shape: &Shape) -> fmt::Result {
    let left = emu_to_px(shape.left());
    let top = emu_to_px(shape.top());
    let width = emu_to_px(shape.width());
    let height = emu_to_px(shape.height());

    write!(
        w,
        "<div class=\"shape\" title=\"{}\" style=\"left:{left:.1}px;top:{top:.1}px;\
         width:{width:.1}px;height:{height:.1}px;\">",
        html_escape(shape.name()),
    )?;

    match shape {
        Shape::AutoShape(auto) => {
            if let Some(ref tf) = auto.text_frame {
                write_text_frame(w, tf)?;
            }
        }
        Shape::Picture(pic) => {
            write_picture(w, pic)?;
        }
        Shape::GraphicFrame(gf) => {
            if gf.has_table {
                // We cannot access table data from GraphicFrame directly
                // (it only stores has_table flag). Show a placeholder.
                write!(
                    w,
                    "<div style=\"padding:8px;font-style:italic;color:#888;\">\
                     [Table: {}]</div>",
                    html_escape(&gf.name)
                )?;
            }
        }
        Shape::GroupShape(group) => {
            for child in &group.shapes {
                write_shape(w, child)?;
            }
        }
        Shape::Connector(_) | Shape::OleObject(_) => {
            // Connectors and OLE objects are not visually rendered in HTML
        }
    }

    w.write_str("</div>\n")
}

fn write_text_frame(w: &mut String, tf: &crate::text::TextFrame) -> fmt::Result {
    w.write_str("<div style=\"padding:4px;\">")?;
    for para in tf.paragraphs() {
        write_paragraph(w, para)?;
    }
    w.write_str("</div>")
}

fn write_paragraph(w: &mut String, para: &crate::text::Paragraph) -> fmt::Result {
    w.write_str("<p style=\"margin:0 0 2px 0;")?;
    if let Some(align) = para.alignment {
        let (align_str, distribute_last_line) = match align {
            crate::enums::text::PpParagraphAlignment::Left => ("left", false),
            crate::enums::text::PpParagraphAlignment::Center => ("center", false),
            crate::enums::text::PpParagraphAlignment::Right => ("right", false),
            crate::enums::text::PpParagraphAlignment::Justify
            | crate::enums::text::PpParagraphAlignment::JustifyLow => ("justify", false),
            crate::enums::text::PpParagraphAlignment::Distribute
            | crate::enums::text::PpParagraphAlignment::ThaiDistribute => ("justify", true),
        };
        write!(w, "text-align:{align_str};")?;
        if distribute_last_line {
            w.write_str("text-align-last:justify;")?;
        }
    }
    if para.level > 0 {
        write!(w, "margin-left:{}px;", u32::from(para.level) * 24)?;
    }
    w.write_str("\">")?;

    if para.runs().is_empty() {
        // Empty paragraph -> line break equivalent
        w.write_str("&nbsp;")?;
    } else {
        for run in para.runs() {
            if run.is_line_break {
                w.write_str("<br>")?;
            } else {
                write_run(w, run)?;
            }
        }
    }

    w.write_str("</p>")
}

pub(super) fn write_run(w: &mut String, run: &crate::text::Run) -> fmt::Result {
    let font = run.font();
    let has_style = font.bold.is_some()
        || font.italic.is_some()
        || font.size.is_some()
        || font.color.is_some()
        || font.underline.is_some()
        || font.strikethrough.is_some()
        || font.name.is_some()
        || font.east_asia_name.is_some()
        || font.complex_script_name.is_some()
        || font.symbol_name.is_some();

    if has_style {
        w.write_str("<span style=\"")?;
        write_font_css(w, font)?;
        w.write_str("\">")?;
    }

    let text = run.text();
    if !text.is_empty() {
        w.write_str(&html_escape(text))?;
    }

    if has_style {
        w.write_str("</span>")?;
    }

    Ok(())
}

fn write_font_css(w: &mut String, font: &crate::text::font::Font) -> fmt::Result {
    let mut families = Vec::new();
    for name in [
        font.name.as_deref(),
        font.east_asia_name.as_deref(),
        font.complex_script_name.as_deref(),
        font.symbol_name.as_deref(),
    ]
    .into_iter()
    .flatten()
    .map(str::trim)
    .filter(|name| !name.is_empty() && !name.starts_with('+'))
    {
        if !families
            .iter()
            .any(|existing: &&str| existing.eq_ignore_ascii_case(name))
        {
            families.push(name);
        }
    }
    if !families.is_empty() {
        w.write_str("font-family:")?;
        for family in families {
            write!(w, "'{}',", html_escape(family))?;
        }
        w.write_str("sans-serif;")?;
    }
    if let Some(size) = font.size {
        write!(w, "font-size:{size:.0}pt;")?;
    }
    if font.bold == Some(true) {
        w.write_str("font-weight:bold;")?;
    }
    if font.italic == Some(true) {
        w.write_str("font-style:italic;")?;
    }
    if let Some(underline) = font.underline {
        if underline != crate::enums::text::MsoTextUnderlineType::None {
            w.write_str("text-decoration:underline;")?;
        }
    }
    if font.strikethrough == Some(true) {
        w.write_str("text-decoration:line-through;")?;
    }
    if let Some(ref color) = font.color {
        write!(w, "color:#{};", color.to_hex())?;
    }
    Ok(())
}

fn write_picture(w: &mut String, pic: &crate::shapes::Picture) -> fmt::Result {
    if let (Some(data), Some(ct)) = (&pic.image_data, &pic.image_content_type) {
        let b64 = base64_encode(data);
        write!(
            w,
            "<img class=\"shape-img\" src=\"data:{ct};base64,{b64}\" alt=\"{}\">",
            html_escape(pic.description.as_deref().unwrap_or(&pic.name)),
        )?;
    } else {
        // No cached image data available; show placeholder
        write!(
            w,
            "<div style=\"width:100%;height:100%;background:#eee;display:flex;\
             align-items:center;justify-content:center;font-size:12px;color:#888;\">\
             [Image: {}]</div>",
            html_escape(&pic.name),
        )?;
    }
    Ok(())
}
