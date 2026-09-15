//! `TextFrame` type for shape text bodies.

use std::fmt;

use crate::enums::text::{MsoAutoSize, MsoVerticalAnchor};
use crate::text::paragraph::Paragraph;
use crate::units::Emu;
use crate::xml_util::xml_escape;
use crate::WriteXml;

/// A text body within a shape, corresponding to the `<p:txBody>` element.
///
/// Contains one or more paragraphs and body-level properties such as
/// word wrap, auto size, and margins (in EMU).
#[derive(Debug, Clone, PartialEq)]
pub struct TextFrame {
    pub(crate) paragraphs: Vec<Paragraph>,
    /// Whether text wraps at the shape boundary.
    pub word_wrap: bool,
    /// Automatic sizing behavior.
    pub auto_size: MsoAutoSize,
    /// Left inset in EMU. Default is 91440 (0.1 inch).
    pub margin_left: Option<Emu>,
    /// Right inset in EMU. Default is 91440 (0.1 inch).
    pub margin_right: Option<Emu>,
    /// Top inset in EMU. Default is 45720 (0.05 inch).
    pub margin_top: Option<Emu>,
    /// Bottom inset in EMU. Default is 45720 (0.05 inch).
    pub margin_bottom: Option<Emu>,
    /// Vertical anchor for text within the frame.
    pub vertical_anchor: Option<MsoVerticalAnchor>,
    /// DrawingML vertical text mode from `a:bodyPr/@vert` (for example
    /// `eaVert` or `vert270`).  The raw OOXML token is retained because the
    /// Office modes have distinct East-Asian glyph-orientation semantics.
    pub vertical_type: Option<String>,
    /// Text rotation in degrees. Stored as 60000ths of a degree in XML.
    pub rotation: Option<f64>,
    /// Font scale for normAutofit, as a percentage (e.g. 80.0 for 80%).
    /// When set along with `auto_size = TextToFitShape`, emits `<a:normAutofit fontScale="X"/>`.
    pub font_scale: Option<f64>,
}

impl Default for TextFrame {
    fn default() -> Self {
        Self {
            paragraphs: vec![Paragraph::new()],
            word_wrap: true,
            auto_size: MsoAutoSize::None,
            margin_left: Some(Emu(91440)),
            margin_right: Some(Emu(91440)),
            margin_top: Some(Emu(45720)),
            margin_bottom: Some(Emu(45720)),
            vertical_anchor: None,
            vertical_type: None,
            rotation: None,
            font_scale: None,
        }
    }
}

impl TextFrame {
    /// Create a new `TextFrame` with a single empty paragraph.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return all text in the text frame as a single string.
    ///
    /// Paragraphs are separated by newline characters (`\n`).
    pub fn text(&self) -> String {
        self.paragraphs
            .iter()
            .map(super::paragraph::Paragraph::text)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Replace all content with a single paragraph containing the given text.
    ///
    /// If the text contains newline characters, each line becomes a separate
    /// paragraph.
    pub fn set_text(&mut self, text: &str) {
        self.paragraphs.clear();
        for line in text.split('\n') {
            let mut para = Paragraph::new();
            if !line.is_empty() {
                let run = para.add_run();
                run.set_text(line);
            }
            self.paragraphs.push(para);
        }
    }

    /// Return a reference to the paragraphs in this text frame.
    #[must_use]
    pub fn paragraphs(&self) -> &[Paragraph] {
        &self.paragraphs
    }

    /// Return a mutable reference to the paragraphs in this text frame.
    pub fn paragraphs_mut(&mut self) -> &mut [Paragraph] {
        &mut self.paragraphs
    }

    /// Append a new empty paragraph and return a mutable reference to it.
    pub fn add_paragraph(&mut self) -> &mut Paragraph {
        self.paragraphs.push(Paragraph::new());
        let last = self.paragraphs.len() - 1;
        &mut self.paragraphs[last]
    }

    /// Remove all paragraphs, leaving a single empty paragraph.
    pub fn clear(&mut self) {
        self.paragraphs.clear();
        self.paragraphs.push(Paragraph::new());
    }

    /// Configure the text frame for "fit text" mode.
    ///
    /// Sets the auto-size to `TextToFitShape` and records a font scale percentage.
    /// In the resulting XML this produces `<a:normAutofit fontScale="X"/>` inside
    /// `<a:bodyPr>`.
    ///
    /// `font_scale_pct` is in percent (e.g. 80.0 for 80%).  If `None`, the
    /// font scale attribute is omitted and `PowerPoint` will compute it.
    pub fn fit_text(&mut self, font_scale_pct: Option<f64>) {
        self.auto_size = MsoAutoSize::TextToFitShape;
        self.font_scale = font_scale_pct;
    }

    /// Write the text body XML using a custom body tag name.
    ///
    /// For shapes on a slide, `body_tag` is typically `"p:txBody"`.
    /// For table cells, it should be `"a:txBody"`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the writer fails.
    pub fn write_xml_with_tag<W: fmt::Write>(&self, w: &mut W, body_tag: &str) -> fmt::Result {
        write!(w, "<{body_tag}>")?;

        // <a:bodyPr>
        w.write_str("<a:bodyPr")?;

        let wrap = if self.word_wrap { "square" } else { "none" };
        write!(w, r#" wrap="{wrap}""#)?;

        if let Some(l) = self.margin_left {
            write!(w, r#" lIns="{l}""#)?;
        }
        if let Some(t) = self.margin_top {
            write!(w, r#" tIns="{t}""#)?;
        }
        if let Some(r) = self.margin_right {
            write!(w, r#" rIns="{r}""#)?;
        }
        if let Some(b) = self.margin_bottom {
            write!(w, r#" bIns="{b}""#)?;
        }

        if let Some(rotation) = self.rotation {
            // OOXML stores rotation in 60000ths of a degree
            #[allow(clippy::cast_possible_truncation)] // intentional f64→i64 for OOXML units
            let rot = (rotation * 60000.0) as i64;
            write!(w, r#" rot="{rot}""#)?;
        }

        if let Some(anchor) = self.vertical_anchor {
            write!(w, r#" anchor="{}""#, anchor.to_xml_str())?;
        }

        if let Some(vertical_type) = self
            .vertical_type
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            write!(w, r#" vert="{}""#, xml_escape(vertical_type))?;
        }

        // Determine if bodyPr needs child elements
        let has_autofit_child = self.auto_size != MsoAutoSize::None;

        if has_autofit_child {
            w.write_char('>')?;
            match self.auto_size {
                MsoAutoSize::TextToFitShape => {
                    w.write_str("<a:normAutofit")?;
                    if let Some(scale) = self.font_scale {
                        // fontScale is in 1000ths of a percent (e.g. 80% = 80000)
                        #[allow(clippy::cast_possible_truncation)]
                        // intentional f64→i64 for OOXML units
                        let val = (scale * 1000.0) as i64;
                        write!(w, r#" fontScale="{val}""#)?;
                    }
                    w.write_str("/>")?;
                }
                MsoAutoSize::ShapeToFitText => {
                    w.write_str("<a:spAutoFit/>")?;
                }
                MsoAutoSize::None => {}
            }
            w.write_str("</a:bodyPr>")?;
        } else {
            w.write_str("/>")?;
        }

        // <a:lstStyle/>
        w.write_str("<a:lstStyle/>")?;

        // <a:p> elements
        for para in &self.paragraphs {
            w.write_str(&para.to_xml_string())?;
        }

        write!(w, "</{body_tag}>")
    }

    /// Generate the `<p:txBody>` XML element string.
    #[must_use]
    pub fn to_xml_string(&self) -> String {
        let mut xml = String::with_capacity(256);
        // fmt::Write for String is infallible; the result is intentionally ignored.
        let _ = self.write_xml_with_tag(&mut xml, "p:txBody");
        xml
    }
}

impl WriteXml for TextFrame {
    fn write_xml<W: fmt::Write>(&self, w: &mut W) -> fmt::Result {
        self.write_xml_with_tag(w, "p:txBody")
    }
}

#[cfg(test)]
#[path = "text_frame_tests.rs"]
mod tests;
