use crate::xml_util::{xml_escape, WriteXml};

use super::Picture;

impl WriteXml for Picture {
    #[allow(clippy::too_many_lines)]
    fn write_xml<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        w.write_str("<p:pic>")?;

        // --- nvPicPr ---
        w.write_str("<p:nvPicPr>")?;

        write!(
            w,
            r#"<p:cNvPr id="{}" name="{}""#,
            self.shape_id,
            xml_escape(&self.name)
        )?;
        if let Some(ref desc) = self.description {
            write!(w, r#" descr="{}""#, xml_escape(desc))?;
        }

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

        w.write_str("<p:cNvPicPr><a:picLocks noChangeAspect=\"1\"/></p:cNvPicPr>")?;

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
        w.write_str("</p:nvPicPr>")?;

        // --- blipFill ---
        w.write_str("<p:blipFill>")?;
        if let Some(ref r_id) = self.image_r_id {
            if let Some(ref duotone) = self.duotone {
                write!(w, r#"<a:blip r:embed="{}">"#, xml_escape(r_id))?;
                duotone.write_xml(w)?;
                w.write_str("</a:blip>")?;
            } else {
                write!(w, r#"<a:blip r:embed="{}"/>"#, xml_escape(r_id))?;
            }
        }

        let has_crop = self.crop_left > 0.0
            || self.crop_top > 0.0
            || self.crop_right > 0.0
            || self.crop_bottom > 0.0;

        if has_crop {
            // Crop values are in 1/100000ths (percentage * 1000)
            // f64→i64: crop percentages are small values, no truncation risk
            #[allow(clippy::cast_possible_truncation)]
            {
                w.write_str("<a:srcRect")?;
                if self.crop_left > 0.0 {
                    write!(w, r#" l="{}""#, (self.crop_left * 100_000.0) as i64)?;
                }
                if self.crop_top > 0.0 {
                    write!(w, r#" t="{}""#, (self.crop_top * 100_000.0) as i64)?;
                }
                if self.crop_right > 0.0 {
                    write!(w, r#" r="{}""#, (self.crop_right * 100_000.0) as i64)?;
                }
                if self.crop_bottom > 0.0 {
                    write!(w, r#" b="{}""#, (self.crop_bottom * 100_000.0) as i64)?;
                }
                w.write_str("/>")?;
            }
        }

        w.write_str("<a:stretch><a:fillRect/></a:stretch>")?;
        w.write_str("</p:blipFill>")?;

        // --- spPr ---
        w.write_str("<p:spPr>")?;
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
        if let Some(shape_type) = self.auto_shape_type {
            write!(
                w,
                r#"<a:prstGeom prst="{}"><a:avLst/></a:prstGeom>"#,
                shape_type.to_xml_str()
            )?;
        } else {
            w.write_str(r#"<a:prstGeom prst="rect"><a:avLst/></a:prstGeom>"#)?;
        }

        if let Some(ref line) = self.line {
            line.write_xml(w)?;
        }

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

        w.write_str("</p:pic>")?;
        Ok(())
    }
}
