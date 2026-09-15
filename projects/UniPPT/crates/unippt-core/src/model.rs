use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Deck {
    pub format: String,
    pub version: u32,
    pub title: String,
    pub width: f64,
    pub height: f64,
    pub source_width_emu: i64,
    pub source_height_emu: i64,
    /// Opaque handle used by the local server to retrieve the untouched source
    /// OPC package for loss-aware export. It is intentionally not a file path.
    #[serde(default)]
    pub source_import_id: Option<String>,
    /// Browser-ready projections of fonts embedded in the source presentation.
    /// Each entry represents one regular/bold/italic/bold-italic face.
    #[serde(default)]
    pub fonts: Vec<EmbeddedFont>,
    /// Namespaced, portable metadata owned by document hosts and plugins.
    ///
    /// The native PowerPoint projection deliberately ignores this map. It is
    /// retained by UDoc and lossless HTML so AI semantics, design tokens and
    /// plugin state can travel with the presentation without being injected
    /// into unrelated OPC XML parts. Namespaces are stable reverse-DNS-like
    /// keys, for example `org.unippt.ai`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extensions: BTreeMap<String, serde_json::Value>,
    pub slides: Vec<Slide>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddedFont {
    pub family: String,
    pub weight: u16,
    pub style: String,
    pub data_uri: String,
    pub mime_type: String,
    pub format: String,
    /// SHA-256 of the browser-ready font bytes. The browser treats this as the
    /// integrity boundary for local-font matching and network fallback.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// MD5 is retained only as a byte-version identity compatible with the
    /// UniDoc font manifest protocol; it is never used as a security digest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub md5: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "bytes")]
    pub byte_size: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postscript_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face_index: Option<u32>,
    #[serde(default)]
    pub source_part_name: Option<String>,
    #[serde(default)]
    pub source_relationship_id: Option<String>,
    #[serde(default)]
    pub font_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Slide {
    pub id: String,
    /// Absolute OPC part name, for example `/ppt/slides/slide1.xml`.
    #[serde(default)]
    pub source_part_name: Option<String>,
    pub name: String,
    pub background: String,
    /// Resolved slide/layout/master background picture as a browser-ready data URI.
    /// The native relationship remains untouched in the source PPTX package.
    #[serde(default)]
    pub background_asset: Option<String>,
    pub notes: String,
    /// Visible, non-placeholder furniture inherited from the slide master.
    /// Kept separate from slide-local objects so editing a slide never writes
    /// a colliding master shape id into `slideN.xml`.
    #[serde(default)]
    pub master_objects: Vec<SceneObject>,
    /// Visible, non-placeholder furniture inherited from the slide layout.
    /// PowerPoint paints this after the master and before slide-local objects.
    #[serde(default)]
    pub layout_objects: Vec<SceneObject>,
    pub objects: Vec<SceneObject>,
    /// Runtime projection of timing declared by the master/layout.  The native
    /// timing XML remains authoritative in those inherited OPC parts.
    #[serde(default)]
    pub inherited_animations: Vec<AnimationEffect>,
    /// Editable, flattened view of PowerPoint's `p:timing` tree.
    #[serde(default)]
    pub animations: Vec<AnimationEffect>,
    /// Native slide transition, when present.
    #[serde(default)]
    pub transition: Option<SlideTransition>,
    /// Exact imported timing fragment. It is retained as the lossless fallback
    /// and remains byte-identical until the animation model is edited.
    #[serde(default)]
    pub source_timing_xml: Option<String>,
    /// Exact imported transition fragment.
    #[serde(default)]
    pub source_transition_xml: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnimationEffect {
    pub id: String,
    #[serde(default)]
    pub source_timing_id: Option<u32>,
    #[serde(default)]
    pub target_object_id: Option<String>,
    #[serde(default)]
    pub target_shape_id: Option<u32>,
    pub effect: AnimationKind,
    pub class: AnimationClass,
    pub trigger: AnimationTrigger,
    pub duration_ms: u64,
    pub delay_ms: u64,
    /// Native `p:cTn/@accel` value in the OOXML 0..100000 time domain.
    /// Keeping this separate from duration lets browser playback preserve the
    /// apparent speed of strongly accelerating PowerPoint effects.
    #[serde(default)]
    pub acceleration: Option<u32>,
    /// Native `p:cTn/@decel` value in the OOXML 0..100000 time domain.
    #[serde(default)]
    pub deceleration: Option<u32>,
    /// Native `p:cTn/@spd` percentage in the signed OOXML 100000 domain.
    /// Negative values play the same effect backwards; magnitudes other than
    /// 100000 also change its effective wall-clock duration.
    #[serde(default)]
    pub speed: Option<i32>,
    /// Piecewise native time mapping from `p:cTn/@tmFilter`.  PowerPoint uses
    /// this for deliberately non-linear and sometimes returning trajectories
    /// which cannot be represented by a generic CSS `ease` token.
    #[serde(default)]
    pub time_filter: Option<String>,
    /// Native `p:cTn/@repeatCount`.  OOXML represents finite counts in its
    /// fixed-point time domain (for example `2000` means two iterations) and
    /// also permits `indefinite`, so the loss-aware model retains the token.
    #[serde(default)]
    pub repeat_count: Option<String>,
    /// Optional absolute cap from `p:cTn/@repeatDur`.
    #[serde(default)]
    pub repeat_duration_ms: Option<u64>,
    /// Play the effect forward and then backwards for each repeat cycle.
    #[serde(default)]
    pub auto_reverse: bool,
    pub order: u32,
    #[serde(default)]
    pub preset_id: Option<u32>,
    #[serde(default)]
    pub preset_subtype: Option<u32>,
    #[serde(default)]
    pub direction: Option<String>,
    #[serde(default)]
    pub motion_path: Option<String>,
    /// Companion fade from the native `p:animEffect` filter list.  PowerPoint
    /// pairs many property presets (float in/out, fold, fade-zoom) with
    /// `filter="fade"`; playback ramps this into the sampled property frames
    /// instead of flattening opacity.  Values are `in` and `out`.
    #[serde(default)]
    pub fade_filter: Option<String>,
    /// Numeric property behaviors emitted as `p:anim`.  PowerPoint uses these
    /// alongside preset effects to describe the exact position, size, and
    /// rotation trajectory (for example `ppt_x`, `ppt_w`, and
    /// `style.rotation`).  Expressions remain in their native normalized
    /// coordinate form so an untouched timing tree can still be written back
    /// losslessly while browser playback can evaluate the same trajectory.
    #[serde(default)]
    pub property_animations: Vec<AnimationPropertyAnimation>,
    /// Native media command (`play`, `pause`, or `stop`) for a `mediacall`
    /// timing node. Older UDOC payloads default to `play` at runtime.
    #[serde(default)]
    pub media_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnimationPropertyAnimation {
    #[serde(default)]
    pub attributes: Vec<String>,
    #[serde(default)]
    pub calculation_mode: Option<String>,
    #[serde(default)]
    pub value_type: Option<String>,
    #[serde(default)]
    pub additive: Option<String>,
    #[serde(default)]
    pub bounce_end: Option<u32>,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub by: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub fill: Option<String>,
    #[serde(default)]
    pub keyframes: Vec<AnimationPropertyKeyframe>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnimationPropertyKeyframe {
    /// Native `p:tav/@tm` value in the OOXML 0..100000 time domain.
    pub time: u32,
    /// Native numeric/string expression, including `#ppt_*` references.
    pub value: String,
    /// Optional native interpolation formula (`p:tav/@fmla`).
    #[serde(default)]
    pub formula: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AnimationKind {
    Appear,
    Fade,
    FlyIn,
    Wipe,
    RandomBars,
    Dissolve,
    Wheel,
    Circle,
    Split,
    Zoom,
    Spin,
    GrowShrink,
    MotionPath,
    Media,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AnimationClass {
    Entrance,
    Emphasis,
    Exit,
    MotionPath,
    Media,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AnimationTrigger {
    OnClick,
    WithPrevious,
    AfterPrevious,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SlideTransition {
    pub kind: String,
    pub duration_ms: u64,
    pub advance_on_click: bool,
    #[serde(default)]
    pub advance_after_ms: Option<u64>,
    #[serde(default)]
    pub direction: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImageCrop {
    #[serde(default)]
    pub left: f64,
    #[serde(default)]
    pub top: f64,
    #[serde(default)]
    pub right: f64,
    #[serde(default)]
    pub bottom: f64,
}

/// Destination offsets for a stretched DrawingML picture fill. Unlike source
/// crop values these fractions may be negative, which expands the image past
/// the shape boundary before the shape geometry clips it.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImageFillRect {
    #[serde(default)]
    pub left: f64,
    #[serde(default)]
    pub top: f64,
    #[serde(default)]
    pub right: f64,
    #[serde(default)]
    pub bottom: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SceneObject {
    pub id: String,
    pub source_shape_id: Option<u32>,
    pub name: String,
    pub kind: ObjectKind,
    pub frame: Frame,
    #[serde(default)]
    pub flip_h: bool,
    #[serde(default)]
    pub flip_v: bool,
    pub text: String,
    /// Run- and paragraph-level PowerPoint text formatting. `text` remains the
    /// editable plain-text fallback for older UDOC files.
    #[serde(default)]
    pub text_paragraphs: Vec<RichTextParagraph>,
    #[serde(default)]
    pub text_frame: TextFrameStyle,
    pub geometry: Option<String>,
    /// Browser-ready projection of an OOXML `a:custGeom` freeform path.
    /// The native XML remains authoritative for loss-aware PPTX export.
    #[serde(default)]
    pub custom_geometry: Option<CustomGeometry>,
    pub asset: Option<String>,
    /// Source-image crop fractions, applied before stretching the image into
    /// the PowerPoint frame. Values are in the inclusive 0..1 range.
    #[serde(default)]
    pub image_crop: ImageCrop,
    /// Signed destination rectangle from `a:stretch/a:fillRect`, normalized
    /// from OOXML 1/100000 percentages for browser positioning.
    #[serde(default)]
    pub image_fill_rect: ImageFillRect,
    /// Browser projection of native effects applied to the picture bitmap.
    /// The original DrawingML remains authoritative for native PPTX export.
    #[serde(default)]
    pub image_effects: ImageEffects,
    /// Browser-ready image used as this shape's fill. `asset` mirrors the
    /// same URI for compatibility with existing asset consumers.
    #[serde(default)]
    pub shape_fill_asset: Option<String>,
    #[serde(default)]
    pub formula: Option<FormulaData>,
    /// Playable audio/video asset linked from this picture's `p:nvPr`.
    /// The ordinary `asset` remains the native poster frame.
    #[serde(default)]
    pub media: Option<MediaData>,
    /// Shape-level PowerPoint actions stored below `p:cNvPr`.  Relationship
    /// identity is retained so an untouched object can round-trip byte for
    /// byte, while edited targets can be written as an additional OPC
    /// relationship without deleting unrelated or vendor relationships.
    #[serde(default)]
    pub hyperlinks: ObjectHyperlinks,
    /// Structured DrawingML table data for `p:graphicFrame/a:tbl` objects.
    /// The original table XML remains in the backing PPTX for lossless export.
    #[serde(default)]
    pub table: Option<TableData>,
    /// Read-only projection of the native DrawingML chart part. The chart
    /// part remains authoritative and is not rewritten by ordinary scene edits.
    #[serde(default)]
    pub chart: Option<crate::chart::ChartData>,
    pub style: VisualStyle,
    pub text_style: TextStyle,
    pub children: Vec<SceneObject>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImageEffects {
    #[serde(default)]
    pub duotone: Option<DuotoneStyle>,
    /// Browser-space radius of the native DrawingML soft-edge feather.
    #[serde(default)]
    pub soft_edge_radius: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DuotoneStyle {
    /// Color used for the darkest source pixels.
    pub shadow_color: String,
    /// Color used for the lightest source pixels.
    pub highlight_color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CustomGeometry {
    pub width: i64,
    pub height: i64,
    pub path_data: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObjectHyperlinks {
    #[serde(default)]
    pub click: Option<HyperlinkAction>,
    #[serde(default)]
    pub hover: Option<HyperlinkAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HyperlinkAction {
    /// Resolved URL for external hyperlinks or absolute OPC part name for an
    /// internal slide jump. Missing relationships deliberately remain `None`.
    #[serde(default)]
    pub target: Option<String>,
    /// Native relationship id from `r:id`, retained as loss-aware evidence.
    #[serde(default)]
    pub relationship_id: Option<String>,
    #[serde(default)]
    pub external: bool,
    /// Raw PowerPoint action URI, for example the next-slide show jump.
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub tooltip: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RichTextParagraph {
    /// Zero-based paragraph position in the source DrawingML text body.
    ///
    /// This identity lets the native writer update a paragraph in place while
    /// preserving unmodelled OOXML children and extension markup.
    #[serde(default)]
    pub source_index: Option<usize>,
    #[serde(default)]
    pub runs: Vec<RichTextRun>,
    pub align: String,
    #[serde(default)]
    pub level: u8,
    #[serde(default)]
    pub bullet: Option<String>,
    #[serde(default)]
    pub line_spacing: Option<f64>,
    #[serde(default)]
    pub space_before: Option<f64>,
    #[serde(default)]
    pub space_after: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RichTextRun {
    /// Zero-based run position in the source DrawingML paragraph. Line-break
    /// runs participate in the same index space as regular text runs.
    #[serde(default)]
    pub source_index: Option<usize>,
    pub text: String,
    pub font_family: String,
    /// Exact physical DrawingML typeface selected for this run. `font_family`
    /// remains the browser CSS fallback stack.
    #[serde(default)]
    pub native_font_family: Option<String>,
    /// Loss-aware DrawingML script font slots.  Unlike `font_family` (CSS)
    /// and the legacy `native_font_family` shortcut, these values map 1:1 to
    /// `<a:latin>`, `<a:ea>`, `<a:cs>`, `<a:sym>` and the `lang` attribute.
    /// Missing slots remain missing so ordinary edits do not materialise or
    /// overwrite inherited typefaces.
    #[serde(default)]
    pub native_fonts: FontSlots,
    pub font_size: f64,
    pub color: String,
    /// Run-level DrawingML gradient fill projected for browser text clipping.
    /// Solid text remains represented by `color` for backward compatibility.
    #[serde(default)]
    pub gradient: Option<GradientFillStyle>,
    pub bold: bool,
    pub italic: bool,
    #[serde(default)]
    pub underline: bool,
    /// Original DrawingML `u` value (`sng`, `dbl`, `wavy`, ...). The boolean
    /// `underline` field remains for compatibility with older UDOC readers.
    #[serde(default)]
    pub underline_style: Option<String>,
    /// Effective text opacity. Kept separately from the CSS color so an alpha
    /// transform can be written back without flattening it into RGB.
    #[serde(default = "default_alpha")]
    pub alpha: f64,
    #[serde(default)]
    pub strikethrough: bool,
    /// CSS-like baseline: `normal`, `sub`, or `super`.
    #[serde(
        default = "default_baseline",
        deserialize_with = "deserialize_baseline"
    )]
    pub baseline: String,
    /// Exact baseline displacement in percent of this run's font size.
    /// Positive raises text; negative lowers it. None uses the script preset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline_offset: Option<f64>,
    /// Run-level DrawingML actions from `a:rPr`. These stay on the exact run;
    /// they must never be promoted to the containing shape action.
    #[serde(default)]
    pub hyperlinks: ObjectHyperlinks,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TextFrameStyle {
    pub margin_left: f64,
    pub margin_right: f64,
    pub margin_top: f64,
    pub margin_bottom: f64,
    pub vertical_align: String,
    /// Raw DrawingML `a:bodyPr/@vert` token. `horz` is the interoperable
    /// default; values such as `eaVert` retain native PowerPoint vertical
    /// East-Asian text flow through import, editing, HTML, and PPTX export.
    #[serde(default = "default_text_vertical_type")]
    pub vertical_type: String,
    pub word_wrap: bool,
    /// DrawingML text autofit policy (`none`, `textToFitShape`, or
    /// `shapeToFitText`).  Keeping this in the scene model lets the browser
    /// reproduce PowerPoint's dynamic font shrinking for narrow text boxes.
    #[serde(default = "default_text_auto_size")]
    pub auto_size: String,
}

impl Default for TextFrameStyle {
    fn default() -> Self {
        Self {
            margin_left: 8.0,
            margin_right: 8.0,
            margin_top: 5.0,
            margin_bottom: 5.0,
            vertical_align: "center".into(),
            vertical_type: default_text_vertical_type(),
            word_wrap: true,
            auto_size: default_text_auto_size(),
        }
    }
}

fn default_text_auto_size() -> String {
    "none".into()
}

fn default_text_vertical_type() -> String {
    "horz".into()
}

fn default_baseline() -> String {
    "normal".into()
}

fn deserialize_baseline<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(default_baseline))
}

const fn default_alpha() -> f64 {
    1.0
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ObjectKind {
    Text,
    Shape,
    Image,
    Math,
    Table,
    Chart,
    SmartArt,
    Group,
    Connector,
    Ole,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MediaKind {
    Audio,
    Video,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MediaData {
    pub kind: MediaKind,
    /// Native package media as a data URI, or its original external URL.
    ///
    /// This value remains untouched even when the source codec is not
    /// browser-decodable.  Browser playback should prefer `playback_asset`
    /// and fall back to this field.
    #[serde(default)]
    pub asset: Option<String>,
    #[serde(default)]
    pub mime_type: Option<String>,
    /// Optional browser-compatible derivative.  The native `asset` above is
    /// retained losslessly for PPTX/UDOC round trips.
    #[serde(default)]
    pub playback_asset: Option<String>,
    #[serde(default)]
    pub playback_mime_type: Option<String>,
    /// Absolute OPC part name retained as loss-aware evidence.
    #[serde(default)]
    pub source_part_name: Option<String>,
    /// Office 2010 `p14:media/@r:embed` relationship identity.
    #[serde(default)]
    pub relationship_id: Option<String>,
    /// Legacy `a:audioFile|a:videoFile/@r:link` relationship identity.
    #[serde(default)]
    pub legacy_relationship_id: Option<String>,
    #[serde(default)]
    pub trim_start_ms: Option<u64>,
    /// Native `p14:trim/@end`: milliseconds removed from the media tail,
    /// not an absolute playback timestamp.
    #[serde(default)]
    pub trim_end_ms: Option<u64>,
    #[serde(default = "default_alpha")]
    pub volume: f64,
    #[serde(default)]
    pub loop_playback: bool,
    #[serde(default)]
    pub play_across_slides: bool,
    #[serde(default = "default_true")]
    pub show_when_stopped: bool,
}

const fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FormulaData {
    /// Canonical editable source. KaTeX markup is never persisted.
    pub latex: String,
    /// Native Office Math payload used for loss-aware PPTX export.
    pub omml: Option<String>,
    pub display: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TableData {
    /// Resolved column widths in scene pixels.
    #[serde(default)]
    pub columns: Vec<f64>,
    #[serde(default)]
    pub rows: Vec<TableRow>,
    #[serde(default)]
    pub first_row: bool,
    #[serde(default)]
    pub first_col: bool,
    #[serde(default)]
    pub last_row: bool,
    #[serde(default)]
    pub last_col: bool,
    #[serde(default)]
    pub band_rows: bool,
    #[serde(default)]
    pub band_cols: bool,
    /// Native table-style GUID, retained as evidence rather than flattened
    /// over the original package's table style definitions.
    #[serde(default)]
    pub style_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TableRow {
    /// Resolved row height in scene pixels.
    pub height: f64,
    #[serde(default)]
    pub cells: Vec<TableCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TableCell {
    pub text: String,
    #[serde(default)]
    pub text_paragraphs: Vec<RichTextParagraph>,
    #[serde(default)]
    pub text_frame: TextFrameStyle,
    #[serde(default)]
    pub text_style: TextStyle,
    #[serde(default = "transparent_fill")]
    pub fill: String,
    #[serde(default)]
    pub borders: TableCellBorders,
    #[serde(default = "one_span")]
    pub grid_span: u32,
    #[serde(default = "one_span")]
    pub row_span: u32,
    #[serde(default)]
    pub h_merge: bool,
    #[serde(default)]
    pub v_merge: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TableCellBorders {
    #[serde(default)]
    pub left: Option<TableCellBorder>,
    #[serde(default)]
    pub right: Option<TableCellBorder>,
    #[serde(default)]
    pub top: Option<TableCellBorder>,
    #[serde(default)]
    pub bottom: Option<TableCellBorder>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TableCellBorder {
    pub color: String,
    pub width: f64,
    #[serde(default)]
    pub dash: Option<String>,
}

fn transparent_fill() -> String {
    "transparent".into()
}

const fn one_span() -> u32 {
    1
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub rotation: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VisualStyle {
    pub fill: String,
    /// Structured gradient evidence retained alongside the browser-ready
    /// `fill` string. This keeps stop positions available to native writers.
    #[serde(default)]
    pub gradient: Option<GradientFillStyle>,
    pub stroke: String,
    pub stroke_width: f64,
    #[serde(default)]
    pub stroke_dash: Option<String>,
    pub opacity: f64,
    #[serde(default)]
    pub shadow: Option<ShadowStyle>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GradientFillStyle {
    /// CSS-compatible direction in degrees.
    pub angle: f64,
    #[serde(default)]
    pub stops: Vec<GradientStopStyle>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GradientStopStyle {
    /// Normalized position from 0.0 through 1.0.
    pub position: f64,
    pub color: String,
    #[serde(default = "default_alpha")]
    pub opacity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShadowStyle {
    pub color: String,
    #[serde(default = "default_alpha")]
    pub opacity: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    pub blur: f64,
    #[serde(default)]
    pub inset: bool,
}

impl Default for VisualStyle {
    fn default() -> Self {
        Self {
            fill: "transparent".into(),
            gradient: None,
            stroke: "transparent".into(),
            stroke_width: 0.0,
            stroke_dash: None,
            opacity: 1.0,
            shadow: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TextStyle {
    pub font_family: String,
    /// Exact physical DrawingML typeface used by the representative run.
    #[serde(default)]
    pub native_font_family: Option<String>,
    /// Representative DrawingML script slots for this text object.  Rich runs
    /// retain their own independent slots in `RichTextRun::native_fonts`.
    #[serde(default)]
    pub native_fonts: FontSlots,
    pub font_size: f64,
    pub color: String,
    pub bold: bool,
    pub italic: bool,
    pub align: String,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_family: "Aptos, 'Microsoft YaHei', sans-serif".into(),
            native_font_family: Some("Aptos".into()),
            native_fonts: FontSlots::default(),
            font_size: 24.0,
            color: "#172033".into(),
            bold: false,
            italic: false,
            align: "left".into(),
        }
    }
}

/// Native DrawingML typefaces split by script, plus the run language.
///
/// Every field is optional on purpose: absence means "inherit" and must be
/// distinguishable from an explicitly selected typeface during loss-aware
/// patching.  `nativeFontFamily` remains alongside this structure for older
/// UDOC consumers, while new readers should prefer `nativeFonts`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FontSlots {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latin: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub east_asia: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub complex_script: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language_id: Option<String>,
}

impl FontSlots {
    #[must_use]
    pub fn unified(typeface: impl Into<String>) -> Self {
        let typeface = typeface.into();
        Self {
            latin: Some(typeface.clone()),
            east_asia: Some(typeface.clone()),
            complex_script: Some(typeface.clone()),
            symbol: Some(typeface),
            language_id: None,
        }
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.latin.is_none()
            && self.east_asia.is_none()
            && self.complex_script.is_none()
            && self.symbol.is_none()
            && self.language_id.is_none()
    }
}

impl Deck {
    #[must_use]
    pub fn demo() -> Self {
        let mut objects = vec![
            scene_shape(
                "shape-accent",
                "强调线",
                Frame {
                    x: 74.0,
                    y: 72.0,
                    width: 13.0,
                    height: 476.0,
                    rotation: 0.0,
                },
                "",
                "#c43e1c",
                "#c43e1c",
            ),
            scene_text(
                "shape-kicker",
                "眉题",
                Frame {
                    x: 111.0,
                    y: 78.0,
                    width: 575.0,
                    height: 38.0,
                    rotation: 0.0,
                },
                "UNIPPT  ·  PRESENTATION ENGINE",
                TextStyle {
                    font_size: 16.0,
                    color: "#c43e1c".into(),
                    bold: true,
                    ..TextStyle::default()
                },
            ),
            scene_text(
                "shape-title",
                "标题",
                Frame {
                    x: 105.0,
                    y: 137.0,
                    width: 665.0,
                    height: 190.0,
                    rotation: 0.0,
                },
                "让观点，\n被真正看见。",
                TextStyle {
                    font_size: 62.0,
                    color: "#1f2430".into(),
                    bold: true,
                    ..TextStyle::default()
                },
            ),
            scene_text(
                "shape-subtitle",
                "副标题",
                Frame {
                    x: 111.0,
                    y: 350.0,
                    width: 625.0,
                    height: 95.0,
                    rotation: 0.0,
                },
                "兼容 PowerPoint 的高保真文档引擎，\n加上浏览器里真正自由的创作体验。",
                TextStyle {
                    font_size: 23.0,
                    color: "#626977".into(),
                    ..TextStyle::default()
                },
            ),
            scene_shape(
                "shape-card",
                "公式卡片",
                Frame {
                    x: 812.0,
                    y: 71.0,
                    width: 389.0,
                    height: 478.0,
                    rotation: 0.0,
                },
                "",
                "linear-gradient(145deg,#d55332,#a92d13)",
                "transparent",
            ),
            scene_text(
                "shape-card-label",
                "卡片标签",
                Frame {
                    x: 853.0,
                    y: 114.0,
                    width: 310.0,
                    height: 50.0,
                    rotation: 0.0,
                },
                "NATIVE OFFICE MATH",
                TextStyle {
                    font_size: 15.0,
                    color: "#ffd8cc".into(),
                    bold: true,
                    ..TextStyle::default()
                },
            ),
            scene_text(
                "shape-card-copy",
                "卡片说明",
                Frame {
                    x: 851.0,
                    y: 364.0,
                    width: 305.0,
                    height: 112.0,
                    rotation: 0.0,
                },
                "LaTeX 可视化编辑\nOMML 原生双向保存",
                TextStyle {
                    font_size: 25.0,
                    color: "#ffffff".into(),
                    bold: true,
                    ..TextStyle::default()
                },
            ),
            scene_text(
                "shape-footer",
                "页脚",
                Frame {
                    x: 111.0,
                    y: 610.0,
                    width: 970.0,
                    height: 42.0,
                    rotation: 0.0,
                },
                "PPTX 导入   ·   场景化编辑   ·   HTML 放映   ·   Loss-aware 回写",
                TextStyle {
                    font_size: 16.0,
                    color: "#505766".into(),
                    bold: true,
                    ..TextStyle::default()
                },
            ),
            scene_text(
                "shape-page",
                "页码",
                Frame {
                    x: 1122.0,
                    y: 610.0,
                    width: 79.0,
                    height: 42.0,
                    rotation: 0.0,
                },
                "01 / 01",
                TextStyle {
                    font_size: 14.0,
                    color: "#8b8f98".into(),
                    align: "right".into(),
                    ..TextStyle::default()
                },
            ),
        ];
        objects.push(SceneObject {
            id: "shape-equation".into(),
            source_shape_id: None,
            name: "公式".into(),
            kind: ObjectKind::Math,
            frame: Frame { x: 842.0, y: 195.0, width: 329.0, height: 116.0, rotation: 0.0 },
            flip_h: false,
            flip_v: false,
            text: "E=mc^2".into(),
            text_paragraphs: vec![],
            text_frame: TextFrameStyle::default(),
            geometry: None,
            custom_geometry: None,
            asset: None,
            image_crop: ImageCrop::default(),
            image_fill_rect: ImageFillRect::default(),
            image_effects: ImageEffects::default(),
            shape_fill_asset: None,
            formula: Some(FormulaData {
                latex: "E=mc^2".into(),
                omml: Some("<m:oMathPara xmlns:m=\"http://schemas.openxmlformats.org/officeDocument/2006/math\"><m:oMath><m:r><m:t>E=m</m:t></m:r><m:sSup><m:e><m:r><m:t>c</m:t></m:r></m:e><m:sup><m:r><m:t>2</m:t></m:r></m:sup></m:sSup></m:oMath></m:oMathPara>".into()),
                display: true,
            }),
            media: None,
            hyperlinks: ObjectHyperlinks::default(),
            table: None,
            chart: None,
            style: VisualStyle::default(),
            text_style: TextStyle { font_size: 42.0, color: "#ffffff".into(), align: "center".into(), ..TextStyle::default() },
            children: vec![],
        });

        Self {
            format: "unippt".into(),
            version: 1,
            title: "演示文稿1".into(),
            width: 1280.0,
            height: 720.0,
            source_width_emu: 12_192_000,
            source_height_emu: 6_858_000,
            source_import_id: None,
            fonts: vec![],
            extensions: BTreeMap::new(),
            slides: vec![Slide {
                id: "slide-1".into(),
                source_part_name: None,
                name: "标题幻灯片".into(),
                background: "linear-gradient(135deg,#fbf8f4,#f3eee8)".into(),
                background_asset: None,
                notes: "双击文本或公式即可编辑；插入选项卡提供原生公式工具。".into(),
                master_objects: vec![],
                layout_objects: vec![],
                objects,
                inherited_animations: vec![],
                animations: vec![
                    AnimationEffect {
                        id: "anim-title".into(),
                        source_timing_id: None,
                        target_object_id: Some("shape-title".into()),
                        target_shape_id: None,
                        effect: AnimationKind::Fade,
                        class: AnimationClass::Entrance,
                        trigger: AnimationTrigger::OnClick,
                        duration_ms: 650,
                        delay_ms: 0,
                        acceleration: None,
                        deceleration: None,
                        speed: None,
                        time_filter: None,
                        repeat_count: None,
                        repeat_duration_ms: None,
                        auto_reverse: false,
                        order: 0,
                        preset_id: Some(10),
                        preset_subtype: Some(0),
                        direction: None,
                        motion_path: None,
                        fade_filter: None,
                        property_animations: vec![],
                        media_action: None,
                    },
                    AnimationEffect {
                        id: "anim-subtitle".into(),
                        source_timing_id: None,
                        target_object_id: Some("shape-subtitle".into()),
                        target_shape_id: None,
                        effect: AnimationKind::FlyIn,
                        class: AnimationClass::Entrance,
                        trigger: AnimationTrigger::AfterPrevious,
                        duration_ms: 550,
                        delay_ms: 120,
                        acceleration: None,
                        deceleration: None,
                        speed: None,
                        time_filter: None,
                        repeat_count: None,
                        repeat_duration_ms: None,
                        auto_reverse: false,
                        order: 1,
                        preset_id: Some(2),
                        preset_subtype: Some(8),
                        direction: Some("left".into()),
                        motion_path: None,
                        fade_filter: None,
                        property_animations: vec![],
                        media_action: None,
                    },
                ],
                transition: None,
                source_timing_xml: None,
                source_transition_xml: None,
            }],
        }
    }
}

fn scene_text(
    id: &str,
    name: &str,
    frame: Frame,
    text: &str,
    text_style: TextStyle,
) -> SceneObject {
    SceneObject {
        id: id.into(),
        source_shape_id: None,
        name: name.into(),
        kind: ObjectKind::Text,
        frame,
        flip_h: false,
        flip_v: false,
        text: text.into(),
        text_paragraphs: vec![],
        text_frame: TextFrameStyle::default(),
        geometry: None,
        custom_geometry: None,
        asset: None,
        image_crop: ImageCrop::default(),
        image_fill_rect: ImageFillRect::default(),
        image_effects: ImageEffects::default(),
        shape_fill_asset: None,
        formula: None,
        media: None,
        hyperlinks: ObjectHyperlinks::default(),
        table: None,
        chart: None,
        style: VisualStyle::default(),
        text_style,
        children: vec![],
    }
}

fn scene_shape(
    id: &str,
    name: &str,
    frame: Frame,
    text: &str,
    fill: &str,
    stroke: &str,
) -> SceneObject {
    SceneObject {
        id: id.into(),
        source_shape_id: None,
        name: name.into(),
        kind: ObjectKind::Shape,
        frame,
        flip_h: false,
        flip_v: false,
        text: text.into(),
        text_paragraphs: vec![],
        text_frame: TextFrameStyle::default(),
        geometry: Some("roundRect".into()),
        custom_geometry: None,
        asset: None,
        image_crop: ImageCrop::default(),
        image_fill_rect: ImageFillRect::default(),
        image_effects: ImageEffects::default(),
        shape_fill_asset: None,
        formula: None,
        media: None,
        hyperlinks: ObjectHyperlinks::default(),
        table: None,
        chart: None,
        style: VisualStyle {
            fill: fill.into(),
            gradient: None,
            stroke: stroke.into(),
            stroke_width: 0.0,
            stroke_dash: None,
            opacity: 1.0,
            shadow: None,
        },
        text_style: TextStyle::default(),
        children: vec![],
    }
}

#[cfg(test)]
mod font_slot_tests {
    use super::*;

    #[test]
    fn legacy_text_style_without_native_fonts_still_deserializes() {
        let style: TextStyle = serde_json::from_str(
            r##"{"fontFamily":"Calibri, sans-serif","nativeFontFamily":"Calibri","fontSize":24.0,"color":"#000000","bold":false,"italic":false,"align":"left"}"##,
        )
        .unwrap();
        assert_eq!(style.native_font_family.as_deref(), Some("Calibri"));
        assert!(style.native_fonts.is_empty());
    }

    #[test]
    fn legacy_null_rich_text_baseline_defaults_to_normal() {
        let run: RichTextRun = serde_json::from_value(serde_json::json!({
            "text": "OCR text",
            "fontFamily": "Aptos",
            "fontSize": 24.0,
            "color": "#ffffff",
            "bold": false,
            "italic": false,
            "baseline": null
        }))
        .unwrap();
        assert_eq!(run.baseline, "normal");
    }

    #[test]
    fn native_fonts_serialize_with_camel_case_script_slots() {
        let slots = FontSlots {
            latin: Some("Calibri".into()),
            east_asia: Some("Microsoft YaHei".into()),
            complex_script: Some("Arial".into()),
            symbol: Some("Wingdings".into()),
            language_id: Some("zh-CN".into()),
        };
        let value = serde_json::to_value(slots).unwrap();
        assert_eq!(value["eastAsia"], "Microsoft YaHei");
        assert_eq!(value["complexScript"], "Arial");
        assert_eq!(value["languageId"], "zh-CN");
    }

    #[test]
    fn namespaced_extensions_are_portable_and_legacy_decks_default_empty() {
        let mut deck = Deck::demo();
        deck.extensions.insert(
            "org.unippt.ai".into(),
            serde_json::json!({
                "schemaVersion": 1,
                "objects": {"shape-title": {"role": "slide-title"}}
            }),
        );
        let json = serde_json::to_string(&deck).unwrap();
        let reopened: Deck = serde_json::from_str(&json).unwrap();
        assert_eq!(reopened.extensions, deck.extensions);

        let mut legacy = serde_json::to_value(&deck).unwrap();
        legacy.as_object_mut().unwrap().remove("extensions");
        let reopened: Deck = serde_json::from_value(legacy).unwrap();
        assert!(reopened.extensions.is_empty());
    }
}
