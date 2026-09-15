#[cfg(test)]
mod tests;

use crate::dml::line::LineFormat;
use crate::enums::shapes::PresetGeometry;
use crate::units::{ConnectionPointIndex, Emu, ShapeId};
use crate::xml_util::{xml_escape, WriteXml};

/// A connector shape (`<p:cxnSp>`).
///
/// Connectors are lines that link two shapes. They have begin/end
/// connection points but no text frame.
#[derive(Debug, Clone, PartialEq)]
pub struct Connector {
    pub shape_id: ShapeId,
    pub name: String,
    pub left: Emu,
    pub top: Emu,
    pub width: Emu,
    pub height: Emu,
    pub rotation: f64,
    /// Whether the connector is flipped horizontally.
    pub flip_h: bool,
    /// Whether the connector is flipped vertically.
    pub flip_v: bool,
    /// The preset geometry type (e.g. `Line`, `BentConnector3`).
    pub prst_geom: Option<PresetGeometry>,
    /// Line (outline) formatting for the connector.
    pub line: Option<LineFormat>,
    /// Shape ID of the shape the connector begins at.
    pub begin_shape_id: Option<ShapeId>,
    /// Connection point index on the begin shape.
    pub begin_cxn_idx: Option<ConnectionPointIndex>,
    /// Shape ID of the shape the connector ends at.
    pub end_shape_id: Option<ShapeId>,
    /// Connection point index on the end shape.
    pub end_cxn_idx: Option<ConnectionPointIndex>,
}

impl Connector {
    /// Create a new `Connector` with the given position and size.
    ///
    /// All optional fields default to `None` or their zero-value equivalents.
    pub fn new(
        shape_id: ShapeId,
        name: impl Into<String>,
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
    ) -> Self {
        Self {
            shape_id,
            name: name.into(),
            left,
            top,
            width,
            height,
            rotation: 0.0,
            flip_h: false,
            flip_v: false,
            prst_geom: None,
            line: None,
            begin_shape_id: None,
            begin_cxn_idx: None,
            end_shape_id: None,
            end_cxn_idx: None,
        }
    }

    /// Create a line connector between two points.
    pub fn line(
        shape_id: ShapeId,
        name: impl Into<String>,
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
    ) -> Self {
        let mut conn = Self::new(shape_id, name, left, top, width, height);
        conn.prst_geom = Some(PresetGeometry::Line);
        conn
    }

    /// Set the begin connection to a shape at the given connection point.
    pub fn set_begin_connection(&mut self, shape_id: ShapeId, cxn_pt_idx: ConnectionPointIndex) {
        self.begin_shape_id = Some(shape_id);
        self.begin_cxn_idx = Some(cxn_pt_idx);
    }

    /// Set the end connection to a shape at the given connection point.
    pub fn set_end_connection(&mut self, shape_id: ShapeId, cxn_pt_idx: ConnectionPointIndex) {
        self.end_shape_id = Some(shape_id);
        self.end_cxn_idx = Some(cxn_pt_idx);
    }

    /// Set the line formatting.
    pub fn set_line(&mut self, line: LineFormat) {
        self.line = Some(line);
    }

    /// Get the X coordinate of the begin point of the connector.
    ///
    /// When `flip_h` is false, the begin point is at `left`.
    /// When `flip_h` is true, the begin point is at `left + width`.
    pub const fn begin_x(&self) -> Emu {
        if self.flip_h {
            Emu(self.left.0 + self.width.0)
        } else {
            self.left
        }
    }

    /// Get the Y coordinate of the begin point of the connector.
    ///
    /// When `flip_v` is false, the begin point is at `top`.
    /// When `flip_v` is true, the begin point is at `top + height`.
    pub const fn begin_y(&self) -> Emu {
        if self.flip_v {
            Emu(self.top.0 + self.height.0)
        } else {
            self.top
        }
    }

    /// Get the X coordinate of the end point of the connector.
    ///
    /// When `flip_h` is false, the end point is at `left + width`.
    /// When `flip_h` is true, the end point is at `left`.
    pub const fn end_x(&self) -> Emu {
        if self.flip_h {
            self.left
        } else {
            Emu(self.left.0 + self.width.0)
        }
    }

    /// Get the Y coordinate of the end point of the connector.
    ///
    /// When `flip_v` is false, the end point is at `top + height`.
    /// When `flip_v` is true, the end point is at `top`.
    pub const fn end_y(&self) -> Emu {
        if self.flip_v {
            self.top
        } else {
            Emu(self.top.0 + self.height.0)
        }
    }
}

impl std::fmt::Display for Connector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Connector(\"{}\")", self.name)
    }
}

impl WriteXml for Connector {
    fn write_xml<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        w.write_str("<p:cxnSp>")?;

        // --- nvCxnSpPr ---
        w.write_str("<p:nvCxnSpPr>")?;
        write!(
            w,
            r#"<p:cNvPr id="{}" name="{}"/>"#,
            self.shape_id,
            xml_escape(&self.name)
        )?;

        // cNvCxnSpPr with optional connection references
        let has_connections = self.begin_shape_id.is_some() || self.end_shape_id.is_some();
        if has_connections {
            w.write_str("<p:cNvCxnSpPr>")?;
            if let (Some(sid), Some(idx)) = (self.begin_shape_id, self.begin_cxn_idx) {
                write!(w, r#"<a:stCxn id="{sid}" idx="{idx}"/>"#)?;
            }
            if let (Some(sid), Some(idx)) = (self.end_shape_id, self.end_cxn_idx) {
                write!(w, r#"<a:endCxn id="{sid}" idx="{idx}"/>"#)?;
            }
            w.write_str("</p:cNvCxnSpPr>")?;
        } else {
            w.write_str("<p:cNvCxnSpPr/>")?;
        }

        w.write_str("<p:nvPr/>")?;
        w.write_str("</p:nvCxnSpPr>")?;

        // --- spPr ---
        w.write_str("<p:spPr>")?;

        // Transform
        w.write_str("<a:xfrm")?;
        if self.rotation != 0.0 {
            // f64→i64: rotation degrees * 60000 fits in i64
            #[allow(clippy::cast_possible_truncation)]
            let rot = (self.rotation * 60000.0) as i64;
            write!(w, r#" rot="{rot}""#)?;
        }
        if self.flip_h {
            w.write_str(r#" flipH="1""#)?;
        }
        if self.flip_v {
            w.write_str(r#" flipV="1""#)?;
        }
        w.write_char('>')?;
        write!(w, r#"<a:off x="{}" y="{}"/>"#, self.left.0, self.top.0)?;
        write!(
            w,
            r#"<a:ext cx="{}" cy="{}"/>"#,
            self.width.0, self.height.0
        )?;
        w.write_str("</a:xfrm>")?;

        // Geometry
        if let Some(ref prst) = self.prst_geom {
            write!(w, r#"<a:prstGeom prst="{prst}"><a:avLst/></a:prstGeom>"#)?;
        }

        // Line
        if let Some(ref line) = self.line {
            line.write_xml(w)?;
        }

        w.write_str("</p:spPr>")?;
        w.write_str("</p:cxnSp>")?;
        Ok(())
    }
}
