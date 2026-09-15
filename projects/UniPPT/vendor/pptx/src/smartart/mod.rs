mod parser;

#[cfg(test)]
mod tests;

pub use parser::parse_smartart_nodes;

/// Raw `SmartArt` diagram parts read from the package.
///
/// `SmartArt` in OOXML is stored as multiple diagram parts under `/ppt/diagrams/`.
/// This struct holds the raw XML bytes for each part, enabling round-trip
/// preservation and basic text extraction via [`parse_smartart_nodes`].
#[derive(Debug, Clone)]
pub struct SmartArt {
    /// Raw diagram data XML (dataN.xml) -- the primary content.
    pub data_xml: Vec<u8>,
    /// Raw diagram colors XML, if present.
    pub colors_xml: Option<Vec<u8>>,
    /// Raw diagram style XML, if present.
    pub style_xml: Option<Vec<u8>>,
    /// Raw diagram layout definition XML, if present.
    pub layout_xml: Option<Vec<u8>>,
    /// Raw diagram drawing XML (rendered shapes), if present.
    pub drawing_xml: Option<Vec<u8>>,
}

/// A node in a `SmartArt` diagram, extracted from the diagram data XML.
///
/// The node tree mirrors the `<dgm:pt>` hierarchy found in
/// `<dgm:ptLst>` within the diagram data part.
#[derive(Debug, Clone, PartialEq)]
pub struct SmartArtNode {
    /// Text content of the node (concatenated `<a:t>` runs).
    pub text: String,
    /// Child nodes in the diagram hierarchy.
    pub children: Vec<SmartArtNode>,
}

/// Relationship types used by `SmartArt` diagram parts.
pub(crate) mod smartart_rel_type {
    pub const DIAGRAM_COLORS: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramColors";
    pub const DIAGRAM_STYLE: &str =
        "http://schemas.microsoft.com/office/2007/relationships/diagramStyle";
    pub const DIAGRAM_LAYOUT: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramLayout";
    pub const DIAGRAM_DRAWING: &str =
        "http://schemas.microsoft.com/office/2007/relationships/diagramDrawing";
}
