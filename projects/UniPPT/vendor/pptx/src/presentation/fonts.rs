//! Embedded font operations on a [`Presentation`].

use crate::error::{PartNotFoundExt, PptxError, PptxResult};
use crate::opc::constants::{content_type as CT, relationship_type as RT};
use crate::opc::part::Part;
use crate::xml_util::xml_escape;

use crate::xml_util::local_name;

use super::Presentation;

impl Presentation {
    /// Read all embedded fonts from the presentation.
    ///
    /// Parses the `<p:embeddedFontLst>` element from presentation.xml and
    /// reads the corresponding font data parts.
    /// # Errors
    ///
    /// Returns an error if the presentation XML cannot be parsed.
    pub fn embedded_fonts(&self) -> PptxResult<Vec<crate::embedded_font::EmbeddedFont>> {
        use quick_xml::events::Event;
        use quick_xml::Reader;

        let pres_part = self.presentation_part()?;
        let pres_xml = std::str::from_utf8(&pres_part.blob)?;

        let mut fonts = Vec::new();

        // Parse <p:embeddedFont> entries -- quick regex-free approach via quick_xml

        let mut reader = Reader::from_str(pres_xml);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        let mut in_embedded_font = false;
        let mut typeface = String::new();
        let mut bold = false;
        let mut italic = false;
        let mut font_r_id: Option<String> = None;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e) | Event::Empty(ref e)) => {
                    let qn = e.name();
                    let ln = local_name(qn.as_ref());
                    match ln {
                        b"embeddedFont" => {
                            in_embedded_font = true;
                            typeface.clear();
                            bold = false;
                            italic = false;
                            font_r_id = None;
                        }
                        b"font" if in_embedded_font => {
                            for attr_result in e.attributes() {
                                let attr = attr_result.map_err(PptxError::XmlAttr)?;
                                let key = local_name(attr.key.as_ref());
                                if key == b"typeface" {
                                    typeface = String::from_utf8_lossy(&attr.value).into_owned();
                                }
                            }
                        }
                        b"regular" | b"bold" | b"italic" | b"boldItalic" if in_embedded_font => {
                            bold = ln == b"bold" || ln == b"boldItalic";
                            italic = ln == b"italic" || ln == b"boldItalic";
                            for attr_result in e.attributes() {
                                let attr = attr_result.map_err(PptxError::XmlAttr)?;
                                let key = local_name(attr.key.as_ref());
                                if key == b"id" || key == b"embed" {
                                    font_r_id =
                                        Some(String::from_utf8_lossy(&attr.value).into_owned());
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => {
                    let qn = e.name();
                    let ln = local_name(qn.as_ref());
                    if ln == b"embeddedFont" && in_embedded_font {
                        // Try to read the font data
                        let font_part = font_r_id
                            .as_deref()
                            .and_then(|rid| pres_part.rels.get(rid))
                            .and_then(|rel| rel.target_partname(pres_part.partname.base_uri()).ok())
                            .and_then(|pn| self.package.part(&pn));
                        if let Some(font_part) = font_part {
                            fonts.push(crate::embedded_font::EmbeddedFont::from_bytes(
                                font_part.blob.clone(),
                                typeface.clone(),
                                bold,
                                italic,
                            ));
                        }
                        in_embedded_font = false;
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(PptxError::Xml(e)),
                _ => {}
            }
            buf.clear();
        }

        Ok(fonts)
    }

    /// Embed a font in the presentation.
    ///
    /// Adds the font data as a part at `/ppt/fonts/fontN.fntdata` and
    /// updates the presentation XML `<p:embeddedFontLst>` element.
    /// # Errors
    ///
    /// Returns an error if the font cannot be embedded.
    pub fn add_embedded_font(
        &mut self,
        font: &crate::embedded_font::EmbeddedFont,
    ) -> PptxResult<()> {
        // Create font data part
        let font_partname = self.package.next_partname("/ppt/fonts/font{}.fntdata")?;

        // Pre-compute relative ref before consuming the partname
        let pres_partname = self.presentation_partname()?;
        let rel_target = font_partname.relative_ref(pres_partname.base_uri());

        let font_part = Part::new(font_partname, CT::X_FONTDATA, font.font_data.clone());
        self.package.put_part(font_part);
        let pres_part = self
            .package
            .part_mut(&pres_partname)
            .or_part_not_found(pres_partname.as_str())?;
        let r_id = pres_part
            .rels
            .add_relationship(RT::FONT, &rel_target, false);

        // Determine which sub-element to use
        let variant_tag = match (font.bold, font.italic) {
            (true, true) => "p:boldItalic",
            (true, false) => "p:bold",
            (false, true) => "p:italic",
            (false, false) => "p:regular",
        };

        // Build the XML snippet for the embedded font entry
        let font_entry = format!(
            r#"<p:embeddedFont><p:font typeface="{}"/><{} r:id="{}"/></p:embeddedFont>"#,
            xml_escape(&font.typeface),
            variant_tag,
            r_id,
        );

        // Insert into presentation XML
        let pres_xml = std::str::from_utf8(&pres_part.blob)?;
        let wrapper_len = "<p:embeddedFontLst></p:embeddedFontLst>".len();

        let new_xml = pres_xml.find("</p:embeddedFontLst>").map_or_else(
            || {
                // Need to add <p:embeddedFontLst> -- insert after <p:sldMasterIdLst...>...</p:sldMasterIdLst>
                // or after </p:sldIdLst> if present
                let insert_pos = pres_xml
                    .find("</p:sldIdLst>")
                    .map(|pos| pos + "</p:sldIdLst>".len())
                    .or_else(|| {
                        pres_xml
                            .find("</p:sldMasterIdLst>")
                            .map(|pos| pos + "</p:sldMasterIdLst>".len())
                    })
                    .or_else(|| pres_xml.find("</p:presentation>"));

                insert_pos.map_or_else(
                    || pres_xml.to_string(),
                    |insert_pos| {
                        let mut s =
                            String::with_capacity(pres_xml.len() + font_entry.len() + wrapper_len);
                        s.push_str(&pres_xml[..insert_pos]);
                        s.push_str("<p:embeddedFontLst>");
                        s.push_str(&font_entry);
                        s.push_str("</p:embeddedFontLst>");
                        s.push_str(&pres_xml[insert_pos..]);
                        s
                    },
                )
            },
            |pos| {
                // Insert before </p:embeddedFontLst>
                let mut s = String::with_capacity(pres_xml.len() + font_entry.len());
                s.push_str(&pres_xml[..pos]);
                s.push_str(&font_entry);
                s.push_str(&pres_xml[pos..]);
                s
            },
        );

        // Update the part with new XML
        let pres_part = self
            .package
            .part_mut(&pres_partname)
            .or_part_not_found(pres_partname.as_str())?;
        pres_part.blob = new_xml.into_bytes();

        Ok(())
    }
}
