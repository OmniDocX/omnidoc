use std::borrow::Cow;

use quick_xml::events::{BytesEnd, BytesStart, Event};
use quick_xml::Reader;
use quick_xml::Writer;

use crate::error::{PptxError, PptxResult};
use crate::opc::constants::namespace;
use crate::opc::relationship::{extract_rid_num, Relationship, Relationships};
use crate::units::RelationshipId;

impl Relationships {
    /// Parse relationships from .rels XML bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the XML is malformed.
    pub fn from_xml(base_uri: &str, xml: &[u8]) -> PptxResult<Self> {
        let mut rels = Self::new(base_uri);
        let mut reader = Reader::from_reader(xml);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(ref e) | Event::Start(ref e))
                    if e.name().as_ref() == b"Relationship" =>
                {
                    let rel = parse_relationship_element(e)?;
                    let num = extract_rid_num(rel.r_id.as_str());
                    if num > rels.max_r_id {
                        rels.max_r_id = num;
                    }
                    rels.rels.push(rel);
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(PptxError::Xml(e)),
                _ => {}
            }
            buf.clear();
        }

        Ok(rels)
    }

    /// Serialize relationships to .rels XML bytes (with XML declaration).
    ///
    /// # Errors
    ///
    /// Returns an error if XML serialization fails.
    pub fn to_xml(&self) -> PptxResult<Vec<u8>> {
        let mut writer = Writer::new(Vec::new());

        // XML declaration
        writer.write_event(Event::Decl(quick_xml::events::BytesDecl::new(
            "1.0",
            Some("UTF-8"),
            Some("yes"),
        )))?;

        // <Relationships xmlns="...">
        let mut elem = BytesStart::new("Relationships");
        elem.push_attribute(("xmlns", namespace::OPC_RELATIONSHIPS));
        writer.write_event(Event::Start(elem))?;

        // Sort relationships by rId for deterministic output
        let mut sorted_rels: Vec<&Relationship> = self.rels.iter().collect();
        sorted_rels.sort_by_key(|r| extract_rid_num(r.r_id.as_str()));

        for rel in sorted_rels {
            let mut rel_elem = BytesStart::new("Relationship");
            rel_elem.push_attribute(("Id", rel.r_id.as_str()));
            rel_elem.push_attribute(("Type", &*rel.rel_type));
            rel_elem.push_attribute(("Target", rel.target_ref.as_str()));
            if rel.is_external {
                rel_elem.push_attribute(("TargetMode", "External"));
            }
            writer.write_event(Event::Empty(rel_elem))?;
        }

        // </Relationships>
        writer.write_event(Event::End(BytesEnd::new("Relationships")))?;

        Ok(writer.into_inner())
    }
}

/// Parse a <Relationship> element from its attributes.
fn parse_relationship_element(elem: &BytesStart<'_>) -> PptxResult<Relationship> {
    let mut r_id: Option<String> = None;
    let mut rel_type: Option<Cow<'static, str>> = None;
    let mut target_ref = None;
    let mut is_external = false;

    for attr in elem.attributes() {
        let attr = attr.map_err(PptxError::XmlAttr)?;
        let key = attr.key.as_ref();

        match key {
            b"Id" | b"Type" | b"Target" => {
                let encoded = std::str::from_utf8(&attr.value)
                    .map_err(|e| PptxError::InvalidXml(format!("invalid UTF-8 in attribute: {e}")))?
                    .to_string();
                let value = quick_xml::escape::unescape(&encoded)
                    .map_err(|e| {
                        PptxError::InvalidXml(format!(
                            "invalid XML entity in relationship attribute: {e}"
                        ))
                    })?
                    .into_owned();
                match key {
                    b"Id" => r_id = Some(value),
                    b"Type" => rel_type = Some(Cow::Owned(value)),
                    b"Target" => target_ref = Some(value),
                    _ => {}
                }
            }
            b"TargetMode" => {
                is_external = attr.value.as_ref() == b"External";
            }
            _ => {}
        }
    }

    let r_id_str =
        r_id.ok_or_else(|| PptxError::InvalidXml("Relationship missing Id".to_string()))?;
    let r_id = RelationshipId::try_from(r_id_str.as_str()).map_err(|_| {
        PptxError::InvalidXml(format!("Relationship Id is not a valid rId: {r_id_str}"))
    })?;
    let rel_type =
        rel_type.ok_or_else(|| PptxError::InvalidXml("Relationship missing Type".to_string()))?;
    let target_ref = target_ref
        .ok_or_else(|| PptxError::InvalidXml("Relationship missing Target".to_string()))?;

    Ok(Relationship::new(r_id, rel_type, target_ref, is_external))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RELS: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
    <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
    <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/thumbnail" Target="docProps/thumbnail.jpeg"/>
    <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
</Relationships>"#;

    #[test]
    fn test_parse_rels() {
        // EXCEPTION(unwrap): test-only code with known-valid input
        let rels = Relationships::from_xml("/", SAMPLE_RELS).unwrap();
        assert_eq!(rels.len(), 3);

        let r1 = rels.get("rId1").unwrap();
        assert_eq!(r1.target_ref, "ppt/presentation.xml");
        assert!(!r1.is_external);
    }

    #[test]
    fn test_round_trip() {
        // EXCEPTION(unwrap): test-only code with known-valid input
        let rels = Relationships::from_xml("/", SAMPLE_RELS).unwrap();
        let xml = rels.to_xml().unwrap();
        let rels2 = Relationships::from_xml("/", &xml).unwrap();
        assert_eq!(rels2.len(), 3);

        let r1 = rels2.get("rId1").unwrap();
        assert_eq!(r1.target_ref, "ppt/presentation.xml");
    }

    #[test]
    fn relationship_target_entities_round_trip_as_the_original_url() {
        let mut rels = Relationships::new("/ppt/slides");
        rels.add_relationship(
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink",
            "https://example.com/path?a=1&b=2",
            true,
        );

        let xml = rels.to_xml().unwrap();
        assert!(std::str::from_utf8(&xml).unwrap().contains("a=1&amp;b=2"));
        let reopened = Relationships::from_xml("/ppt/slides", &xml).unwrap();

        assert_eq!(
            reopened.iter().next().unwrap().target_ref,
            "https://example.com/path?a=1&b=2"
        );
    }

    #[test]
    fn test_add_relationship() {
        let mut rels = Relationships::new("/ppt");
        let r_id = rels.add_relationship(
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide",
            "slides/slide1.xml",
            false,
        );
        assert_eq!(r_id, "rId1");
        assert_eq!(rels.len(), 1);
    }

    #[test]
    fn test_or_add() {
        let mut rels = Relationships::new("/ppt");
        let r_id1 = rels.or_add(
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide",
            "slides/slide1.xml",
            false,
        );
        let r_id2 = rels.or_add(
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide",
            "slides/slide1.xml",
            false,
        );
        assert_eq!(r_id1, r_id2);
        assert_eq!(rels.len(), 1);
    }

    #[test]
    fn test_target_partname() {
        // EXCEPTION(unwrap): test-only code with known-valid input
        let r_id = RelationshipId::try_from("rId1").unwrap();
        let rel = Relationship::new(
            r_id,
            "some-type".to_string(),
            "slides/slide1.xml".to_string(),
            false,
        );
        let partname = rel.target_partname("/ppt").unwrap();
        assert_eq!(partname.as_str(), "/ppt/slides/slide1.xml");
    }
}
