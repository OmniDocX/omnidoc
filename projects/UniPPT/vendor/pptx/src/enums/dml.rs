//! Enumerations used by `DrawingML` objects.
//!
//! Types are defined in sub-modules and re-exported here for backwards
//! compatibility.

pub use super::dml_bevel::{BevelType, MaterialPreset};
pub use super::dml_camera::CameraPreset;
pub use super::dml_core::{
    MsoColorType, MsoFillType, MsoLineDashStyle, MsoThemeColor, MsoThemeColorIndex,
};
pub use super::dml_pattern::MsoPatternType;
pub use super::dml_scene3d::{LightDirection, LightRigType};

// Re-export color value enums from their dedicated module.
pub use super::color_val::{PresetColorVal, SystemColorVal};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_color_roundtrip() {
        let colors = [
            MsoThemeColorIndex::Accent1,
            MsoThemeColorIndex::Dark1,
            MsoThemeColorIndex::Light1,
            MsoThemeColorIndex::Text1,
            MsoThemeColorIndex::Background1,
            MsoThemeColorIndex::Hyperlink,
            MsoThemeColorIndex::FollowedHyperlink,
        ];
        for c in colors {
            let xml = c.to_xml_str();
            assert_eq!(MsoThemeColorIndex::from_xml_str(xml), Some(c));
        }
    }

    #[test]
    fn test_line_dash_roundtrip() {
        let styles = [
            MsoLineDashStyle::Solid,
            MsoLineDashStyle::Dash,
            MsoLineDashStyle::DashDot,
            MsoLineDashStyle::DashDotDot,
            MsoLineDashStyle::LongDash,
            MsoLineDashStyle::LongDashDot,
            MsoLineDashStyle::RoundDot,
            MsoLineDashStyle::SquareDot,
        ];
        for s in styles {
            let xml = s.to_xml_str();
            assert_eq!(MsoLineDashStyle::from_xml_str(xml), Some(s));
        }
    }

    #[test]
    fn test_pattern_roundtrip() {
        let patterns = [
            MsoPatternType::Cross,
            MsoPatternType::Horizontal,
            MsoPatternType::Vertical,
            MsoPatternType::Percent50,
            MsoPatternType::Wave,
        ];
        for p in patterns {
            let xml = p.to_xml_str();
            assert_eq!(MsoPatternType::from_xml_str(xml), Some(p));
        }
    }

    #[test]
    fn test_unknown_theme_color() {
        assert_eq!(MsoThemeColorIndex::from_xml_str("unknown"), None);
    }

    #[test]
    fn test_unknown_line_dash() {
        assert_eq!(MsoLineDashStyle::from_xml_str("unknown"), None);
    }

    #[test]
    fn test_camera_preset_roundtrip() {
        let presets = [
            CameraPreset::OrthographicFront,
            CameraPreset::PerspectiveFront,
            CameraPreset::IsometricTopUp,
            CameraPreset::PerspectiveAbove,
            CameraPreset::PerspectiveRelaxed,
        ];
        for p in presets {
            let xml = p.to_xml_str();
            assert_eq!(CameraPreset::from_xml_str(xml), p);
        }
    }

    #[test]
    fn test_camera_preset_other() {
        let p = CameraPreset::from_xml_str("customCamera");
        assert_eq!(p, CameraPreset::Other("customCamera".to_string()));
        assert_eq!(p.to_xml_str(), "customCamera");
    }

    #[test]
    fn test_light_rig_type_roundtrip() {
        let types = [
            LightRigType::ThreePt,
            LightRigType::Balanced,
            LightRigType::Harsh,
            LightRigType::Flood,
            LightRigType::Flat,
            LightRigType::Morning,
        ];
        for t in types {
            let xml = t.to_xml_str();
            assert_eq!(LightRigType::from_xml_str(xml), t);
        }
    }

    #[test]
    fn test_light_rig_type_other() {
        let t = LightRigType::from_xml_str("customRig");
        assert_eq!(t, LightRigType::Other("customRig".to_string()));
        assert_eq!(t.to_xml_str(), "customRig");
    }

    #[test]
    fn test_light_direction_roundtrip() {
        let dirs = [
            LightDirection::Top,
            LightDirection::TopLeft,
            LightDirection::TopRight,
            LightDirection::Left,
            LightDirection::Right,
            LightDirection::Bottom,
            LightDirection::BottomLeft,
            LightDirection::BottomRight,
        ];
        for d in dirs {
            let xml = d.to_xml_str();
            assert_eq!(LightDirection::from_xml_str(xml), d);
        }
    }

    #[test]
    fn test_light_direction_other() {
        let d = LightDirection::from_xml_str("customDir");
        assert_eq!(d, LightDirection::Other("customDir".to_string()));
        assert_eq!(d.to_xml_str(), "customDir");
    }

    #[test]
    fn test_bevel_type_roundtrip() {
        let types = [
            BevelType::Circle,
            BevelType::RelaxedInset,
            BevelType::Cross,
            BevelType::CoolSlant,
            BevelType::Angle,
            BevelType::SoftRound,
            BevelType::Convex,
            BevelType::Slope,
            BevelType::Divot,
            BevelType::Riblet,
            BevelType::HardEdge,
            BevelType::ArtDeco,
        ];
        for t in types {
            let xml = t.to_xml_str();
            assert_eq!(BevelType::from_xml_str(xml), t);
        }
    }

    #[test]
    fn test_bevel_type_other() {
        let t = BevelType::from_xml_str("customBevel");
        assert_eq!(t, BevelType::Other("customBevel".to_string()));
        assert_eq!(t.to_xml_str(), "customBevel");
    }

    #[test]
    fn test_material_preset_roundtrip() {
        let presets = [
            MaterialPreset::WarmMatte,
            MaterialPreset::Metal,
            MaterialPreset::Plastic,
            MaterialPreset::DkEdge,
            MaterialPreset::Flat,
            MaterialPreset::SoftMetal,
        ];
        for p in presets {
            let xml = p.to_xml_str();
            assert_eq!(MaterialPreset::from_xml_str(xml), p);
        }
    }

    #[test]
    fn test_material_preset_other() {
        let p = MaterialPreset::from_xml_str("customMaterial");
        assert_eq!(p, MaterialPreset::Other("customMaterial".to_string()));
        assert_eq!(p.to_xml_str(), "customMaterial");
    }
}
