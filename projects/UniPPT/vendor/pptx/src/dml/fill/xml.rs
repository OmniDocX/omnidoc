//! `WriteXml` implementation for `FillFormat`.

use super::FillFormat;
use crate::dml::color::ColorFormat;
use crate::xml_util::WriteXml;

fn write_color_with_opacity<W: std::fmt::Write>(
    color: &ColorFormat,
    opacity: f64,
    writer: &mut W,
) -> std::fmt::Result {
    let opacity = opacity.clamp(0.0, 1.0);
    if (opacity - 1.0).abs() < f64::EPSILON {
        return color.write_xml(writer);
    }

    let tag = match color {
        ColorFormat::Rgb(_) => "srgbClr",
        ColorFormat::Theme(_) => "schemeClr",
        ColorFormat::Hsl(_) => "hslClr",
        ColorFormat::System(_) => "sysClr",
        ColorFormat::Preset(_) => "prstClr",
    };
    let alpha = (opacity * 100_000.0).round() as i64;
    let mut xml = color.to_xml_string();
    let alpha_xml = format!(r#"<a:alpha val="{alpha}"/>"#);
    if xml.ends_with("/>") {
        xml.truncate(xml.len() - 2);
        write!(writer, "{xml}>{alpha_xml}</a:{tag}>")
    } else if let Some(index) = xml.rfind(&format!("</a:{tag}>")) {
        xml.insert_str(index, &alpha_xml);
        writer.write_str(&xml)
    } else {
        // Forward-compatible fallback for a future `ColorFormat` variant.
        color.write_xml(writer)
    }
}

impl WriteXml for FillFormat {
    fn write_xml<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        match self {
            Self::NoFill => w.write_str("<a:noFill/>"),
            Self::Solid(sf) => {
                w.write_str("<a:solidFill>")?;
                write_color_with_opacity(&sf.color, sf.opacity, w)?;
                w.write_str("</a:solidFill>")
            }
            Self::Gradient(gf) => {
                w.write_str("<a:gradFill>")?;
                w.write_str("<a:gsLst>")?;
                for stop in &gf.stops {
                    // EMU values fit in i64 range
                    #[allow(clippy::cast_possible_truncation)]
                    let pos = (stop.position * 100_000.0) as i64;
                    write!(w, r#"<a:gs pos="{pos}">"#)?;
                    write_color_with_opacity(&stop.color, stop.opacity, w)?;
                    w.write_str("</a:gs>")?;
                }
                w.write_str("</a:gsLst>")?;
                if let Some(angle) = gf.angle {
                    let cw_angle = if angle == 0.0 { 0.0 } else { 360.0 - angle };
                    // EMU values fit in i64 range
                    #[allow(clippy::cast_possible_truncation)]
                    let ang = (cw_angle * 60_000.0) as i64;
                    write!(w, r#"<a:lin ang="{ang}" scaled="0"/>"#)?;
                }
                w.write_str("</a:gradFill>")
            }
            Self::Pattern(pf) => {
                w.write_str("<a:pattFill")?;
                if let Some(ref preset) = pf.preset {
                    write!(w, r#" prst="{}""#, preset.to_xml_str())?;
                }
                w.write_char('>')?;
                if let Some(ref fg) = pf.fore_color {
                    w.write_str("<a:fgClr>")?;
                    fg.write_xml(w)?;
                    w.write_str("</a:fgClr>")?;
                }
                if let Some(ref bg) = pf.back_color {
                    w.write_str("<a:bgClr>")?;
                    bg.write_xml(w)?;
                    w.write_str("</a:bgClr>")?;
                }
                w.write_str("</a:pattFill>")
            }
            Self::Picture(pf) => {
                w.write_str("<a:blipFill>")?;
                write!(w, r#"<a:blip r:embed="{}"/>"#, pf.image_r_id)?;
                if let Some(rect) = pf.source_rect {
                    write!(
                        w,
                        r#"<a:srcRect l="{}" t="{}" r="{}" b="{}"/>"#,
                        rect.left, rect.top, rect.right, rect.bottom
                    )?;
                }
                if pf.tile {
                    w.write_str("<a:tile/>")?;
                } else if pf.stretch {
                    w.write_str("<a:stretch>")?;
                    if let Some(rect) = pf.fill_rect {
                        write!(
                            w,
                            r#"<a:fillRect l="{}" t="{}" r="{}" b="{}"/>"#,
                            rect.left, rect.top, rect.right, rect.bottom
                        )?;
                    } else {
                        w.write_str("<a:fillRect/>")?;
                    }
                    w.write_str("</a:stretch>")?;
                }
                w.write_str("</a:blipFill>")
            }
            Self::Background => w.write_str("<a:grpFill/>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dml::color::ColorFormat;
    use crate::dml::fill::types::PatternFill;
    use crate::enums::dml::MsoFillType;
    use crate::enums::dml_pattern::MsoPatternType;

    #[test]
    fn test_no_fill_xml() {
        let f = FillFormat::no_fill();
        assert_eq!(f.to_xml_string(), "<a:noFill/>");
    }

    #[test]
    fn test_solid_fill_xml() {
        let f = FillFormat::solid(ColorFormat::rgb(255, 0, 0));
        let xml = f.to_xml_string();
        assert!(xml.starts_with("<a:solidFill>"));
        assert!(xml.contains(r#"<a:srgbClr val="FF0000"/>"#));
        assert!(xml.ends_with("</a:solidFill>"));
    }

    #[test]
    fn test_solid_fill_alpha_xml() {
        let f = FillFormat::solid_with_opacity(
            ColorFormat::theme(crate::enums::dml::MsoThemeColorIndex::Background1),
            0.7,
        );
        assert_eq!(
            f.to_xml_string(),
            r#"<a:solidFill><a:schemeClr val="bg1"><a:alpha val="70000"/></a:schemeClr></a:solidFill>"#
        );
    }

    #[test]
    fn test_gradient_fill_xml() {
        let f = FillFormat::linear_gradient(
            ColorFormat::rgb(255, 0, 0),
            ColorFormat::rgb(0, 0, 255),
            90.0,
        );
        let xml = f.to_xml_string();
        assert!(xml.starts_with("<a:gradFill>"));
        assert!(xml.contains("<a:gsLst>"));
        assert!(xml.contains(r#"pos="0""#));
        assert!(xml.contains(r#"pos="100000""#));
        assert!(xml.contains("FF0000"));
        assert!(xml.contains("0000FF"));
        assert!(xml.contains("<a:lin"));
        assert!(xml.ends_with("</a:gradFill>"));
    }

    #[test]
    fn test_gradient_stop_alpha_xml() {
        let f = FillFormat::Gradient(crate::dml::fill::GradientFill {
            stops: vec![crate::dml::fill::GradientStop::with_opacity(
                0.25,
                ColorFormat::rgb(1, 2, 3),
                0.42,
            )
            .unwrap()],
            angle: Some(0.0),
        });
        let xml = f.to_xml_string();
        assert!(xml.contains(r#"<a:srgbClr val="010203"><a:alpha val="42000"/></a:srgbClr>"#));
    }

    #[test]
    fn test_pattern_fill_xml() {
        let f = FillFormat::Pattern(PatternFill {
            preset: Some(MsoPatternType::Cross),
            fore_color: Some(ColorFormat::rgb(0, 0, 0)),
            back_color: Some(ColorFormat::rgb(255, 255, 255)),
        });
        let xml = f.to_xml_string();
        assert!(xml.contains(r#"prst="cross""#));
        assert!(xml.contains("<a:fgClr>"));
        assert!(xml.contains("<a:bgClr>"));
        assert!(xml.contains("000000"));
        assert!(xml.contains("FFFFFF"));
    }

    #[test]
    fn test_pattern_fill_with_enum_variants() {
        let f = FillFormat::Pattern(PatternFill {
            preset: Some(MsoPatternType::Percent50),
            fore_color: None,
            back_color: None,
        });
        let xml = f.to_xml_string();
        assert!(xml.contains(r#"prst="pct50""#));
    }

    #[test]
    fn test_pattern_fill_no_preset() {
        let f = FillFormat::Pattern(PatternFill {
            preset: None,
            fore_color: Some(ColorFormat::rgb(255, 0, 0)),
            back_color: None,
        });
        let xml = f.to_xml_string();
        assert!(xml.starts_with("<a:pattFill>"));
        assert!(!xml.contains("prst="));
    }

    #[test]
    fn test_solid_fill_convenience() {
        let f = FillFormat::solid(ColorFormat::rgb(128, 128, 128));
        match &f {
            FillFormat::Solid(sf) => {
                assert_eq!(sf.color, ColorFormat::rgb(128, 128, 128));
            }
            _ => panic!("expected Solid variant"), // EXCEPTION(test-only)
        }
    }

    #[test]
    fn test_picture_fill_stretch() {
        // EXCEPTION(unwrap): test-only code with known-valid input
        let f = FillFormat::picture("rId1").unwrap();
        let xml = f.to_xml_string();
        assert!(xml.contains(r#"r:embed="rId1""#));
        assert!(xml.contains("<a:stretch>"));
        assert!(!xml.contains("<a:tile/>"));
    }

    #[test]
    fn test_picture_fill_tiled() {
        // EXCEPTION(unwrap): test-only code with known-valid input
        let f = FillFormat::picture_tiled("rId2").unwrap();
        let xml = f.to_xml_string();
        assert!(xml.contains(r#"r:embed="rId2""#));
        assert!(xml.contains("<a:tile/>"));
        assert!(!xml.contains("<a:stretch>"));
    }

    #[test]
    fn test_background_fill_xml() {
        let f = FillFormat::background();
        assert_eq!(f.to_xml_string(), "<a:grpFill/>");
    }

    #[test]
    fn test_fill_type_no_fill() {
        let f = FillFormat::no_fill();
        assert_eq!(f.fill_type(), MsoFillType::Background);
    }

    #[test]
    fn test_fill_type_solid() {
        let f = FillFormat::solid(ColorFormat::rgb(255, 0, 0));
        assert_eq!(f.fill_type(), MsoFillType::Solid);
    }

    #[test]
    fn test_fill_type_gradient() {
        let f = FillFormat::linear_gradient(
            ColorFormat::rgb(255, 0, 0),
            ColorFormat::rgb(0, 0, 255),
            90.0,
        );
        assert_eq!(f.fill_type(), MsoFillType::Gradient);
    }

    #[test]
    fn test_fill_type_pattern() {
        let f = FillFormat::Pattern(PatternFill {
            preset: Some(MsoPatternType::Cross),
            fore_color: None,
            back_color: None,
        });
        assert_eq!(f.fill_type(), MsoFillType::Patterned);
    }

    #[test]
    fn test_fill_type_picture() {
        // EXCEPTION(unwrap): test-only code with known-valid input
        let f = FillFormat::picture("rId1").unwrap();
        assert_eq!(f.fill_type(), MsoFillType::Picture);
    }

    #[test]
    fn test_fill_type_background() {
        let f = FillFormat::background();
        assert_eq!(f.fill_type(), MsoFillType::Group);
    }
}
