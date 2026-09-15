//! XML parsing and serialization for core properties.

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Reader;
use quick_xml::Writer;

use crate::error::{PptxError, PptxResult};
use crate::oxml::ns::{NS_CP, NS_DC, NS_DCMITYPE, NS_DCTERMS, NS_XSI};
use crate::xml_util::local_name_owned;

use super::CoreProperties;

impl CoreProperties {
    /// Parse `CoreProperties` from the `docProps/core.xml` content.
    ///
    /// # Errors
    ///
    /// Returns an error if the XML is malformed or cannot be decoded.
    pub fn from_xml(xml: &[u8]) -> PptxResult<Self> {
        let mut props = Self::new();
        let mut reader = Reader::from_reader(xml);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        let mut current_element: Option<String> = None;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    let qname = e.name();
                    current_element = Some(local_name_owned(qname.as_ref()));
                }
                Ok(Event::Text(ref e)) => {
                    if let Some(ref elem_name) = current_element {
                        let text = e.decode().map_err(|err| {
                            PptxError::InvalidXml(format!("XML text error: {err}"))
                        })?;
                        let text = text.into_owned();
                        match elem_name.as_str() {
                            "title" => props.title = text,
                            "creator" => props.author = text,
                            "subject" => props.subject = text,
                            "keywords" => props.keywords = text,
                            "description" => props.comments = text,
                            "category" => props.category = text,
                            "created" => props.created = text,
                            "modified" => props.modified = text,
                            "lastModifiedBy" => props.last_modified_by = text,
                            "revision" => props.revision = text,
                            "contentStatus" => props.content_status = text,
                            "language" => props.language = text,
                            "version" => props.version = text,
                            "identifier" => props.identifier = Some(text),
                            "lastPrinted" => props.last_printed = Some(text),
                            _ => {}
                        }
                    }
                }
                Ok(Event::End(_)) => {
                    current_element = None;
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(PptxError::Xml(e)),
                _ => {}
            }
            buf.clear();
        }

        Ok(props)
    }

    /// Serialize to XML bytes suitable for `docProps/core.xml`.
    ///
    /// # Errors
    ///
    /// Returns an error if XML serialization fails.
    pub fn to_xml(&self) -> PptxResult<Vec<u8>> {
        let mut writer = Writer::new(Vec::new());

        // XML declaration
        writer.write_event(Event::Decl(BytesDecl::new(
            "1.0",
            Some("UTF-8"),
            Some("yes"),
        )))?;

        // <cp:coreProperties>
        let mut root = BytesStart::new("cp:coreProperties");
        root.push_attribute(("xmlns:cp", NS_CP));
        root.push_attribute(("xmlns:dc", NS_DC));
        root.push_attribute(("xmlns:dcterms", NS_DCTERMS));
        root.push_attribute(("xmlns:dcmitype", NS_DCMITYPE));
        root.push_attribute(("xmlns:xsi", NS_XSI));
        writer.write_event(Event::Start(root))?;

        // dc:title
        if !self.title.is_empty() {
            write_simple_element(&mut writer, "dc:title", &self.title)?;
        }

        // dc:creator (author)
        if !self.author.is_empty() {
            write_simple_element(&mut writer, "dc:creator", &self.author)?;
        }

        // dc:subject
        if !self.subject.is_empty() {
            write_simple_element(&mut writer, "dc:subject", &self.subject)?;
        }

        // dc:description (comments)
        if !self.comments.is_empty() {
            write_simple_element(&mut writer, "dc:description", &self.comments)?;
        }

        // cp:keywords
        if !self.keywords.is_empty() {
            write_simple_element(&mut writer, "cp:keywords", &self.keywords)?;
        }

        // cp:category
        if !self.category.is_empty() {
            write_simple_element(&mut writer, "cp:category", &self.category)?;
        }

        // cp:lastModifiedBy
        if !self.last_modified_by.is_empty() {
            write_simple_element(&mut writer, "cp:lastModifiedBy", &self.last_modified_by)?;
        }

        // cp:revision
        if !self.revision.is_empty() {
            write_simple_element(&mut writer, "cp:revision", &self.revision)?;
        }

        // cp:contentStatus
        if !self.content_status.is_empty() {
            write_simple_element(&mut writer, "cp:contentStatus", &self.content_status)?;
        }

        // dc:language
        if !self.language.is_empty() {
            write_simple_element(&mut writer, "dc:language", &self.language)?;
        }

        // cp:version
        if !self.version.is_empty() {
            write_simple_element(&mut writer, "cp:version", &self.version)?;
        }

        // dc:identifier
        if let Some(ref identifier) = self.identifier {
            write_simple_element(&mut writer, "dc:identifier", identifier)?;
        }

        // cp:lastPrinted
        if let Some(ref last_printed) = self.last_printed {
            write_simple_element(&mut writer, "cp:lastPrinted", last_printed)?;
        }

        // dcterms:created with xsi:type attribute
        if !self.created.is_empty() {
            write_datetime_element(&mut writer, "dcterms:created", &self.created)?;
        }

        // dcterms:modified with xsi:type attribute
        if !self.modified.is_empty() {
            write_datetime_element(&mut writer, "dcterms:modified", &self.modified)?;
        }

        // </cp:coreProperties>
        writer.write_event(Event::End(BytesEnd::new("cp:coreProperties")))?;

        Ok(writer.into_inner())
    }
}

/// Write a simple element like `<tag>text</tag>`.
fn write_simple_element(writer: &mut Writer<Vec<u8>>, tag: &str, text: &str) -> PptxResult<()> {
    writer.write_event(Event::Start(BytesStart::new(tag)))?;
    writer.write_event(Event::Text(BytesText::new(text)))?;
    writer.write_event(Event::End(BytesEnd::new(tag)))?;
    Ok(())
}

/// Write a datetime element with `xsi:type="dcterms:W3CDTF"` attribute.
fn write_datetime_element(writer: &mut Writer<Vec<u8>>, tag: &str, text: &str) -> PptxResult<()> {
    let mut elem = BytesStart::new(tag);
    elem.push_attribute(("xsi:type", "dcterms:W3CDTF"));
    writer.write_event(Event::Start(elem))?;
    writer.write_event(Event::Text(BytesText::new(text)))?;
    writer.write_event(Event::End(BytesEnd::new(tag)))?;
    Ok(())
}
