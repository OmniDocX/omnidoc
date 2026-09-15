//! XML serialisation for digital signatures and OPC package constants.

use std::fmt;

use crate::xml_util::{xml_escape, WriteXml};

use super::types::DigitalSignature;

/// XML namespace for W3C XML Signatures.
const XMLDSIG_NS: &str = "http://www.w3.org/2000/09/xmldsig#";

/// XML namespace for OPC digital signatures.
const OPC_DIGSIG_NS: &str = "http://schemas.openxmlformats.org/package/2006/digital-signature";

/// Content type for XML digital signature origin.
pub const DIGITAL_SIGNATURE_ORIGIN_CT: &str =
    "application/vnd.openxmlformats-package.digital-signature-origin";

/// Content type for XML digital signature parts.
pub const DIGITAL_SIGNATURE_XML_CT: &str =
    "application/vnd.openxmlformats-package.digital-signature-xmlsignature+xml";

/// Relationship type for the digital signature origin.
pub const DIGITAL_SIGNATURE_ORIGIN_RT: &str =
    "http://schemas.openxmlformats.org/package/2006/relationships/digital-signature/origin";

/// Relationship type from origin to individual signature parts.
pub const DIGITAL_SIGNATURE_RT: &str =
    "http://schemas.openxmlformats.org/package/2006/relationships/digital-signature/signature";

impl WriteXml for DigitalSignature {
    /// Writes the structural W3C XML Signature document for this metadata.
    ///
    /// **Important**: This method produces metadata/structural XML only. `DigestValue` and
    /// `SignatureValue` are empty placeholders — no cryptographic computation is performed.
    /// To produce a cryptographically valid signature, pass the generated PPTX through an
    /// external signing tool or ceremony after saving.
    fn write_xml<W: fmt::Write>(&self, w: &mut W) -> fmt::Result {
        // Open <Signature> element
        write!(w, r#"<Signature xmlns="{XMLDSIG_NS}" Id="idSignature">"#,)?;

        // <SignedInfo>
        write!(w, "<SignedInfo>")?;
        write!(
            w,
            r#"<CanonicalizationMethod Algorithm="http://www.w3.org/TR/2001/REC-xml-c14n-20010315"/>"#,
        )?;
        write!(
            w,
            r#"<SignatureMethod Algorithm="{}"/>"#,
            self.hash_algorithm.algorithm_uri(),
        )?;

        // Reference to the presentation part
        write!(w, r#"<Reference URI="/ppt/presentation.xml">"#,)?;
        write!(
            w,
            r#"<DigestMethod Algorithm="{}"/>"#,
            self.hash_algorithm.algorithm_uri(),
        )?;
        // NOTE: Placeholder only. Cryptographic signing requires an external signing ceremony.
        write!(w, "<DigestValue/>")?;
        write!(w, "</Reference>")?;

        write!(w, "</SignedInfo>")?;

        // NOTE: Placeholder only. Cryptographic signing requires an external signing ceremony.
        write!(w, "<SignatureValue/>")?;

        // <Object> with signature properties
        write!(w, r#"<Object xmlns:mdssi="{OPC_DIGSIG_NS}">"#,)?;

        // <SignatureProperties>
        write!(w, "<SignatureProperties>")?;

        // Signer name property
        write!(
            w,
            r##"<SignatureProperty Id="idSignerName" Target="#idSignature">"##,
        )?;
        write!(
            w,
            "<mdssi:SignerName>{}</mdssi:SignerName>",
            xml_escape(&self.signer.name),
        )?;
        write!(w, "</SignatureProperty>")?;

        // Signer email property (if present)
        if let Some(ref email) = self.signer.email {
            write!(
                w,
                r##"<SignatureProperty Id="idSignerEmail" Target="#idSignature">"##,
            )?;
            write!(
                w,
                "<mdssi:SignerEmail>{}</mdssi:SignerEmail>",
                xml_escape(email),
            )?;
            write!(w, "</SignatureProperty>")?;
        }

        // Signer title property (if present)
        if let Some(ref title) = self.signer.title {
            write!(
                w,
                r##"<SignatureProperty Id="idSignerTitle" Target="#idSignature">"##,
            )?;
            write!(
                w,
                "<mdssi:SignerTitle>{}</mdssi:SignerTitle>",
                xml_escape(title),
            )?;
            write!(w, "</SignatureProperty>")?;
        }

        // Sign time property (if present)
        if let Some(ref sign_time) = self.sign_time {
            write!(
                w,
                r##"<SignatureProperty Id="idSignTime" Target="#idSignature">"##,
            )?;
            write!(
                w,
                "<mdssi:SignatureTime><mdssi:Format>YYYY-MM-DDThh:mm:ssTZD</mdssi:Format><mdssi:Value>{}</mdssi:Value></mdssi:SignatureTime>",
                xml_escape(sign_time),
            )?;
            write!(w, "</SignatureProperty>")?;
        }

        // Commitment type property
        write!(
            w,
            r##"<SignatureProperty Id="idCommitment" Target="#idSignature">"##,
        )?;
        write!(
            w,
            r"<mdssi:CommitmentTypeIndication><mdssi:CommitmentTypeId>{}</mdssi:CommitmentTypeId></mdssi:CommitmentTypeIndication>",
            self.commitment.commitment_uri(),
        )?;
        write!(w, "</SignatureProperty>")?;

        write!(w, "</SignatureProperties>")?;
        write!(w, "</Object>")?;

        write!(w, "</Signature>")?;
        Ok(())
    }
}

/// Generate the full XML document for a signature part.
pub(crate) fn signature_to_xml(sig: &DigitalSignature) -> Vec<u8> {
    let mut xml = String::with_capacity(2048);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    sig.write_xml(&mut xml)
        .unwrap_or_else(|_| unreachable!("fmt::Write for String is infallible"));
    xml.into_bytes()
}
