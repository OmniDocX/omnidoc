//! Internal helpers for xlsx generation: XML templates, cell writing, ZIP assembly.

use std::io::{Cursor, Write};

use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::error::PptxResult;
use crate::xml_util::xml_escape;

#[derive(Debug)]
pub(super) enum CellValue {
    Empty,
    Number(f64),
    SharedString(usize),
}

/// Find the index of a string in the shared strings table.
///
/// Returns `None` if `value` is not present in `shared_strings`.
#[must_use]
pub(super) fn shared_string_index(shared_strings: &[String], value: &str) -> Option<usize> {
    shared_strings.iter().position(|s| s == value)
}

/// Convert a zero-based column index to an Excel column letter (0->"A", 25->"Z", 26->"AA").
pub(super) fn col_letter(col: usize) -> String {
    let mut result = String::new();
    let mut n = col;
    loop {
        let ch = b'A'
            + u8::try_from(n % 26).unwrap_or_else(|_| unreachable!("n % 26 always fits in u8"));
        result.insert(0, ch as char);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    result
}

/// Build the complete xlsx ZIP from row data and shared strings.
pub(super) fn build_xlsx(
    rows: &[Vec<CellValue>],
    shared_strings: &[String],
) -> PptxResult<Vec<u8>> {
    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("[Content_Types].xml", options)?;
    zip.write_all(xlsx_content_types_xml().as_bytes())?;

    zip.start_file("_rels/.rels", options)?;
    zip.write_all(xlsx_rels_xml().as_bytes())?;

    zip.start_file("xl/workbook.xml", options)?;
    zip.write_all(xlsx_workbook_xml().as_bytes())?;

    zip.start_file("xl/_rels/workbook.xml.rels", options)?;
    zip.write_all(xlsx_workbook_rels_xml().as_bytes())?;

    zip.start_file("xl/styles.xml", options)?;
    zip.write_all(xlsx_styles_xml().as_bytes())?;

    zip.start_file("xl/sharedStrings.xml", options)?;
    zip.write_all(xlsx_shared_strings_xml(shared_strings).as_bytes())?;

    zip.start_file("xl/worksheets/sheet1.xml", options)?;
    zip.write_all(xlsx_sheet_xml(rows).as_bytes())?;

    let cursor = zip.finish()?;
    Ok(cursor.into_inner())
}

fn xlsx_content_types_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
  <Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>
  <Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/>
</Types>"#
        .to_string()
}

fn xlsx_rels_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#
        .to_string()
}

fn xlsx_workbook_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
    <sheet name="Sheet1" sheetId="1" r:id="rId1"/>
  </sheets>
</workbook>"#
        .to_string()
}

fn xlsx_workbook_rels_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/>
</Relationships>"#
        .to_string()
}

fn xlsx_styles_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <fonts count="1">
    <font>
      <sz val="11"/>
      <name val="Calibri"/>
    </font>
  </fonts>
  <fills count="2">
    <fill><patternFill patternType="none"/></fill>
    <fill><patternFill patternType="gray125"/></fill>
  </fills>
  <borders count="1">
    <border><left/><right/><top/><bottom/><diagonal/></border>
  </borders>
  <cellStyleXfs count="1">
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0"/>
  </cellStyleXfs>
  <cellXfs count="1">
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
  </cellXfs>
</styleSheet>"#
        .to_string()
}

fn xlsx_shared_strings_xml(strings: &[String]) -> String {
    let mut xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="{}" uniqueCount="{}">"#,
        strings.len(),
        strings.len()
    );
    for s in strings {
        xml.push_str("<si><t>");
        xml.push_str(&xml_escape(s));
        xml.push_str("</t></si>");
    }
    xml.push_str("</sst>");
    xml
}

fn xlsx_sheet_xml(rows: &[Vec<CellValue>]) -> String {
    use std::fmt::Write as _;

    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>"#,
    );

    for (row_idx, row) in rows.iter().enumerate() {
        let row_num = row_idx + 1;
        write!(xml, "\n    <row r=\"{row_num}\">")
            .unwrap_or_else(|_| unreachable!("fmt::Write for String is infallible"));
        for (col_idx, cell) in row.iter().enumerate() {
            let cell_ref = format!("{}{}", col_letter(col_idx), row_num);
            match cell {
                CellValue::Empty => {}
                CellValue::Number(v) => {
                    write!(xml, "<c r=\"{cell_ref}\"><v>{v}</v></c>")
                        .unwrap_or_else(|_| unreachable!("fmt::Write for String is infallible"));
                }
                CellValue::SharedString(idx) => {
                    write!(xml, "<c r=\"{cell_ref}\" t=\"s\"><v>{idx}</v></c>")
                        .unwrap_or_else(|_| unreachable!("fmt::Write for String is infallible"));
                }
            }
        }
        xml.push_str("</row>");
    }

    xml.push_str("\n  </sheetData>\n</worksheet>");
    xml
}
