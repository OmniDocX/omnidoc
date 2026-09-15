use super::*;
use crate::dml::color::ColorFormat;
use crate::dml::fill::FillFormat;
use crate::dml::line::LineFormat;
use crate::enums::dml::{MsoLineDashStyle, MsoThemeColorIndex, SystemColorVal};
use crate::enums::text::{
    MsoAutoSize, MsoTextUnderlineType, MsoVerticalAnchor, PpParagraphAlignment,
};
use crate::text::font::RgbColor;
use crate::units::Emu;
use crate::xml_util::WriteXml;

#[test]
fn test_parse_solid_fill_rgb() {
    let xml = br#"<a:spPr><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></a:spPr>"#;
    let fill = parse_fill_from_xml(xml).unwrap();
    assert!(fill.is_some());
    match fill.unwrap() {
        FillFormat::Solid(sf) => match sf.color {
            ColorFormat::Rgb(rgb) => {
                assert_eq!(rgb.r, 255);
                assert_eq!(rgb.g, 0);
                assert_eq!(rgb.b, 0);
            }
            _ => panic!("Expected RGB color"),
        },
        _ => panic!("Expected Solid fill"),
    }
}

#[test]
fn test_parse_no_fill() {
    let xml = br#"<a:spPr><a:noFill/></a:spPr>"#;
    let fill = parse_fill_from_xml(xml).unwrap();
    assert_eq!(fill, Some(FillFormat::NoFill));
}

#[test]
fn test_parse_gradient_fill() {
    let xml = br#"<a:spPr><a:gradFill><a:gsLst><a:gs pos="0"><a:srgbClr val="FF0000"/></a:gs><a:gs pos="100000"><a:srgbClr val="0000FF"/></a:gs></a:gsLst><a:lin ang="16200000" scaled="0"/></a:gradFill></a:spPr>"#;
    let fill = parse_fill_from_xml(xml).unwrap();
    assert!(fill.is_some());
    match fill.unwrap() {
        FillFormat::Gradient(gf) => {
            assert_eq!(gf.stops.len(), 2);
            assert_eq!(gf.stops[0].position, 0.0);
            assert_eq!(gf.stops[1].position, 1.0);
            assert!(gf.angle.is_some());
        }
        _ => panic!("Expected Gradient fill"),
    }
}

#[test]
fn test_parse_solid_fill_scheme_color() {
    let xml = br#"<a:spPr><a:solidFill><a:schemeClr val="accent1"/></a:solidFill></a:spPr>"#;
    let fill = parse_fill_from_xml(xml).unwrap();
    assert!(fill.is_some());
    match fill.unwrap() {
        FillFormat::Solid(sf) => match sf.color {
            ColorFormat::Theme(tc) => {
                assert_eq!(tc.theme_color, MsoThemeColorIndex::Accent1);
                assert!(tc.brightness.is_none());
            }
            _ => panic!("Expected Theme color"),
        },
        _ => panic!("Expected Solid fill"),
    }
}

#[test]
fn test_parse_solid_fill_direct_alpha() {
    let xml = br#"<a:spPr><a:solidFill><a:schemeClr val="bg1"><a:alpha val="70000"/></a:schemeClr></a:solidFill></a:spPr>"#;
    let fill = parse_fill_from_xml(xml).unwrap().unwrap();
    let FillFormat::Solid(fill) = fill else {
        panic!("Expected Solid fill");
    };
    assert_eq!(fill.opacity, 0.7);
    assert!(matches!(fill.color, ColorFormat::Theme(_)));
    assert!(FillFormat::Solid(fill)
        .to_xml_string()
        .contains(r#"<a:alpha val="70000"/>"#));
}

#[test]
fn test_parse_gradient_stop_direct_alpha() {
    let xml = br#"<a:spPr><a:gradFill><a:gsLst><a:gs pos="25000"><a:srgbClr val="112233"><a:alpha val="42000"/></a:srgbClr></a:gs></a:gsLst><a:lin ang="0"/></a:gradFill></a:spPr>"#;
    let fill = parse_fill_from_xml(xml).unwrap().unwrap();
    let FillFormat::Gradient(fill) = fill else {
        panic!("Expected Gradient fill");
    };
    assert_eq!(fill.stops.len(), 1);
    assert_eq!(fill.stops[0].position, 0.25);
    assert_eq!(fill.stops[0].opacity, 0.42);
    assert!(FillFormat::Gradient(fill)
        .to_xml_string()
        .contains(r#"<a:alpha val="42000"/>"#));
}

#[test]
fn test_parse_line_solid() {
    let xml = br#"<a:ln w="12700"><a:solidFill><a:srgbClr val="000000"/></a:solidFill></a:ln>"#;
    let line = parse_line_from_xml(xml).unwrap();
    assert!(line.is_some());
    let l = line.unwrap();
    assert_eq!(l.width, Some(Emu(12700)));
    assert!(l.color.is_some());
    assert!(l.fill.is_some());
}

#[test]
fn test_parse_line_with_dash() {
    let xml = br#"<a:ln w="25400"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill><a:prstDash val="dash"/></a:ln>"#;
    let line = parse_line_from_xml(xml).unwrap();
    assert!(line.is_some());
    let l = line.unwrap();
    assert_eq!(l.width, Some(Emu(25400)));
    assert_eq!(l.dash_style, Some(MsoLineDashStyle::Dash));
}

#[test]
fn test_parse_text_frame_basic() {
    let xml = br#"<p:txBody><a:bodyPr wrap="square" lIns="91440" tIns="45720" rIns="91440" bIns="45720" anchor="ctr"/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1800" b="1"/><a:t>Hello World</a:t></a:r></a:p></p:txBody>"#;
    let tf = parse_text_frame_from_xml(xml).unwrap();
    assert!(tf.is_some());
    let tf = tf.unwrap();
    assert!(tf.word_wrap);
    assert_eq!(tf.vertical_anchor, Some(MsoVerticalAnchor::Middle));
    assert_eq!(tf.margin_left, Some(crate::units::Emu(91440)));
    assert_eq!(tf.paragraphs().len(), 1);
    let para = &tf.paragraphs()[0];
    assert_eq!(para.alignment, Some(PpParagraphAlignment::Center));
    assert_eq!(para.runs().len(), 1);
    assert_eq!(para.runs()[0].text(), "Hello World");
    assert_eq!(para.runs()[0].font().bold, Some(true));
    assert_eq!(para.runs()[0].font().size, Some(18.0));
}

#[test]
fn test_parse_text_frame_east_asian_vertical_mode() {
    let xml = r#"<p:txBody><a:bodyPr vert="eaVert" wrap="none"/><a:lstStyle/><a:p><a:r><a:t>我的世界</a:t></a:r></a:p></p:txBody>"#;
    let tf = parse_text_frame_from_xml(xml.as_bytes()).unwrap().unwrap();
    assert_eq!(tf.vertical_type.as_deref(), Some("eaVert"));
    assert!(!tf.word_wrap);
}

#[test]
fn test_parse_text_frame_multi_paragraph() {
    let xml = br#"<p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang="en-US"/><a:t>Line 1</a:t></a:r></a:p><a:p><a:r><a:rPr lang="en-US"/><a:t>Line 2</a:t></a:r></a:p></p:txBody>"#;
    let tf = parse_text_frame_from_xml(xml).unwrap().unwrap();
    assert_eq!(tf.paragraphs().len(), 2);
    assert_eq!(tf.text(), "Line 1\nLine 2");
}

#[test]
fn test_parse_text_frame_with_font_color() {
    let xml = br#"<p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang="en-US" sz="2400" b="1" i="1" u="sng"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill><a:latin typeface="Arial"/></a:rPr><a:t>Styled</a:t></a:r></a:p></p:txBody>"#;
    let tf = parse_text_frame_from_xml(xml).unwrap().unwrap();
    let run = &tf.paragraphs()[0].runs()[0];
    let font = run.font();
    assert_eq!(font.bold, Some(true));
    assert_eq!(font.italic, Some(true));
    assert_eq!(font.size, Some(24.0));
    assert_eq!(font.underline, Some(MsoTextUnderlineType::SingleLine));
    assert_eq!(font.color, Some(RgbColor::new(255, 0, 0)));
    assert_eq!(font.name.as_deref(), Some("Arial"));
}

#[test]
fn test_parse_text_frame_theme_color_with_luminance_modifier() {
    let xml = br#"<p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang="en-US"><a:solidFill><a:schemeClr val="bg1"><a:lumMod val="50000"/></a:schemeClr></a:solidFill></a:rPr><a:t>40</a:t></a:r></a:p></p:txBody>"#;
    let frame = parse_text_frame_from_xml(xml).unwrap().unwrap();
    let fill = frame.paragraphs()[0].runs()[0]
        .font()
        .fill
        .as_ref()
        .expect("theme font fill");
    let FillFormat::Solid(solid) = fill else {
        panic!("expected solid font fill");
    };
    let ColorFormat::Theme(theme) = &solid.color else {
        panic!("expected theme font color");
    };
    assert_eq!(theme.theme_color, MsoThemeColorIndex::Background1);
    assert_eq!(theme.brightness, Some(-0.5));
}

#[test]
fn test_parse_text_frame_with_gradient_font_fill() {
    let xml = r#"<p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang="zh-CN" sz="3600"><a:gradFill><a:gsLst><a:gs pos="0"><a:srgbClr val="FFE09F"><a:alpha val="80000"/></a:srgbClr></a:gs><a:gs pos="93000"><a:srgbClr val="FF779B"><a:alpha val="80000"/></a:srgbClr></a:gs></a:gsLst><a:lin ang="2700000" scaled="1"/></a:gradFill></a:rPr><a:t>岗位认知</a:t></a:r></a:p></p:txBody>"#;
    let frame = parse_text_frame_from_xml(xml.as_bytes()).unwrap().unwrap();
    let fill = frame.paragraphs()[0].runs()[0]
        .font()
        .fill
        .as_ref()
        .expect("run gradient fill");
    let FillFormat::Gradient(gradient) = fill else {
        panic!("expected gradient font fill");
    };
    assert_eq!(gradient.stops.len(), 2);
    assert_eq!(gradient.stops[0].opacity, 0.8);
    assert_eq!(gradient.stops[1].position, 0.93);
    assert_eq!(gradient.angle, Some(315.0));
}

#[test]
fn test_parse_text_frame_no_wrap() {
    let xml = br#"<p:txBody><a:bodyPr wrap="none"/><a:lstStyle/><a:p><a:endParaRPr lang="en-US"/></a:p></p:txBody>"#;
    let tf = parse_text_frame_from_xml(xml).unwrap().unwrap();
    assert!(!tf.word_wrap);
}

#[test]
fn test_parse_text_frame_autofit() {
    let xml = br#"<p:txBody><a:bodyPr><a:normAutofit fontScale="80000"/></a:bodyPr><a:lstStyle/><a:p><a:endParaRPr lang="en-US"/></a:p></p:txBody>"#;
    let tf = parse_text_frame_from_xml(xml).unwrap().unwrap();
    assert_eq!(tf.auto_size, MsoAutoSize::TextToFitShape);
    assert_eq!(tf.font_scale, Some(80.0));
}

#[test]
fn test_parse_sp_pr() {
    let xml = br#"<p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100" cy="100"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="00FF00"/></a:solidFill><a:ln w="12700"><a:solidFill><a:srgbClr val="000000"/></a:solidFill></a:ln></p:spPr>"#;
    let (fill, line) = parse_sp_pr(xml).unwrap();
    assert!(fill.is_some());
    assert!(line.is_some());
    match fill.unwrap() {
        FillFormat::Solid(sf) => match sf.color {
            ColorFormat::Rgb(rgb) => {
                assert_eq!(rgb.r, 0);
                assert_eq!(rgb.g, 255);
                assert_eq!(rgb.b, 0);
            }
            _ => panic!("Expected RGB color"),
        },
        _ => panic!("Expected Solid fill"),
    }
    let l = line.unwrap();
    assert_eq!(l.width, Some(Emu(12700)));
}

#[test]
fn test_parse_text_frame_empty_body() {
    let xml =
        br#"<p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr lang="en-US"/></a:p></p:txBody>"#;
    let tf = parse_text_frame_from_xml(xml).unwrap().unwrap();
    assert_eq!(tf.paragraphs().len(), 1);
    assert_eq!(tf.text(), "");
}

#[test]
fn test_parse_line_no_fill() {
    let xml = br#"<a:ln w="9525"><a:noFill/></a:ln>"#;
    let line = parse_line_from_xml(xml).unwrap();
    assert!(line.is_some());
    let l = line.unwrap();
    assert_eq!(l.width, Some(Emu(9525)));
    assert_eq!(l.fill, Some(FillFormat::NoFill));
}

#[test]
fn test_parse_fill_background() {
    let xml = br#"<a:spPr><a:grpFill/></a:spPr>"#;
    let fill = parse_fill_from_xml(xml).unwrap();
    assert_eq!(fill, Some(FillFormat::Background));
}

#[test]
fn test_parse_color_system() {
    let xml = br#"<a:sysClr val="windowText" lastClr="000000"/>"#;
    let color = parse_color_from_xml(xml).unwrap();
    assert!(color.is_some());
    match color.unwrap() {
        ColorFormat::System(sys) => {
            assert_eq!(sys.val, SystemColorVal::WindowText);
            assert_eq!(sys.last_color.as_deref(), Some("000000"));
        }
        _ => panic!("Expected System color"),
    }
}

#[test]
fn test_parse_text_frame_paragraph_level() {
    let xml = br#"<p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr lvl="2" algn="r"/><a:r><a:rPr lang="en-US"/><a:t>Indented</a:t></a:r></a:p></p:txBody>"#;
    let tf = parse_text_frame_from_xml(xml).unwrap().unwrap();
    let para = &tf.paragraphs()[0];
    assert_eq!(para.level, 2);
    assert_eq!(para.alignment, Some(PpParagraphAlignment::Right));
}

#[test]
fn test_round_trip_text_frame() {
    // Parse a text frame, then generate XML and parse again
    let xml = br#"<p:txBody><a:bodyPr wrap="square" lIns="91440" tIns="45720" rIns="91440" bIns="45720" anchor="ctr"/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr lang="en-US" sz="1800" b="1"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill><a:latin typeface="Arial"/></a:rPr><a:t>Hello</a:t></a:r></a:p></p:txBody>"#;
    let tf = parse_text_frame_from_xml(xml).unwrap().unwrap();
    assert_eq!(tf.text(), "Hello");
    assert_eq!(tf.paragraphs()[0].runs()[0].font().size, Some(18.0));
    assert_eq!(tf.paragraphs()[0].runs()[0].font().bold, Some(true));
    assert_eq!(
        tf.paragraphs()[0].runs()[0].font().color,
        Some(RgbColor::new(255, 0, 0))
    );
    assert_eq!(
        tf.paragraphs()[0].runs()[0].font().name.as_deref(),
        Some("Arial")
    );

    // Generate XML from parsed struct
    let generated = tf.to_xml_string();
    // Re-parse the generated XML
    let tf2 = parse_text_frame_from_xml(generated.as_bytes())
        .unwrap()
        .unwrap();
    assert_eq!(tf2.text(), "Hello");
    assert_eq!(tf2.paragraphs()[0].runs()[0].font().bold, Some(true));
    assert_eq!(tf2.paragraphs()[0].runs()[0].font().size, Some(18.0));
}

#[test]
fn test_round_trip_fill_and_line() {
    // Create fill + line, generate XML, parse back
    let fill = FillFormat::solid(ColorFormat::rgb(128, 0, 255));
    let line = LineFormat::solid(ColorFormat::rgb(0, 0, 0), Emu(12700));

    // Generate XML
    let mut sp_pr = String::from("<p:spPr>");
    sp_pr.push_str(&fill.to_xml_string());
    if let Some(ln_xml) = line.to_xml_string() {
        sp_pr.push_str(&ln_xml);
    }
    sp_pr.push_str("</p:spPr>");

    // Parse back
    let (parsed_fill, parsed_line) = parse_sp_pr(sp_pr.as_bytes()).unwrap();
    assert!(parsed_fill.is_some());
    assert!(parsed_line.is_some());

    match parsed_fill.unwrap() {
        FillFormat::Solid(sf) => match sf.color {
            ColorFormat::Rgb(rgb) => {
                assert_eq!(rgb.r, 128);
                assert_eq!(rgb.g, 0);
                assert_eq!(rgb.b, 255);
            }
            _ => panic!("Expected RGB"),
        },
        _ => panic!("Expected Solid"),
    }

    let l = parsed_line.unwrap();
    assert_eq!(l.width, Some(Emu(12700)));
}

#[test]
fn test_parse_text_frame_script_typefaces() {
    let xml = r#"<p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr><a:defRPr><a:latin typeface="+mn-lt"/><a:ea typeface="+mn-ea"/><a:cs typeface="+mn-cs"/></a:defRPr></a:pPr><a:r><a:rPr lang="zh-CN"><a:latin typeface="Calibri"/><a:ea typeface="Microsoft YaHei"/><a:cs typeface="Arial"/><a:sym typeface="Wingdings"/></a:rPr><a:t>ABC中文</a:t></a:r></a:p></p:txBody>"#;
    let tf = parse_text_frame_from_xml(xml.as_bytes()).unwrap().unwrap();
    let paragraph = &tf.paragraphs()[0];
    let paragraph_font = paragraph.font.as_ref().unwrap();
    assert_eq!(paragraph_font.name.as_deref(), Some("+mn-lt"));
    assert_eq!(paragraph_font.east_asia_name.as_deref(), Some("+mn-ea"));
    assert_eq!(
        paragraph_font.complex_script_name.as_deref(),
        Some("+mn-cs")
    );

    let font = paragraph.runs()[0].font();
    assert_eq!(font.name.as_deref(), Some("Calibri"));
    assert_eq!(font.east_asia_name.as_deref(), Some("Microsoft YaHei"));
    assert_eq!(font.complex_script_name.as_deref(), Some("Arial"));
    assert_eq!(font.symbol_name.as_deref(), Some("Wingdings"));
}

#[test]
fn test_parse_text_frame_preserves_whitespace_only_runs() {
    let xml = br#"<p:txBody xmlns:p="p" xmlns:a="a"><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr/><a:t>Lorem</a:t></a:r><a:r><a:rPr/><a:t xml:space="preserve"> </a:t></a:r><a:r><a:rPr/><a:t>ipsum</a:t></a:r></a:p></p:txBody>"#;
    let frame = parse_text_frame_from_xml(xml).unwrap().unwrap();
    assert_eq!(frame.paragraphs()[0].runs()[0].text(), "Lorem");
    assert_eq!(frame.paragraphs()[0].runs()[1].text(), " ");
    assert_eq!(frame.paragraphs()[0].runs()[2].text(), "ipsum");
}

#[test]
fn test_parse_text_frame_paragraph_spacing_children() {
    let xml = br#"<p:txBody xmlns:p="p" xmlns:a="a"><a:bodyPr/><a:lstStyle/><a:p><a:pPr><a:lnSpc><a:spcPct val="190000"/></a:lnSpc><a:spcBef><a:spcPts val="600"/></a:spcBef><a:spcAft><a:spcPts val="300"/></a:spcAft></a:pPr><a:r><a:t>Text</a:t></a:r></a:p></p:txBody>"#;
    let frame = parse_text_frame_from_xml(xml).unwrap().unwrap();
    let paragraph = &frame.paragraphs()[0];
    assert_eq!(paragraph.line_spacing, Some(1.9));
    assert_eq!(paragraph.space_before, Some(6.0));
    assert_eq!(paragraph.space_after, Some(3.0));
}

#[test]
fn test_parse_picture_fill_crop_and_tile() {
    let xml = br#"<p:spPr xmlns:p="p" xmlns:a="a" xmlns:r="r"><a:blipFill><a:blip r:embed="rId9"/><a:srcRect l="1000" t="2000" r="3000" b="4000"/><a:tile/></a:blipFill></p:spPr>"#;
    let fill = parse_fill_from_xml(xml).unwrap().unwrap();
    let FillFormat::Picture(fill) = fill else {
        panic!("Expected Picture fill");
    };
    assert_eq!(fill.image_r_id.as_str(), "rId9");
    assert!(fill.tile);
    assert!(!fill.stretch);
    assert_eq!(
        fill.source_rect,
        Some(crate::dml::fill::PictureSourceRect {
            left: 1000,
            top: 2000,
            right: 3000,
            bottom: 4000,
        })
    );
    assert!(fill.image_data.is_none());
}

#[test]
fn test_parse_sp_pr_picture_fill_stretch() {
    let xml = br#"<p:spPr xmlns:p="p" xmlns:a="a" xmlns:r="r"><a:blipFill><a:blip r:embed="rId4"/><a:stretch><a:fillRect l="-15048" t="-15048" r="-15048" b="-15048"/></a:stretch></a:blipFill></p:spPr>"#;
    let (fill, line) = parse_sp_pr(xml).unwrap();
    assert!(line.is_none());
    let Some(FillFormat::Picture(fill)) = fill else {
        panic!("Expected Picture fill");
    };
    assert_eq!(fill.image_r_id.as_str(), "rId4");
    assert!(fill.stretch);
    assert!(!fill.tile);
    assert_eq!(
        fill.fill_rect,
        Some(crate::dml::fill::PictureSourceRect {
            left: -15048,
            top: -15048,
            right: -15048,
            bottom: -15048,
        })
    );
}

#[test]
fn test_parse_custom_geometry_to_svg_path() {
    let xml = br#"<p:spPr xmlns:p="p" xmlns:a="a"><a:custGeom><a:avLst/><a:pathLst><a:path w="200" h="100"><a:moveTo><a:pt x="0" y="100"/></a:moveTo><a:lnTo><a:pt x="100" y="0"/></a:lnTo><a:cubicBezTo><a:pt x="120" y="10"/><a:pt x="180" y="50"/><a:pt x="200" y="100"/></a:cubicBezTo><a:close/></a:path></a:pathLst></a:custGeom></p:spPr>"#;
    let geometry = parse_custom_geometry_from_xml(xml).unwrap().unwrap();

    assert_eq!(geometry.width(), 200);
    assert_eq!(geometry.height(), 100);
    assert_eq!(
        geometry.to_svg_path_data(),
        "M 0 100 L 100 0 C 120 10 180 50 200 100 Z"
    );
}
