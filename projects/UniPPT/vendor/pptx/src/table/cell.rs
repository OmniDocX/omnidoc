//! Table cell types and XML generation.

use crate::dml::color::ColorFormat;
use crate::dml::fill::FillFormat;
use crate::enums::text::MsoVerticalAnchor;
use crate::text::TextFrame;
use crate::units::Emu;
use crate::xml_util::WriteXml;

/// A table cell.
// Cannot derive Eq: contains f64 fields (via TextFrame)
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    /// The text content of the cell.
    pub text_frame: TextFrame,
    /// Cell fill. `None` means inherited from the table style.
    pub fill: Option<FillFormat>,
    /// Number of columns this cell spans (default 1).
    pub grid_span: u32,
    /// Number of rows this cell spans (default 1).
    pub row_span: u32,
    /// Whether this cell is horizontally merged (spanned by another cell).
    pub h_merge: bool,
    /// Whether this cell is vertically merged (spanned by another cell).
    pub v_merge: bool,
    /// Left margin in EMU.  Default is 91440 (0.1 inch).
    pub margin_left: Option<Emu>,
    /// Right margin in EMU.
    pub margin_right: Option<Emu>,
    /// Top margin in EMU.  Default is 45720 (0.05 inch).
    pub margin_top: Option<Emu>,
    /// Bottom margin in EMU.
    pub margin_bottom: Option<Emu>,
    /// Border formatting for the cell.
    pub borders: CellBorders,
    /// Vertical anchor for text within the cell. Emitted as `anchor` attribute
    /// on `<a:tcPr>`.
    pub vertical_anchor: Option<MsoVerticalAnchor>,
}

/// Border formatting for the four sides of a cell.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CellBorders {
    pub left: Option<CellBorder>,
    pub right: Option<CellBorder>,
    pub top: Option<CellBorder>,
    pub bottom: Option<CellBorder>,
}

/// A single cell border.
#[derive(Debug, Clone, PartialEq)]
pub struct CellBorder {
    /// Border color.
    pub color: ColorFormat,
    /// Border width in EMU.
    pub width: Emu,
}

impl Cell {
    /// Create a new empty cell with default margins.
    #[must_use]
    pub fn new() -> Self {
        Self {
            text_frame: TextFrame::new(),
            fill: None,
            grid_span: 1,
            row_span: 1,
            h_merge: false,
            v_merge: false,
            margin_left: Some(Emu(91440)),
            margin_right: Some(Emu(91440)),
            margin_top: Some(Emu(45720)),
            margin_bottom: Some(Emu(45720)),
            borders: CellBorders::default(),
            vertical_anchor: None,
        }
    }

    /// Get the text content of this cell as a single string.
    #[must_use]
    pub fn text(&self) -> String {
        self.text_frame.text()
    }

    /// Set the text content of this cell, replacing all existing content.
    pub fn set_text(&mut self, text: &str) {
        self.text_frame.set_text(text);
    }

    /// Set this cell as a merge origin spanning the given number of columns and rows.
    ///
    /// `span_width` is the number of columns to span (must be >= 1).
    /// `span_height` is the number of rows to span (must be >= 1).
    pub fn merge_with(&mut self, span_width: u32, span_height: u32) {
        self.grid_span = span_width.max(1);
        self.row_span = span_height.max(1);
    }

    /// Whether this cell is a merge origin (spans multiple rows or columns).
    #[must_use]
    pub const fn is_merge_origin(&self) -> bool {
        self.grid_span > 1 || self.row_span > 1
    }

    /// Whether this cell is spanned (hidden) by a merge origin.
    #[must_use]
    pub const fn is_spanned(&self) -> bool {
        self.h_merge || self.v_merge
    }

    /// Reset this cell's merge state, effectively "unmerging" it.
    ///
    /// Sets `grid_span` and `row_span` back to 1 and clears the
    /// `h_merge` and `v_merge` flags.
    pub fn split(&mut self) {
        self.grid_span = 1;
        self.row_span = 1;
        self.h_merge = false;
        self.v_merge = false;
    }

    /// Write the `<a:tc>` XML element into the given writer.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the writer fails.
    pub fn write_xml<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        w.write_str("<a:tc")?;

        if self.grid_span > 1 {
            write!(w, r#" gridSpan="{}""#, self.grid_span)?;
        }
        if self.row_span > 1 {
            write!(w, r#" rowSpan="{}""#, self.row_span)?;
        }
        if self.h_merge {
            w.write_str(r#" hMerge="1""#)?;
        }
        if self.v_merge {
            w.write_str(r#" vMerge="1""#)?;
        }

        w.write_char('>')?;

        // <a:txBody> - table cells use "a:txBody" instead of "p:txBody"
        self.text_frame.write_xml_with_tag(w, "a:txBody")?;

        // <a:tcPr>
        w.write_str("<a:tcPr")?;
        if let Some(l) = self.margin_left {
            write!(w, r#" marL="{l}""#)?;
        }
        if let Some(r) = self.margin_right {
            write!(w, r#" marR="{r}""#)?;
        }
        if let Some(t) = self.margin_top {
            write!(w, r#" marT="{t}""#)?;
        }
        if let Some(b) = self.margin_bottom {
            write!(w, r#" marB="{b}""#)?;
        }
        if let Some(anchor) = self.vertical_anchor {
            write!(w, r#" anchor="{}""#, anchor.to_xml_str())?;
        }

        let has_tc_children = self.fill.is_some() || self.borders.has_any();

        if has_tc_children {
            w.write_char('>')?;

            // Border elements
            if let Some(ref border) = self.borders.left {
                write_border_xml(w, "a:lnL", border)?;
            }
            if let Some(ref border) = self.borders.right {
                write_border_xml(w, "a:lnR", border)?;
            }
            if let Some(ref border) = self.borders.top {
                write_border_xml(w, "a:lnT", border)?;
            }
            if let Some(ref border) = self.borders.bottom {
                write_border_xml(w, "a:lnB", border)?;
            }

            // Fill
            if let Some(ref fill) = self.fill {
                fill.write_xml(w)?;
            }

            w.write_str("</a:tcPr>")?;
        } else {
            w.write_str("/>")?;
        }

        w.write_str("</a:tc>")
    }

    /// Generate the `<a:tc>` XML element string.
    ///
    /// # Panics
    ///
    /// Panics if writing to a `String` fails (should never happen).
    #[must_use]
    pub fn to_xml_string(&self) -> String {
        // Estimate: ~200 bytes for a typical cell element
        let mut s = String::with_capacity(200);
        // SAFETY: fmt::Write for String is infallible
        self.write_xml(&mut s)
            .unwrap_or_else(|_| unreachable!("fmt::Write for String is infallible"));
        s
    }
}

/// Creates an empty table cell with a default text frame and no borders or merge info.
impl Default for Cell {
    fn default() -> Self {
        Self::new()
    }
}

impl CellBorders {
    /// Whether any border is defined.
    #[must_use]
    pub const fn has_any(&self) -> bool {
        self.left.is_some() || self.right.is_some() || self.top.is_some() || self.bottom.is_some()
    }
}

/// Write XML for a cell border element into the given writer.
fn write_border_xml<W: std::fmt::Write>(
    w: &mut W,
    tag: &str,
    border: &CellBorder,
) -> std::fmt::Result {
    write!(w, r#"<{} w="{}">"#, tag, border.width.0)?;
    w.write_str("<a:solidFill>")?;
    border.color.write_xml(w)?;
    w.write_str("</a:solidFill>")?;
    write!(w, "</{tag}>")
}
