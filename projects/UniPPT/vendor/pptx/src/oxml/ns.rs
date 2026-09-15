//! XML namespace constants and utilities for OOXML.
//!
//! Not all constants are currently used — they are defined for completeness
//! of the OOXML namespace registry and may be needed as features are added.
#![allow(dead_code)]

use crate::error::PptxError;

/// All known `PowerPoint` XML namespace prefix-to-URI mappings.
pub const NSMAP: &[(&str, &str)] = &[
    ("a", NS_A),
    ("c", NS_C),
    ("cp", NS_CP),
    ("ct", NS_CT),
    ("dc", NS_DC),
    ("dcmitype", NS_DCMITYPE),
    ("dcterms", NS_DCTERMS),
    ("ep", NS_EP),
    ("i", NS_I),
    ("m", NS_M),
    ("mo", NS_MO),
    ("mv", NS_MV),
    ("o", NS_O),
    ("p", NS_P),
    ("pd", NS_PD),
    ("pic", NS_PIC),
    ("pr", NS_PR),
    ("r", NS_R),
    ("sl", NS_SL),
    ("v", NS_V),
    ("ve", NS_VE),
    ("w", NS_W),
    ("w10", NS_W10),
    ("wne", NS_WNE),
    ("wp", NS_WP),
    ("xsi", NS_XSI),
    ("p14", NS_P14),
    ("dgm", NS_DGM),
    ("mc", NS_MC),
];

// DrawingML main namespace
pub const NS_A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
// DrawingML chart namespace
pub const NS_C: &str = "http://schemas.openxmlformats.org/drawingml/2006/chart";
// Core properties namespace
pub const NS_CP: &str = "http://schemas.openxmlformats.org/package/2006/metadata/core-properties";
// Content types namespace
pub const NS_CT: &str = "http://schemas.openxmlformats.org/package/2006/content-types";
// Dublin Core elements namespace
pub const NS_DC: &str = "http://purl.org/dc/elements/1.1/";
// Dublin Core DCMI type namespace
pub const NS_DCMITYPE: &str = "http://purl.org/dc/dcmitype/";
// Dublin Core terms namespace
pub const NS_DCTERMS: &str = "http://purl.org/dc/terms/";
// Extended properties namespace
pub const NS_EP: &str = "http://schemas.openxmlformats.org/officeDocument/2006/extended-properties";
// Image relationship namespace
pub const NS_I: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";
// Office math namespace
pub const NS_M: &str = "http://schemas.openxmlformats.org/officeDocument/2006/math";
// Mac Office namespace
pub const NS_MO: &str = "http://schemas.microsoft.com/office/mac/office/2008/main";
// Mac VML namespace
pub const NS_MV: &str = "urn:schemas-microsoft-com:mac:vml";
// Office namespace
pub const NS_O: &str = "urn:schemas-microsoft-com:office:office";
// PresentationML main namespace
pub const NS_P: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
// Presentation drawing namespace
pub const NS_PD: &str = "http://schemas.openxmlformats.org/drawingml/2006/presentationDrawing";
// DrawingML picture namespace
pub const NS_PIC: &str = "http://schemas.openxmlformats.org/drawingml/2006/picture";
// Package relationships namespace
pub const NS_PR: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
// Office document relationships namespace
pub const NS_R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
// Slide layout relationship namespace
pub const NS_SL: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout";
// VML namespace
pub const NS_V: &str = "urn:schemas-microsoft-com:vml";
// Markup compatibility namespace
pub const NS_VE: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
// WordprocessingML namespace
pub const NS_W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
// Word 2003 namespace
pub const NS_W10: &str = "urn:schemas-microsoft-com:office:word";
// Word 2006 namespace
pub const NS_WNE: &str = "http://schemas.microsoft.com/office/word/2006/wordml";
// WordprocessingML drawing namespace
pub const NS_WP: &str = "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing";
// XML Schema instance namespace
pub const NS_XSI: &str = "http://www.w3.org/2001/XMLSchema-instance";
// PowerPoint 2010 namespace
pub const NS_P14: &str = "http://schemas.microsoft.com/office/powerpoint/2010/main";
// DrawingML diagram namespace
pub const NS_DGM: &str = "http://schemas.openxmlformats.org/drawingml/2006/diagram";
// Markup compatibility namespace (alias)
pub const NS_MC: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";

/// Look up the namespace URI for a given prefix.
///
/// Returns an error if the prefix is not in the namespace map.
pub fn nsuri(prefix: &str) -> Result<&'static str, PptxError> {
    NSMAP
        .iter()
        .find(|(pfx, _)| *pfx == prefix)
        .map(|(_, uri)| *uri)
        .ok_or_else(|| PptxError::InvalidXml(format!("unknown namespace prefix: {prefix:?}")))
}

/// Look up the prefix for a given namespace URI.
///
/// Returns `None` if the URI is not in the namespace map.
pub fn prefix_for(uri: &str) -> Option<&'static str> {
    NSMAP.iter().find(|(_, ns)| *ns == uri).map(|(pfx, _)| *pfx)
}

/// Resolve a namespace-prefixed tag (e.g. `"a:t"`) to Clark notation
/// (e.g. `"{http://schemas.openxmlformats.org/drawingml/2006/main}t"`).
///
/// Returns an error if the tag does not contain a `:` delimiter or uses an
/// unknown prefix.
pub fn qn(tag: &str) -> Result<String, PptxError> {
    let (prefix, local) = tag.split_once(':').ok_or_else(|| {
        PptxError::InvalidXml(format!("tag must contain a ':' delimiter: {tag:?}"))
    })?;
    let uri = nsuri(prefix)?;
    Ok(format!("{{{uri}}}{local}"))
}

/// Build an `xmlns` declarations string for the given prefixes.
///
/// Example: `nsdecls(&["a", "r"])` returns
/// `xmlns:a="http://..." xmlns:r="http://..."`.
///
/// Returns an error if any prefix is not in the namespace map.
pub fn nsdecls(prefixes: &[&str]) -> Result<String, PptxError> {
    let mut parts = Vec::with_capacity(prefixes.len());
    for pfx in prefixes {
        let uri = nsuri(pfx)?;
        parts.push(format!("xmlns:{pfx}=\"{uri}\""));
    }
    Ok(parts.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qn_resolves_prefixed_tag() {
        assert_eq!(
            qn("p:cSld").unwrap(),
            "{http://schemas.openxmlformats.org/presentationml/2006/main}cSld"
        );
        assert_eq!(
            qn("a:t").unwrap(),
            "{http://schemas.openxmlformats.org/drawingml/2006/main}t"
        );
        assert_eq!(
            qn("r:id").unwrap(),
            "{http://schemas.openxmlformats.org/officeDocument/2006/relationships}id"
        );
    }

    #[test]
    fn test_nsuri_known_prefix() {
        assert_eq!(
            nsuri("p").unwrap(),
            "http://schemas.openxmlformats.org/presentationml/2006/main"
        );
    }

    #[test]
    fn test_nsuri_unknown_prefix() {
        let err = nsuri("unknown").unwrap_err();
        assert!(err.to_string().contains("unknown namespace prefix"));
    }

    #[test]
    fn test_prefix_for_known_uri() {
        assert_eq!(
            prefix_for("http://schemas.openxmlformats.org/drawingml/2006/main"),
            Some("a")
        );
    }

    #[test]
    fn test_prefix_for_unknown_uri() {
        assert_eq!(prefix_for("http://unknown"), None);
    }

    #[test]
    fn test_nsdecls() {
        let result = nsdecls(&["a", "r"]).unwrap();
        assert!(result.contains("xmlns:a="));
        assert!(result.contains("xmlns:r="));
    }

    #[test]
    fn test_qn_error_on_missing_colon() {
        let err = qn("noprefix").unwrap_err();
        assert!(err.to_string().contains("tag must contain a ':' delimiter"));
    }

    #[test]
    fn test_qn_error_on_unknown_prefix() {
        let err = qn("zzz:tag").unwrap_err();
        assert!(err.to_string().contains("unknown namespace prefix"));
    }

    #[test]
    fn test_nsdecls_error_on_unknown_prefix() {
        let err = nsdecls(&["a", "zzz"]).unwrap_err();
        assert!(err.to_string().contains("unknown namespace prefix"));
    }
}
