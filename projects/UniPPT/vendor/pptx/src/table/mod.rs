//! Table support for `PowerPoint` slides.
//!
//! Tables in OOXML are embedded inside a `<p:graphicFrame>` as
//! `<a:graphic><a:graphicData uri="...table"><a:tbl>...</a:tbl></a:graphicData></a:graphic>`.
//!
//! This module provides `Table`, `Row`, `Column`, and `Cell` structs
//! that model the table content.  Each `Cell` contains a `TextFrame`
//! for its text content.

mod cell;

pub use cell::{Cell, CellBorder, CellBorders};

use crate::units::Emu;

/// A table column definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    /// Width of the column in EMU.
    pub width: Emu,
}

/// A table row.
// Cannot derive Eq: contains f64 fields (via Cell -> TextFrame)
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// Height of the row in EMU.
    pub height: Emu,
    /// The cells in this row, one per column.
    pub cells: Vec<Cell>,
}

/// A `DrawingML` table (`<a:tbl>`).
///
/// Tables consist of a grid of columns and rows. Each row contains
/// cells, and each cell contains a text frame.
// Cannot derive Eq: contains f64 fields (via Cell -> TextFrame)
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    /// Column definitions (widths).
    pub columns: Vec<Column>,
    /// Row data (each row contains cells).
    pub rows: Vec<Row>,
    /// Whether the first row should be visually distinct (header row).
    pub first_row: bool,
    /// Whether the first column should be visually distinct.
    pub first_col: bool,
    /// Whether the last row should be visually distinct (totals row).
    pub last_row: bool,
    /// Whether the last column should be visually distinct.
    pub last_col: bool,
    /// Whether rows should have alternating shading.
    pub horz_banding: bool,
    /// Whether columns should have alternating shading.
    pub vert_banding: bool,
    /// Table style GUID (e.g. `"{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}"`).
    /// When set, emitted as `<a:tblStyleId>` inside `<a:tblPr>`.
    pub table_style_id: Option<String>,
}

impl Table {
    /// Create a new table with the specified number of rows and columns.
    ///
    /// All columns will have equal width based on `total_width / cols`,
    /// and all rows will have the given `row_height`.
    #[must_use]
    pub fn new(rows: usize, cols: usize, total_width: Emu, row_height: Emu) -> Self {
        #[allow(clippy::cast_possible_wrap)] // cols is a small table dimension, safe to cast
        let col_width = Emu(total_width.0 / cols as i64);
        let columns: Vec<Column> = (0..cols).map(|_| Column { width: col_width }).collect();
        let table_rows: Vec<Row> = (0..rows)
            .map(|_| Row {
                height: row_height,
                cells: (0..cols).map(|_| Cell::new()).collect(),
            })
            .collect();
        Self {
            columns,
            rows: table_rows,
            first_row: true,
            first_col: false,
            last_row: false,
            last_col: false,
            horz_banding: true,
            vert_banding: false,
            table_style_id: None,
        }
    }

    /// Return the number of rows.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Return the number of columns.
    #[must_use]
    pub fn col_count(&self) -> usize {
        self.columns.len()
    }

    /// Get a reference to the cell at (`row_idx`, `col_idx`).
    ///
    /// # Panics
    /// Panics if `row_idx` or `col_idx` is out of bounds.
    #[must_use]
    pub fn cell(&self, row_idx: usize, col_idx: usize) -> &Cell {
        &self.rows[row_idx].cells[col_idx]
    }

    /// Get a mutable reference to the cell at (`row_idx`, `col_idx`).
    ///
    /// # Panics
    /// Panics if `row_idx` or `col_idx` is out of bounds.
    pub fn cell_mut(&mut self, row_idx: usize, col_idx: usize) -> &mut Cell {
        &mut self.rows[row_idx].cells[col_idx]
    }

    /// Get a reference to the cell at (`row_idx`, `col_idx`), or `None` if out of bounds.
    #[must_use]
    pub fn get_cell(&self, row_idx: usize, col_idx: usize) -> Option<&Cell> {
        self.rows.get(row_idx)?.cells.get(col_idx)
    }

    /// Get a mutable reference to the cell at (`row_idx`, `col_idx`), or `None` if out of bounds.
    pub fn get_cell_mut(&mut self, row_idx: usize, col_idx: usize) -> Option<&mut Cell> {
        self.rows.get_mut(row_idx)?.cells.get_mut(col_idx)
    }

    /// Iterate over all cells in left-to-right, top-to-bottom order.
    pub fn iter_cells(&self) -> impl Iterator<Item = &Cell> {
        self.rows.iter().flat_map(|r| r.cells.iter())
    }

    /// Return a reference to the rows.
    #[must_use]
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// Return a mutable reference to the rows.
    pub fn rows_mut(&mut self) -> &mut [Row] {
        &mut self.rows
    }

    /// Append a new row with one empty cell per column.
    ///
    /// The new row uses the same height as the first row (or a default of 370840 EMU).
    /// Returns a mutable reference to the new row.
    ///
    /// # Panics
    ///
    /// Panics if the internal rows vector is empty after push (should never happen).
    pub fn add_row(&mut self) -> &mut Row {
        let height = self.rows.first().map_or(Emu(370_840), |r| r.height);
        let cols = self.columns.len();
        let row = Row {
            height,
            cells: (0..cols).map(|_| Cell::new()).collect(),
        };
        self.rows.push(row);
        let idx = self.rows.len() - 1;
        &mut self.rows[idx]
    }

    /// Append a new column with the given width and add a new empty cell to every existing row.
    pub fn add_column(&mut self, width: Emu) {
        self.columns.push(Column { width });
        for row in &mut self.rows {
            row.cells.push(Cell::new());
        }
    }

    /// Write the `<a:tbl>` XML element into the given writer.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the writer fails.
    pub fn write_xml<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        w.write_str("<a:tbl>")?;

        // <a:tblPr>
        w.write_str("<a:tblPr")?;
        if self.first_row {
            w.write_str(r#" firstRow="1""#)?;
        }
        if self.first_col {
            w.write_str(r#" firstCol="1""#)?;
        }
        if self.last_row {
            w.write_str(r#" lastRow="1""#)?;
        }
        if self.last_col {
            w.write_str(r#" lastCol="1""#)?;
        }
        if self.horz_banding {
            w.write_str(r#" bandRow="1""#)?;
        }
        if self.vert_banding {
            w.write_str(r#" bandCol="1""#)?;
        }
        if let Some(ref style_id) = self.table_style_id {
            w.write_char('>')?;
            write!(w, "<a:tblStyleId>{style_id}</a:tblStyleId>")?;
            w.write_str("</a:tblPr>")?;
        } else {
            w.write_str("/>")?;
        }

        // <a:tblGrid>
        w.write_str("<a:tblGrid>")?;
        for col in &self.columns {
            write!(w, r#"<a:gridCol w="{}"/>"#, col.width.0)?;
        }
        w.write_str("</a:tblGrid>")?;

        // <a:tr> rows
        for row in &self.rows {
            write!(w, r#"<a:tr h="{}">"#, row.height.0)?;
            for cell in &row.cells {
                cell.write_xml(w)?;
            }
            w.write_str("</a:tr>")?;
        }

        w.write_str("</a:tbl>")
    }

    /// Generate the `<a:tbl>` XML element string.
    ///
    /// # Panics
    ///
    /// Panics if writing to a `String` fails (should never happen).
    #[must_use]
    pub fn to_xml_string(&self) -> String {
        // Estimate: ~200 bytes overhead + ~200 bytes per cell
        let cell_count = self.rows.len() * self.columns.len();
        let mut s = String::with_capacity(200 + cell_count * 200);
        self.write_xml(&mut s)
            .unwrap_or_else(|_| unreachable!("fmt::Write for String is infallible"));
        s
    }

    /// Generate the full graphic frame table XML, wrapped in `<a:graphic>` and `<a:graphicData>`.
    #[must_use]
    pub fn to_graphic_data_xml(&self) -> String {
        // Estimate: ~300 bytes wrapper + ~200 bytes per cell
        let cell_count = self.rows.len() * self.columns.len();
        let mut s = String::with_capacity(300 + cell_count * 200);
        s.push_str(r#"<a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">"#);
        self.write_xml(&mut s)
            .unwrap_or_else(|_| unreachable!("fmt::Write for String is infallible"));
        s.push_str("</a:graphicData></a:graphic>");
        s
    }
}

#[cfg(test)]
mod tests;
