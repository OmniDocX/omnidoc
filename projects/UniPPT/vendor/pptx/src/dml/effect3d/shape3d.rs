//! 3D shape properties: extrusion, contour, and material.

use super::bevel::Bevel;
use crate::dml::color::ColorFormat;
use crate::enums::dml::MaterialPreset;
use crate::xml_util::WriteXml;

/// 3D shape properties.
///
/// Corresponds to `<a:sp3d>` within `<p:spPr>`, controlling extrusion depth,
/// bevel effects, contour, and material.
#[derive(Debug, Clone, PartialEq)]
pub struct Shape3D {
    /// Top bevel effect.
    pub bevel_top: Option<Bevel>,
    /// Bottom bevel effect.
    pub bevel_bottom: Option<Bevel>,
    /// Extrusion color.
    pub extrusion_color: Option<ColorFormat>,
    /// Extrusion height (depth) in EMU.
    pub extrusion_height: Option<i64>,
    /// Contour color.
    pub contour_color: Option<ColorFormat>,
    /// Contour width in EMU.
    pub contour_width: Option<i64>,
    /// Material preset (e.g. `MaterialPreset::WarmMatte`).
    pub material: Option<MaterialPreset>,
}

impl Shape3D {
    /// Create an empty `Shape3D` with no effects.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bevel_top: None,
            bevel_bottom: None,
            extrusion_color: None,
            extrusion_height: None,
            contour_color: None,
            contour_width: None,
            material: None,
        }
    }

    /// Create a `Shape3D` with a top bevel only.
    #[must_use]
    pub fn with_bevel_top(bevel: Bevel) -> Self {
        Self {
            bevel_top: Some(bevel),
            ..Self::new()
        }
    }

    /// Create a `Shape3D` with extrusion settings.
    #[must_use]
    pub fn with_extrusion(height: i64, color: Option<ColorFormat>) -> Self {
        Self {
            extrusion_height: Some(height),
            extrusion_color: color,
            ..Self::new()
        }
    }

    /// Set the top bevel.
    pub fn set_bevel_top(&mut self, bevel: Bevel) {
        self.bevel_top = Some(bevel);
    }

    /// Set the bottom bevel.
    pub fn set_bevel_bottom(&mut self, bevel: Bevel) {
        self.bevel_bottom = Some(bevel);
    }

    /// Set the material preset.
    pub fn set_material(&mut self, material: impl Into<MaterialPreset>) {
        self.material = Some(material.into());
    }
}

impl WriteXml for Shape3D {
    fn write_xml<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        w.write_str("<a:sp3d")?;

        if let Some(h) = self.extrusion_height {
            write!(w, r#" extrusionH="{h}""#)?;
        }
        if let Some(cw) = self.contour_width {
            write!(w, r#" contourW="{cw}""#)?;
        }
        if let Some(ref mat) = self.material {
            write!(w, r#" prstMaterial="{}""#, mat.to_xml_str())?;
        }

        w.write_char('>')?;

        if let Some(ref bevel) = self.bevel_top {
            bevel.write_xml_with_tag(w, "a:bevelT")?;
        }
        if let Some(ref bevel) = self.bevel_bottom {
            bevel.write_xml_with_tag(w, "a:bevelB")?;
        }
        if let Some(ref color) = self.extrusion_color {
            w.write_str("<a:extrusionClr>")?;
            color.write_xml(w)?;
            w.write_str("</a:extrusionClr>")?;
        }
        if let Some(ref color) = self.contour_color {
            w.write_str("<a:contourClr>")?;
            color.write_xml(w)?;
            w.write_str("</a:contourClr>")?;
        }

        w.write_str("</a:sp3d>")
    }
}

/// Creates an empty 3D shape with no rotation, bevel, or extrusion.
impl Default for Shape3D {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dml::effect3d::rotation::Rotation3D;
    use crate::dml::effect3d::scene3d::{Camera, LightRig, Scene3D};
    use crate::enums::dml::BevelType;

    #[test]
    fn test_shape3d_new_empty() {
        let sp3d = Shape3D::new();
        assert!(sp3d.bevel_top.is_none());
        assert!(sp3d.bevel_bottom.is_none());
        assert!(sp3d.extrusion_color.is_none());
        assert!(sp3d.extrusion_height.is_none());
        assert!(sp3d.contour_color.is_none());
        assert!(sp3d.contour_width.is_none());
        assert!(sp3d.material.is_none());
    }

    #[test]
    fn test_shape3d_default() {
        let sp3d = Shape3D::default();
        assert!(sp3d.bevel_top.is_none());
    }

    #[test]
    fn test_shape3d_empty_xml() {
        let sp3d = Shape3D::new();
        let xml = sp3d.to_xml_string();
        assert_eq!(xml, "<a:sp3d></a:sp3d>");
    }

    #[test]
    fn test_shape3d_with_bevel_top() {
        let sp3d = Shape3D::with_bevel_top(Bevel::circle(63500, 25400));
        let xml = sp3d.to_xml_string();
        assert!(xml.starts_with("<a:sp3d>"));
        assert!(xml.contains(r#"<a:bevelT w="63500" h="25400" prst="circle"/>"#));
        assert!(xml.ends_with("</a:sp3d>"));
    }

    #[test]
    fn test_shape3d_with_extrusion() {
        let sp3d = Shape3D::with_extrusion(76200, Some(ColorFormat::rgb(255, 0, 0)));
        let xml = sp3d.to_xml_string();
        assert!(xml.contains(r#"extrusionH="76200""#));
        assert!(xml.contains("<a:extrusionClr>"));
        assert!(xml.contains("FF0000"));
        assert!(xml.contains("</a:extrusionClr>"));
    }

    #[test]
    fn test_shape3d_full_xml() {
        let mut sp3d = Shape3D::new();
        sp3d.extrusion_height = Some(76200);
        sp3d.contour_width = Some(12700);
        sp3d.material = Some(MaterialPreset::WarmMatte);
        sp3d.bevel_top = Some(Bevel::circle(63500, 25400));
        sp3d.bevel_bottom = Some(Bevel::new(BevelType::Angle, 63500, 25400));
        sp3d.extrusion_color = Some(ColorFormat::rgb(255, 0, 0));
        sp3d.contour_color = Some(ColorFormat::rgb(0, 0, 255));

        let xml = sp3d.to_xml_string();
        assert!(xml.contains(r#"extrusionH="76200""#));
        assert!(xml.contains(r#"contourW="12700""#));
        assert!(xml.contains(r#"prstMaterial="warmMatte""#));
        assert!(xml.contains(r#"<a:bevelT w="63500" h="25400" prst="circle"/>"#));
        assert!(xml.contains(r#"<a:bevelB w="63500" h="25400" prst="angle"/>"#));
        assert!(xml.contains("<a:extrusionClr>"));
        assert!(xml.contains("FF0000"));
        assert!(xml.contains("<a:contourClr>"));
        assert!(xml.contains("0000FF"));
    }

    #[test]
    fn test_shape3d_setters() {
        let mut sp3d = Shape3D::new();
        sp3d.set_bevel_top(Bevel::circle(50800, 25400));
        sp3d.set_bevel_bottom(Bevel::new(BevelType::SoftRound, 38100, 19050));
        sp3d.set_material(MaterialPreset::Metal);

        assert_eq!(sp3d.bevel_top.as_ref().unwrap().width, 50800); // EXCEPTION(test-only)
        assert_eq!(
            sp3d.bevel_bottom.as_ref().unwrap().bevel_type, // EXCEPTION(test-only)
            BevelType::SoftRound
        );
        assert_eq!(sp3d.material.as_ref().unwrap(), &MaterialPreset::Metal); // EXCEPTION(test-only)
    }

    #[test]
    fn test_shape3d_extrusion_no_color() {
        let sp3d = Shape3D::with_extrusion(50800, None);
        let xml = sp3d.to_xml_string();
        assert!(xml.contains(r#"extrusionH="50800""#));
        assert!(!xml.contains("extrusionClr"));
    }

    #[test]
    fn test_shape3d_contour_only() {
        let mut sp3d = Shape3D::new();
        sp3d.contour_width = Some(25400);
        sp3d.contour_color = Some(ColorFormat::rgb(0, 128, 0));
        let xml = sp3d.to_xml_string();
        assert!(xml.contains(r#"contourW="25400""#));
        assert!(xml.contains("<a:contourClr>"));
        assert!(xml.contains("008000"));
    }

    #[test]
    fn test_shape3d_material_only() {
        let mut sp3d = Shape3D::new();
        sp3d.set_material("plastic");
        let xml = sp3d.to_xml_string();
        assert!(xml.contains(r#"prstMaterial="plastic""#));
    }

    #[test]
    fn test_shape3d_clone_eq() {
        let mut sp3d = Shape3D::new();
        sp3d.set_bevel_top(Bevel::circle(63500, 25400));
        sp3d.extrusion_height = Some(76200);
        let sp3d2 = sp3d.clone();
        assert_eq!(sp3d, sp3d2);
    }

    #[test]
    fn test_full_3d_setup_xml() {
        let scene = Scene3D::new(
            Camera::with_rotation("orthographicFront", Rotation3D::new(0, 0, 0)),
            LightRig::new("threePt", "t"),
        );
        let mut sp3d = Shape3D::new();
        sp3d.extrusion_height = Some(76200);
        sp3d.contour_width = Some(12700);
        sp3d.set_material("warmMatte");
        sp3d.set_bevel_top(Bevel::circle(63500, 25400));
        sp3d.set_bevel_bottom(Bevel::circle(63500, 25400));
        sp3d.extrusion_color = Some(ColorFormat::rgb(255, 0, 0));
        sp3d.contour_color = Some(ColorFormat::rgb(0, 0, 255));

        let scene_xml = scene.to_xml_string();
        let sp3d_xml = sp3d.to_xml_string();

        assert!(scene_xml.starts_with("<a:scene3d>"));
        assert!(scene_xml.contains("<a:camera"));
        assert!(scene_xml.contains("<a:lightRig"));
        assert!(scene_xml.ends_with("</a:scene3d>"));

        assert!(sp3d_xml.starts_with("<a:sp3d"));
        assert!(sp3d_xml.contains("a:bevelT"));
        assert!(sp3d_xml.contains("a:bevelB"));
        assert!(sp3d_xml.contains("a:extrusionClr"));
        assert!(sp3d_xml.contains("a:contourClr"));
        assert!(sp3d_xml.ends_with("</a:sp3d>"));
    }
}
