use super::*;
use crate::dml::color::ColorFormat;
use crate::dml::fill::FillFormat;
use crate::enums::text::MsoVerticalAnchor;
use crate::units::Emu;

#[test]
fn test_table_new() {
    let t = Table::new(3, 4, Emu(6096000), Emu(370840));
    assert_eq!(t.row_count(), 3);
    assert_eq!(t.col_count(), 4);
    assert_eq!(t.columns[0].width, Emu(1524000));
    assert_eq!(t.rows[0].height, Emu(370840));
    assert_eq!(t.rows[0].cells.len(), 4);
}

#[test]
fn test_cell_text() {
    let t = Table::new(2, 2, Emu(4000000), Emu(300000));
    assert_eq!(t.cell(0, 0).text(), "");
}

#[test]
fn test_cell_set_text() {
    let mut t = Table::new(2, 2, Emu(4000000), Emu(300000));
    t.cell_mut(0, 0).set_text("Hello");
    assert_eq!(t.cell(0, 0).text(), "Hello");
}

#[test]
fn test_iter_cells() {
    let mut t = Table::new(2, 3, Emu(6000000), Emu(300000));
    t.cell_mut(0, 0).set_text("A");
    t.cell_mut(0, 1).set_text("B");
    t.cell_mut(0, 2).set_text("C");
    t.cell_mut(1, 0).set_text("D");
    t.cell_mut(1, 1).set_text("E");
    t.cell_mut(1, 2).set_text("F");

    let texts: Vec<String> = t.iter_cells().map(|c| c.text()).collect();
    assert_eq!(texts, vec!["A", "B", "C", "D", "E", "F"]);
}

#[test]
fn test_cell_merge_origin() {
    let mut cell = Cell::new();
    assert!(!cell.is_merge_origin());
    assert!(!cell.is_spanned());

    cell.grid_span = 2;
    assert!(cell.is_merge_origin());

    cell.grid_span = 1;
    cell.row_span = 3;
    assert!(cell.is_merge_origin());
}

#[test]
fn test_cell_spanned() {
    let mut cell = Cell::new();
    cell.h_merge = true;
    assert!(cell.is_spanned());

    cell.h_merge = false;
    cell.v_merge = true;
    assert!(cell.is_spanned());
}

#[test]
fn test_table_xml_structure() {
    let t = Table::new(2, 2, Emu(4000000), Emu(300000));
    let xml = t.to_xml_string();

    assert!(xml.starts_with("<a:tbl>"));
    assert!(xml.ends_with("</a:tbl>"));
    assert!(xml.contains("<a:tblPr"));
    assert!(xml.contains(r#"firstRow="1""#));
    assert!(xml.contains(r#"bandRow="1""#));
    assert!(xml.contains("<a:tblGrid>"));
    assert!(xml.contains("<a:gridCol"));
    assert!(xml.contains("<a:tr"));
    assert!(xml.contains("<a:tc"));
    assert!(xml.contains("<a:txBody>"));
    assert!(xml.contains("<a:tcPr"));
}

#[test]
fn test_table_graphic_data_xml() {
    let t = Table::new(1, 1, Emu(2000000), Emu(300000));
    let xml = t.to_graphic_data_xml();
    assert!(xml.starts_with("<a:graphic>"));
    assert!(xml.contains("graphicData"));
    assert!(xml.contains("drawingml/2006/table"));
    assert!(xml.contains("<a:tbl>"));
}

#[test]
fn test_cell_with_fill() {
    let mut cell = Cell::new();
    cell.fill = Some(FillFormat::solid(ColorFormat::rgb(200, 200, 200)));
    let xml = cell.to_xml_string();
    assert!(xml.contains("<a:solidFill>"));
    assert!(xml.contains("C8C8C8"));
    // tcPr should have closing tag (not self-closing) because it has children
    assert!(xml.contains("</a:tcPr>"));
}

#[test]
fn test_cell_with_borders() {
    let mut cell = Cell::new();
    cell.borders.top = Some(CellBorder {
        color: ColorFormat::rgb(0, 0, 0),
        width: Emu(12700),
    });
    let xml = cell.to_xml_string();
    assert!(xml.contains("<a:lnT"));
    assert!(xml.contains(r#"w="12700""#));
    assert!(xml.contains("000000"));
}

#[test]
fn test_cell_merge_span_xml() {
    let mut cell = Cell::new();
    cell.grid_span = 2;
    cell.row_span = 3;
    let xml = cell.to_xml_string();
    assert!(xml.contains(r#"gridSpan="2""#));
    assert!(xml.contains(r#"rowSpan="3""#));
}

#[test]
fn test_cell_default_margins_in_xml() {
    let cell = Cell::new();
    let xml = cell.to_xml_string();
    assert!(xml.contains(r#"marL="91440""#));
    assert!(xml.contains(r#"marR="91440""#));
    assert!(xml.contains(r#"marT="45720""#));
    assert!(xml.contains(r#"marB="45720""#));
}

#[test]
fn test_table_properties_flags() {
    let mut t = Table::new(2, 2, Emu(4000000), Emu(300000));
    t.first_col = true;
    t.last_row = true;
    t.last_col = true;
    t.vert_banding = true;
    let xml = t.to_xml_string();
    assert!(xml.contains(r#"firstCol="1""#));
    assert!(xml.contains(r#"lastRow="1""#));
    assert!(xml.contains(r#"lastCol="1""#));
    assert!(xml.contains(r#"bandCol="1""#));
}

// -----------------------------------------------------------------------
// Cell::split tests
// -----------------------------------------------------------------------

#[test]
fn test_cell_split_resets_merge() {
    let mut cell = Cell::new();
    cell.grid_span = 3;
    cell.row_span = 2;
    cell.h_merge = true;
    cell.v_merge = true;
    cell.split();
    assert_eq!(cell.grid_span, 1);
    assert_eq!(cell.row_span, 1);
    assert!(!cell.h_merge);
    assert!(!cell.v_merge);
}

#[test]
fn test_cell_split_already_unmerged() {
    let mut cell = Cell::new();
    // splitting an already unmerged cell is a no-op
    cell.split();
    assert_eq!(cell.grid_span, 1);
    assert_eq!(cell.row_span, 1);
    assert!(!cell.h_merge);
    assert!(!cell.v_merge);
}

#[test]
fn test_cell_split_xml_no_span_attrs() {
    let mut cell = Cell::new();
    cell.grid_span = 2;
    cell.row_span = 3;
    cell.split();
    let xml = cell.to_xml_string();
    assert!(!xml.contains("gridSpan"));
    assert!(!xml.contains("rowSpan"));
    assert!(!xml.contains("hMerge"));
    assert!(!xml.contains("vMerge"));
}

// -----------------------------------------------------------------------
// Table::table_style_id tests
// -----------------------------------------------------------------------

#[test]
fn test_table_no_style_id() {
    let t = Table::new(1, 1, Emu(2000000), Emu(300000));
    let xml = t.to_xml_string();
    assert!(!xml.contains("tblStyleId"));
    // tblPr should be self-closing
    assert!(xml.contains("<a:tblPr"));
    assert!(xml.contains("/>"));
}

#[test]
fn test_table_with_style_id() {
    let mut t = Table::new(1, 1, Emu(2000000), Emu(300000));
    t.table_style_id = Some("{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}".to_string());
    let xml = t.to_xml_string();
    assert!(xml.contains("<a:tblStyleId>{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}</a:tblStyleId>"));
    assert!(xml.contains("</a:tblPr>"));
}

// -----------------------------------------------------------------------
// Cell::vertical_anchor tests
// -----------------------------------------------------------------------

#[test]
fn test_cell_vertical_anchor_default_none() {
    let cell = Cell::new();
    assert!(cell.vertical_anchor.is_none());
    let xml = cell.to_xml_string();
    assert!(!xml.contains("anchor="));
}

#[test]
fn test_cell_vertical_anchor_top() {
    let mut cell = Cell::new();
    cell.vertical_anchor = Some(MsoVerticalAnchor::Top);
    let xml = cell.to_xml_string();
    assert!(xml.contains(r#"anchor="t""#));
}

#[test]
fn test_cell_vertical_anchor_middle() {
    let mut cell = Cell::new();
    cell.vertical_anchor = Some(MsoVerticalAnchor::Middle);
    let xml = cell.to_xml_string();
    assert!(xml.contains(r#"anchor="ctr""#));
}

#[test]
fn test_cell_vertical_anchor_bottom() {
    let mut cell = Cell::new();
    cell.vertical_anchor = Some(MsoVerticalAnchor::Bottom);
    let xml = cell.to_xml_string();
    assert!(xml.contains(r#"anchor="b""#));
}
