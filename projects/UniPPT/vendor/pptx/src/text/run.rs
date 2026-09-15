//! Run type for text runs within paragraphs.

use crate::shapes::action::Hyperlink;
use crate::text::font::Font;
use crate::xml_util::xml_escape;

/// A run of text with uniform character formatting, corresponding to `<a:r>`.
///
/// When `is_line_break` is `true`, the run is serialized as `<a:br>` instead
/// of `<a:r>`, and its text content is ignored.
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    text: String,
    font: Font,
    /// Hyperlink for this run. When present, adds `<a:hlinkClick>` to the run properties.
    pub hyperlink: Option<Hyperlink>,
    /// When `true`, this run represents a line break (`<a:br>`) rather than
    /// a normal text run (`<a:r>`).
    pub is_line_break: bool,
}

impl Default for Run {
    fn default() -> Self {
        Self {
            text: String::new(),
            font: Font::new(),
            hyperlink: None,
            is_line_break: false,
        }
    }
}

impl Run {
    /// Create a new empty run.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the text content of this run.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Set the text content of this run.
    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
    }

    /// Return a reference to the font properties for this run.
    #[must_use]
    pub const fn font(&self) -> &Font {
        &self.font
    }

    /// Return a mutable reference to the font properties for this run.
    pub fn font_mut(&mut self) -> &mut Font {
        &mut self.font
    }

    /// Set the language identifier for this run (e.g. `"ar-SA"`, `"he-IL"`).
    ///
    /// This sets the `lang` attribute on `<a:rPr>`, which is important for
    /// correct rendering of Arabic, Hebrew, and other scripts.
    pub fn set_language(&mut self, lang: &str) {
        self.font.language_id = Some(lang.to_string());
    }

    /// Set a hyperlink on this run.
    pub fn set_hyperlink(&mut self, hyperlink: Hyperlink) {
        self.hyperlink = Some(hyperlink);
    }

    /// Generate the `<a:r>` (or `<a:br>` for line breaks) XML element string.
    #[must_use]
    pub fn to_xml_string(&self) -> String {
        if self.is_line_break {
            // Line break element: <a:br><a:rPr .../></a:br>
            let mut xml = String::from("<a:br>");
            xml.push_str(&self.font.to_xml_string());
            xml.push_str("</a:br>");
            return xml;
        }

        let mut xml = String::from("<a:r>");

        // Generate rPr, potentially with hlinkClick
        if let Some(ref hlink) = self.hyperlink {
            // We need to emit hlinkClick inside rPr.
            // Get the font XML and inject the hlinkClick before the closing tag.
            let font_xml = self.font.to_xml_string();
            if font_xml.ends_with("/>") {
                // Self-closing <a:rPr .../> — convert to open/close with hlinkClick child
                let prefix = &font_xml[..font_xml.len() - 2];
                xml.push_str(prefix);
                xml.push('>');
                xml.push_str(&hyperlink_to_xml(hlink));
                xml.push_str("</a:rPr>");
            } else {
                // Already has children </a:rPr> — insert hlinkClick before </a:rPr>
                let close_tag = "</a:rPr>";
                if let Some(pos) = font_xml.rfind(close_tag) {
                    xml.push_str(&font_xml[..pos]);
                    xml.push_str(&hyperlink_to_xml(hlink));
                    xml.push_str(close_tag);
                } else {
                    xml.push_str(&font_xml);
                }
            }
        } else {
            xml.push_str(&self.font.to_xml_string());
        }

        xml.push_str("<a:t>");
        xml.push_str(&xml_escape(&self.text));
        xml.push_str("</a:t>");
        xml.push_str("</a:r>");
        xml
    }
}

/// Generate the `<a:hlinkClick>` XML for a hyperlink within a run.
fn hyperlink_to_xml(hlink: &Hyperlink) -> String {
    let mut xml = String::from("<a:hlinkClick");
    if let Some(ref rid) = hlink.r_id {
        xml.push_str(&format!(r#" r:id="{}""#, xml_escape(rid.as_str())));
    }
    if let Some(ref tooltip) = hlink.tooltip {
        xml.push_str(&format!(r#" tooltip="{}""#, xml_escape(tooltip)));
    }
    xml.push_str("/>");
    xml
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
