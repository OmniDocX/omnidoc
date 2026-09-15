//! Named color value enumerations for `DrawingML`.
//!
//! These large enums map system- and preset-color names to their OOXML XML
//! attribute values. They are separated from the main `dml` module to keep
//! file sizes manageable.

pub use super::color_val_preset::PresetColorVal;
pub use super::color_val_system::SystemColorVal;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_color_val_roundtrip() {
        let colors = [
            SystemColorVal::WindowText,
            SystemColorVal::Window,
            SystemColorVal::BtnFace,
            SystemColorVal::Highlight,
            SystemColorVal::MenuBar,
            SystemColorVal::ThreeDDkShadow,
            SystemColorVal::ThreeDLight,
        ];
        for c in colors {
            let xml = c.to_xml_str();
            assert_eq!(SystemColorVal::from_xml_str(xml), c);
        }
    }

    #[test]
    fn test_system_color_val_unknown() {
        let val = SystemColorVal::from_xml_str("customUnknown");
        assert_eq!(val, SystemColorVal::Other("customUnknown".to_string()));
        assert_eq!(val.to_xml_str(), "customUnknown");
    }

    #[test]
    fn test_preset_color_val_roundtrip() {
        let colors = [
            PresetColorVal::Red,
            PresetColorVal::Blue,
            PresetColorVal::Green,
            PresetColorVal::Black,
            PresetColorVal::White,
            PresetColorVal::DarkBlue,
            PresetColorVal::LightGray,
        ];
        for c in colors {
            let xml = c.to_xml_str();
            assert_eq!(PresetColorVal::from_xml_str(xml), c);
        }
    }

    #[test]
    fn test_preset_color_val_unknown() {
        let val = PresetColorVal::from_xml_str("customColor");
        assert_eq!(val, PresetColorVal::Other("customColor".to_string()));
        assert_eq!(val.to_xml_str(), "customColor");
    }
}
