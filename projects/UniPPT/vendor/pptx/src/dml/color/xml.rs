//! `WriteXml` implementation for `ColorFormat`.

use super::ColorFormat;
use crate::xml_util::WriteXml;

impl WriteXml for ColorFormat {
    fn write_xml<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        match self {
            Self::Rgb(rgb) => {
                write!(w, r#"<a:srgbClr val="{}"/>"#, rgb.to_hex())
            }
            Self::Theme(tc) => {
                let val = tc.theme_color.to_xml_str();
                match tc.brightness {
                    Some(b) if b > 0.0 => {
                        // EMU values fit in i64 range
                        #[allow(clippy::cast_possible_truncation)]
                        let lum_mod = ((1.0 - b) * 100_000.0) as i64;
                        #[allow(clippy::cast_possible_truncation)]
                        let lum_off = (b * 100_000.0) as i64;
                        write!(
                            w,
                            r#"<a:schemeClr val="{val}"><a:lumMod val="{lum_mod}"/><a:lumOff val="{lum_off}"/></a:schemeClr>"#
                        )
                    }
                    Some(b) if b < 0.0 => {
                        // EMU values fit in i64 range
                        #[allow(clippy::cast_possible_truncation)]
                        let lum_mod = ((1.0 - b.abs()) * 100_000.0) as i64;
                        write!(
                            w,
                            r#"<a:schemeClr val="{val}"><a:lumMod val="{lum_mod}"/></a:schemeClr>"#
                        )
                    }
                    _ => {
                        write!(w, r#"<a:schemeClr val="{val}"/>"#)
                    }
                }
            }
            Self::Hsl(hsl) => {
                // EMU values fit in i64 range
                #[allow(clippy::cast_possible_truncation)]
                let hue = (hsl.hue * 60_000.0) as i64;
                #[allow(clippy::cast_possible_truncation)]
                let sat = (hsl.saturation * 1_000.0) as i64;
                #[allow(clippy::cast_possible_truncation)]
                let lum = (hsl.luminance * 1_000.0) as i64;
                write!(w, r#"<a:hslClr hue="{hue}" sat="{sat}" lum="{lum}"/>"#)
            }
            Self::System(sys) => {
                let val = sys.val.to_xml_str();
                if let Some(ref lc) = sys.last_color {
                    write!(w, r#"<a:sysClr val="{val}" lastClr="{lc}"/>"#)
                } else {
                    write!(w, r#"<a:sysClr val="{val}"/>"#)
                }
            }
            Self::Preset(pc) => {
                write!(w, r#"<a:prstClr val="{}"/>"#, pc.val.to_xml_str())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enums::dml::{MsoThemeColorIndex, PresetColorVal, SystemColorVal};

    #[test]
    fn test_rgb_color_xml() {
        let c = ColorFormat::rgb(255, 0, 0);
        assert_eq!(c.to_xml_string(), r#"<a:srgbClr val="FF0000"/>"#);
    }

    #[test]
    fn test_theme_color_xml_no_brightness() {
        let c = ColorFormat::theme(MsoThemeColorIndex::Accent1);
        assert_eq!(c.to_xml_string(), r#"<a:schemeClr val="accent1"/>"#);
    }

    #[test]
    fn test_theme_color_xml_tint() {
        let c = ColorFormat::theme_with_brightness(MsoThemeColorIndex::Accent1, 0.4);
        let xml = c.to_xml_string();
        assert!(xml.contains(r#"val="accent1""#));
        assert!(xml.contains(r#"<a:lumMod val="60000"/>"#));
        assert!(xml.contains(r#"<a:lumOff val="40000"/>"#));
    }

    #[test]
    fn test_theme_color_xml_shade() {
        let c = ColorFormat::theme_with_brightness(MsoThemeColorIndex::Dark1, -0.25);
        let xml = c.to_xml_string();
        assert!(xml.contains(r#"val="dk1""#));
        assert!(xml.contains(r#"<a:lumMod val="75000"/>"#));
        assert!(!xml.contains("lumOff"));
    }

    #[test]
    fn test_theme_color_zero_brightness() {
        let c = ColorFormat::theme_with_brightness(MsoThemeColorIndex::Text1, 0.0);
        assert_eq!(c.to_xml_string(), r#"<a:schemeClr val="tx1"/>"#);
    }

    #[test]
    fn test_hsl_color_xml() {
        let c = ColorFormat::hsl(180.0, 50.0, 75.0);
        assert_eq!(
            c.to_xml_string(),
            r#"<a:hslClr hue="10800000" sat="50000" lum="75000"/>"#
        );
    }

    #[test]
    fn test_hsl_color_zero() {
        let c = ColorFormat::hsl(0.0, 0.0, 0.0);
        assert_eq!(c.to_xml_string(), r#"<a:hslClr hue="0" sat="0" lum="0"/>"#);
    }

    #[test]
    fn test_hsl_color_max() {
        let c = ColorFormat::hsl(360.0, 100.0, 100.0);
        assert_eq!(
            c.to_xml_string(),
            r#"<a:hslClr hue="21600000" sat="100000" lum="100000"/>"#
        );
    }

    #[test]
    fn test_system_color_xml() {
        let c = ColorFormat::system("windowText");
        assert_eq!(c.to_xml_string(), r#"<a:sysClr val="windowText"/>"#);
    }

    #[test]
    fn test_system_color_with_last_color() {
        let c = ColorFormat::system_with_last_color("windowText", "000000");
        assert_eq!(
            c.to_xml_string(),
            r#"<a:sysClr val="windowText" lastClr="000000"/>"#
        );
    }

    #[test]
    fn test_preset_color_xml() {
        let c = ColorFormat::preset("red");
        assert_eq!(c.to_xml_string(), r#"<a:prstClr val="red"/>"#);
    }

    #[test]
    fn test_preset_color_blue() {
        let c = ColorFormat::preset("blue");
        assert_eq!(c.to_xml_string(), r#"<a:prstClr val="blue"/>"#);
    }

    #[test]
    fn test_hsl_color_struct() {
        let c = ColorFormat::hsl(120.0, 80.0, 60.0);
        match &c {
            ColorFormat::Hsl(hsl) => {
                assert_eq!(hsl.hue, 120.0);
                assert_eq!(hsl.saturation, 80.0);
                assert_eq!(hsl.luminance, 60.0);
            }
            _ => panic!("expected Hsl variant"), // EXCEPTION(test-only)
        }
    }

    #[test]
    fn test_system_color_struct() {
        let c = ColorFormat::system("window");
        match &c {
            ColorFormat::System(sys) => {
                assert_eq!(sys.val, SystemColorVal::Window);
                assert!(sys.last_color.is_none());
            }
            _ => panic!("expected System variant"), // EXCEPTION(test-only)
        }
    }

    #[test]
    fn test_preset_color_struct() {
        let c = ColorFormat::preset("green");
        match &c {
            ColorFormat::Preset(pc) => {
                assert_eq!(pc.val, PresetColorVal::Green);
            }
            _ => panic!("expected Preset variant"), // EXCEPTION(test-only)
        }
    }

    #[test]
    fn test_color_clone_eq() {
        let c1 = ColorFormat::hsl(90.0, 50.0, 50.0);
        let c2 = c1.clone();
        assert_eq!(c1, c2);

        let c3 = ColorFormat::system("windowText");
        let c4 = c3.clone();
        assert_eq!(c3, c4);

        let c5 = ColorFormat::preset("red");
        let c6 = c5.clone();
        assert_eq!(c5, c6);
    }

    #[test]
    fn test_color_type_rgb() {
        let c = ColorFormat::rgb(255, 0, 0);
        assert_eq!(c.color_type(), crate::enums::dml::MsoColorType::Rgb);
    }

    #[test]
    fn test_color_type_theme() {
        let c = ColorFormat::theme(MsoThemeColorIndex::Accent1);
        assert_eq!(c.color_type(), crate::enums::dml::MsoColorType::Scheme);
    }

    #[test]
    fn test_color_type_hsl() {
        let c = ColorFormat::hsl(180.0, 50.0, 75.0);
        assert_eq!(c.color_type(), crate::enums::dml::MsoColorType::Hsl);
    }

    #[test]
    fn test_color_type_system() {
        let c = ColorFormat::system("windowText");
        assert_eq!(c.color_type(), crate::enums::dml::MsoColorType::System);
    }

    #[test]
    fn test_color_type_preset() {
        let c = ColorFormat::preset("red");
        assert_eq!(c.color_type(), crate::enums::dml::MsoColorType::Preset);
    }
}
