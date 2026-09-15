//! XML generation methods for creating shape elements.

use crate::units::{Emu, ShapeId};
use crate::xml_util::xml_escape;

use super::ShapeTree;

/// Map a preset geometry name to a human-readable base name for auto-naming.
#[allow(clippy::redundant_pub_crate)]
pub(crate) fn shape_name_for_prst(prst: &str) -> &'static str {
    match prst {
        "rect" => "Rectangle",
        "roundRect" => "Rounded Rectangle",
        "ellipse" => "Oval",
        "diamond" => "Diamond",
        "triangle" => "Isosceles Triangle",
        "rtTriangle" => "Right Triangle",
        "parallelogram" => "Parallelogram",
        "trapezoid" => "Trapezoid",
        "pentagon" => "Regular Pentagon",
        "homePlate" => "Pentagon",
        "hexagon" => "Hexagon",
        "heptagon" => "Heptagon",
        "octagon" => "Octagon",
        "decagon" => "Decagon",
        "dodecagon" => "Dodecagon",
        "star4" => "4-Point Star",
        "star5" => "5-Point Star",
        "star6" => "6-Point Star",
        "star7" => "7-Point Star",
        "star8" => "8-Point Star",
        "star10" => "10-Point Star",
        "star12" => "12-Point Star",
        "star16" => "16-Point Star",
        "star24" => "24-Point Star",
        "star32" => "32-Point Star",
        "cloud" => "Cloud",
        "heart" => "Heart",
        "sun" => "Sun",
        "moon" => "Moon",
        "arc" => "Arc",
        "donut" => "Donut",
        "plus" => "Cross",
        "can" => "Can",
        "cube" => "Cube",
        "bevel" => "Bevel",
        "frame" => "Frame",
        "pie" => "Pie",
        "chord" => "Chord",
        "plaque" => "Plaque",
        "funnel" => "Funnel",
        "wave" => "Wave",
        "doubleWave" => "Double Wave",
        "smileyFace" => "Smiley Face",
        "noSmoking" => "No Symbol",
        "lightningBolt" => "Lightning Bolt",
        "teardrop" => "Tear",
        "foldedCorner" => "Folded Corner",
        "leftArrow" => "Left Arrow",
        "rightArrow" => "Right Arrow",
        "upArrow" => "Up Arrow",
        "downArrow" => "Down Arrow",
        "leftRightArrow" => "Left-Right Arrow",
        "upDownArrow" => "Up-Down Arrow",
        _ => "Freeform",
    }
}

impl ShapeTree {
    /// Generate XML for a new `<p:sp>` element (textbox).
    #[must_use]
    pub fn new_textbox_xml(
        shape_id: ShapeId,
        name: &str,
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
    ) -> String {
        format!(
            r#"<p:sp xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:nvSpPr><p:cNvPr id="{}" name="{}"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/></p:spPr><p:txBody><a:bodyPr wrap="square" rtlCol="0"/><a:lstStyle/><a:p><a:endParaRPr lang="en-US"/></a:p></p:txBody></p:sp>"#,
            shape_id,
            xml_escape(name),
            left.0,
            top.0,
            width.0,
            height.0,
        )
    }

    /// Generate XML for a new `<p:sp>` element (auto shape with preset geometry).
    #[must_use]
    pub fn new_autoshape_xml(
        shape_id: ShapeId,
        name: &str,
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
        prst: &str,
    ) -> String {
        format!(
            r#"<p:sp xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:nvSpPr><p:cNvPr id="{}" name="{}"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/></a:xfrm><a:prstGeom prst="{}"><a:avLst/></a:prstGeom></p:spPr><p:txBody><a:bodyPr rtlCol="0" anchor="ctr"/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:endParaRPr lang="en-US"/></a:p></p:txBody></p:sp>"#,
            shape_id,
            xml_escape(name),
            left.0,
            top.0,
            width.0,
            height.0,
            prst,
        )
    }

    /// Generate XML for a new `<p:pic>` element.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new_picture_xml(
        shape_id: ShapeId,
        name: &str,
        desc: &str,
        r_id: &str,
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
    ) -> String {
        format!(
            r#"<p:pic xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:nvPicPr><p:cNvPr id="{}" name="{}" descr="{}"/><p:cNvPicPr><a:picLocks noChangeAspect="1"/></p:cNvPicPr><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="{}"/><a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr><a:xfrm><a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr></p:pic>"#,
            shape_id,
            xml_escape(name),
            xml_escape(desc),
            r_id,
            left.0,
            top.0,
            width.0,
            height.0,
        )
    }

    /// Generate XML for a new `<p:cxnSp>` element (connector).
    #[must_use]
    pub fn new_connector_xml(
        shape_id: ShapeId,
        name: &str,
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
        prst: &str,
    ) -> String {
        format!(
            r#"<p:cxnSp xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:nvCxnSpPr><p:cNvPr id="{}" name="{}"/><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr><p:spPr><a:xfrm><a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/></a:xfrm><a:prstGeom prst="{}"><a:avLst/></a:prstGeom></p:spPr></p:cxnSp>"#,
            shape_id,
            xml_escape(name),
            left.0,
            top.0,
            width.0,
            height.0,
            prst,
        )
    }

    /// Generate XML for a new `<p:graphicFrame>` element containing a chart.
    ///
    /// The `r_id` is the relationship ID linking the slide to the chart part.
    #[must_use]
    pub fn new_chart_graphic_frame_xml(
        shape_id: ShapeId,
        name: &str,
        r_id: &str,
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
    ) -> String {
        format!(
            r#"<p:graphicFrame xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:nvGraphicFramePr><p:cNvPr id="{}" name="{}"/><p:cNvGraphicFramePr><a:graphicFrameLocks noGrp="1"/></p:cNvGraphicFramePr><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" r:id="{}"/></a:graphicData></a:graphic></p:graphicFrame>"#,
            shape_id,
            xml_escape(name),
            left.0,
            top.0,
            width.0,
            height.0,
            r_id,
        )
    }

    /// Generate XML for a new `<p:graphicFrame>` element containing a table.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new_table_xml(
        shape_id: ShapeId,
        name: &str,
        rows: u32,
        cols: u32,
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
    ) -> String {
        let col_width = if cols > 0 {
            width.0 / i64::from(cols)
        } else {
            width.0
        };
        let row_height = if rows > 0 {
            height.0 / i64::from(rows)
        } else {
            height.0
        };

        let mut cols_xml = String::new();
        for _ in 0..cols {
            cols_xml.push_str(&format!(r#"<a:gridCol w="{col_width}"/>"#));
        }

        let mut rows_xml = String::new();
        for _ in 0..rows {
            let mut cells_xml = String::new();
            for _ in 0..cols {
                cells_xml.push_str(r#"<a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr lang="en-US"/></a:p></a:txBody><a:tcPr/></a:tc>"#);
            }
            rows_xml.push_str(&format!(r#"<a:tr h="{row_height}">{cells_xml}</a:tr>"#));
        }

        format!(
            r#"<p:graphicFrame xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:nvGraphicFramePr><p:cNvPr id="{}" name="{}"/><p:cNvGraphicFramePr><a:graphicFrameLocks noGrp="1"/></p:cNvGraphicFramePr><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr firstRow="1" bandRow="1"><a:tableStyleId>{{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}}</a:tableStyleId></a:tblPr><a:tblGrid>{}</a:tblGrid>{}</a:tbl></a:graphicData></a:graphic></p:graphicFrame>"#,
            shape_id,
            xml_escape(name),
            left.0,
            top.0,
            width.0,
            height.0,
            cols_xml,
            rows_xml,
        )
    }

    /// Generate XML for a new `<p:cxnSp>` element with flip attributes.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new_connector_xml_with_flip(
        shape_id: ShapeId,
        name: &str,
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
        prst: &str,
        flip_h: bool,
        flip_v: bool,
    ) -> String {
        let mut flip_attrs = String::new();
        if flip_h {
            flip_attrs.push_str(r#" flipH="1""#);
        }
        if flip_v {
            flip_attrs.push_str(r#" flipV="1""#);
        }
        format!(
            r#"<p:cxnSp xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:nvCxnSpPr><p:cNvPr id="{}" name="{}"/><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr><p:spPr><a:xfrm{}><a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/></a:xfrm><a:prstGeom prst="{}"><a:avLst/></a:prstGeom></p:spPr></p:cxnSp>"#,
            shape_id,
            xml_escape(name),
            flip_attrs,
            left.0,
            top.0,
            width.0,
            height.0,
            prst,
        )
    }

    /// Generate XML for a new `<p:pic>` element representing a movie (video) shape.
    ///
    /// The video is referenced via `<a:videoFile r:link="..."/>` inside `<p:nvPr>`,
    /// and the poster frame image is embedded via `<a:blip r:embed="..."/>`.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new_movie_xml(
        shape_id: ShapeId,
        name: &str,
        video_r_id: &str,
        poster_r_id: &str,
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
    ) -> String {
        format!(
            r#"<p:pic xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:nvPicPr><p:cNvPr id="{}" name="{}"/><p:cNvPicPr><a:picLocks noChangeAspect="1"/></p:cNvPicPr><p:nvPr><a:videoFile r:link="{}"/></p:nvPr></p:nvPicPr><p:blipFill><a:blip r:embed="{}"/><a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr><a:xfrm><a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr></p:pic>"#,
            shape_id,
            xml_escape(name),
            video_r_id,
            poster_r_id,
            left.0,
            top.0,
            width.0,
            height.0,
        )
    }

    /// Generate XML for a new `<p:grpSp>` element (empty group shape).
    #[must_use]
    pub fn new_group_shape_xml(
        shape_id: ShapeId,
        name: &str,
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
    ) -> String {
        format!(
            r#"<p:grpSp xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:nvGrpSpPr><p:cNvPr id="{}" name="{}"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/><a:chOff x="{}" y="{}"/><a:chExt cx="{}" cy="{}"/></a:xfrm></p:grpSpPr></p:grpSp>"#,
            shape_id,
            xml_escape(name),
            left.0,
            top.0,
            width.0,
            height.0,
            left.0,
            top.0,
            width.0,
            height.0,
        )
    }

    /// Generate XML for a new native audio picture shape.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new_audio_xml(
        shape_id: ShapeId,
        name: &str,
        audio_r_id: &str,
        media_r_id: &str,
        poster_r_id: &str,
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
    ) -> String {
        format!(
            r#"<p:pic xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:p14="http://schemas.microsoft.com/office/powerpoint/2010/main"><p:nvPicPr><p:cNvPr id="{}" name="{}"/><p:cNvPicPr><a:picLocks noChangeAspect="1"/></p:cNvPicPr><p:nvPr><a:audioFile r:link="{}"/><p:extLst><p:ext uri="{{DAA4B4D4-6D71-4841-9C4C-AB7A84B4E8A2}}"><p14:media r:embed="{}"/></p:ext></p:extLst></p:nvPr></p:nvPicPr><p:blipFill><a:blip r:embed="{}"/><a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr><a:xfrm><a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr></p:pic>"#,
            shape_id,
            xml_escape(name),
            audio_r_id,
            media_r_id,
            poster_r_id,
            left.0,
            top.0,
            width.0,
            height.0,
        )
    }
}
