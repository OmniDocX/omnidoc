//! Chart legend.

use crate::enums::chart::XlLegendPosition;
use crate::text::font::Font;

/// Represents the legend in a chart. A chart can have at most one legend.
#[derive(Debug, Clone)]
pub struct Legend {
    position: XlLegendPosition,
    include_in_layout: bool,
    overlay: bool,
    font: Option<Font>,
    entries: Vec<LegendEntry>,
    horz_offset: Option<f64>,
}

impl Legend {
    /// Create a new legend with default settings.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            position: XlLegendPosition::Right,
            include_in_layout: true,
            overlay: false,
            font: None,
            entries: Vec::new(),
            horz_offset: None,
        }
    }

    /// The legend position.
    #[must_use]
    pub const fn position(&self) -> XlLegendPosition {
        self.position
    }

    /// Set the legend position.
    pub fn set_position(&mut self, position: XlLegendPosition) {
        self.position = position;
    }

    /// Whether the legend is included in the chart layout.
    #[must_use]
    pub const fn include_in_layout(&self) -> bool {
        self.include_in_layout
    }

    /// Set whether the legend is included in the chart layout.
    pub fn set_include_in_layout(&mut self, value: bool) {
        self.include_in_layout = value;
    }

    /// Whether the legend overlays the plot area.
    #[must_use]
    pub const fn overlay(&self) -> bool {
        self.overlay
    }

    /// Set whether the legend overlays the plot area.
    pub fn set_overlay(&mut self, value: bool) {
        self.overlay = value;
    }

    /// The font for legend text, if set.
    #[must_use]
    pub const fn font(&self) -> Option<&Font> {
        self.font.as_ref()
    }

    /// Mutable access to the font. Creates a default if `None`.
    pub fn font_mut(&mut self) -> &mut Font {
        self.font.get_or_insert_with(Font::new)
    }

    /// Set the font for legend text.
    pub fn set_font(&mut self, font: Font) {
        self.font = Some(font);
    }

    /// The legend entries for per-entry visibility/formatting.
    #[must_use]
    pub fn legend_entries(&self) -> &[LegendEntry] {
        &self.entries
    }

    /// Add a legend entry.
    pub fn add_legend_entry(&mut self, entry: LegendEntry) {
        self.entries.push(entry);
    }

    /// The horizontal offset of the legend, as a fraction of the chart width.
    /// Valid range is -1.0 to 1.0. Positive values move the legend to the right.
    #[must_use]
    pub const fn horz_offset(&self) -> Option<f64> {
        self.horz_offset
    }

    /// Set the horizontal offset of the legend (-1.0 to 1.0).
    pub fn set_horz_offset(&mut self, value: Option<f64>) {
        self.horz_offset = value;
    }
}

/// Creates an empty legend with no entries and default position.
impl Default for Legend {
    fn default() -> Self {
        Self::new()
    }
}

/// A single entry in the legend, controlling per-series legend visibility.
#[derive(Debug, Clone)]
pub struct LegendEntry {
    index: usize,
    is_deleted: bool,
    font: Option<Font>,
}

impl LegendEntry {
    /// Create a new legend entry for the series at the given index.
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self {
            index,
            is_deleted: false,
            font: None,
        }
    }

    /// The series index this entry corresponds to.
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Whether this legend entry is hidden (deleted).
    #[must_use]
    pub const fn is_deleted(&self) -> bool {
        self.is_deleted
    }

    /// Set whether this legend entry is hidden.
    pub fn set_deleted(&mut self, value: bool) {
        self.is_deleted = value;
    }

    /// The font override for this entry.
    #[must_use]
    pub const fn font(&self) -> Option<&Font> {
        self.font.as_ref()
    }

    /// Set the font for this entry.
    pub fn set_font(&mut self, font: Font) {
        self.font = Some(font);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::font::RgbColor;

    #[test]
    fn test_legend_font() {
        let mut legend = Legend::new();
        assert!(legend.font().is_none());

        let font = legend.font_mut();
        font.bold = Some(true);
        font.size = Some(12.0);
        assert!(legend.font().is_some());
        assert_eq!(legend.font().unwrap().bold, Some(true));
    }

    #[test]
    fn test_legend_entries() {
        let mut legend = Legend::new();
        assert!(legend.legend_entries().is_empty());

        let mut entry = LegendEntry::new(0);
        entry.set_deleted(true);
        legend.add_legend_entry(entry);

        let entry = LegendEntry::new(1);
        legend.add_legend_entry(entry);

        assert_eq!(legend.legend_entries().len(), 2);
        assert!(legend.legend_entries()[0].is_deleted());
        assert!(!legend.legend_entries()[1].is_deleted());
    }

    #[test]
    fn test_legend_entry_font() {
        let mut entry = LegendEntry::new(0);
        assert!(entry.font().is_none());

        let mut font = Font::new();
        font.color = Some(RgbColor::new(255, 0, 0));
        entry.set_font(font);
        assert!(entry.font().is_some());
    }

    #[test]
    fn test_legend_horz_offset() {
        let mut legend = Legend::new();
        assert!(legend.horz_offset().is_none());

        legend.set_horz_offset(Some(0.5));
        assert_eq!(legend.horz_offset(), Some(0.5));

        legend.set_horz_offset(Some(-0.3));
        assert_eq!(legend.horz_offset(), Some(-0.3));

        legend.set_horz_offset(None);
        assert!(legend.horz_offset().is_none());
    }

    #[test]
    fn test_legend_horz_offset_bounds() {
        let mut legend = Legend::new();
        legend.set_horz_offset(Some(1.0));
        assert_eq!(legend.horz_offset(), Some(1.0));

        legend.set_horz_offset(Some(-1.0));
        assert_eq!(legend.horz_offset(), Some(-1.0));
    }
}
