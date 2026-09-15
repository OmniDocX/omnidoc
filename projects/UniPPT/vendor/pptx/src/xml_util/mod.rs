//! Shared XML utilities used across the crate.
//!
//! Contains the `WriteXml` trait for types that serialize to XML, XML escaping
//! helpers, and common XML parsing helpers used by multiple modules.

mod escape;

pub use escape::{write_xml_escaped, xml_escape, xml_escape_char};

use std::borrow::Cow;
use std::fmt;

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::error::{PptxError, PptxResult};

/// Trait for types that can serialize themselves as XML.
///
/// Implementors provide [`write_xml`](WriteXml::write_xml) to write their XML
/// representation into any `fmt::Write` destination.  The trait provides a
/// default [`to_xml_string`](WriteXml::to_xml_string) that writes into a
/// `String`.
pub trait WriteXml {
    /// Write the XML representation of this value into `w`.
    ///
    /// # Errors
    ///
    /// Returns `fmt::Error` if the underlying writer fails.
    fn write_xml<W: fmt::Write>(&self, w: &mut W) -> fmt::Result;

    /// Convenience method: serialize to a new `String`.
    fn to_xml_string(&self) -> String {
        let mut s = String::new();
        // SAFETY: fmt::Write for String is infallible; writing to a String never returns Err.
        self.write_xml(&mut s)
            .unwrap_or_else(|_| unreachable!("fmt::Write for String is infallible"));
        s
    }
}

// ============================================================
// XML parsing helpers
// ============================================================

/// Extract the local name from a possibly namespace-prefixed `QName`.
///
/// e.g. `p:sldIdLst` -> `sldIdLst`, `sldIdLst` -> `sldIdLst`.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(local_name(b"p:sldIdLst"), b"sldIdLst");
/// ```
#[inline]
pub fn local_name(name: &[u8]) -> &[u8] {
    name.iter()
        .position(|&b| b == b':')
        .map_or(name, |pos| &name[pos + 1..])
}

/// Like [`local_name`], but returns `&str` (empty string on invalid UTF-8).
#[inline]
pub fn local_name_str(name: &[u8]) -> &str {
    std::str::from_utf8(local_name(name)).unwrap_or("")
}

/// Like [`local_name`], but returns an owned `String`.
#[inline]
pub fn local_name_owned(name: &[u8]) -> String {
    let ln = local_name(name);
    String::from_utf8_lossy(ln).into_owned()
}

/// Get an attribute value by key, matching either the full key or its local
/// name (i.e. ignoring namespace prefix on the attribute key).
///
/// This is the most commonly used variant: it matches `key` against both the
/// full attribute name and the local part after any `:` prefix.
///
/// # Errors
///
/// Returns `PptxError::XmlAttr` if any attribute in the element is malformed.
#[inline]
pub fn attr_value<'a>(e: &'a BytesStart<'_>, key: &[u8]) -> PptxResult<Option<Cow<'a, str>>> {
    for attr_result in e.attributes() {
        let attr = attr_result.map_err(PptxError::XmlAttr)?;
        let k = attr.key.as_ref();
        if k == key || local_name(k) == key {
            let value = match attr.value {
                Cow::Borrowed(bytes) => Some(Cow::Borrowed(
                    std::str::from_utf8(bytes).map_err(PptxError::Utf8Str)?,
                )),
                Cow::Owned(vec) => {
                    Some(Cow::Owned(String::from_utf8(vec).map_err(PptxError::Utf8)?))
                }
            };
            return Ok(value);
        }
    }
    Ok(None)
}

/// Get an attribute value where only the local name of the attribute key
/// matches (for namespaced attributes like `r:embed`).
///
/// # Errors
///
/// Returns `PptxError::XmlAttr` if any attribute in the element is malformed.
#[inline]
pub fn attr_value_ns<'a>(
    e: &'a BytesStart<'_>,
    local_key: &[u8],
) -> PptxResult<Option<Cow<'a, str>>> {
    for attr_result in e.attributes() {
        let attr = attr_result.map_err(PptxError::XmlAttr)?;
        if local_name(attr.key.as_ref()) == local_key {
            let value = match attr.value {
                Cow::Borrowed(bytes) => Some(Cow::Borrowed(
                    std::str::from_utf8(bytes).map_err(PptxError::Utf8Str)?,
                )),
                Cow::Owned(vec) => {
                    Some(Cow::Owned(String::from_utf8(vec).map_err(PptxError::Utf8)?))
                }
            };
            return Ok(value);
        }
    }
    Ok(None)
}

/// Read all inner XML of the currently-open element as raw bytes.
///
/// The reader must be positioned just after the `Start` event of the element.
/// `end_tag` is the local name of the element (used only for verification).
#[inline]
pub fn read_inner_xml(
    reader: &mut Reader<&[u8]>,
    end_tag: &str,
) -> Result<Vec<u8>, quick_xml::Error> {
    let mut inner = Vec::new();
    let mut buf = Vec::new();
    let mut depth = 1u32;

    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                depth += 1;
                inner.extend_from_slice(b"<");
                inner.extend_from_slice(e.name().as_ref());
                for attr_result in e.attributes() {
                    let attr = attr_result?;
                    inner.extend_from_slice(b" ");
                    inner.extend_from_slice(attr.key.as_ref());
                    inner.extend_from_slice(b"=\"");
                    inner.extend_from_slice(&attr.value);
                    inner.extend_from_slice(b"\"");
                }
                inner.extend_from_slice(b">");
            }
            Ok(Event::Empty(ref e)) => {
                inner.extend_from_slice(b"<");
                inner.extend_from_slice(e.name().as_ref());
                for attr_result in e.attributes() {
                    let attr = attr_result?;
                    inner.extend_from_slice(b" ");
                    inner.extend_from_slice(attr.key.as_ref());
                    inner.extend_from_slice(b"=\"");
                    inner.extend_from_slice(&attr.value);
                    inner.extend_from_slice(b"\"");
                }
                inner.extend_from_slice(b"/>");
            }
            Ok(Event::End(ref e)) => {
                depth -= 1;
                if depth == 0 {
                    let qn = e.name();
                    let local = local_name_str(qn.as_ref());
                    if local == end_tag {
                        break;
                    }
                }
                inner.extend_from_slice(b"</");
                inner.extend_from_slice(e.name().as_ref());
                inner.extend_from_slice(b">");
            }
            Ok(Event::Text(ref t)) => {
                inner.extend_from_slice(t.as_ref());
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e),
            _ => {}
        }
    }

    Ok(inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- local_name tests ---

    #[test]
    fn local_name_with_namespace_prefix() {
        assert_eq!(local_name(b"a:body"), b"body");
    }

    #[test]
    fn local_name_without_namespace() {
        assert_eq!(local_name(b"name"), b"name");
    }

    #[test]
    fn local_name_with_p_prefix() {
        assert_eq!(local_name(b"p:sp"), b"sp");
    }

    #[test]
    fn local_name_empty_input() {
        assert_eq!(local_name(b""), b"");
    }

    // --- local_name_str tests ---

    #[test]
    fn local_name_str_with_namespace() {
        assert_eq!(local_name_str(b"p:sldIdLst"), "sldIdLst");
    }

    #[test]
    fn local_name_str_without_namespace() {
        assert_eq!(local_name_str(b"sldIdLst"), "sldIdLst");
    }

    // --- attr_value tests ---

    #[test]
    fn attr_value_from_bytes_start() {
        let xml = br#"<elem foo="bar" ns:baz="qux"/>"#;
        let mut reader = Reader::from_reader(&xml[..]);
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) => {
                    // Test full key match
                    let val = attr_value(e, b"foo").unwrap();
                    assert_eq!(val.as_deref(), Some("bar"));

                    // Test namespace-prefixed key (match on local name "baz")
                    let val2 = attr_value(e, b"baz").unwrap();
                    assert_eq!(val2.as_deref(), Some("qux"));

                    // Test missing key
                    let val3 = attr_value(e, b"missing").unwrap();
                    assert!(val3.is_none());
                    break;
                }
                Ok(Event::Eof) => break,
                _ => {}
            }
        }
    }
}
