//! Digital signature operations on a [`Presentation`].

use crate::error::PptxResult;
use crate::opc::pack_uri::PackURI;
use crate::opc::part::Part;
use crate::signature::{
    signature_to_xml, DigitalSignature, HashAlgorithm, SignatureCommitment, SignerInfo,
    DIGITAL_SIGNATURE_ORIGIN_CT, DIGITAL_SIGNATURE_ORIGIN_RT, DIGITAL_SIGNATURE_RT,
    DIGITAL_SIGNATURE_XML_CT,
};
use crate::xml_util::local_name;

use super::Presentation;

impl Presentation {
    /// Add a digital signature to the presentation.
    ///
    /// Creates the `_xmlsignatures/origin.sigs` origin part (if not already
    /// present) and a new signature part at `_xmlsignatures/sig{N}.xml`.
    /// # Errors
    ///
    /// Returns an error if the signature cannot be added.
    pub fn add_digital_signature(&mut self, sig: &DigitalSignature) -> PptxResult<()> {
        let origin_uri = PackURI::new("/_xmlsignatures/origin.sigs")?;

        // Ensure the origin part exists
        if self.package.part(&origin_uri).is_none() {
            let origin_part =
                Part::new(origin_uri.clone(), DIGITAL_SIGNATURE_ORIGIN_CT, Vec::new());
            self.package.put_part(origin_part);

            // Add package-level relationship to the origin
            self.package.pkg_rels.or_add(
                DIGITAL_SIGNATURE_ORIGIN_RT,
                "_xmlsignatures/origin.sigs",
                false,
            );
        }

        // Find the next available signature partname
        let sig_partname = self.package.next_partname("/_xmlsignatures/sig{}.xml")?;

        // Generate the signature XML
        let sig_xml = signature_to_xml(sig);
        let sig_part = Part::new(sig_partname.clone(), DIGITAL_SIGNATURE_XML_CT, sig_xml);
        self.package.put_part(sig_part);

        // Add relationship from the origin to this signature
        let rel_target = sig_partname.relative_ref(origin_uri.base_uri());
        let origin_part = self.package.part_mut(&origin_uri).ok_or_else(|| {
            crate::error::PptxError::Package(crate::error::PackageError::PartNotFound(
                origin_uri.to_string(),
            ))
        })?;
        origin_part
            .rels
            .add_relationship(DIGITAL_SIGNATURE_RT, rel_target, false);

        Ok(())
    }

    /// Read all digital signatures from the presentation.
    ///
    /// Returns an empty `Vec` if the presentation has no signatures.
    /// # Errors
    ///
    /// Returns an error if signature parts cannot be read.
    pub fn digital_signatures(&self) -> PptxResult<Vec<DigitalSignature>> {
        let origin_uri = PackURI::new("/_xmlsignatures/origin.sigs")?;
        let Some(origin_part) = self.package.part(&origin_uri) else {
            return Ok(Vec::new());
        };

        let sig_rels = origin_part.rels.all_by_reltype(DIGITAL_SIGNATURE_RT);
        let mut signatures = Vec::with_capacity(sig_rels.len());

        for rel in sig_rels {
            let sig_partname = rel.target_partname(origin_part.partname.base_uri())?;
            let Some(sig_part) = self.package.part(&sig_partname) else {
                continue;
            };

            let sig = parse_signature_xml(&sig_part.blob)?;
            signatures.push(sig);
        }

        Ok(signatures)
    }
}

/// Parse a `DigitalSignature` from a signature XML part.
fn parse_signature_xml(xml_bytes: &[u8]) -> Result<DigitalSignature, crate::error::PptxError> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let xml = std::str::from_utf8(xml_bytes)?;
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut signer_name = String::new();
    let mut signer_email: Option<String> = None;
    let mut signer_title: Option<String> = None;
    let mut sign_time: Option<String> = None;
    let mut hash_algorithm = HashAlgorithm::Sha1;
    let mut commitment = SignatureCommitment::Created;

    // Track which element we are currently reading text for
    let mut current_element: Option<String> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let qn = e.name();
                let ln = local_name(qn.as_ref());
                match ln {
                    b"SignatureMethod" => {
                        for attr in e.attributes().flatten() {
                            let key = local_name(attr.key.as_ref());
                            if key == b"Algorithm" {
                                let val = String::from_utf8_lossy(&attr.value);
                                hash_algorithm = match val.as_ref() {
                                    s if s.contains("sha256") => HashAlgorithm::Sha256,
                                    s if s.contains("sha512") => HashAlgorithm::Sha512,
                                    _ => HashAlgorithm::Sha1,
                                };
                            }
                        }
                    }
                    b"SignerName" => current_element = Some("SignerName".to_string()),
                    b"SignerEmail" => current_element = Some("SignerEmail".to_string()),
                    b"SignerTitle" => current_element = Some("SignerTitle".to_string()),
                    b"Value" => current_element = Some("Value".to_string()),
                    b"CommitmentTypeId" => current_element = Some("CommitmentTypeId".to_string()),
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                let qn = e.name();
                let ln = local_name(qn.as_ref());
                if ln == b"SignatureMethod" {
                    for attr in e.attributes().flatten() {
                        let key = local_name(attr.key.as_ref());
                        if key == b"Algorithm" {
                            let val = String::from_utf8_lossy(&attr.value);
                            hash_algorithm = match val.as_ref() {
                                s if s.contains("sha256") => HashAlgorithm::Sha256,
                                s if s.contains("sha512") => HashAlgorithm::Sha512,
                                _ => HashAlgorithm::Sha1,
                            };
                        }
                    }
                }
            }
            Ok(Event::Text(ref t)) => {
                if let Some(ref elem) = current_element {
                    let text = t
                        .decode()
                        .map_err(|e| crate::error::PptxError::InvalidXml(e.to_string()))?
                        .into_owned();
                    match elem.as_str() {
                        "SignerName" => signer_name = text,
                        "SignerEmail" => signer_email = Some(text),
                        "SignerTitle" => signer_title = Some(text),
                        "Value" => sign_time = Some(text),
                        "CommitmentTypeId" => {
                            commitment = if text.ends_with("/approved") {
                                SignatureCommitment::Approved
                            } else if text.ends_with("/reviewed") {
                                SignatureCommitment::Reviewed
                            } else {
                                SignatureCommitment::Created
                            };
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::End(_)) => {
                current_element = None;
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(crate::error::PptxError::Xml(e)),
            _ => {}
        }
        buf.clear();
    }

    let mut signer = SignerInfo::new(signer_name);
    if let Some(email) = signer_email {
        signer = signer.with_email(email);
    }
    if let Some(title) = signer_title {
        signer = signer.with_title(title);
    }

    let mut sig = DigitalSignature::new(signer, hash_algorithm).with_commitment(commitment);
    if let Some(time) = sign_time {
        sig = sig.with_sign_time(time);
    }

    Ok(sig)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signature::{DigitalSignature, HashAlgorithm, SignatureCommitment, SignerInfo};

    #[test]
    fn test_add_and_read_digital_signature() {
        let mut prs = Presentation::new().unwrap();

        let signer = SignerInfo::new("Test User")
            .with_email("test@example.com")
            .with_title("Author");
        let sig = DigitalSignature::new(signer, HashAlgorithm::Sha256)
            .with_commitment(SignatureCommitment::Approved)
            .with_sign_time("2024-06-15T10:30:00Z");

        prs.add_digital_signature(&sig).unwrap();

        let signatures = prs.digital_signatures().unwrap();
        assert_eq!(signatures.len(), 1);

        let read_sig = &signatures[0];
        assert_eq!(read_sig.signer.name, "Test User");
        assert_eq!(read_sig.signer.email.as_deref(), Some("test@example.com"));
        assert_eq!(read_sig.signer.title.as_deref(), Some("Author"));
        assert_eq!(read_sig.hash_algorithm, HashAlgorithm::Sha256);
        assert_eq!(read_sig.commitment, SignatureCommitment::Approved);
        assert_eq!(read_sig.sign_time.as_deref(), Some("2024-06-15T10:30:00Z"));
    }

    #[test]
    fn test_no_signatures_returns_empty() {
        let prs = Presentation::new().unwrap();
        let signatures = prs.digital_signatures().unwrap();
        assert!(signatures.is_empty());
    }

    #[test]
    fn test_multiple_signatures() {
        let mut prs = Presentation::new().unwrap();

        let sig1 = DigitalSignature::new(SignerInfo::new("Alice"), HashAlgorithm::Sha1)
            .with_commitment(SignatureCommitment::Created);
        let sig2 = DigitalSignature::new(SignerInfo::new("Bob"), HashAlgorithm::Sha256)
            .with_commitment(SignatureCommitment::Reviewed);

        prs.add_digital_signature(&sig1).unwrap();
        prs.add_digital_signature(&sig2).unwrap();

        let signatures = prs.digital_signatures().unwrap();
        assert_eq!(signatures.len(), 2);
    }

    #[test]
    fn test_signature_round_trip_save_load() {
        let mut prs = Presentation::new().unwrap();

        let signer = SignerInfo::new("Round Trip User");
        let sig = DigitalSignature::new(signer, HashAlgorithm::Sha1)
            .with_sign_time("2024-01-01T00:00:00Z");

        prs.add_digital_signature(&sig).unwrap();

        // Save to bytes and reload
        let bytes = prs.to_bytes().unwrap();
        let prs2 = Presentation::from_bytes(&bytes).unwrap();

        let signatures = prs2.digital_signatures().unwrap();
        assert_eq!(signatures.len(), 1);
        assert_eq!(signatures[0].signer.name, "Round Trip User");
        assert_eq!(signatures[0].hash_algorithm, HashAlgorithm::Sha1);
        assert_eq!(
            signatures[0].sign_time.as_deref(),
            Some("2024-01-01T00:00:00Z")
        );
    }

    #[test]
    fn test_parse_signature_xml_basic() {
        let xml = r##"<?xml version="1.0" encoding="UTF-8"?><Signature xmlns="http://www.w3.org/2000/09/xmldsig#" Id="idSignature"><SignedInfo><CanonicalizationMethod Algorithm="http://www.w3.org/TR/2001/REC-xml-c14n-20010315"/><SignatureMethod Algorithm="http://www.w3.org/2001/04/xmlenc#sha256"/><Reference URI="/ppt/presentation.xml"><DigestMethod Algorithm="http://www.w3.org/2001/04/xmlenc#sha256"/><DigestValue/></Reference></SignedInfo><SignatureValue/><Object xmlns:mdssi="http://schemas.openxmlformats.org/package/2006/digital-signature"><SignatureProperties><SignatureProperty Id="idSignerName" Target="#idSignature"><mdssi:SignerName>Test</mdssi:SignerName></SignatureProperty></SignatureProperties></Object></Signature>"##;

        let sig = parse_signature_xml(xml.as_bytes()).unwrap();
        assert_eq!(sig.signer.name, "Test");
        assert_eq!(sig.hash_algorithm, HashAlgorithm::Sha256);
    }
}
