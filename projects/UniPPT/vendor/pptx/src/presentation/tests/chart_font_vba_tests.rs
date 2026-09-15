use crate::media::Image;
use crate::opc::constants::relationship_type as RT;
use crate::opc::pack_uri::PackURI;
use crate::presentation::Presentation;
use crate::units::Emu;

#[test]
fn test_add_image_dedup() {
    let mut prs = Presentation::new().unwrap();

    let img = Image::from_bytes(vec![1, 2, 3, 4, 5], "image/png");
    let partname1 = prs.add_image(&img).unwrap();
    let partname2 = prs.add_image(&img).unwrap();

    // Same image should return the same partname (dedup)
    assert_eq!(partname1, partname2);
}

#[test]
fn test_add_chart_to_slide() {
    use crate::chart::data::CategoryChartData;
    use crate::enums::chart::XlChartType;
    use crate::shapes::shapetree::ShapeTree;
    use crate::units::Inches;

    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let slide_ref = prs.add_slide(&layouts[0]).unwrap();

    // Build chart data
    let mut chart_data = CategoryChartData::new();
    chart_data.add_category("Q1");
    chart_data.add_category("Q2");
    chart_data.add_category("Q3");
    chart_data.add_series("Sales", &[100.0, 150.0, 200.0]);
    chart_data.add_series("Expenses", &[80.0, 120.0, 160.0]);

    let left: Emu = Inches(1.0).into();
    let top: Emu = Inches(1.5).into();
    let width: Emu = Inches(6.0).into();
    let height: Emu = Inches(4.0).into();

    // Add the chart
    prs.add_chart_to_slide(
        &slide_ref,
        &chart_data,
        XlChartType::ColumnClustered,
        left,
        top,
        width,
        height,
    )
    .unwrap();

    // Verify the chart part was created
    let chart_partname = PackURI::new("/ppt/charts/chart1.xml").unwrap();
    let chart_part = prs.package().part(&chart_partname);
    assert!(chart_part.is_some());

    // Verify chart XML content
    let chart_xml = String::from_utf8_lossy(&chart_part.unwrap().blob);
    assert!(chart_xml.contains("<c:barChart>"));
    assert!(chart_xml.contains("<c:barDir val=\"col\"/>"));
    assert!(chart_xml.contains("Sales"));
    assert!(chart_xml.contains("Q1"));

    // Verify the slide has a graphicFrame with chart reference
    let slide_xml = prs.slide_xml(&slide_ref).unwrap();
    let slide_str = String::from_utf8_lossy(slide_xml);
    assert!(slide_str.contains("graphicFrame"));
    assert!(slide_str.contains("chart"));

    // Verify the slide has a chart relationship
    let slide_part = prs.package().part(&slide_ref.partname).unwrap();
    let chart_rels = slide_part.rels.all_by_reltype(RT::CHART);
    assert_eq!(chart_rels.len(), 1);

    // Parse shapes and verify graphicFrame is present
    let tree = ShapeTree::from_slide_xml(slide_xml).unwrap();
    assert_eq!(tree.len(), 1);
    if let crate::shapes::Shape::GraphicFrame(gf) = &tree.shapes[0] {
        assert!(gf.has_chart);
    } else {
        panic!("Expected GraphicFrame shape");
    }

    // Save and reopen to verify round-trip
    let bytes = prs.to_bytes().unwrap();
    let prs2 = Presentation::from_bytes(&bytes).unwrap();
    let chart_part2 = prs2.package().part(&chart_partname);
    assert!(chart_part2.is_some());
}

#[test]
fn test_add_multiple_charts() {
    use crate::chart::data::CategoryChartData;
    use crate::enums::chart::XlChartType;
    use crate::units::Inches;

    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let slide_ref = prs.add_slide(&layouts[0]).unwrap();

    let mut data1 = CategoryChartData::new();
    data1.add_category("A");
    data1.add_series("S1", &[1.0]);

    let mut data2 = CategoryChartData::new();
    data2.add_category("X");
    data2.add_series("S2", &[2.0]);

    let left: Emu = Inches(0.5).into();
    let top: Emu = Inches(0.5).into();
    let w: Emu = Inches(4.0).into();
    let h: Emu = Inches(3.0).into();

    // Add two charts to the same slide
    prs.add_chart_to_slide(&slide_ref, &data1, XlChartType::Pie, left, top, w, h)
        .unwrap();
    prs.add_chart_to_slide(&slide_ref, &data2, XlChartType::Line, left, top, w, h)
        .unwrap();

    // Verify both chart parts exist
    let c1 = PackURI::new("/ppt/charts/chart1.xml").unwrap();
    let c2 = PackURI::new("/ppt/charts/chart2.xml").unwrap();
    assert!(prs.package().part(&c1).is_some());
    assert!(prs.package().part(&c2).is_some());

    // Verify two chart relationships on the slide
    let slide_part = prs.package().part(&slide_ref.partname).unwrap();
    let chart_rels = slide_part.rels.all_by_reltype(RT::CHART);
    assert_eq!(chart_rels.len(), 2);

    // Verify chart1 is a pie chart, chart2 is a line chart
    let c1_xml = String::from_utf8_lossy(&prs.package().part(&c1).unwrap().blob);
    let c2_xml = String::from_utf8_lossy(&prs.package().part(&c2).unwrap().blob);
    assert!(c1_xml.contains("<c:pieChart>"));
    assert!(c2_xml.contains("<c:lineChart>"));
}

// --- Embedded font tests ---

#[test]
fn test_embedded_fonts_empty() {
    let prs = Presentation::new().unwrap();
    let fonts = prs.embedded_fonts().unwrap();
    assert!(fonts.is_empty());
}

#[test]
fn test_add_and_read_embedded_font() {
    let mut prs = Presentation::new().unwrap();

    let font = crate::embedded_font::EmbeddedFont::from_bytes(
        vec![0x00, 0x01, 0x00, 0x00, 0xAA, 0xBB],
        "TestFont",
        false,
        false,
    );
    prs.add_embedded_font(&font).unwrap();

    // Verify the font data part exists
    let font_uri = crate::opc::pack_uri::PackURI::new("/ppt/fonts/font1.fntdata").unwrap();
    let font_part = prs.package().part(&font_uri);
    assert!(font_part.is_some());
    assert_eq!(
        font_part.unwrap().blob,
        vec![0x00, 0x01, 0x00, 0x00, 0xAA, 0xBB]
    );

    // Verify the presentation XML contains the embedded font entry
    let pres_part = prs
        .package()
        .part_by_reltype(crate::opc::constants::relationship_type::OFFICE_DOCUMENT)
        .unwrap();
    let pres_xml = std::str::from_utf8(&pres_part.blob).unwrap();
    assert!(pres_xml.contains("embeddedFontLst"));
    assert!(pres_xml.contains("TestFont"));
}

#[test]
fn test_add_bold_italic_font() {
    let mut prs = Presentation::new().unwrap();
    let font =
        crate::embedded_font::EmbeddedFont::from_bytes(vec![1, 2, 3], "BoldItalicFont", true, true);
    prs.add_embedded_font(&font).unwrap();

    let pres_part = prs
        .package()
        .part_by_reltype(crate::opc::constants::relationship_type::OFFICE_DOCUMENT)
        .unwrap();
    let pres_xml = std::str::from_utf8(&pres_part.blob).unwrap();
    assert!(pres_xml.contains("p:boldItalic"));
}

// --- .pptm (macro-enabled) tests ---

#[test]
fn test_is_macro_enabled_default() {
    let prs = Presentation::new().unwrap();
    assert!(!prs.is_macro_enabled());
}

#[test]
fn test_save_as_pptm_changes_content_type() {
    let mut prs = Presentation::new().unwrap();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();

    prs.save_as_pptm(&path).unwrap();
    assert!(prs.is_macro_enabled());

    // Reopen and verify
    let prs2 = Presentation::open(&path).unwrap();
    assert!(prs2.is_macro_enabled());
}

#[test]
fn test_vba_project_none_by_default() {
    let prs = Presentation::new().unwrap();
    let vba = prs.vba_project().unwrap();
    assert!(vba.is_none());
}

#[test]
fn test_set_and_get_vba_project() {
    let mut prs = Presentation::new().unwrap();
    let vba_data = vec![0xCC, 0xDD, 0xEE, 0xFF];
    prs.set_vba_project(vba_data.clone()).unwrap();

    let retrieved = prs.vba_project().unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap(), vba_data);
}

#[test]
fn test_pptm_round_trip_with_vba() {
    let mut prs = Presentation::new().unwrap();
    let vba_data = vec![0x01, 0x02, 0x03];
    prs.set_vba_project(vba_data.clone()).unwrap();

    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    prs.save_as_pptm(&path).unwrap();

    let prs2 = Presentation::open(&path).unwrap();
    assert!(prs2.is_macro_enabled());
    let vba2 = prs2.vba_project().unwrap();
    assert_eq!(vba2.unwrap(), vba_data);
}
