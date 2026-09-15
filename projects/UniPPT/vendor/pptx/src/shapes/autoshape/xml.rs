use crate::xml_util::{xml_escape, WriteXml};

use super::AutoShape;

impl WriteXml for AutoShape {
    fn write_xml<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        w.write_str("<p:sp>")?;

        // --- nvSpPr ---
        w.write_str("<p:nvSpPr>")?;
        write!(
            w,
            r#"<p:cNvPr id="{}" name="{}""#,
            self.shape_id,
            xml_escape(&self.name)
        )?;

        // Click/hover actions as children of cNvPr
        if self.click_action.is_some() || self.hover_action.is_some() {
            w.write_char('>')?;
            if let Some(ref action) = self.click_action {
                action.write_xml(w, None)?;
            }
            if let Some(ref action) = self.hover_action {
                action.write_hover_xml(w, None)?;
            }
            w.write_str("</p:cNvPr>")?;
        } else {
            w.write_str("/>")?;
        }

        if self.is_textbox {
            w.write_str(r#"<p:cNvSpPr txBox="1"/>"#)?;
        } else {
            w.write_str("<p:cNvSpPr/>")?;
        }

        // nvPr with optional placeholder
        if let Some(ref ph) = self.placeholder {
            w.write_str("<p:nvPr>")?;
            w.write_str("<p:ph")?;
            if let Some(pt) = ph.ph_type {
                write!(w, r#" type="{}""#, pt.to_xml_str())?;
            }
            if ph.idx.0 > 0 {
                write!(w, r#" idx="{}""#, ph.idx)?;
            }
            w.write_str("/>")?;
            w.write_str("</p:nvPr>")?;
        } else {
            w.write_str("<p:nvPr/>")?;
        }
        w.write_str("</p:nvSpPr>")?;

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
        w.write_char('>')?;
        write!(w, r#"<a:off x="{}" y="{}"/>"#, self.left.0, self.top.0)?;
        write!(
            w,
            r#"<a:ext cx="{}" cy="{}"/>"#,
            self.width.0, self.height.0
        )?;
        w.write_str("</a:xfrm>")?;

        // Geometry
        if let Some(ref geom) = self.custom_geometry {
            geom.write_xml(w)?;
        } else if let Some(ref prst) = self.prst_geom {
            write!(w, r#"<a:prstGeom prst="{prst}">"#)?;
            if self.adjustments.is_empty() {
                w.write_str("<a:avLst/>")?;
            } else {
                w.write_str("<a:avLst>")?;
                for (i, val) in self.adjustments.iter().enumerate() {
                    // Adjustment values are stored as 1/100000ths
                    // f64→i64: adjustment percentages * 100000 fits in i64
                    #[allow(clippy::cast_possible_truncation)]
                    let adj_val = (*val * 100_000.0) as i64;
                    write!(w, r#"<a:gd name="adj{}" fmla="val {}"/>"#, i + 1, adj_val)?;
                }
                w.write_str("</a:avLst>")?;
            }
            w.write_str("</a:prstGeom>")?;
        }

        // Fill
        if let Some(ref fill) = self.fill {
            fill.write_xml(w)?;
        }

        // Line
        if let Some(ref line) = self.line {
            line.write_xml(w)?;
        }

        // Shadow / effects
        if let Some(ref shadow) = self.shadow {
            shadow.write_xml(w)?;
        }

        // 3D scene (camera, lighting)
        if let Some(ref scene) = self.scene_3d {
            scene.write_xml(w)?;
        }

        // 3D shape properties (bevel, extrusion, material)
        if let Some(ref sp3d) = self.shape_3d {
            sp3d.write_xml(w)?;
        }

        w.write_str("</p:spPr>")?;

        // --- txBody ---
        if let Some(ref tf) = self.text_frame {
            w.write_str(&tf.to_xml_string())?;
        } else {
            // Minimal text body
            w.write_str("<p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr lang=\"en-US\"/></a:p></p:txBody>")?;
        }

        w.write_str("</p:sp>")?;
        Ok(())
    }
}
