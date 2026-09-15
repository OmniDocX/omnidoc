//! Digital signature support for PPTX files.
//!
//! Digital signatures live in `_xmlsignatures/` parts within the OPC package
//! and allow asserting authorship and integrity of the presentation.

mod types;
mod xml;

pub use types::{DigitalSignature, HashAlgorithm, SignatureCommitment, SignerInfo};
pub(crate) use xml::signature_to_xml;
pub use xml::{
    DIGITAL_SIGNATURE_ORIGIN_CT, DIGITAL_SIGNATURE_ORIGIN_RT, DIGITAL_SIGNATURE_RT,
    DIGITAL_SIGNATURE_XML_CT,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signer_info_new() {
        let info = SignerInfo::new("Alice");
        assert_eq!(info.name, "Alice");
        assert!(info.email.is_none());
        assert!(info.title.is_none());
    }

    #[test]
    fn test_signer_info_with_email_and_title() {
        let info = SignerInfo::new("Bob")
            .with_email("bob@example.com")
            .with_title("Manager");
        assert_eq!(info.name, "Bob");
        assert_eq!(info.email.as_deref(), Some("bob@example.com"));
        assert_eq!(info.title.as_deref(), Some("Manager"));
    }

    #[test]
    fn test_digital_signature_new() {
        let signer = SignerInfo::new("Alice");
        let sig = DigitalSignature::new(signer, HashAlgorithm::Sha256);
        assert_eq!(sig.signer.name, "Alice");
        assert_eq!(sig.hash_algorithm, HashAlgorithm::Sha256);
        assert!(sig.sign_time.is_none());
        assert_eq!(sig.commitment, SignatureCommitment::Created);
    }

    #[test]
    fn test_digital_signature_builder() {
        let signer = SignerInfo::new("Bob").with_email("bob@example.com");
        let sig = DigitalSignature::new(signer, HashAlgorithm::Sha1)
            .with_commitment(SignatureCommitment::Approved)
            .with_sign_time("2024-06-15T10:30:00Z");

        assert_eq!(sig.signer.name, "Bob");
        assert_eq!(sig.signer.email.as_deref(), Some("bob@example.com"));
        assert_eq!(sig.hash_algorithm, HashAlgorithm::Sha1);
        assert_eq!(sig.commitment, SignatureCommitment::Approved);
        assert_eq!(sig.sign_time.as_deref(), Some("2024-06-15T10:30:00Z"));
    }

    #[test]
    fn test_hash_algorithm_uri() {
        assert_eq!(
            HashAlgorithm::Sha1.algorithm_uri(),
            "http://www.w3.org/2000/09/xmldsig#sha1"
        );
        assert_eq!(
            HashAlgorithm::Sha256.algorithm_uri(),
            "http://www.w3.org/2001/04/xmlenc#sha256"
        );
        assert_eq!(
            HashAlgorithm::Sha512.algorithm_uri(),
            "http://www.w3.org/2001/04/xmlenc#sha512"
        );
    }

    #[test]
    fn test_hash_algorithm_name() {
        assert_eq!(HashAlgorithm::Sha1.name(), "SHA1");
        assert_eq!(HashAlgorithm::Sha256.name(), "SHA256");
        assert_eq!(HashAlgorithm::Sha512.name(), "SHA512");
    }

    #[test]
    fn test_signature_commitment_uri() {
        let created = SignatureCommitment::Created.commitment_uri();
        assert!(created.ends_with("/created"));
        let approved = SignatureCommitment::Approved.commitment_uri();
        assert!(approved.ends_with("/approved"));
        let reviewed = SignatureCommitment::Reviewed.commitment_uri();
        assert!(reviewed.ends_with("/reviewed"));
    }

    #[test]
    fn test_write_xml_basic() {
        use crate::xml_util::WriteXml;
        let signer = SignerInfo::new("Alice");
        let sig = DigitalSignature::new(signer, HashAlgorithm::Sha1);
        let xml = sig.to_xml_string();

        assert!(xml.contains("<Signature xmlns="));
        assert!(xml.contains("xmldsig#"));
        assert!(xml.contains("<SignedInfo>"));
        assert!(xml.contains("<SignatureMethod"));
        assert!(xml.contains("sha1"));
        assert!(xml.contains("<Reference URI=\"/ppt/presentation.xml\">"));
        assert!(xml.contains("<DigestMethod"));
        assert!(xml.contains("<DigestValue/>"));
        assert!(xml.contains("<SignatureValue/>"));
        assert!(xml.contains("<mdssi:SignerName>Alice</mdssi:SignerName>"));
        assert!(xml.contains("</Signature>"));
    }

    #[test]
    fn test_write_xml_with_all_properties() {
        use crate::xml_util::WriteXml;
        let signer = SignerInfo::new("Bob")
            .with_email("bob@example.com")
            .with_title("Reviewer");
        let sig = DigitalSignature::new(signer, HashAlgorithm::Sha256)
            .with_commitment(SignatureCommitment::Reviewed)
            .with_sign_time("2024-01-01T00:00:00Z");
        let xml = sig.to_xml_string();

        assert!(xml.contains("<mdssi:SignerName>Bob</mdssi:SignerName>"));
        assert!(xml.contains("<mdssi:SignerEmail>bob@example.com</mdssi:SignerEmail>"));
        assert!(xml.contains("<mdssi:SignerTitle>Reviewer</mdssi:SignerTitle>"));
        assert!(xml.contains("<mdssi:Value>2024-01-01T00:00:00Z</mdssi:Value>"));
        assert!(xml.contains("/reviewed"));
        assert!(xml.contains("sha256"));
    }

    #[test]
    fn test_write_xml_escapes_special_chars() {
        use crate::xml_util::WriteXml;
        let signer = SignerInfo::new("Alice & Bob <Corp>").with_email("a&b@example.com");
        let sig = DigitalSignature::new(signer, HashAlgorithm::Sha1);
        let xml = sig.to_xml_string();

        assert!(xml.contains("Alice &amp; Bob &lt;Corp&gt;"));
        assert!(xml.contains("a&amp;b@example.com"));
    }

    #[test]
    fn test_signature_to_xml_has_declaration() {
        let signer = SignerInfo::new("Alice");
        let sig = DigitalSignature::new(signer, HashAlgorithm::Sha1);
        let xml_bytes = signature_to_xml(&sig);
        let xml = String::from_utf8(xml_bytes).unwrap();

        assert!(xml.starts_with(r#"<?xml version="1.0" encoding="UTF-8"?>"#));
        assert!(xml.contains("<Signature"));
    }

    #[test]
    fn test_digital_signature_clone() {
        let signer = SignerInfo::new("Alice").with_email("alice@test.com");
        let sig = DigitalSignature::new(signer, HashAlgorithm::Sha256)
            .with_sign_time("2024-01-01T00:00:00Z");
        let cloned = sig.clone();
        assert_eq!(cloned, sig);
    }

    #[test]
    fn test_signer_info_clone() {
        let info = SignerInfo::new("Test")
            .with_email("test@test.com")
            .with_title("Title");
        let cloned = info.clone();
        assert_eq!(cloned, info);
    }
}
