//! Minimal Excel workbook (`.xlsx`) generation for embedded chart data.
//!
//! `PowerPoint` charts require an embedded `.xlsx` file containing the source
//! data so that charts can be edited. This module generates a minimal xlsx
//! (a ZIP of XML files) that is sufficient for `PowerPoint` to read.

#[path = "xlsx_parts.rs"]
mod xlsx_parts;

use crate::chart::data::{BubbleChartData, CategoryChartData, XyChartData};
use crate::error::PptxResult;

use xlsx_parts::{build_xlsx, shared_string_index, CellValue};

/// Generate a minimal `.xlsx` file containing the data from a `CategoryChartData`.
///
/// The worksheet layout is:
/// - Row 1: empty cell, then series names as column headers
/// - Row 2+: category label, then data values for each series
///
/// # Errors
/// Returns an error if ZIP assembly fails.
pub fn generate_category_xlsx(data: &CategoryChartData) -> PptxResult<Vec<u8>> {
    let categories = data.categories();
    let series = data.series();

    let mut shared_strings: Vec<String> = Vec::new();
    for s in series {
        shared_strings.push(s.name().to_string());
    }
    for cat in categories {
        shared_strings.push(cat.clone());
    }

    let mut rows: Vec<Vec<CellValue>> = Vec::new();

    // Header row
    let mut header = vec![CellValue::Empty];
    for s in series {
        let idx = shared_string_index(&shared_strings, s.name()).ok_or_else(|| {
            crate::error::PptxError::InvalidXml(format!(
                "series name {:?} not found in shared strings",
                s.name()
            ))
        })?;
        header.push(CellValue::SharedString(idx));
    }
    rows.push(header);

    // Data rows
    for (cat_idx, cat_label) in categories.iter().enumerate() {
        let cat_ss_idx = shared_string_index(&shared_strings, cat_label).ok_or_else(|| {
            crate::error::PptxError::InvalidXml(format!(
                "category label {cat_label:?} not found in shared strings"
            ))
        })?;
        let mut row = vec![CellValue::SharedString(cat_ss_idx)];
        for s in series {
            if let Some(Some(v)) = s.values().get(cat_idx) {
                row.push(CellValue::Number(*v));
            } else {
                row.push(CellValue::Empty);
            }
        }
        rows.push(row);
    }

    build_xlsx(&rows, &shared_strings)
}

/// Generate a minimal `.xlsx` file containing the data from an `XyChartData`.
///
/// The worksheet layout is:
/// - For each series: two columns (X values, Y values) with the series name as header
///
/// # Errors
/// Returns an error if ZIP assembly fails.
pub fn generate_xy_xlsx(data: &XyChartData) -> PptxResult<Vec<u8>> {
    let series = data.series();

    let mut shared_strings: Vec<String> = Vec::new();
    for s in series {
        shared_strings.push(format!("{} X", s.name()));
        shared_strings.push(format!("{} Y", s.name()));
    }

    let max_points = series
        .iter()
        .map(|s| s.data_points().len())
        .max()
        .unwrap_or(0);

    let mut rows: Vec<Vec<CellValue>> = Vec::new();

    // Header row
    let mut header = Vec::new();
    for (i, _s) in series.iter().enumerate() {
        header.push(CellValue::SharedString(i * 2));
        header.push(CellValue::SharedString(i * 2 + 1));
    }
    rows.push(header);

    // Data rows
    for pt_idx in 0..max_points {
        let mut row = Vec::new();
        for s in series {
            if let Some(dp) = s.data_points().get(pt_idx) {
                row.push(CellValue::Number(dp.x));
                row.push(CellValue::Number(dp.y));
            } else {
                row.push(CellValue::Empty);
                row.push(CellValue::Empty);
            }
        }
        rows.push(row);
    }

    build_xlsx(&rows, &shared_strings)
}

/// Generate a minimal `.xlsx` file containing the data from a `BubbleChartData`.
///
/// The worksheet layout is:
/// - For each series: three columns (X, Y, Size) with headers
///
/// # Errors
/// Returns an error if ZIP assembly fails.
pub fn generate_bubble_xlsx(data: &BubbleChartData) -> PptxResult<Vec<u8>> {
    let series = data.series();

    let mut shared_strings: Vec<String> = Vec::new();
    for s in series {
        shared_strings.push(format!("{} X", s.name()));
        shared_strings.push(format!("{} Y", s.name()));
        shared_strings.push(format!("{} Size", s.name()));
    }

    let max_points = series
        .iter()
        .map(|s| s.data_points().len())
        .max()
        .unwrap_or(0);

    let mut rows: Vec<Vec<CellValue>> = Vec::new();

    // Header row
    let mut header = Vec::new();
    for (i, _s) in series.iter().enumerate() {
        header.push(CellValue::SharedString(i * 3));
        header.push(CellValue::SharedString(i * 3 + 1));
        header.push(CellValue::SharedString(i * 3 + 2));
    }
    rows.push(header);

    // Data rows
    for pt_idx in 0..max_points {
        let mut row = Vec::new();
        for s in series {
            if let Some(dp) = s.data_points().get(pt_idx) {
                row.push(CellValue::Number(dp.x));
                row.push(CellValue::Number(dp.y));
                row.push(CellValue::Number(dp.size));
            } else {
                row.push(CellValue::Empty);
                row.push(CellValue::Empty);
                row.push(CellValue::Empty);
            }
        }
        rows.push(row);
    }

    build_xlsx(&rows, &shared_strings)
}

#[cfg(test)]
#[path = "xlsx_tests.rs"]
mod tests;
