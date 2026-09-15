//! OLE object embedding support.
//!
//! OLE objects are embedded via `<p:graphicFrame>` elements with an
//! OLE-specific graphic data URI.

use crate::enums::misc::ProgId;
use crate::units::{Emu, RelationshipId, ShapeId};
use crate::xml_util::{xml_escape, WriteXml};

/// An embedded OLE object shape.
///
/// OLE objects appear as `<p:graphicFrame>` elements in the slide XML,
/// with `<a:graphicData>` using the OLE URI.
#[derive(Debug, Clone, PartialEq)]
pub struct OleObject {
    /// The shape ID within the slide.
    pub shape_id: ShapeId,
    /// The display name.
    pub name: String,
    /// Left position in EMU.
    pub left: Emu,
    /// Top position in EMU.
    pub top: Emu,
    /// Width in EMU.
    pub width: Emu,
    /// Height in EMU.
    pub height: Emu,
    /// Rotation in degrees.
    pub rotation: f64,
    /// The OLE program identifier.
    pub prog_id: ProgId,
    /// The embedded file data.
    pub data: Vec<u8>,
    /// The relationship ID for the embedded OLE object part.
    pub r_id: Option<RelationshipId>,
    /// The relationship ID for the icon/preview image.
    pub icon_r_id: Option<RelationshipId>,
}

impl OleObject {
    /// Create a new OLE object.
    #[must_use]
    pub const fn new(
        prog_id: ProgId,
        data: Vec<u8>,
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
    ) -> Self {
        Self {
            shape_id: ShapeId(0),
            name: String::new(),
            left,
            top,
            width,
            height,
            rotation: 0.0,
            prog_id,
            data,
            r_id: None,
            icon_r_id: None,
        }
    }
}

impl std::fmt::Display for OleObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OleObject(\"{}\")", self.name)
    }
}

impl WriteXml for OleObject {
    fn write_xml<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        w.write_str("<p:graphicFrame>")?;

        // --- nvGraphicFramePr ---
        w.write_str("<p:nvGraphicFramePr>")?;
        write!(
            w,
            r#"<p:cNvPr id="{}" name="{}"/>"#,
            self.shape_id,
            xml_escape(&self.name)
        )?;
        w.write_str(
            r#"<p:cNvGraphicFramePr><a:graphicFrameLocks noGrp="1" noChangeAspect="1"/></p:cNvGraphicFramePr>"#,
        )?;
        w.write_str("<p:nvPr/>")?;
        w.write_str("</p:nvGraphicFramePr>")?;

        // --- xfrm ---
        w.write_str("<p:xfrm")?;
        if self.rotation != 0.0 {
            // f64→i64: rotation degrees * 60000 fits in i64
            #[allow(clippy::cast_possible_truncation)]
            let rot = (self.rotation * 60000.0) as i64;
            write!(w, r#" rot="{rot}""#)?;
        }
        w.write_char('>')?;
        write!(w, r#"<a:off x="{}" y="{}"/>"#, self.left.0, self.top.0)?;
        write!(
            w,
            r#"<a:ext cx="{}" cy="{}"/>"#,
            self.width.0, self.height.0
        )?;
        w.write_str("</p:xfrm>")?;

        // --- graphic ---
        w.write_str("<a:graphic>")?;
        w.write_str(
            r#"<a:graphicData uri="http://schemas.openxmlformats.org/presentationml/2006/ole">"#,
        )?;

        // OLE object element
        write!(w, r#"<p:oleObj progId="{}""#, self.prog_id.to_prog_id_str())?;
        if let Some(ref r_id) = self.r_id {
            write!(w, r#" r:id="{}""#, xml_escape(r_id.as_ref()))?;
        }
        if let Some(ref icon_r_id) = self.icon_r_id {
            write!(w, r#" imgW="{}" imgH="{}""#, self.width.0, self.height.0)?;
            w.write_char('>')?;
            write!(
                w,
                r#"<p:embed/><p:pic><p:nvPicPr><p:cNvPr id="0" name=""/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="{}"/><a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr><a:xfrm><a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr></p:pic>"#,
                xml_escape(icon_r_id.as_ref()),
                self.left.0,
                self.top.0,
                self.width.0,
                self.height.0
            )?;
            w.write_str("</p:oleObj>")?;
        } else {
            w.write_str("><p:embed/></p:oleObj>")?;
        }

        w.write_str("</a:graphicData>")?;
        w.write_str("</a:graphic>")?;
        w.write_str("</p:graphicFrame>")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ole_object_new() {
        let ole = OleObject::new(
            ProgId::ExcelWorksheet,
            vec![1, 2, 3],
            Emu(914400),
            Emu(914400),
            Emu(1828800),
            Emu(1371600),
        );
        assert_eq!(ole.prog_id, ProgId::ExcelWorksheet);
        assert_eq!(ole.data, vec![1, 2, 3]);
        assert_eq!(ole.left, Emu(914400));
    }

    #[test]
    fn test_ole_object_xml() {
        let mut ole = OleObject::new(
            ProgId::ExcelWorksheet,
            vec![],
            Emu(914400),
            Emu(914400),
            Emu(1828800),
            Emu(1371600),
        );
        ole.shape_id = ShapeId(5);
        ole.name = "Object 1".to_string();
        ole.r_id = Some(RelationshipId::try_from("rId3").unwrap());

        let xml = ole.to_xml_string();
        assert!(xml.contains("<p:graphicFrame>"));
        assert!(xml.contains(r#"id="5""#));
        assert!(xml.contains(r#"name="Object 1""#));
        assert!(xml.contains(r#"progId="Excel.Sheet.12""#));
        assert!(xml.contains(r#"r:id="rId3""#));
        assert!(xml.contains("presentationml/2006/ole"));
        assert!(xml.contains("</p:graphicFrame>"));
    }

    #[test]
    fn test_ole_object_with_icon() {
        let mut ole = OleObject::new(
            ProgId::WordDocument,
            vec![],
            Emu(0),
            Emu(0),
            Emu(914400),
            Emu(914400),
        );
        ole.shape_id = ShapeId(10);
        ole.name = "Object 2".to_string();
        ole.r_id = Some(RelationshipId::try_from("rId4").unwrap());
        ole.icon_r_id = Some(RelationshipId::try_from("rId5").unwrap());

        let xml = ole.to_xml_string();
        assert!(xml.contains(r#"progId="Word.Document.12""#));
        assert!(xml.contains(r#"r:embed="rId5""#));
        assert!(xml.contains("<p:pic>"));
    }

    #[test]
    fn test_ole_object_with_rotation() {
        let mut ole = OleObject::new(
            ProgId::Package,
            vec![],
            Emu(0),
            Emu(0),
            Emu(914400),
            Emu(914400),
        );
        ole.shape_id = ShapeId(1);
        ole.name = "Obj".to_string();
        ole.rotation = 45.0;

        let xml = ole.to_xml_string();
        assert!(xml.contains(r#"rot="2700000""#));
    }

    #[test]
    fn test_ole_prog_ids() {
        let prog_ids = [
            ProgId::ExcelWorksheet,
            ProgId::ExcelChart,
            ProgId::WordDocument,
            ProgId::PowerPointPresentation,
            ProgId::AcrobatDocument,
            ProgId::Package,
        ];
        for prog_id in prog_ids {
            let mut ole = OleObject::new(prog_id, vec![], Emu(0), Emu(0), Emu(100), Emu(100));
            ole.shape_id = ShapeId(1);
            ole.name = "Test".to_string();
            let xml = ole.to_xml_string();
            assert!(xml.contains(&format!(r#"progId="{}""#, prog_id.to_prog_id_str())));
        }
    }
}
