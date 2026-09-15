//! PPTX to HTML export.
//!
//! Produces a self-contained HTML string with inline CSS. Each slide becomes
//! a `<div class="slide">` element.

mod render;
mod utils;

use crate::error::{PptxError, PptxResult};
use crate::presentation::Presentation;
use crate::shapes::ShapeTree;
use crate::units::Emu;
use utils::emu_to_px;

/// Converts a `Presentation` into a self-contained HTML document.
pub struct HtmlExporter<'a> {
    prs: &'a Presentation,
}

impl<'a> HtmlExporter<'a> {
    /// Create a new exporter for the given presentation.
    #[must_use]
    pub const fn new(prs: &'a Presentation) -> Self {
        Self { prs }
    }

    /// Export the presentation as a complete HTML document string.
    ///
    /// # Errors
    ///
    /// Returns an error if slide XML cannot be read or parsed.
    #[allow(clippy::similar_names)]
    pub fn export(&self) -> PptxResult<String> {
        let slides = self.prs.slides()?;
        let slide_size = self.prs.slide_size()?.unwrap_or((9_144_000, 6_858_000));
        let slide_w_px = emu_to_px(Emu(slide_size.0));
        let slide_h_px = emu_to_px(Emu(slide_size.1));

        let mut html = String::with_capacity(4096);

        render::write_doc_header(&mut html, slide_w_px, slide_h_px)
            .map_err(|e| PptxError::InvalidXml(e.to_string()))?;

        for (idx, slide_ref) in slides.iter().enumerate() {
            let slide_xml = self.prs.slide_xml(slide_ref)?;
            let tree = ShapeTree::from_slide_xml(slide_xml)?;
            let slide_name = self.prs.slide_name(slide_ref)?.unwrap_or_default();
            render::write_slide(&mut html, idx, &slide_name, &tree, slide_w_px, slide_h_px)
                .map_err(|e| PptxError::InvalidXml(e.to_string()))?;
        }

        render::write_doc_footer(&mut html).map_err(|e| PptxError::InvalidXml(e.to_string()))?;

        Ok(html)
    }
}

/// Standalone function to export a presentation to HTML.
///
/// # Errors
///
/// Returns an error if slide XML cannot be read or parsed.
pub fn export_to_html(prs: &Presentation) -> PptxResult<String> {
    HtmlExporter::new(prs).export()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presentation::Presentation;
    use crate::shapes::ShapeTree;
    use crate::text::font::RgbColor;
    use crate::units::Emu;
    use utils::{base64_encode, html_escape};

    #[test]
    fn html_escape_special_chars() {
        assert_eq!(html_escape("<b>A&B</b>"), "&lt;b&gt;A&amp;B&lt;/b&gt;");
        assert_eq!(html_escape("\"it's\""), "&quot;it&#39;s&quot;");
    }

    #[test]
    fn base64_encode_empty() {
        assert_eq!(base64_encode(b""), "");
    }

    #[test]
    fn base64_encode_basic() {
        assert_eq!(base64_encode(b"Hello"), "SGVsbG8=");
        assert_eq!(base64_encode(b"Hi"), "SGk=");
        assert_eq!(base64_encode(b"abc"), "YWJj");
    }

    #[test]
    fn export_empty_presentation() {
        let prs = Presentation::new().unwrap();
        let html = HtmlExporter::new(&prs).export().unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("</html>"));
        assert!(html.contains("<style>"));
    }

    #[test]
    fn export_presentation_with_slide() {
        let mut prs = Presentation::new().unwrap();
        let layouts = prs.slide_layouts().unwrap();
        let layout = layouts.first().unwrap();
        let slide_ref = prs.add_slide(layout).unwrap();

        // Add a textbox with some text
        let slide_xml = prs.slide_xml(&slide_ref).unwrap().to_vec();
        let updated = ShapeTree::add_textbox(
            &slide_xml,
            Emu(914400),
            Emu(914400),
            Emu(3657600),
            Emu(457200),
        )
        .unwrap();
        *prs.slide_xml_mut(&slide_ref).unwrap() = updated;

        let html = HtmlExporter::new(&prs).export().unwrap();
        assert!(html.contains("class=\"slide\""));
        assert!(html.contains("class=\"slide-number\">1</div>"));
    }

    #[test]
    fn export_standalone_function() {
        let prs = Presentation::new().unwrap();
        let html = export_to_html(&prs).unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
    }

    #[test]
    fn write_font_css_produces_styles() {
        let mut font = crate::text::font::Font::new();
        font.bold = Some(true);
        font.italic = Some(true);
        font.size = Some(24.0);
        font.color = Some(RgbColor::new(255, 0, 0));
        font.name = Some("Arial".to_string());
        font.east_asia_name = Some("Microsoft YaHei".to_string());

        let mut run = crate::text::Run::new();
        run.set_text("styled");
        *run.font_mut() = font;

        let mut css = String::new();
        render::write_run(&mut css, &run).unwrap();
        assert!(css.contains("font-weight:bold;"));
        assert!(css.contains("font-style:italic;"));
        assert!(css.contains("font-size:24pt;"));
        assert!(css.contains("color:#FF0000;"));
        assert!(css.contains("font-family:'Arial'"));
        assert!(css.contains("'Microsoft YaHei',sans-serif"));
    }

    #[test]
    fn write_run_with_formatting() {
        let mut run = crate::text::Run::new();
        run.set_text("Bold text");
        run.font_mut().bold = Some(true);

        let mut html = String::new();
        render::write_run(&mut html, &run).unwrap();
        assert!(html.contains("<span"));
        assert!(html.contains("font-weight:bold"));
        assert!(html.contains("Bold text"));
        assert!(html.contains("</span>"));
    }

    #[test]
    fn write_run_without_formatting() {
        let mut run = crate::text::Run::new();
        run.set_text("Plain text");

        let mut html = String::new();
        render::write_run(&mut html, &run).unwrap();
        assert_eq!(html, "Plain text");
    }

    #[test]
    fn emu_to_px_conversion() {
        // 1 inch = 914400 EMU = 96 px
        let px = emu_to_px(Emu(914400));
        assert!((px - 96.0).abs() < 0.01);
    }

    #[test]
    fn export_presentation_method() {
        let prs = Presentation::new().unwrap();
        let html = prs.export_html().unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Presentation"));
    }
}
