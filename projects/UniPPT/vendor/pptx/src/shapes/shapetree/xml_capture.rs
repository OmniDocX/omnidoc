//! XML capture types for recording raw XML sub-elements during event-based parsing.

use quick_xml::events::BytesStart;

/// Target of an XML capture operation.
#[derive(Debug)]
pub(super) enum CaptureTarget {
    SpPr,
    TxBody,
    Duotone,
}

/// Captures raw XML bytes for a sub-element during event-based parsing.
pub(super) struct XmlCapture {
    pub(super) target: CaptureTarget,
    pub(super) xml: Vec<u8>,
    pub(super) depth: u32,
}

impl XmlCapture {
    pub(super) const fn new(target: CaptureTarget) -> Self {
        Self {
            target,
            xml: Vec::new(),
            depth: 0,
        }
    }

    pub(super) fn push_start_with_tag(&mut self, tag: &str, e: &BytesStart<'_>) {
        self.xml.extend_from_slice(b"<");
        self.xml.extend_from_slice(tag.as_bytes());
        for attr in e.attributes().flatten() {
            self.xml.extend_from_slice(b" ");
            self.xml.extend_from_slice(attr.key.as_ref());
            self.xml.extend_from_slice(b"=\"");
            self.xml.extend_from_slice(&attr.value);
            self.xml.extend_from_slice(b"\"");
        }
        self.xml.extend_from_slice(b">");
    }

    pub(super) fn push_start(&mut self, e: &BytesStart<'_>) {
        self.xml.extend_from_slice(b"<");
        self.xml.extend_from_slice(e.name().as_ref());
        for attr in e.attributes().flatten() {
            self.xml.extend_from_slice(b" ");
            self.xml.extend_from_slice(attr.key.as_ref());
            self.xml.extend_from_slice(b"=\"");
            self.xml.extend_from_slice(&attr.value);
            self.xml.extend_from_slice(b"\"");
        }
        self.xml.extend_from_slice(b">");
    }

    pub(super) fn push_empty(&mut self, e: &BytesStart<'_>) {
        self.xml.extend_from_slice(b"<");
        self.xml.extend_from_slice(e.name().as_ref());
        for attr in e.attributes().flatten() {
            self.xml.extend_from_slice(b" ");
            self.xml.extend_from_slice(attr.key.as_ref());
            self.xml.extend_from_slice(b"=\"");
            self.xml.extend_from_slice(&attr.value);
            self.xml.extend_from_slice(b"\"");
        }
        self.xml.extend_from_slice(b"/>");
    }

    pub(super) fn push_end_raw(&mut self, name: &[u8]) {
        self.xml.extend_from_slice(b"</");
        self.xml.extend_from_slice(name);
        self.xml.extend_from_slice(b">");
    }

    pub(super) fn push_text(&mut self, text: &[u8]) {
        self.xml.extend_from_slice(text);
    }
}
