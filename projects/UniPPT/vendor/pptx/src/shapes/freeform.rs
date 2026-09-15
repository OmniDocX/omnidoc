//! Freeform shape builder for custom geometry paths.

use crate::xml_util::WriteXml;

/// A segment in a freeform path.
#[derive(Debug, Clone, PartialEq)]
enum FreeformSegment {
    LineTo(i64, i64),
    MoveTo(i64, i64),
    CurveTo(i64, i64, i64, i64, i64, i64),
    Close,
}

/// A builder for freeform (custom geometry) shapes.
///
/// Creates an `<a:custGeom>` element with a path list that can include
/// lines, moves, cubic bezier curves, and close operations.
#[derive(Debug, Clone, PartialEq)]
pub struct FreeformBuilder {
    start_x: i64,
    start_y: i64,
    segments: Vec<FreeformSegment>,
    width: i64,
    height: i64,
}

impl FreeformBuilder {
    /// Create a new freeform builder with the starting point and bounding dimensions.
    #[must_use]
    pub const fn new(start_x: i64, start_y: i64, width: i64, height: i64) -> Self {
        Self {
            start_x,
            start_y,
            segments: Vec::new(),
            width,
            height,
        }
    }

    /// Add a line-to segment.
    pub fn line_to(&mut self, x: i64, y: i64) -> &mut Self {
        self.segments.push(FreeformSegment::LineTo(x, y));
        self
    }

    /// Add a move-to segment.
    pub fn move_to(&mut self, x: i64, y: i64) -> &mut Self {
        self.segments.push(FreeformSegment::MoveTo(x, y));
        self
    }

    /// Add a cubic bezier curve-to segment.
    ///
    /// Parameters are: control point 1 (x1, y1), control point 2 (x2, y2),
    /// and end point (x, y).
    pub fn curve_to(&mut self, x1: i64, y1: i64, x2: i64, y2: i64, x: i64, y: i64) -> &mut Self {
        self.segments
            .push(FreeformSegment::CurveTo(x1, y1, x2, y2, x, y));
        self
    }

    /// Close the current sub-path.
    pub fn close(&mut self) -> &mut Self {
        self.segments.push(FreeformSegment::Close);
        self
    }

    /// Width of the DrawingML path coordinate space.
    #[must_use]
    pub const fn width(&self) -> i64 {
        self.width
    }

    /// Height of the DrawingML path coordinate space.
    #[must_use]
    pub const fn height(&self) -> i64 {
        self.height
    }

    /// Convert the DrawingML freeform commands to SVG path data.
    ///
    /// Both formats use absolute move/line/cubic commands, so this projection
    /// is lossless for the command subset represented by `FreeformBuilder`.
    #[must_use]
    pub fn to_svg_path_data(&self) -> String {
        use std::fmt::Write as _;

        let mut output = format!("M {} {}", self.start_x, self.start_y);
        for segment in &self.segments {
            match segment {
                FreeformSegment::LineTo(x, y) => {
                    let _ = write!(output, " L {x} {y}");
                }
                FreeformSegment::MoveTo(x, y) => {
                    let _ = write!(output, " M {x} {y}");
                }
                FreeformSegment::CurveTo(x1, y1, x2, y2, x, y) => {
                    let _ = write!(output, " C {x1} {y1} {x2} {y2} {x} {y}");
                }
                FreeformSegment::Close => output.push_str(" Z"),
            }
        }
        output
    }
}

impl WriteXml for FreeformBuilder {
    fn write_xml<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        w.write_str("<a:custGeom>")?;
        w.write_str("<a:avLst/>")?;
        w.write_str("<a:gdLst/>")?;
        w.write_str("<a:ahLst/>")?;
        w.write_str("<a:cxnLst/>")?;
        w.write_str("<a:rect l=\"l\" t=\"t\" r=\"r\" b=\"b\"/>")?;
        w.write_str("<a:pathLst>")?;
        write!(w, r#"<a:path w="{}" h="{}">"#, self.width, self.height)?;

        // Starting moveTo
        write!(
            w,
            r#"<a:moveTo><a:pt x="{}" y="{}"/></a:moveTo>"#,
            self.start_x, self.start_y
        )?;

        for seg in &self.segments {
            match seg {
                FreeformSegment::LineTo(x, y) => {
                    write!(w, r#"<a:lnTo><a:pt x="{x}" y="{y}"/></a:lnTo>"#)?;
                }
                FreeformSegment::MoveTo(x, y) => {
                    write!(w, r#"<a:moveTo><a:pt x="{x}" y="{y}"/></a:moveTo>"#)?;
                }
                FreeformSegment::CurveTo(x1, y1, x2, y2, x, y) => {
                    write!(
                        w,
                        r#"<a:cubicBezTo><a:pt x="{x1}" y="{y1}"/><a:pt x="{x2}" y="{y2}"/><a:pt x="{x}" y="{y}"/></a:cubicBezTo>"#
                    )?;
                }
                FreeformSegment::Close => {
                    w.write_str("<a:close/>")?;
                }
            }
        }

        w.write_str("</a:path>")?;
        w.write_str("</a:pathLst>")?;
        w.write_str("</a:custGeom>")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_freeform_triangle() {
        let mut builder = FreeformBuilder::new(0, 100, 200, 100);
        builder.line_to(100, 0).line_to(200, 100).close();

        let xml = builder.to_xml_string();
        assert!(xml.starts_with("<a:custGeom>"));
        assert!(xml.contains("<a:pathLst>"));
        assert!(xml.contains(r#"<a:path w="200" h="100">"#));
        assert!(xml.contains(r#"<a:moveTo><a:pt x="0" y="100"/></a:moveTo>"#));
        assert!(xml.contains(r#"<a:lnTo><a:pt x="100" y="0"/></a:lnTo>"#));
        assert!(xml.contains(r#"<a:lnTo><a:pt x="200" y="100"/></a:lnTo>"#));
        assert!(xml.contains("<a:close/>"));
        assert!(xml.ends_with("</a:custGeom>"));
    }

    #[test]
    fn test_freeform_with_curve() {
        let mut builder = FreeformBuilder::new(0, 0, 500, 500);
        builder.curve_to(100, 200, 300, 200, 500, 0);

        let xml = builder.to_xml_string();
        assert!(xml.contains("<a:cubicBezTo>"));
        assert!(xml.contains(r#"<a:pt x="100" y="200"/>"#));
        assert!(xml.contains(r#"<a:pt x="300" y="200"/>"#));
        assert!(xml.contains(r#"<a:pt x="500" y="0"/>"#));
    }

    #[test]
    fn test_freeform_move_to() {
        let mut builder = FreeformBuilder::new(0, 0, 100, 100);
        builder.line_to(50, 50).move_to(75, 0).line_to(100, 50);

        let xml = builder.to_xml_string();
        // Should have two moveTo elements (initial + explicit)
        let move_to_count = xml.matches("<a:moveTo>").count();
        assert_eq!(move_to_count, 2);
    }

    #[test]
    fn test_freeform_svg_path_data() {
        let mut builder = FreeformBuilder::new(0, 100, 200, 100);
        builder
            .line_to(100, 0)
            .curve_to(120, 10, 180, 50, 200, 100)
            .close();

        assert_eq!(
            builder.to_svg_path_data(),
            "M 0 100 L 100 0 C 120 10 180 50 200 100 Z"
        );
        assert_eq!(builder.width(), 200);
        assert_eq!(builder.height(), 100);
    }
}
