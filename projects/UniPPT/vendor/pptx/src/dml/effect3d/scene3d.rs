//! 3D scene types: camera, light rig, and scene.

use super::rotation::Rotation3D;
use crate::enums::dml::{CameraPreset, LightDirection, LightRigType};
use crate::xml_util::WriteXml;

/// Camera settings for 3D scene.
///
/// Corresponds to `<a:camera prst="...">` within `<a:scene3d>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Camera {
    /// Camera preset type (e.g. `CameraPreset::OrthographicFront`).
    pub preset: CameraPreset,
    /// Optional field of view in 60,000ths of a degree.
    pub fov: Option<i64>,
    /// Optional rotation.
    pub rot: Option<Rotation3D>,
}

impl Camera {
    /// Create a camera with the given preset and no rotation.
    pub fn new(preset: impl Into<CameraPreset>) -> Self {
        Self {
            preset: preset.into(),
            fov: None,
            rot: None,
        }
    }

    /// Create a camera with preset and rotation.
    pub fn with_rotation(preset: impl Into<CameraPreset>, rot: Rotation3D) -> Self {
        Self {
            preset: preset.into(),
            fov: None,
            rot: Some(rot),
        }
    }
}

impl WriteXml for Camera {
    fn write_xml<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        write!(w, r#"<a:camera prst="{}""#, self.preset.to_xml_str())?;
        if let Some(fov) = self.fov {
            write!(w, r#" fov="{fov}""#)?;
        }
        if let Some(ref rot) = self.rot {
            w.write_char('>')?;
            rot.write_xml(w)?;
            w.write_str("</a:camera>")
        } else {
            w.write_str("/>")
        }
    }
}

/// Light rig settings for 3D scene.
///
/// Corresponds to `<a:lightRig rig="..." dir="..."/>` within `<a:scene3d>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LightRig {
    /// Light rig type (e.g. `LightRigType::ThreePt`).
    pub rig_type: LightRigType,
    /// Light direction (e.g. `LightDirection::Top`).
    pub direction: LightDirection,
    /// Optional rotation for the light rig.
    pub rot: Option<Rotation3D>,
}

impl LightRig {
    /// Create a light rig with the given type and direction.
    pub fn new(rig_type: impl Into<LightRigType>, direction: impl Into<LightDirection>) -> Self {
        Self {
            rig_type: rig_type.into(),
            direction: direction.into(),
            rot: None,
        }
    }
}

impl WriteXml for LightRig {
    fn write_xml<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        write!(
            w,
            r#"<a:lightRig rig="{}" dir="{}""#,
            self.rig_type.to_xml_str(),
            self.direction.to_xml_str()
        )?;
        if let Some(ref rot) = self.rot {
            w.write_char('>')?;
            rot.write_xml(w)?;
            w.write_str("</a:lightRig>")
        } else {
            w.write_str("/>")
        }
    }
}

/// 3D scene properties.
///
/// Corresponds to `<a:scene3d>` containing camera and light rig settings.
/// Used within `<a:bodyPr>` or as a standalone scene specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scene3D {
    /// Camera settings.
    pub camera: Camera,
    /// Light rig settings.
    pub light_rig: LightRig,
}

impl Scene3D {
    /// Create a scene with the given camera and light rig.
    #[must_use]
    pub const fn new(camera: Camera, light_rig: LightRig) -> Self {
        Self { camera, light_rig }
    }

    /// Create a default scene with orthographic front camera and three-point lighting.
    #[must_use]
    #[deprecated(note = "Use `Scene3D::default()` instead")]
    pub fn default_scene() -> Self {
        Self::default()
    }
}

/// Creates a default 3D scene with orthographic front camera and three-point lighting.
impl Default for Scene3D {
    fn default() -> Self {
        Self {
            camera: Camera::new(CameraPreset::OrthographicFront),
            light_rig: LightRig::new(LightRigType::ThreePt, LightDirection::Top),
        }
    }
}

impl WriteXml for Scene3D {
    fn write_xml<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        w.write_str("<a:scene3d>")?;
        self.camera.write_xml(w)?;
        self.light_rig.write_xml(w)?;
        w.write_str("</a:scene3d>")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_new() {
        let cam = Camera::new(CameraPreset::OrthographicFront);
        assert_eq!(cam.preset, CameraPreset::OrthographicFront);
        assert!(cam.fov.is_none());
        assert!(cam.rot.is_none());
    }

    #[test]
    fn test_camera_no_rotation_xml() {
        let cam = Camera::new("orthographicFront");
        assert_eq!(
            cam.to_xml_string(),
            r#"<a:camera prst="orthographicFront"/>"#
        );
    }

    #[test]
    fn test_camera_with_rotation_xml() {
        let cam = Camera::with_rotation("perspectiveFront", Rotation3D::new(0, 0, 0));
        let xml = cam.to_xml_string();
        assert!(xml.starts_with(r#"<a:camera prst="perspectiveFront">"#));
        assert!(xml.contains(r#"<a:rot lat="0" lon="0" rev="0"/>"#));
        assert!(xml.ends_with("</a:camera>"));
    }

    #[test]
    fn test_camera_with_fov_xml() {
        let mut cam = Camera::new("perspectiveRelaxed");
        cam.fov = Some(4500000);
        let xml = cam.to_xml_string();
        assert!(xml.contains(r#"fov="4500000""#));
    }

    #[test]
    fn test_camera_with_fov_and_rotation() {
        let mut cam =
            Camera::with_rotation("perspectiveFront", Rotation3D::from_degrees(30.0, 0.0, 0.0));
        cam.fov = Some(3000000);
        let xml = cam.to_xml_string();
        assert!(xml.contains(r#"prst="perspectiveFront""#));
        assert!(xml.contains(r#"fov="3000000""#));
        assert!(xml.contains(r#"lat="1800000""#));
        assert!(xml.ends_with("</a:camera>"));
    }

    #[test]
    fn test_light_rig_new() {
        let lr = LightRig::new(LightRigType::ThreePt, LightDirection::Top);
        assert_eq!(lr.rig_type, LightRigType::ThreePt);
        assert_eq!(lr.direction, LightDirection::Top);
        assert!(lr.rot.is_none());
    }

    #[test]
    fn test_light_rig_no_rotation_xml() {
        let lr = LightRig::new("threePt", "t");
        assert_eq!(lr.to_xml_string(), r#"<a:lightRig rig="threePt" dir="t"/>"#);
    }

    #[test]
    fn test_light_rig_with_rotation_xml() {
        let mut lr = LightRig::new("balanced", "tl");
        lr.rot = Some(Rotation3D::new(0, 0, 1200000));
        let xml = lr.to_xml_string();
        assert!(xml.starts_with(r#"<a:lightRig rig="balanced" dir="tl">"#));
        assert!(xml.contains(r#"rev="1200000""#));
        assert!(xml.ends_with("</a:lightRig>"));
    }

    #[test]
    fn test_scene3d_default() {
        let scene = Scene3D::default();
        assert_eq!(scene.camera.preset, CameraPreset::OrthographicFront);
        assert_eq!(scene.light_rig.rig_type, LightRigType::ThreePt);
        assert_eq!(scene.light_rig.direction, LightDirection::Top);
    }

    #[test]
    fn test_scene3d_default_xml() {
        let scene = Scene3D::default();
        let xml = scene.to_xml_string();
        assert!(xml.starts_with("<a:scene3d>"));
        assert!(xml.contains(r#"<a:camera prst="orthographicFront"/>"#));
        assert!(xml.contains(r#"<a:lightRig rig="threePt" dir="t"/>"#));
        assert!(xml.ends_with("</a:scene3d>"));
    }

    #[test]
    fn test_scene3d_custom_xml() {
        let scene = Scene3D::new(
            Camera::with_rotation("perspectiveFront", Rotation3D::new(0, 0, 0)),
            LightRig::new("harsh", "tr"),
        );
        let xml = scene.to_xml_string();
        assert!(xml.contains(r#"prst="perspectiveFront""#));
        assert!(xml.contains(r#"rig="harsh""#));
        assert!(xml.contains(r#"dir="tr""#));
    }

    #[test]
    fn test_scene3d_clone_eq() {
        let scene = Scene3D::default();
        let scene2 = scene.clone();
        assert_eq!(scene, scene2);
    }
}
