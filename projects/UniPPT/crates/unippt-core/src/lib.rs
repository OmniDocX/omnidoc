//! UniPPT's stable, browser-oriented presentation scene model.

mod animation;
mod chart;
mod compact;
mod drawingml_text_edit;
mod embedded_fonts;
mod exporter;
mod font_policy;
mod hyperlink;
mod importer;
mod media;
mod model;

pub use chart::{ChartData, ChartKind, ChartLegend, ChartSeries};
pub use compact::{CompactImport, ImportedAsset, ASSET_REFERENCE_PREFIX};
pub use exporter::{export_pptx, export_pptx_with_baseline, ExportError};
pub use importer::{import_pptx, import_pptx_compact, ImportError};
pub use model::{
    AnimationClass, AnimationEffect, AnimationKind, AnimationPropertyAnimation,
    AnimationPropertyKeyframe, AnimationTrigger, Deck, DuotoneStyle, EmbeddedFont, FormulaData,
    Frame, GradientFillStyle, GradientStopStyle, HyperlinkAction, ImageEffects, MediaData,
    MediaKind, ObjectHyperlinks, ObjectKind, RichTextParagraph, RichTextRun, SceneObject,
    ShadowStyle, Slide, SlideTransition, TableCell, TableCellBorder, TableCellBorders, TableData,
    TableRow, TextFrameStyle, TextStyle, VisualStyle,
};
