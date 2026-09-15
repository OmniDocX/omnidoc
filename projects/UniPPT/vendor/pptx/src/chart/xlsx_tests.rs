use super::*;
use std::io::Cursor;
use std::io::Read;
use zip::ZipArchive;

use crate::chart::data::{BubbleChartData, CategoryChartData, XyChartData};

#[test]
fn test_col_letter() {
    use super::xlsx_parts::col_letter;
    assert_eq!(col_letter(0), "A");
    assert_eq!(col_letter(1), "B");
    assert_eq!(col_letter(25), "Z");
    assert_eq!(col_letter(26), "AA");
    assert_eq!(col_letter(27), "AB");
    assert_eq!(col_letter(51), "AZ");
    assert_eq!(col_letter(52), "BA");
}

#[test]
fn test_generate_category_xlsx_is_valid_zip() {
    let mut data = CategoryChartData::new();
    data.add_category("Q1");
    data.add_category("Q2");
    data.add_series("Sales", &[100.0, 150.0]);
    data.add_series("Expenses", &[80.0, 120.0]);

    let xlsx_bytes = generate_category_xlsx(&data).unwrap();
    assert!(!xlsx_bytes.is_empty());

    let cursor = Cursor::new(&xlsx_bytes);
    let mut archive = ZipArchive::new(cursor).unwrap();

    let expected_files = [
        "[Content_Types].xml",
        "_rels/.rels",
        "xl/workbook.xml",
        "xl/_rels/workbook.xml.rels",
        "xl/styles.xml",
        "xl/sharedStrings.xml",
        "xl/worksheets/sheet1.xml",
    ];
    for name in &expected_files {
        assert!(
            archive.by_name(name).is_ok(),
            "Missing file in xlsx: {}",
            name
        );
    }
}

#[test]
fn test_generate_category_xlsx_sheet_content() {
    let mut data = CategoryChartData::new();
    data.add_category("Q1");
    data.add_category("Q2");
    data.add_series("Sales", &[100.0, 150.0]);

    let xlsx_bytes = generate_category_xlsx(&data).unwrap();
    let cursor = Cursor::new(&xlsx_bytes);
    let mut archive = ZipArchive::new(cursor).unwrap();

    let mut sheet_xml = String::new();
    archive
        .by_name("xl/worksheets/sheet1.xml")
        .unwrap()
        .read_to_string(&mut sheet_xml)
        .unwrap();

    assert!(
        sheet_xml.contains("t=\"s\""),
        "Should contain shared string references"
    );
    assert!(sheet_xml.contains("<v>100</v>"));
    assert!(sheet_xml.contains("<v>150</v>"));
}

#[test]
fn test_generate_category_xlsx_shared_strings() {
    let mut data = CategoryChartData::new();
    data.add_category("East");
    data.add_category("West");
    data.add_series("Revenue", &[10.0, 20.0]);

    let xlsx_bytes = generate_category_xlsx(&data).unwrap();
    let cursor = Cursor::new(&xlsx_bytes);
    let mut archive = ZipArchive::new(cursor).unwrap();

    let mut ss_xml = String::new();
    archive
        .by_name("xl/sharedStrings.xml")
        .unwrap()
        .read_to_string(&mut ss_xml)
        .unwrap();

    assert!(ss_xml.contains("<t>Revenue</t>"));
    assert!(ss_xml.contains("<t>East</t>"));
    assert!(ss_xml.contains("<t>West</t>"));
}

#[test]
fn test_generate_category_xlsx_xml_escape() {
    let mut data = CategoryChartData::new();
    data.add_category("A&B");
    data.add_category("C<D");
    data.add_series("S&eries", &[1.0, 2.0]);

    let xlsx_bytes = generate_category_xlsx(&data).unwrap();
    let cursor = Cursor::new(&xlsx_bytes);
    let mut archive = ZipArchive::new(cursor).unwrap();

    let mut ss_xml = String::new();
    archive
        .by_name("xl/sharedStrings.xml")
        .unwrap()
        .read_to_string(&mut ss_xml)
        .unwrap();

    assert!(ss_xml.contains("A&amp;B"));
    assert!(ss_xml.contains("C&lt;D"));
    assert!(ss_xml.contains("S&amp;eries"));
}

#[test]
fn test_generate_xy_xlsx_is_valid_zip() {
    let mut data = XyChartData::new();
    let series = data.add_series("Measurements");
    series.add_data_point(1.0, 2.5);
    series.add_data_point(3.0, 4.0);

    let xlsx_bytes = generate_xy_xlsx(&data).unwrap();
    assert!(!xlsx_bytes.is_empty());

    let cursor = Cursor::new(&xlsx_bytes);
    let mut archive = ZipArchive::new(cursor).unwrap();

    assert!(archive.by_name("[Content_Types].xml").is_ok());
    assert!(archive.by_name("xl/worksheets/sheet1.xml").is_ok());
    assert!(archive.by_name("xl/sharedStrings.xml").is_ok());
}

#[test]
fn test_generate_xy_xlsx_content() {
    let mut data = XyChartData::new();
    let series = data.add_series("Points");
    series.add_data_point(10.0, 20.0);
    series.add_data_point(30.0, 40.0);

    let xlsx_bytes = generate_xy_xlsx(&data).unwrap();
    let cursor = Cursor::new(&xlsx_bytes);
    let mut archive = ZipArchive::new(cursor).unwrap();

    let mut sheet_xml = String::new();
    archive
        .by_name("xl/worksheets/sheet1.xml")
        .unwrap()
        .read_to_string(&mut sheet_xml)
        .unwrap();

    assert!(sheet_xml.contains("<v>10</v>"));
    assert!(sheet_xml.contains("<v>20</v>"));
    assert!(sheet_xml.contains("<v>30</v>"));
    assert!(sheet_xml.contains("<v>40</v>"));

    let mut ss_xml = String::new();
    archive
        .by_name("xl/sharedStrings.xml")
        .unwrap()
        .read_to_string(&mut ss_xml)
        .unwrap();

    assert!(ss_xml.contains("<t>Points X</t>"));
    assert!(ss_xml.contains("<t>Points Y</t>"));
}

#[test]
fn test_generate_bubble_xlsx_is_valid_zip() {
    let mut data = BubbleChartData::new();
    let series = data.add_series("Dataset");
    series.add_data_point(1.0, 2.0, 10.0);
    series.add_data_point(3.0, 4.0, 20.0);

    let xlsx_bytes = generate_bubble_xlsx(&data).unwrap();
    assert!(!xlsx_bytes.is_empty());

    let cursor = Cursor::new(&xlsx_bytes);
    let mut archive = ZipArchive::new(cursor).unwrap();

    assert!(archive.by_name("[Content_Types].xml").is_ok());
    assert!(archive.by_name("xl/worksheets/sheet1.xml").is_ok());
}

#[test]
fn test_generate_bubble_xlsx_content() {
    let mut data = BubbleChartData::new();
    let series = data.add_series("Bubbles");
    series.add_data_point(1.0, 2.0, 5.0);

    let xlsx_bytes = generate_bubble_xlsx(&data).unwrap();
    let cursor = Cursor::new(&xlsx_bytes);
    let mut archive = ZipArchive::new(cursor).unwrap();

    let mut sheet_xml = String::new();
    archive
        .by_name("xl/worksheets/sheet1.xml")
        .unwrap()
        .read_to_string(&mut sheet_xml)
        .unwrap();

    assert!(sheet_xml.contains("<v>1</v>"));
    assert!(sheet_xml.contains("<v>2</v>"));
    assert!(sheet_xml.contains("<v>5</v>"));

    let mut ss_xml = String::new();
    archive
        .by_name("xl/sharedStrings.xml")
        .unwrap()
        .read_to_string(&mut ss_xml)
        .unwrap();

    assert!(ss_xml.contains("<t>Bubbles X</t>"));
    assert!(ss_xml.contains("<t>Bubbles Y</t>"));
    assert!(ss_xml.contains("<t>Bubbles Size</t>"));
}

#[test]
fn test_generate_category_xlsx_empty_data() {
    let data = CategoryChartData::new();
    let xlsx_bytes = generate_category_xlsx(&data).unwrap();
    assert!(!xlsx_bytes.is_empty());

    let cursor = Cursor::new(&xlsx_bytes);
    let archive = ZipArchive::new(cursor).unwrap();
    assert!(!archive.is_empty());
}

#[test]
fn test_generate_category_xlsx_missing_values() {
    let mut data = CategoryChartData::new();
    data.add_category("A");
    data.add_category("B");
    data.add_category("C");
    data.add_series_with_options("S1", &[Some(1.0), None, Some(3.0)]);

    let xlsx_bytes = generate_category_xlsx(&data).unwrap();
    let cursor = Cursor::new(&xlsx_bytes);
    let mut archive = ZipArchive::new(cursor).unwrap();

    let mut sheet_xml = String::new();
    archive
        .by_name("xl/worksheets/sheet1.xml")
        .unwrap()
        .read_to_string(&mut sheet_xml)
        .unwrap();

    assert!(sheet_xml.contains("<v>1</v>"));
    assert!(sheet_xml.contains("<v>3</v>"));
}
