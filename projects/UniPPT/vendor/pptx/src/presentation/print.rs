//! Print settings operations on a [`Presentation`].

use crate::error::{PartNotFoundExt, PptxError, PptxResult};
use crate::print_settings::PrintSettings;
use crate::xml_util::{local_name, WriteXml};

use super::{remove_xml_element, Presentation};

impl Presentation {
    /// Get the print settings of the presentation.
    ///
    /// Reads from the `<p:prnPr>` element in presentation.xml.
    /// Returns default settings if no `<p:prnPr>` element is present.
    /// # Errors
    ///
    /// Returns an error if the presentation XML cannot be parsed.
    pub fn print_settings(&self) -> PptxResult<PrintSettings> {
        use quick_xml::events::Event;
        use quick_xml::Reader;

        let pres_part = self.presentation_part()?;
        let pres_xml = std::str::from_utf8(&pres_part.blob)?;

        let mut reader = Reader::from_str(pres_xml);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e) | Event::Empty(ref e)) => {
                    let qn = e.name();
                    let ln = local_name(qn.as_ref());
                    if ln == b"prnPr" {
                        return PrintSettings::from_xml_element(e);
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(PptxError::Xml(e)),
                _ => {}
            }
            buf.clear();
        }

        Ok(PrintSettings::new())
    }

    /// Set the print settings of the presentation.
    ///
    /// Writes a `<p:prnPr>` element into presentation.xml.
    /// If the presentation already has print settings, they are replaced.
    /// # Errors
    ///
    /// Returns an error if the presentation XML cannot be updated.
    pub fn set_print_settings(&mut self, settings: &PrintSettings) -> PptxResult<()> {
        let pres_partname = self.presentation_partname()?;
        let pres_part = self
            .package
            .part_mut(&pres_partname)
            .or_part_not_found(pres_partname.as_str())?;

        let pres_xml = std::str::from_utf8(&pres_part.blob)?;
        let settings_xml = settings.to_xml_string();

        // Remove existing <p:prnPr.../> or <p:prnPr>...</p:prnPr>
        let result = remove_xml_element(pres_xml, "p:prnPr");

        // Insert before </p:presentation>
        let pos = result.rfind("</p:presentation>").ok_or_else(|| {
            PptxError::InvalidXml("presentation XML does not contain </p:presentation>".to_string())
        })?;
        let mut updated = String::with_capacity(result.len() + settings_xml.len());
        updated.push_str(&result[..pos]);
        updated.push_str(&settings_xml);
        updated.push_str(&result[pos..]);

        pres_part.blob = updated.into_bytes();
        Ok(())
    }
}
