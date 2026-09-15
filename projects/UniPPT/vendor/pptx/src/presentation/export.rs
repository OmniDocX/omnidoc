use crate::error::PptxResult;
use crate::export::HtmlExporter;

use super::Presentation;

/// Export methods for `Presentation`.
impl Presentation {
    /// Export the presentation as a self-contained HTML string.
    ///
    /// Each slide becomes a `<div class="slide">` element with shapes rendered
    /// using absolute CSS positioning. Text formatting (bold, italic, color,
    /// size) is preserved. Images with cached data are embedded as base64
    /// data URIs.
    /// # Errors
    ///
    /// Returns an error if the presentation cannot be exported.
    pub fn export_html(&self) -> PptxResult<String> {
        HtmlExporter::new(self).export()
    }
}
