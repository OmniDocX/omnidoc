//! Paragraph type for text paragraphs within a text frame.

use crate::dml::color::ColorFormat;
use crate::enums::text::{PpParagraphAlignment, TextDirection};
use crate::text::bullet::BulletFormat;
use crate::text::font::Font;
use crate::text::run::Run;
use crate::xml_util::{xml_escape, xml_escape_char};
use crate::WriteXml;

/// A paragraph within a text frame, corresponding to the `<a:p>` element.
///
/// Contains runs of text and paragraph-level formatting such as alignment
/// and indent level.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Paragraph {
    pub(crate) runs: Vec<Run>,
    /// Horizontal alignment.
    pub alignment: Option<PpParagraphAlignment>,
    /// Indent level (0-8).
    pub level: u8,
    /// Space before the paragraph, in points.
    pub space_before: Option<f64>,
    /// Space after the paragraph, in points.
    pub space_after: Option<f64>,
    /// Line spacing multiplier (e.g. 1.5 for 150% line spacing).
    pub line_spacing: Option<f64>,
    /// Bullet format for this paragraph.
    pub bullet: Option<BulletFormat>,
    /// Bullet color (`<a:buClr>`).
    pub bullet_color: Option<ColorFormat>,
    /// Bullet font typeface (`<a:buFont>`).
    pub bullet_font: Option<String>,
    /// Bullet size as a percentage of the text size (`<a:buSzPct>`, e.g. 100.0 for 100%).
    pub bullet_size_pct: Option<f64>,
    /// Bullet size in points (`<a:buSzPts>`, e.g. 12.0 for 12pt).
    pub bullet_size_pts: Option<f64>,
    /// Default run properties for this paragraph (`<a:defRPr>`).
    pub font: Option<Font>,
    /// Text direction (LTR or RTL) for this paragraph.
    pub text_direction: Option<TextDirection>,
}

impl Paragraph {
    /// Create a new empty paragraph.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the concatenated text of all runs in this paragraph.
    ///
    /// Line-break runs contribute a `'\n'` character to the output.
    #[must_use]
    pub fn text(&self) -> String {
        self.runs
            .iter()
            .map(|r| if r.is_line_break { "\n" } else { r.text() })
            .collect()
    }

    /// Return a reference to the runs in this paragraph.
    #[must_use]
    pub fn runs(&self) -> &[Run] {
        &self.runs
    }

    /// Return a mutable reference to the runs in this paragraph.
    pub fn runs_mut(&mut self) -> &mut [Run] {
        &mut self.runs
    }

    /// Append a new empty run and return a mutable reference to it.
    ///
    /// # Panics
    ///
    /// Panics if the internal runs vector is empty after push (should never happen).
    pub fn add_run(&mut self) -> &mut Run {
        self.runs.push(Run::new());
        let idx = self.runs.len() - 1;
        &mut self.runs[idx]
    }

    /// Append a line break element (`<a:br>`) to this paragraph.
    ///
    /// The returned mutable reference allows setting font properties on the
    /// line break (which controls the height of the blank line).  The font
    /// defaults to the paragraph's default font when available.
    ///
    /// # Panics
    ///
    /// Panics if the internal runs vector is empty after push (should never happen).
    pub fn add_line_break(&mut self) -> &mut Run {
        let mut run = Run::new();
        run.is_line_break = true;
        if let Some(ref default_font) = self.font {
            *run.font_mut() = default_font.clone();
        }
        self.runs.push(run);
        let idx = self.runs.len() - 1;
        &mut self.runs[idx]
    }

    /// Set the horizontal alignment of this paragraph.
    pub fn set_alignment(&mut self, align: PpParagraphAlignment) {
        self.alignment = Some(align);
    }

    /// Set the text direction (LTR or RTL) of this paragraph.
    pub fn set_text_direction(&mut self, direction: TextDirection) {
        self.text_direction = Some(direction);
    }

    /// Remove all runs from this paragraph (keeps paragraph properties).
    pub fn clear(&mut self) {
        self.runs.clear();
    }

    /// Set the bullet format for this paragraph.
    pub fn set_bullet(&mut self, bullet: BulletFormat) {
        self.bullet = Some(bullet);
    }

    /// Generate the `<a:p>` XML element string.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn to_xml_string(&self) -> String {
        let mut xml = String::from("<a:p>");

        // <a:pPr> - only emit if there are properties to set
        let has_ppr = self.alignment.is_some()
            || self.level > 0
            || self.text_direction.is_some()
            || self.space_before.is_some()
            || self.space_after.is_some()
            || self.line_spacing.is_some()
            || self.bullet.is_some()
            || self.bullet_color.is_some()
            || self.bullet_font.is_some()
            || self.bullet_size_pct.is_some()
            || self.bullet_size_pts.is_some()
            || self.font.is_some();

        if has_ppr {
            xml.push_str("<a:pPr");

            if let Some(align) = self.alignment {
                xml.push_str(&format!(r#" algn="{}""#, align.to_xml_str()));
            }

            if self.level > 0 {
                xml.push_str(&format!(r#" lvl="{}""#, self.level));
            }

            if let Some(dir) = self.text_direction {
                xml.push_str(&format!(r#" rtl="{}""#, dir.to_xml_attr()));
            }

            let has_children = self.space_before.is_some()
                || self.space_after.is_some()
                || self.line_spacing.is_some()
                || self.bullet.is_some()
                || self.bullet_color.is_some()
                || self.bullet_font.is_some()
                || self.bullet_size_pct.is_some()
                || self.bullet_size_pts.is_some()
                || self.font.is_some();

            if has_children {
                xml.push('>');

                if let Some(spacing) = self.line_spacing {
                    // Line spacing as percentage (e.g. 1.5 -> 150000)
                    #[allow(clippy::cast_possible_truncation)]
                    // intentional f64→i64 for OOXML units
                    let val = (spacing * 100_000.0) as i64;
                    xml.push_str(&format!(r#"<a:lnSpc><a:spcPct val="{val}"/></a:lnSpc>"#));
                }

                if let Some(before) = self.space_before {
                    // Space before in hundredths of a point
                    #[allow(clippy::cast_possible_truncation)]
                    // intentional f64→i64 for OOXML units
                    let val = (before * 100.0) as i64;
                    xml.push_str(&format!(r#"<a:spcBef><a:spcPts val="{val}"/></a:spcBef>"#));
                }

                if let Some(after) = self.space_after {
                    // Space after in hundredths of a point
                    #[allow(clippy::cast_possible_truncation)]
                    // intentional f64→i64 for OOXML units
                    let val = (after * 100.0) as i64;
                    xml.push_str(&format!(r#"<a:spcAft><a:spcPts val="{val}"/></a:spcAft>"#));
                }

                // Bullet color
                if let Some(ref color) = self.bullet_color {
                    xml.push_str(&format!("<a:buClr>{}</a:buClr>", color.to_xml_string()));
                }

                // Bullet size
                if let Some(pct) = self.bullet_size_pct {
                    #[allow(clippy::cast_possible_truncation)]
                    // intentional f64→i64 for OOXML units
                    let val = (pct * 1000.0) as i64;
                    xml.push_str(&format!(r#"<a:buSzPct val="{val}"/>"#));
                } else if let Some(pts) = self.bullet_size_pts {
                    #[allow(clippy::cast_possible_truncation)]
                    // intentional f64→i64 for OOXML units
                    let val = (pts * 100.0) as i64;
                    xml.push_str(&format!(r#"<a:buSzPts val="{val}"/>"#));
                }

                // Bullet font
                if let Some(ref font_name) = self.bullet_font {
                    xml.push_str(&format!(
                        r#"<a:buFont typeface="{}"/>"#,
                        xml_escape(font_name)
                    ));
                }

                // Bullet format
                if let Some(ref bullet) = self.bullet {
                    match bullet {
                        BulletFormat::Character(ch) => {
                            xml.push_str(&format!(
                                r#"<a:buChar char="{}"/>"#,
                                xml_escape_char(*ch)
                            ));
                        }
                        BulletFormat::AutoNumbered(numbering_type) => {
                            xml.push_str(&format!(
                                r#"<a:buAutoNum type="{}"/>"#,
                                xml_escape(numbering_type)
                            ));
                        }
                        BulletFormat::Picture(r_id) => {
                            xml.push_str(&format!(
                                r#"<a:buBlip><a:blip r:embed="{}"/></a:buBlip>"#,
                                xml_escape(r_id)
                            ));
                        }
                        BulletFormat::None => {
                            xml.push_str("<a:buNone/>");
                        }
                    }
                }

                // Default run properties
                if let Some(ref font) = self.font {
                    // Reuse Font::to_xml_string() but replace <a:rPr with <a:defRPr
                    let rpr = font.to_xml_string();
                    let def_rpr = rpr
                        .replace("<a:rPr ", "<a:defRPr ")
                        .replace("<a:rPr/>", "<a:defRPr/>")
                        .replace("</a:rPr>", "</a:defRPr>");
                    xml.push_str(&def_rpr);
                }

                xml.push_str("</a:pPr>");
            } else {
                xml.push_str("/>");
            }
        }

        // <a:r> elements
        for run in &self.runs {
            xml.push_str(&run.to_xml_string());
        }

        xml.push_str("</a:p>");
        xml
    }
}

#[cfg(test)]
#[path = "paragraph_tests.rs"]
mod tests;
