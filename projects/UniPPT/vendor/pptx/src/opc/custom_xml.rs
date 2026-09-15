use crate::error::{PptxError, PptxResult};
use crate::opc::pack_uri::PackURI;
use crate::opc::part::Part;

/// A custom XML data part within an OPC package.
///
/// Custom XML parts allow embedding arbitrary XML data in a .pptx file.
/// They have content type `"application/xml"` and are linked via the
/// `customXml` relationship type.
#[derive(Debug, Clone)]
pub struct CustomXmlPart {
    /// The partname of this custom XML part.
    pub partname: PackURI,
    /// The raw XML content.
    pub xml_data: Vec<u8>,
}

impl CustomXmlPart {
    /// Create a new `CustomXmlPart` with the given partname and XML data.
    #[must_use]
    pub const fn new(partname: PackURI, xml_data: Vec<u8>) -> Self {
        Self { partname, xml_data }
    }

    /// Create a `CustomXmlPart` from a string of XML.
    #[must_use]
    pub fn from_str(partname: PackURI, xml: &str) -> Self {
        Self {
            partname,
            xml_data: xml.as_bytes().to_vec(),
        }
    }

    /// Get the XML content as a byte slice.
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.xml_data
    }

    /// Get the XML content as a UTF-8 string, if valid.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is not valid UTF-8.
    pub fn data_str(&self) -> PptxResult<&str> {
        std::str::from_utf8(&self.xml_data)
            .map_err(|e| PptxError::InvalidXml(format!("custom XML is not valid UTF-8: {e}")))
    }

    /// Convert this into an OPC `Part` suitable for inclusion in a package.
    #[must_use]
    pub fn into_part(self) -> Part {
        Part::new(self.partname, "application/xml", self.xml_data)
    }

    /// Create a `CustomXmlPart` from an existing OPC `Part`.
    ///
    /// Clones the partname and blob because `CustomXmlPart` owns its data.
    /// Adding a lifetime parameter to avoid the clone would propagate
    /// lifetimes through the public API, so the owned copy is intentional.
    #[must_use]
    pub fn from_part(part: &Part) -> Self {
        Self {
            partname: part.partname.clone(),
            xml_data: part.blob.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_xml_part_new() {
        let partname = PackURI::new("/customXml/item1.xml").unwrap();
        let data = b"<root><item>hello</item></root>".to_vec();
        let part = CustomXmlPart::new(partname.clone(), data.clone());
        assert_eq!(part.partname.as_str(), "/customXml/item1.xml");
        assert_eq!(part.data(), &data[..]);
    }

    #[test]
    fn test_custom_xml_part_from_str() {
        let partname = PackURI::new("/customXml/item1.xml").unwrap();
        let xml = "<data><value>42</value></data>";
        let part = CustomXmlPart::from_str(partname, xml);
        assert_eq!(part.data_str().unwrap(), xml);
    }

    #[test]
    fn test_custom_xml_into_part() {
        let partname = PackURI::new("/customXml/item1.xml").unwrap();
        let xml = "<test/>";
        let custom = CustomXmlPart::from_str(partname, xml);
        let opc_part = custom.into_part();
        assert_eq!(opc_part.content_type, "application/xml");
        assert_eq!(opc_part.partname.as_str(), "/customXml/item1.xml");
        assert_eq!(String::from_utf8(opc_part.blob).unwrap(), "<test/>");
    }

    #[test]
    fn test_custom_xml_from_part() {
        let partname = PackURI::new("/customXml/item1.xml").unwrap();
        let opc_part = Part::new(partname, "application/xml", b"<data/>".to_vec());
        let custom = CustomXmlPart::from_part(&opc_part);
        assert_eq!(custom.data_str().unwrap(), "<data/>");
    }
}
