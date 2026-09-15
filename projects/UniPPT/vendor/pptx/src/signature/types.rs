//! Core types for digital signatures: enums and structs.

/// The hash algorithm used for digest computation.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    /// **Warning**: SHA-1 is cryptographically broken. Use SHA-256 or stronger for new signatures.
    Sha1,
    Sha256,
    Sha512,
}

impl HashAlgorithm {
    /// Return the W3C XML Signature algorithm URI for this hash algorithm.
    #[must_use]
    pub const fn algorithm_uri(&self) -> &'static str {
        match self {
            Self::Sha1 => "http://www.w3.org/2000/09/xmldsig#sha1",
            Self::Sha256 => "http://www.w3.org/2001/04/xmlenc#sha256",
            Self::Sha512 => "http://www.w3.org/2001/04/xmlenc#sha512",
        }
    }

    /// Return the algorithm name as used in XML attributes.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Sha1 => "SHA1",
            Self::Sha256 => "SHA256",
            Self::Sha512 => "SHA512",
        }
    }
}

/// The commitment type for the signature.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureCommitment {
    Created,
    Approved,
    Reviewed,
}

impl SignatureCommitment {
    /// Return the OPC signature commitment type URI.
    #[must_use]
    pub const fn commitment_uri(&self) -> &'static str {
        match self {
            Self::Created => {
                "http://schemas.openxmlformats.org/package/2006/RelationshipTransform/opc-SignatureOrigin/created"
            }
            Self::Approved => {
                "http://schemas.openxmlformats.org/package/2006/RelationshipTransform/opc-SignatureOrigin/approved"
            }
            Self::Reviewed => {
                "http://schemas.openxmlformats.org/package/2006/RelationshipTransform/opc-SignatureOrigin/reviewed"
            }
        }
    }
}

/// Information about the person who created the signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignerInfo {
    /// The signer's display name.
    pub name: String,
    /// The signer's email address.
    pub email: Option<String>,
    /// The signer's title or role.
    pub title: Option<String>,
}

impl SignerInfo {
    /// Create a new `SignerInfo` with only a name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            email: None,
            title: None,
        }
    }

    /// Builder method: set the email address.
    #[must_use]
    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Builder method: set the title.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

/// Represents the metadata structure for a digital signature part.
///
/// Generates a W3C XML Signature skeleton within the `_xmlsignatures/` directory of the OPC
/// package. **Does not perform cryptographic signing** — `DigestValue` and `SignatureValue` are
/// empty placeholders. To produce a valid signature, use an external signing tool or ceremony
/// after generating the PPTX.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigitalSignature {
    /// Information about the signer.
    pub signer: SignerInfo,
    /// The hash algorithm for digest computation.
    pub hash_algorithm: HashAlgorithm,
    /// Optional ISO 8601 datetime string for when the signature was created.
    pub sign_time: Option<String>,
    /// The commitment type for the signature.
    pub commitment: SignatureCommitment,
}

impl DigitalSignature {
    /// Create a new digital signature with the given signer and algorithm.
    #[must_use]
    pub const fn new(signer: SignerInfo, algorithm: HashAlgorithm) -> Self {
        Self {
            signer,
            hash_algorithm: algorithm,
            sign_time: None,
            commitment: SignatureCommitment::Created,
        }
    }

    /// Builder method: set the commitment type.
    #[must_use]
    pub const fn with_commitment(mut self, commitment: SignatureCommitment) -> Self {
        self.commitment = commitment;
        self
    }

    /// Builder method: set the signing time.
    #[must_use]
    pub fn with_sign_time(mut self, time: impl Into<String>) -> Self {
        self.sign_time = Some(time.into());
        self
    }
}
