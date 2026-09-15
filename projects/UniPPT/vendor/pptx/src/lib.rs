#![forbid(unsafe_code)]
//! # API naming conventions
//!
//! This crate follows these naming patterns:
//!
//! - **`new()`** / **`with_*()`**: Constructors. `new` is the primary constructor;
//!   `with_*` creates an instance with additional parameters (mirrors `Vec::with_capacity`).
//! - **`set_*()`**: Mutating setters that take `&mut self` and set a value.
//! - **`*_mut()`**: Methods returning a mutable reference to an inner value.
//! - **Builder-pattern `with_*()` / `without_*()`**: Methods that consume `self` and
//!   return `Self`, used on small value types (e.g. `Comment`, `SlideTransition`).
//! - **`add_*()`**: Methods that append a new item to an internal collection and typically
//!   return `&mut Item`.
//! - **Enum prefixes**: `Xl*` (Excel/chart enums), `Mso*` (Microsoft Office shared enums),
//!   and `Pp*` (PowerPoint-specific enums) follow python-pptx naming for familiarity.
//! - **Public fields**: Used for simple data types without invariants. Setters are
//!   provided only when validation or side-effects are needed.

pub(crate) mod xml_util;

pub mod animation;
pub mod chart;
pub mod comment;
pub mod core_properties;
pub mod dml;
pub mod embedded_font;
pub mod enums;
pub mod error;
pub mod export;
pub mod media;
pub mod opc;
pub(crate) mod oxml;
pub mod presentation;
pub mod print_settings;
pub mod repair;
pub mod section;
pub mod shapes;
pub mod signature;
pub mod slide;
pub mod smartart;
pub mod table;
pub mod text;
pub mod theme;
pub mod transition;
pub mod units;

// --- Convenience re-exports for the most-used public types ---

pub use error::{PackageError, PartNotFoundExt, PptxError, PptxResult, SlideError};
pub use presentation::Presentation;
pub use units::{
    Centipoints, Cm, ConnectionPointIndex, DurationMs, Emu, Inches, Mm, PlaceholderIndex, Pt,
    ShapeId, SlideId, Twips,
};
pub use xml_util::WriteXml;

// Shapes
pub use shapes::{
    AutoShape, Connector, GraphicFrame, GroupShape, OleObject, Picture, PlaceholderFormat, Shape,
    ShapeProperties, ShapeTree,
};

// Text
pub use text::font::RgbColor;
pub use text::{BulletFormat, Font, Paragraph, Run, TextFrame};

// Table
pub use table::{Cell, CellBorder, Column, Row, Table};

// DML
pub use dml::{
    ColorFormat, FillFormat, GradientFill, GradientStop, HslColor, LineFormat, PatternFill,
    PictureFill, PictureSourceRect, PresetColor, SolidFill, SystemColor, ThemeColor,
};

// Chart
pub use chart::{
    AxisTitle, BubbleChartData, Categories, Category, CategoryAxis, CategoryChartData,
    CategoryLevel, Chart, ChartFormat, ChartTitle, ChartXmlWriter, ComboChartData, ComboSeriesData,
    ComboSeriesType, DataLabel, DataLabels, DateAxis, DateAxisChartData, Legend, LegendEntry,
    Marker, MarkerFormat, Plot, PlotProperties, Point, Series, SeriesCollection, SeriesFormat,
    TickLabels, ValueAxis, XyChartData,
};

// Media
pub use media::{Audio, Image, Video};

// Core Properties
pub use core_properties::CoreProperties;

// Actions & Hyperlinks
pub use shapes::action::{ActionSetting, Hyperlink};

// Freeform
pub use shapes::freeform::FreeformBuilder;

// Effects
pub use dml::effect::{
    DuotoneColor, DuotoneColorTransform, DuotoneEffect, ShadowFormat, ShadowType,
};

// 3D Effects
pub use dml::effect3d::{Bevel, Camera, LightRig, Rotation3D, Scene3D, Shape3D};

// Transitions
pub use transition::{SlideTransition, TransitionType};

// Animations
pub use animation::{
    AnimationEffect, AnimationSequence, AnimationTrigger, EmphasisType, EntranceType, ExitType,
    SlideAnimation,
};

// Sections
pub use section::Section;

// Comments
pub use comment::Comment;

// Enums
pub use enums::action::PpActionType;
pub use enums::chart::{
    XlAxisCrosses, XlCategoryType, XlChartType, XlDataLabelPosition, XlLegendPosition,
    XlMarkerStyle, XlTickLabelPosition, XlTickMark,
};
pub use enums::dml::{
    BevelType, CameraPreset, LightDirection, LightRigType, MaterialPreset, MsoColorType,
    MsoFillType, PresetColorVal, SystemColorVal,
};
pub use enums::misc::{
    ExcelNumFormat, HandoutLayout, PpMediaType, PrintColorMode, PrintOrientation, PrintWhat,
};
pub use enums::shapes::{PlaceholderOrientation, PlaceholderSize, PresetGeometry};
pub use enums::text::TextDirection;

// Theme
pub use theme::{parse_theme_color_scheme, update_theme_color_scheme, ThemeColorScheme};

// SmartArt
pub use smartart::{SmartArt, SmartArtNode};

// Embedded Fonts
pub use embedded_font::EmbeddedFont;

// Print Settings
pub use print_settings::PrintSettings;

// Repair & Validation
pub use repair::{
    IssueCategory, PptxRepairer, PptxValidator, RepairReport, Severity, ValidationIssue,
};

// Digital Signatures
pub use signature::{DigitalSignature, HashAlgorithm, SignatureCommitment, SignerInfo};

// Export
pub use export::HtmlExporter;

#[cfg(test)]
mod thread_safety {
    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn key_types_are_send_and_sync() {
        assert_send_sync::<crate::presentation::Presentation>();
        assert_send_sync::<crate::shapes::Shape>();
        assert_send_sync::<crate::chart::Chart>();
        assert_send_sync::<crate::text::paragraph::Paragraph>();
        assert_send_sync::<crate::text::run::Run>();
        assert_send_sync::<crate::dml::color::ColorFormat>();
        assert_send_sync::<crate::text::font::Font>();
        assert_send_sync::<crate::text::font::RgbColor>();
        assert_send_sync::<crate::table::Table>();
        assert_send_sync::<crate::error::PptxError>();
    }
}
