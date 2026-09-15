//! End-to-end integration tests for the rust-pptx library.
//!
//! These tests exercise the public API to verify that presentations can be
//! created, populated with content, saved, and re-opened.

use pptx::chart::data::CategoryChartData;
use pptx::chart::xmlwriter::ChartXmlWriter;
use pptx::core_properties::CoreProperties;
use pptx::dml::effect::{ShadowFormat, ShadowType};
use pptx::dml::ColorFormat;
use pptx::enums::chart::XlChartType;
use pptx::enums::dml::MsoThemeColorIndex;
use pptx::enums::text::PpParagraphAlignment;
use pptx::shapes::action::ActionSetting;
use pptx::shapes::autoshape::AutoShape;
use pptx::shapes::freeform::FreeformBuilder;
use pptx::text::font::RgbColor;
use pptx::text::BulletFormat;
use pptx::units::{Emu, ShapeId};
use pptx::{FillFormat, Inches, LineFormat, Presentation, ShapeTree, Table, TextFrame, WriteXml};

// ---------------------------------------------------------------------------
// Test 1: Create empty presentation from default template and save
// ---------------------------------------------------------------------------

#[test]
fn test_create_empty_presentation_and_save() {
    let prs = Presentation::new().unwrap();
    assert_eq!(prs.slide_count().unwrap(), 0);

    // Save to bytes and verify it's a valid ZIP/PPTX
    let bytes = prs.to_bytes().unwrap();
    assert!(bytes.len() > 100);

    // ZIP files begin with PK signature (0x50, 0x4B)
    assert_eq!(&bytes[0..2], b"PK");
}

#[test]
fn test_create_empty_presentation_save_to_file() {
    let prs = Presentation::new().unwrap();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();

    prs.save(&path).unwrap();

    // Verify file was created and is non-empty
    let metadata = std::fs::metadata(&path).unwrap();
    assert!(metadata.len() > 100);
}

// ---------------------------------------------------------------------------
// Test 2: Open existing pptx file (round-trip the default template)
// ---------------------------------------------------------------------------

#[test]
fn test_open_from_bytes() {
    let prs = Presentation::new().unwrap();
    let bytes = prs.to_bytes().unwrap();

    let prs2 = Presentation::from_bytes(&bytes).unwrap();
    assert_eq!(prs2.slide_count().unwrap(), 0);
    assert_eq!(prs2.slide_layouts().unwrap().len(), 11);
}

#[test]
fn test_open_from_file() {
    let prs = Presentation::new().unwrap();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    prs.save(&path).unwrap();

    let prs2 = Presentation::open(&path).unwrap();
    assert_eq!(prs2.slide_count().unwrap(), 0);
}

#[test]
fn test_open_from_reader() {
    let prs = Presentation::new().unwrap();
    let bytes = prs.to_bytes().unwrap();
    let cursor = std::io::Cursor::new(bytes);

    let prs2 = Presentation::from_reader(cursor).unwrap();
    assert_eq!(prs2.slide_count().unwrap(), 0);
}

// ---------------------------------------------------------------------------
// Test 3: Add slides from layouts
// ---------------------------------------------------------------------------

#[test]
fn test_add_slide_from_layout() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    assert!(!layouts.is_empty());

    // The default template should have "Title Slide" as the first layout
    assert_eq!(layouts[0].name, "Title Slide");

    let slide_ref = prs.add_slide(&layouts[0]).unwrap();
    assert_eq!(slide_ref.partname.as_str(), "/ppt/slides/slide1.xml");
    assert_eq!(prs.slide_count().unwrap(), 1);
}

#[test]
fn test_add_multiple_slides_different_layouts() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();

    // Add slides using different layouts
    let s1 = prs.add_slide(&layouts[0]).unwrap(); // Title Slide
    let s2 = prs.add_slide(&layouts[1]).unwrap(); // Title and Content
    let s3 = prs.add_slide(&layouts[5]).unwrap(); // Blank (typically index 5 or 6)

    assert_eq!(prs.slide_count().unwrap(), 3);

    // Slides should be accessible in order
    let slides = prs.slides().unwrap();
    assert_eq!(slides.len(), 3);
    assert_eq!(slides[0].partname, s1.partname);
    assert_eq!(slides[1].partname, s2.partname);
    assert_eq!(slides[2].partname, s3.partname);
}

// ---------------------------------------------------------------------------
// Test 4: Create a textbox and set formatted text
// ---------------------------------------------------------------------------

#[test]
fn test_add_textbox_with_formatted_text() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let slide_ref = prs.add_slide(&layouts[0]).unwrap();

    // Generate textbox XML
    let left: Emu = Inches(1.0).into();
    let top: Emu = Inches(2.0).into();
    let width: Emu = Inches(5.0).into();
    let height: Emu = Inches(1.0).into();

    let textbox_xml =
        ShapeTree::new_textbox_xml(ShapeId(10), "TextBox 1", left, top, width, height);

    // Verify the generated XML is well-formed and contains expected elements
    assert!(textbox_xml.contains(r#"txBox="1""#));
    assert!(textbox_xml.contains(r#"id="10""#));
    assert!(textbox_xml.contains(r#"name="TextBox 1""#));

    // Inject the textbox XML into the slide's spTree
    let slide_xml = prs.slide_xml(&slide_ref).unwrap();
    let slide_str = String::from_utf8_lossy(slide_xml).into_owned();
    let updated = slide_str.replace("</p:spTree>", &format!("{}</p:spTree>", textbox_xml));
    *prs.slide_xml_mut(&slide_ref).unwrap() = updated.into_bytes();

    // Parse the updated slide and verify the shape is there
    let xml = prs.slide_xml(&slide_ref).unwrap();
    let tree = ShapeTree::from_slide_xml(xml).unwrap();
    assert!(!tree.is_empty());

    // Find our textbox
    let tb = tree.shapes.iter().find(|s| s.name() == "TextBox 1");
    assert!(tb.is_some());
    let tb = tb.unwrap();
    assert_eq!(tb.shape_id(), ShapeId(10));
    assert!(tb.has_text_frame());
}

#[test]
fn test_textframe_formatted_text_generation() {
    // Verify that TextFrame generates valid XML with formatting
    let mut tf = TextFrame::new();
    {
        let p = &mut tf.paragraphs_mut()[0];
        p.set_alignment(PpParagraphAlignment::Center);
        let r = p.add_run();
        r.set_text("Hello World");
        r.font_mut().bold = Some(true);
        r.font_mut().size = Some(24.0);
        r.font_mut().color = Some(RgbColor::new(255, 0, 0));
    }

    let xml = tf.to_xml_string();
    assert!(xml.contains(r#"algn="ctr""#));
    assert!(xml.contains(r#"b="1""#));
    assert!(xml.contains(r#"sz="2400""#));
    assert!(xml.contains("FF0000"));
    assert!(xml.contains("<a:t>Hello World</a:t>"));
}

// ---------------------------------------------------------------------------
// Test 5: Create a shape (rectangle) with fill
// ---------------------------------------------------------------------------

#[test]
fn test_add_autoshape_rectangle() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let slide_ref = prs.add_slide(&layouts[0]).unwrap();

    let left: Emu = Inches(2.0).into();
    let top: Emu = Inches(2.0).into();
    let width: Emu = Inches(3.0).into();
    let height: Emu = Inches(1.5).into();

    let shape_xml =
        ShapeTree::new_autoshape_xml(ShapeId(20), "Rectangle 1", left, top, width, height, "rect");

    assert!(shape_xml.contains(r#"prst="rect""#));
    assert!(shape_xml.contains(r#"id="20""#));

    // Inject into slide
    let slide_xml = prs.slide_xml(&slide_ref).unwrap();
    let slide_str = String::from_utf8_lossy(slide_xml).into_owned();
    let updated = slide_str.replace("</p:spTree>", &format!("{}</p:spTree>", shape_xml));
    *prs.slide_xml_mut(&slide_ref).unwrap() = updated.into_bytes();

    // Verify
    let xml = prs.slide_xml(&slide_ref).unwrap();
    let tree = ShapeTree::from_slide_xml(xml).unwrap();
    let rect = tree.shapes.iter().find(|s| s.name() == "Rectangle 1");
    assert!(rect.is_some());
    let rect = rect.unwrap();
    assert_eq!(rect.width(), width);
    assert_eq!(rect.height(), height);
}

#[test]
fn test_fill_format_xml_generation() {
    // Verify that fill formats produce valid XML that can be embedded
    let solid = FillFormat::solid(ColorFormat::rgb(0, 128, 255));
    let xml = solid.to_xml_string();
    assert!(xml.contains("<a:solidFill>"));
    assert!(xml.contains("0080FF"));

    let theme_fill = FillFormat::solid(ColorFormat::theme(MsoThemeColorIndex::Accent1));
    let xml = theme_fill.to_xml_string();
    assert!(xml.contains("<a:schemeClr"));
    assert!(xml.contains(r#"val="accent1""#));
}

#[test]
fn test_line_format_xml_generation() {
    let line = LineFormat::solid(ColorFormat::rgb(0, 0, 0), Emu(12700));
    let xml = line.to_xml_string().unwrap();
    assert!(xml.contains("<a:ln"));
    assert!(xml.contains(r#"w="12700""#));
    assert!(xml.contains("000000"));
}

// ---------------------------------------------------------------------------
// Test 6: Create a table with data
// ---------------------------------------------------------------------------

#[test]
fn test_add_table_to_slide() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let slide_ref = prs.add_slide(&layouts[0]).unwrap();

    let left: Emu = Inches(1.0).into();
    let top: Emu = Inches(2.0).into();
    let width: Emu = Inches(6.0).into();
    let height: Emu = Inches(2.0).into();

    // Use ShapeTree's XML generator for the graphicFrame container
    let table_xml =
        ShapeTree::new_table_xml(ShapeId(30), "Table 1", 3, 4, left, top, width, height);

    assert!(table_xml.contains("a:tbl"));
    assert!(table_xml.contains("a:tblGrid"));
    assert!(table_xml.contains("a:gridCol"));
    assert!(table_xml.contains("a:tr"));
    assert!(table_xml.contains("a:tc"));

    // Inject into slide
    let slide_xml = prs.slide_xml(&slide_ref).unwrap();
    let slide_str = String::from_utf8_lossy(slide_xml).into_owned();
    let updated = slide_str.replace("</p:spTree>", &format!("{}</p:spTree>", table_xml));
    *prs.slide_xml_mut(&slide_ref).unwrap() = updated.into_bytes();

    // Parse and verify
    let xml = prs.slide_xml(&slide_ref).unwrap();
    let tree = ShapeTree::from_slide_xml(xml).unwrap();
    let table_shape = tree.shapes.iter().find(|s| s.name() == "Table 1");
    assert!(table_shape.is_some());
    assert!(table_shape.unwrap().has_table());
}

#[test]
fn test_table_struct_with_data() {
    // Test the Table struct and its XML generation
    let mut table = Table::new(3, 2, Emu(5486400), Emu(1371600));

    // Populate with data
    let headers = ["Name", "Score"];
    let data = [["Alice", "95"], ["Bob", "87"]];

    for (i, header) in headers.iter().enumerate() {
        table.cell_mut(0, i).set_text(header);
    }
    for (row_idx, row_data) in data.iter().enumerate() {
        for (col_idx, value) in row_data.iter().enumerate() {
            table.cell_mut(row_idx + 1, col_idx).set_text(value);
        }
    }

    // Verify data
    assert_eq!(table.cell(0, 0).text(), "Name");
    assert_eq!(table.cell(0, 1).text(), "Score");
    assert_eq!(table.cell(1, 0).text(), "Alice");
    assert_eq!(table.cell(1, 1).text(), "95");
    assert_eq!(table.cell(2, 0).text(), "Bob");
    assert_eq!(table.cell(2, 1).text(), "87");

    // Verify XML generation
    let xml = table.to_xml_string();
    assert!(xml.contains("<a:tbl>"));
    assert!(xml.contains("Name"));
    assert!(xml.contains("Alice"));
    assert!(xml.contains("87"));
}

#[test]
fn test_table_with_formatting() {
    let mut table = Table::new(2, 2, Emu(4000000), Emu(800000));

    // Set cell text
    table.cell_mut(0, 0).set_text("Header");

    // Apply fill to a cell
    table.cell_mut(0, 0).fill = Some(FillFormat::solid(ColorFormat::rgb(200, 220, 240)));

    // Verify fill appears in XML
    let xml = table.to_xml_string();
    assert!(xml.contains("C8DCF0")); // RGB hex for (200, 220, 240)
    assert!(xml.contains("<a:solidFill>"));
}

// ---------------------------------------------------------------------------
// Test 7: Round-trip test (create -> save -> reopen -> verify)
// ---------------------------------------------------------------------------

#[test]
fn test_full_round_trip() {
    // Create a presentation with multiple slides and content
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();

    // Add three slides
    let s1 = prs.add_slide(&layouts[0]).unwrap();
    let s2 = prs.add_slide(&layouts[1]).unwrap();
    let _s3 = prs.add_slide(&layouts[0]).unwrap();

    // Add a textbox to slide 1
    let textbox_xml = ShapeTree::new_textbox_xml(
        ShapeId(10),
        "TextBox 1",
        Emu(914400),
        Emu(914400),
        Emu(4572000),
        Emu(457200),
    );
    {
        let slide_xml = prs.slide_xml(&s1).unwrap();
        let slide_str = String::from_utf8_lossy(slide_xml).into_owned();
        let updated = slide_str.replace("</p:spTree>", &format!("{}</p:spTree>", textbox_xml));
        *prs.slide_xml_mut(&s1).unwrap() = updated.into_bytes();
    }

    // Add a rectangle to slide 2
    let rect_xml = ShapeTree::new_autoshape_xml(
        ShapeId(20),
        "Rectangle 1",
        Emu(1828800),
        Emu(1828800),
        Emu(2743200),
        Emu(1371600),
        "rect",
    );
    {
        let slide_xml = prs.slide_xml(&s2).unwrap();
        let slide_str = String::from_utf8_lossy(slide_xml).into_owned();
        let updated = slide_str.replace("</p:spTree>", &format!("{}</p:spTree>", rect_xml));
        *prs.slide_xml_mut(&s2).unwrap() = updated.into_bytes();
    }

    // Save to bytes
    let bytes = prs.to_bytes().unwrap();
    assert!(bytes.len() > 1000);

    // Reopen from bytes
    let prs2 = Presentation::from_bytes(&bytes).unwrap();

    // Verify slide count
    assert_eq!(prs2.slide_count().unwrap(), 3);

    // Verify slides are in order
    let slides = prs2.slides().unwrap();
    assert_eq!(slides[0].partname.as_str(), "/ppt/slides/slide1.xml");
    assert_eq!(slides[1].partname.as_str(), "/ppt/slides/slide2.xml");
    assert_eq!(slides[2].partname.as_str(), "/ppt/slides/slide3.xml");

    // Verify slide 1 still has the textbox
    let xml1 = prs2.slide_xml(&slides[0]).unwrap();
    let tree1 = ShapeTree::from_slide_xml(xml1).unwrap();
    let tb = tree1.shapes.iter().find(|s| s.name() == "TextBox 1");
    assert!(
        tb.is_some(),
        "TextBox 1 should exist on slide 1 after round-trip"
    );

    // Verify slide 2 still has the rectangle
    let xml2 = prs2.slide_xml(&slides[1]).unwrap();
    let tree2 = ShapeTree::from_slide_xml(xml2).unwrap();
    let rect = tree2.shapes.iter().find(|s| s.name() == "Rectangle 1");
    assert!(
        rect.is_some(),
        "Rectangle 1 should exist on slide 2 after round-trip"
    );

    // Verify layouts are still available
    let layouts2 = prs2.slide_layouts().unwrap();
    assert_eq!(layouts2.len(), 11);
}

#[test]
fn test_round_trip_via_file() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    prs.add_slide(&layouts[0]).unwrap();
    prs.add_slide(&layouts[1]).unwrap();

    // Save to file
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    prs.save(&path).unwrap();

    // Re-open from file
    let prs2 = Presentation::open(&path).unwrap();
    assert_eq!(prs2.slide_count().unwrap(), 2);

    // Save again and verify
    let tmp2 = tempfile::NamedTempFile::new().unwrap();
    let path2 = tmp2.path().to_path_buf();
    prs2.save(&path2).unwrap();

    let prs3 = Presentation::open(&path2).unwrap();
    assert_eq!(prs3.slide_count().unwrap(), 2);
}

// ---------------------------------------------------------------------------
// Test 8: Slide size and presentation metadata
// ---------------------------------------------------------------------------

#[test]
fn test_slide_size_default() {
    let prs = Presentation::new().unwrap();
    let size = prs.slide_size().unwrap();
    // Default is 10" x 7.5" (widescreen)
    assert_eq!(size, Some((9144000, 6858000)));
}

// ---------------------------------------------------------------------------
// Test 9: Unit conversions end-to-end
// ---------------------------------------------------------------------------

#[test]
fn test_unit_conversions_in_context() {
    // Verify that unit conversions work correctly in a shape context
    let left: Emu = Inches(1.0).into();
    let top: Emu = Inches(2.0).into();
    let width: Emu = Inches(5.0).into();
    let height: Emu = Inches(1.0).into();

    assert_eq!(left, Emu(914400));
    assert_eq!(top, Emu(1828800));
    assert_eq!(width, Emu(4572000));
    assert_eq!(height, Emu(914400));

    // Generate XML with these units and verify
    let xml = ShapeTree::new_textbox_xml(ShapeId(1), "Test", left, top, width, height);
    assert!(xml.contains(r#"x="914400""#));
    assert!(xml.contains(r#"y="1828800""#));
    assert!(xml.contains(r#"cx="4572000""#));
    assert!(xml.contains(r#"cy="914400""#));
}

// ---------------------------------------------------------------------------
// Test 10: Multiple shape types on same slide
// ---------------------------------------------------------------------------

#[test]
fn test_multiple_shape_types_on_slide() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let slide_ref = prs.add_slide(&layouts[0]).unwrap();

    // Add a textbox, a rectangle, and a table to the same slide
    let textbox = ShapeTree::new_textbox_xml(
        ShapeId(10),
        "TextBox",
        Emu(100000),
        Emu(100000),
        Emu(2000000),
        Emu(500000),
    );
    let rect = ShapeTree::new_autoshape_xml(
        ShapeId(11),
        "Rectangle",
        Emu(100000),
        Emu(700000),
        Emu(2000000),
        Emu(500000),
        "rect",
    );
    let table = ShapeTree::new_table_xml(
        ShapeId(12),
        "Table",
        2,
        3,
        Emu(100000),
        Emu(1300000),
        Emu(4000000),
        Emu(800000),
    );

    let slide_xml = prs.slide_xml(&slide_ref).unwrap();
    let slide_str = String::from_utf8_lossy(slide_xml).into_owned();
    let all_shapes = format!("{}{}{}", textbox, rect, table);
    let updated = slide_str.replace("</p:spTree>", &format!("{}</p:spTree>", all_shapes));
    *prs.slide_xml_mut(&slide_ref).unwrap() = updated.into_bytes();

    // Parse and verify all shapes
    let xml = prs.slide_xml(&slide_ref).unwrap();
    let tree = ShapeTree::from_slide_xml(xml).unwrap();

    assert!(tree.shapes.iter().any(|s| s.name() == "TextBox"));
    assert!(tree.shapes.iter().any(|s| s.name() == "Rectangle"));
    assert!(tree
        .shapes
        .iter()
        .any(|s| s.name() == "Table" && s.has_table()));

    // Round-trip and verify
    let bytes = prs.to_bytes().unwrap();
    let prs2 = Presentation::from_bytes(&bytes).unwrap();
    let slides = prs2.slides().unwrap();
    let xml2 = prs2.slide_xml(&slides[0]).unwrap();
    let tree2 = ShapeTree::from_slide_xml(xml2).unwrap();
    assert_eq!(
        tree2
            .shapes
            .iter()
            .filter(|s| s.name() == "TextBox")
            .count(),
        1
    );
    assert_eq!(
        tree2
            .shapes
            .iter()
            .filter(|s| s.name() == "Rectangle")
            .count(),
        1
    );
    assert_eq!(tree2.shapes.iter().filter(|s| s.has_table()).count(), 1);
}

// ---------------------------------------------------------------------------
// Test 11: DML color and fill integration
// ---------------------------------------------------------------------------

#[test]
fn test_dml_color_theme_with_brightness() {
    let color = ColorFormat::theme_with_brightness(MsoThemeColorIndex::Accent1, 0.4);
    let xml = color.to_xml_string();
    assert!(xml.contains("accent1"));
    assert!(xml.contains("lumMod"));
    assert!(xml.contains("lumOff"));
}

#[test]
fn test_gradient_fill_integration() {
    let fill = FillFormat::linear_gradient(
        ColorFormat::rgb(255, 255, 255),
        ColorFormat::rgb(0, 0, 128),
        90.0,
    );
    let xml = fill.to_xml_string();
    assert!(xml.contains("<a:gradFill>"));
    assert!(xml.contains("<a:gsLst>"));
    assert!(xml.contains("<a:lin"));
    assert!(xml.contains("FFFFFF"));
    assert!(xml.contains("000080"));
}

// ---------------------------------------------------------------------------
// Test 12: Table graphic data XML
// ---------------------------------------------------------------------------

#[test]
fn test_table_graphic_data_xml() {
    let table = Table::new(2, 3, Emu(5486400), Emu(914400));
    let gd_xml = table.to_graphic_data_xml();
    assert!(gd_xml.starts_with("<a:graphic>"));
    assert!(gd_xml.contains("drawingml/2006/table"));
    assert!(gd_xml.contains("<a:tbl>"));
    assert!(gd_xml.contains("<a:tblGrid>"));
    // 3 columns
    assert_eq!(gd_xml.matches("<a:gridCol").count(), 3);
    // 2 rows
    assert_eq!(gd_xml.matches("<a:tr").count(), 2);
}

// ---------------------------------------------------------------------------
// Test 13: write_to
// ---------------------------------------------------------------------------

#[test]
fn test_write_to_buffer() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    prs.add_slide(&layouts[0]).unwrap();

    let mut buf = Vec::new();
    prs.write_to(&mut buf).unwrap();

    // Verify it's a valid pptx
    assert!(buf.len() > 100);
    assert_eq!(&buf[0..2], b"PK");

    // Can reopen from the buffer
    let prs2 = Presentation::from_bytes(&buf).unwrap();
    assert_eq!(prs2.slide_count().unwrap(), 1);
}

// ===========================================================================
// Phase 2 Tests: New features added in the second development wave
// ===========================================================================

// ---------------------------------------------------------------------------
// Test 14: Chart data and XML generation
// ---------------------------------------------------------------------------

#[test]
fn test_chart_data_creation() {
    let mut chart_data = CategoryChartData::new();
    chart_data.add_category("Q1");
    chart_data.add_category("Q2");
    chart_data.add_category("Q3");
    chart_data.add_series("Revenue", &[100.0, 150.0, 200.0]);
    chart_data.add_series("Expenses", &[80.0, 120.0, 140.0]);

    assert_eq!(chart_data.categories().len(), 3);
    assert_eq!(chart_data.series().len(), 2);
    assert_eq!(chart_data.series()[0].name(), "Revenue");
    assert_eq!(
        chart_data.series()[1].values(),
        &[Some(80.0), Some(120.0), Some(140.0)]
    );
}

#[test]
fn test_chart_xml_generation() {
    let mut chart_data = CategoryChartData::new();
    chart_data.add_category("Jan");
    chart_data.add_category("Feb");
    chart_data.add_series("Sales", &[100.0, 150.0]);

    let xml = ChartXmlWriter::write_category(&chart_data, XlChartType::ColumnClustered).unwrap();
    assert!(xml.contains("c:chartSpace"));
    assert!(xml.contains("c:chart"));
    assert!(xml.contains("c:plotArea"));
    assert!(xml.contains("c:barChart") || xml.contains("c:bar3DChart") || xml.contains("barDir"));
    assert!(xml.contains("Sales"));
}

#[test]
fn test_chart_add_to_presentation() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let slide_ref = prs.add_slide(&layouts[0]).unwrap();

    let mut chart_data = CategoryChartData::new();
    chart_data.add_category("A");
    chart_data.add_category("B");
    chart_data.add_series("Data", &[10.0, 20.0]);

    let result = prs.add_chart_to_slide(
        &slide_ref,
        &chart_data,
        XlChartType::ColumnClustered,
        Emu(914400),
        Emu(914400),
        Emu(4572000),
        Emu(3429000),
    );
    assert!(result.is_ok());

    // Verify the slide now has a chart graphic frame
    let bytes = prs.to_bytes().unwrap();
    assert!(bytes.len() > 1000);

    // Round-trip should preserve chart
    let prs2 = Presentation::from_bytes(&bytes).unwrap();
    assert_eq!(prs2.slide_count().unwrap(), 1);
}

// ---------------------------------------------------------------------------
// Test 15: Core properties
// ---------------------------------------------------------------------------

#[test]
fn test_core_properties() {
    let mut props = CoreProperties::new();
    props.set_title("Test Presentation");
    props.set_author("Rust Developer");
    props.set_subject("Testing");
    props.set_keywords("rust, pptx, test");
    props.set_category("Development");

    assert_eq!(props.title(), "Test Presentation");
    assert_eq!(props.author(), "Rust Developer");
    assert_eq!(props.subject(), "Testing");
    assert_eq!(props.keywords(), "rust, pptx, test");
    assert_eq!(props.category(), "Development");

    // Verify XML generation
    let xml = props.to_xml().unwrap();
    let xml_str = String::from_utf8(xml).unwrap();
    assert!(xml_str.contains("Test Presentation"));
    assert!(xml_str.contains("Rust Developer"));
}

#[test]
fn test_core_properties_in_presentation() {
    let mut prs = Presentation::new().unwrap();

    // Read existing core properties
    let props = prs.core_properties();
    assert!(props.is_ok());

    // Set new core properties
    let mut new_props = CoreProperties::new();
    new_props.set_title("My New Deck");
    new_props.set_author("Alice");
    let result = prs.set_core_properties(&new_props);
    assert!(result.is_ok());

    // Round-trip
    let bytes = prs.to_bytes().unwrap();
    let prs2 = Presentation::from_bytes(&bytes).unwrap();
    let props2 = prs2.core_properties().unwrap();
    assert_eq!(props2.title(), "My New Deck");
    assert_eq!(props2.author(), "Alice");
}

// ---------------------------------------------------------------------------
// Test 16: AutoShape with fill, line, and text frame
// ---------------------------------------------------------------------------

#[test]
fn test_autoshape_full_xml() {
    let mut shape = AutoShape {
        shape_id: ShapeId(5),
        name: "Rounded Rect".to_string(),
        left: Emu(914400),
        top: Emu(914400),
        width: Emu(2743200),
        height: Emu(1371600),
        rotation: 0.0,
        prst_geom: Some(pptx::enums::shapes::PresetGeometry::RoundRect),
        is_textbox: false,
        placeholder: None,
        tx_body_xml: None,
        fill: Some(FillFormat::solid(ColorFormat::rgb(0, 100, 200))),
        line: Some(LineFormat::solid(ColorFormat::rgb(0, 0, 0), Emu(12700))),
        text_frame: None,
        click_action: None,
        hover_action: None,
        adjustments: vec![0.16667],
        shadow: None,
        custom_geometry: None,
        scene_3d: None,
        shape_3d: None,
    };

    // Add text
    let mut tf = TextFrame::new();
    tf.set_text("Hello!");
    shape.text_frame = Some(tf);

    let xml = shape.to_xml_string();
    assert!(xml.contains("<p:sp>"));
    assert!(xml.contains("roundRect"));
    assert!(xml.contains("<a:solidFill>"));
    assert!(xml.contains("0064C8")); // RGB for (0,100,200)
    assert!(xml.contains("<a:ln"));
    assert!(xml.contains("Hello!"));
    assert!(xml.contains("adj1"));
}

// ---------------------------------------------------------------------------
// Test 17: Font strikethrough, subscript, superscript
// ---------------------------------------------------------------------------

#[test]
fn test_font_extended_properties() {
    let mut tf = TextFrame::new();
    {
        let p = &mut tf.paragraphs_mut()[0];
        let r = p.add_run();
        r.set_text("Strikethrough text");
        r.font_mut().strikethrough = Some(true);
    }
    let xml = tf.to_xml_string();
    assert!(xml.contains("strike="));

    let mut tf2 = TextFrame::new();
    {
        let p = &mut tf2.paragraphs_mut()[0];
        let r = p.add_run();
        r.set_text("Sub");
        r.font_mut().subscript = Some(true);
    }
    let xml2 = tf2.to_xml_string();
    assert!(xml2.contains("baseline="));
}

// ---------------------------------------------------------------------------
// Test 18: Paragraph bullets
// ---------------------------------------------------------------------------

#[test]
fn test_paragraph_bullets() {
    let mut tf = TextFrame::new();
    {
        let p = &mut tf.paragraphs_mut()[0];
        p.set_bullet(BulletFormat::Character('\u{2022}')); // bullet character
        let r = p.add_run();
        r.set_text("First item");
    }
    {
        let p = tf.add_paragraph();
        p.set_bullet(BulletFormat::Character('\u{2022}'));
        let r = p.add_run();
        r.set_text("Second item");
    }

    let xml = tf.to_xml_string();
    assert!(xml.contains("<a:buChar"));
    assert!(xml.contains("First item"));
    assert!(xml.contains("Second item"));
}

// ---------------------------------------------------------------------------
// Test 19: Table cell merge
// ---------------------------------------------------------------------------

#[test]
fn test_table_cell_merge() {
    let mut table = Table::new(3, 3, Emu(5486400), Emu(1371600));
    table.cell_mut(0, 0).set_text("Merged Header");
    table.cell_mut(0, 0).merge_with(3, 1); // span 3 columns

    assert!(table.cell(0, 0).is_merge_origin());
    let xml = table.to_xml_string();
    assert!(xml.contains("gridSpan"));
    assert!(xml.contains("Merged Header"));
}

// ---------------------------------------------------------------------------
// Test 20: FreeformBuilder
// ---------------------------------------------------------------------------

#[test]
fn test_freeform_builder() {
    let mut fb = FreeformBuilder::new(0, 0, 100000, 100000);
    fb.line_to(100000, 0);
    fb.line_to(100000, 100000);
    fb.line_to(0, 100000);
    fb.close();

    let xml = fb.to_xml_string();
    assert!(xml.contains("a:custGeom"));
    assert!(xml.contains("a:pathLst"));
    assert!(xml.contains("a:path"));
    assert!(xml.contains("a:lnTo") || xml.contains("lnTo"));
    assert!(xml.contains("a:close") || xml.contains("close"));
}

// ---------------------------------------------------------------------------
// Test 21: Action / Hyperlink
// ---------------------------------------------------------------------------

#[test]
fn test_hyperlink_on_shape() {
    let action = ActionSetting::hyperlink_with_tooltip("https://example.com", "Click me");
    let xml = action.to_xml_string(Some("rId5"));
    assert!(xml.contains("hlinkClick"));
    assert!(xml.contains("Click me") || xml.contains("example.com"));
}

#[test]
fn test_action_next_slide() {
    let action = ActionSetting::next_slide();
    let xml = action.to_xml_string(None);
    assert!(xml.contains("hlinkClick") || xml.contains("nextslide") || xml.contains("ppaction"));
}

// ---------------------------------------------------------------------------
// Test 22: Shadow effect
// ---------------------------------------------------------------------------

#[test]
fn test_shadow_format() {
    let shadow = ShadowFormat {
        shadow_type: ShadowType::Outer,
        color: Some(ColorFormat::rgb(0, 0, 0)),
        blur_radius: Some(Emu(50800)),
        distance: Some(Emu(38100)),
        direction: Some(315.0),
        opacity: Some(0.5),
    };
    let xml = shadow.to_xml_string();
    assert!(xml.contains("outerShdw") || xml.contains("effectLst"));
}

// ---------------------------------------------------------------------------
// Test 23: Slide deletion
// ---------------------------------------------------------------------------

#[test]
fn test_slide_deletion() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let s1 = prs.add_slide(&layouts[0]).unwrap();
    let _s2 = prs.add_slide(&layouts[1]).unwrap();
    assert_eq!(prs.slide_count().unwrap(), 2);

    prs.delete_slide(&s1).unwrap();
    assert_eq!(prs.slide_count().unwrap(), 1);

    // Round-trip
    let bytes = prs.to_bytes().unwrap();
    let prs2 = Presentation::from_bytes(&bytes).unwrap();
    assert_eq!(prs2.slide_count().unwrap(), 1);
}

// ---------------------------------------------------------------------------
// Test 24: Slide width/height set
// ---------------------------------------------------------------------------

#[test]
fn test_set_slide_size() {
    let mut prs = Presentation::new().unwrap();

    // Set to 16:9 widescreen (12192000 x 6858000 EMU)
    prs.set_slide_width(12192000).unwrap();
    prs.set_slide_height(6858000).unwrap();

    let size = prs.slide_size().unwrap();
    assert_eq!(size, Some((12192000, 6858000)));

    // Round-trip
    let bytes = prs.to_bytes().unwrap();
    let prs2 = Presentation::from_bytes(&bytes).unwrap();
    let size2 = prs2.slide_size().unwrap();
    assert_eq!(size2, Some((12192000, 6858000)));
}

// ---------------------------------------------------------------------------
// Test 25: Malformed input tests for Presentation::from_bytes
// ---------------------------------------------------------------------------

#[test]
fn test_from_bytes_empty_returns_err() {
    let result = Presentation::from_bytes(&[]);
    assert!(result.is_err(), "empty bytes should return Err");
}

#[test]
fn test_from_bytes_garbage_returns_err() {
    let result = Presentation::from_bytes(&[0xFF, 0xFE, 0x00, 0x01]);
    assert!(result.is_err(), "garbage bytes should return Err");
}

#[test]
fn test_from_bytes_truncated_pptx_returns_err() {
    let prs = Presentation::new().unwrap();
    let bytes = prs.to_bytes().unwrap();
    // Take only the first 100 bytes of a valid pptx
    let truncated = &bytes[..100.min(bytes.len())];
    let result = Presentation::from_bytes(truncated);
    assert!(result.is_err(), "truncated pptx should return Err");
}

#[test]
fn test_from_bytes_plain_text_returns_err() {
    let text_content = b"This is just a plain text file, not a pptx.";
    let result = Presentation::from_bytes(text_content);
    assert!(result.is_err(), "plain text content should return Err");
}

// ---------------------------------------------------------------------------
// Test 26: Full round-trip with Phase 2 features
// ---------------------------------------------------------------------------

#[test]
fn test_full_round_trip_phase2() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();

    // Set properties
    let mut props = CoreProperties::new();
    props.set_title("Phase 2 Test");
    props.set_author("Test Suite");
    prs.set_core_properties(&props).unwrap();

    // Add slides
    let s1 = prs.add_slide(&layouts[0]).unwrap();
    let s2 = prs.add_slide(&layouts[1]).unwrap();

    // Add chart to slide 1
    let mut chart_data = CategoryChartData::new();
    chart_data.add_category("X");
    chart_data.add_category("Y");
    chart_data.add_series("Values", &[42.0, 84.0]);
    prs.add_chart_to_slide(
        &s1,
        &chart_data,
        XlChartType::ColumnClustered,
        Emu(914400),
        Emu(914400),
        Emu(4572000),
        Emu(3429000),
    )
    .unwrap();

    // Add shapes to slide 2
    let rect_xml = ShapeTree::new_autoshape_xml(
        ShapeId(20),
        "Rect",
        Emu(914400),
        Emu(914400),
        Emu(2743200),
        Emu(1371600),
        "rect",
    );
    let slide_xml = prs.slide_xml(&s2).unwrap();
    let slide_str = String::from_utf8_lossy(slide_xml).into_owned();
    let updated = slide_str.replace("</p:spTree>", &format!("{}</p:spTree>", rect_xml));
    *prs.slide_xml_mut(&s2).unwrap() = updated.into_bytes();

    // Save and reopen
    let bytes = prs.to_bytes().unwrap();
    let prs2 = Presentation::from_bytes(&bytes).unwrap();

    assert_eq!(prs2.slide_count().unwrap(), 2);

    let props2 = prs2.core_properties().unwrap();
    assert_eq!(props2.title(), "Phase 2 Test");
    assert_eq!(props2.author(), "Test Suite");
}
