use crate::shapes::placeholder::PlaceholderFormat;
use crate::units::{Emu, ShapeId};

/// A graphic frame shape (`<p:graphicFrame>`).
///
/// Graphic frames host embedded objects such as tables, charts, and diagrams.
/// The actual content is in the `<a:graphic>` / `<a:graphicData>` child element.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphicFrame {
    pub shape_id: ShapeId,
    pub name: String,
    pub left: Emu,
    pub top: Emu,
    pub width: Emu,
    pub height: Emu,
    pub rotation: f64,
    /// Whether this graphic frame contains a table.
    pub has_table: bool,
    /// Whether this graphic frame contains a chart.
    pub has_chart: bool,
    /// The URI of the graphic data type.
    pub graphic_data_uri: Option<String>,
    /// Placeholder information, if this shape is a placeholder.
    pub placeholder: Option<PlaceholderFormat>,
    /// Relationship ID for the `SmartArt` diagram data part, if this frame
    /// contains a `SmartArt` diagram.
    pub smartart_r_id: Option<String>,
}

impl std::fmt::Display for GraphicFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GraphicFrame(\"{}\")", self.name)
    }
}

impl GraphicFrame {
    /// Returns `true` if this graphic frame contains a `SmartArt` diagram.
    #[must_use]
    pub fn has_smartart(&self) -> bool {
        self.smartart_r_id.is_some()
            || self.graphic_data_uri.as_deref() == Some(graphic_data_uri::DIAGRAM)
    }
}

/// Well-known graphic data URIs.
pub mod graphic_data_uri {
    pub const TABLE: &str = "http://schemas.openxmlformats.org/drawingml/2006/table";
    pub const CHART: &str = "http://schemas.openxmlformats.org/drawingml/2006/chart";
    pub const DIAGRAM: &str = "http://schemas.openxmlformats.org/drawingml/2006/diagram";
    pub const OLE: &str = "http://schemas.openxmlformats.org/presentationml/2006/ole";
}
