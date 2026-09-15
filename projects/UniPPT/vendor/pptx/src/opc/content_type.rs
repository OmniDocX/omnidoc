use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::Reader;
use quick_xml::Writer;

use crate::error::{PackageError, PptxError, PptxResult};
use crate::opc::constants::{content_type, namespace, DEFAULT_CONTENT_TYPES};
use crate::opc::pack_uri::PackURI;

/// Maps partnames to their content types (MIME types) based on
/// the `[Content_Types].xml` file in an OPC package.
///
/// Supports two lookup mechanisms:
/// - **Default**: Maps file extensions to content types (e.g., "xml" -> "application/xml")
/// - **Override**: Maps specific partnames to content types
///
/// Uses `Vec` internally for cache-friendly lookup; typical packages have
/// fewer than 10 defaults and fewer than 50 overrides.
#[derive(Debug, Clone)]
pub struct ContentTypeMap {
    /// Extension -> content type (case-insensitive on extension).
    defaults: Vec<(String, String)>,
    /// Absolute partname -> content type (case-insensitive on partname).
    overrides: Vec<(String, String)>,
}

impl ContentTypeMap {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            defaults: Vec::new(),
            overrides: Vec::new(),
        }
    }

    /// Parse a `ContentTypeMap` from `[Content_Types].xml` bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the XML is malformed or missing required attributes.
    pub fn from_xml(xml: &[u8]) -> PptxResult<Self> {
        let mut map = Self::new();
        let mut reader = Reader::from_reader(xml);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(ref e) | Event::Start(ref e)) => {
                    let name = e.name();
                    let local = name.as_ref();
                    if local == b"Default" {
                        let (ext, ct) = parse_default_element(e)?;
                        map.add_default(ext, ct);
                    } else if local == b"Override" {
                        let (partname, ct) = parse_override_element(e)?;
                        map.add_override(partname, ct);
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(PptxError::Xml(e)),
                _ => {}
            }
            buf.clear();
        }

        Ok(map)
    }

    /// Serialize this `ContentTypeMap` to `[Content_Types].xml` bytes (with XML declaration).
    ///
    /// # Errors
    ///
    /// Returns an error if XML serialization fails.
    pub fn to_xml(&self) -> PptxResult<Vec<u8>> {
        let mut writer = Writer::new(Vec::new());

        writer.write_event(Event::Decl(BytesDecl::new(
            "1.0",
            Some("UTF-8"),
            Some("yes"),
        )))?;

        let mut types_elem = BytesStart::new("Types");
        types_elem.push_attribute(("xmlns", namespace::OPC_CONTENT_TYPES));
        writer.write_event(Event::Start(types_elem))?;

        // Write Default elements, sorted by extension
        let mut defaults: Vec<&(String, String)> = self.defaults.iter().collect();
        defaults.sort_by_key(|(ext, _)| ext.as_str());
        for (ext, ct) in defaults {
            let mut elem = BytesStart::new("Default");
            elem.push_attribute(("Extension", ext.as_str()));
            elem.push_attribute(("ContentType", ct.as_str()));
            writer.write_event(Event::Empty(elem))?;
        }

        // Write Override elements, sorted by partname
        let mut overrides: Vec<&(String, String)> = self.overrides.iter().collect();
        overrides.sort_by_key(|(partname, _)| partname.as_str());
        for (partname, ct) in overrides {
            let mut elem = BytesStart::new("Override");
            elem.push_attribute(("PartName", partname.as_str()));
            elem.push_attribute(("ContentType", ct.as_str()));
            writer.write_event(Event::Empty(elem))?;
        }

        writer.write_event(Event::End(BytesEnd::new("Types")))?;

        Ok(writer.into_inner())
    }

    /// Look up the content type for a given partname.
    ///
    /// First checks overrides (by partname), then falls back to defaults (by extension).
    ///
    /// # Errors
    ///
    /// Returns an error if no content type is found for the given partname.
    pub fn get(&self, partname: &PackURI) -> PptxResult<&str> {
        if let Some((_, ct)) = self
            .overrides
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(partname.as_str()))
        {
            return Ok(ct);
        }
        let ext = partname.ext();
        if let Some((_, ct)) = self
            .defaults
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(ext))
        {
            return Ok(ct);
        }
        Err(PptxError::Package(PackageError::ContentTypeNotFound(
            partname.to_string(),
        )))
    }

    /// Add a default (extension-based) content type mapping.
    pub fn add_default(&mut self, extension: impl Into<String>, content_type: impl Into<String>) {
        let key = extension.into().to_lowercase();
        let val = content_type.into();
        if let Some((_, existing)) = self.defaults.iter_mut().find(|(k, _)| *k == key) {
            *existing = val;
        } else {
            self.defaults.push((key, val));
        }
    }

    /// Add an override (partname-specific) content type mapping.
    pub fn add_override(&mut self, partname: impl Into<String>, content_type: impl Into<String>) {
        // OPC part names are compared case-insensitively by this map, but their
        // original casing must be preserved when serializing. Open XML SDK uses
        // the exact package URI to associate an Override with a part; emitting
        // `/docprops/core.xml` for `/docProps/core.xml` makes it fall back to the
        // generic XML content type and reject the package.
        let key = partname.into();
        let val = content_type.into();
        if let Some((_, existing)) = self
            .overrides
            .iter_mut()
            .find(|(existing, _)| existing.eq_ignore_ascii_case(&key))
        {
            *existing = val;
        } else {
            self.overrides.push((key, val));
        }
    }

    /// Build a `ContentTypeMap` from a collection of parts (partname, `content_type` pairs).
    ///
    /// Uses default content types where possible, overrides for the rest.
    /// Always includes the standard "rels" and "xml" defaults.
    pub fn from_parts<'a>(parts: impl Iterator<Item = (&'a PackURI, &'a str)>) -> Self {
        let mut map = Self::new();

        // Always include standard defaults
        map.add_default("rels", content_type::OPC_RELATIONSHIPS);
        map.add_default("xml", content_type::XML);

        for (partname, ct) in parts {
            let ext = partname.ext();
            // Check if this content type matches a known default for this extension.
            // DEFAULT_CONTENT_TYPES extensions are already lowercase, so
            // case-insensitive comparison avoids allocating a lowered copy.
            let is_default = DEFAULT_CONTENT_TYPES
                .iter()
                .any(|(e, c)| e.eq_ignore_ascii_case(ext) && *c == ct);

            if is_default {
                map.add_default(ext, ct);
            } else {
                map.add_override(partname.as_str(), ct);
            }
        }

        map
    }
}

/// Creates an empty content type map with no defaults or overrides.
impl Default for ContentTypeMap {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_default_element(elem: &BytesStart<'_>) -> PptxResult<(String, String)> {
    let mut extension = None;
    let mut ct = None;

    for attr in elem.attributes() {
        let attr = attr.map_err(PptxError::XmlAttr)?;
        let key = attr.key.as_ref();
        let value = String::from_utf8(attr.value.to_vec())
            .map_err(|e| PptxError::InvalidXml(format!("invalid UTF-8: {e}")))?;
        match key {
            b"Extension" => extension = Some(value),
            b"ContentType" => ct = Some(value),
            _ => {}
        }
    }

    let extension =
        extension.ok_or_else(|| PptxError::InvalidXml("Default missing Extension".to_string()))?;
    let ct = ct.ok_or_else(|| PptxError::InvalidXml("Default missing ContentType".to_string()))?;
    Ok((extension, ct))
}

fn parse_override_element(elem: &BytesStart<'_>) -> PptxResult<(String, String)> {
    let mut partname = None;
    let mut ct = None;

    for attr in elem.attributes() {
        let attr = attr.map_err(PptxError::XmlAttr)?;
        let key = attr.key.as_ref();
        let value = String::from_utf8(attr.value.to_vec())
            .map_err(|e| PptxError::InvalidXml(format!("invalid UTF-8: {e}")))?;
        match key {
            b"PartName" => partname = Some(value),
            b"ContentType" => ct = Some(value),
            _ => {}
        }
    }

    let partname =
        partname.ok_or_else(|| PptxError::InvalidXml("Override missing PartName".to_string()))?;
    let ct = ct.ok_or_else(|| PptxError::InvalidXml("Override missing ContentType".to_string()))?;
    Ok((partname, ct))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_CT: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
    <Default Extension="xml" ContentType="application/xml"/>
    <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
    <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
</Types>"#;

    #[test]
    fn test_parse_content_types() {
        let map = ContentTypeMap::from_xml(SAMPLE_CT).unwrap();

        let uri = PackURI::new("/ppt/presentation.xml").unwrap();
        assert_eq!(
            map.get(&uri).unwrap(),
            "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"
        );

        // Falls back to default for unknown partname with known extension
        let uri2 = PackURI::new("/ppt/unknown.xml").unwrap();
        assert_eq!(map.get(&uri2).unwrap(), "application/xml");
    }

    #[test]
    fn test_round_trip() {
        let map = ContentTypeMap::from_xml(SAMPLE_CT).unwrap();
        let xml = map.to_xml().unwrap();
        let map2 = ContentTypeMap::from_xml(&xml).unwrap();

        let uri = PackURI::new("/ppt/presentation.xml").unwrap();
        assert_eq!(
            map2.get(&uri).unwrap(),
            "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"
        );
    }

    #[test]
    fn override_serialization_preserves_part_name_case() {
        let map = ContentTypeMap::from_parts(
            [(
                &PackURI::new("/docProps/core.xml").unwrap(),
                content_type::OPC_CORE_PROPERTIES,
            )]
            .into_iter(),
        );
        let xml = String::from_utf8(map.to_xml().unwrap()).unwrap();
        assert!(xml.contains("PartName=\"/docProps/core.xml\""));
        assert!(!xml.contains("PartName=\"/docprops/core.xml\""));
    }

    #[test]
    fn test_not_found() {
        let map = ContentTypeMap::new();
        let uri = PackURI::new("/ppt/slides/slide1.xml").unwrap();
        assert!(map.get(&uri).is_err());
    }
}
