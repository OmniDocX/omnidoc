mod xml;

#[cfg(test)]
mod tests;

use crate::dml::effect::ShadowFormat;
use crate::dml::effect3d::{Scene3D, Shape3D};
use crate::dml::fill::FillFormat;
use crate::dml::line::LineFormat;
use crate::enums::shapes::PresetGeometry;
use crate::shapes::action::ActionSetting;
use crate::shapes::freeform::FreeformBuilder;
use crate::shapes::placeholder::PlaceholderFormat;
use crate::text::TextFrame;
use crate::units::{Emu, ShapeId};

/// A regular shape (`<p:sp>`) that can contain text and has geometry.
///
/// This includes textboxes, rectangles, ovals, and other preset geometry shapes.
#[derive(Debug, Clone, PartialEq)]
pub struct AutoShape {
    pub shape_id: ShapeId,
    pub name: String,
    pub left: Emu,
    pub top: Emu,
    pub width: Emu,
    pub height: Emu,
    pub rotation: f64,
    /// The preset geometry type (e.g. `Rect`, `Ellipse`, `RoundRect`).
    /// None for freeform or custom geometry shapes.
    pub prst_geom: Option<PresetGeometry>,
    /// Whether this shape is a textbox (`<p:cNvSpPr txBox="1"/>`).
    pub is_textbox: bool,
    /// Placeholder information, if this shape is a placeholder.
    pub placeholder: Option<PlaceholderFormat>,
    /// Raw XML content of `<p:txBody>`, if present.
    pub tx_body_xml: Option<Vec<u8>>,
    /// Fill formatting for the shape.
    pub fill: Option<FillFormat>,
    /// Line (outline) formatting for the shape.
    pub line: Option<LineFormat>,
    /// Structured text frame for building text content.
    pub text_frame: Option<TextFrame>,
    /// Click action (hyperlink, navigation, etc.).
    pub click_action: Option<ActionSetting>,
    /// Hover action (hyperlink, navigation, etc.) triggered on mouse hover.
    pub hover_action: Option<ActionSetting>,
    /// Shape adjustment values for preset geometry.
    pub adjustments: Vec<f64>,
    /// Shadow effect.
    pub shadow: Option<ShadowFormat>,
    /// Custom geometry (overrides `prst_geom` when present).
    pub custom_geometry: Option<FreeformBuilder>,
    /// 3D scene properties (camera, lighting).
    pub scene_3d: Option<Scene3D>,
    /// 3D shape properties (bevel, extrusion, material).
    pub shape_3d: Option<Shape3D>,
}

impl AutoShape {
    /// Create a new `AutoShape` with the given position and size.
    ///
    /// All optional fields default to `None` or their zero-value equivalents.
    pub fn new(
        shape_id: ShapeId,
        name: impl Into<String>,
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
    ) -> Self {
        Self {
            shape_id,
            name: name.into(),
            left,
            top,
            width,
            height,
            rotation: 0.0,
            prst_geom: None,
            is_textbox: false,
            placeholder: None,
            tx_body_xml: None,
            fill: None,
            line: None,
            text_frame: None,
            click_action: None,
            hover_action: None,
            adjustments: Vec::new(),
            shadow: None,
            custom_geometry: None,
            scene_3d: None,
            shape_3d: None,
        }
    }

    /// Create a textbox shape with the given position and size.
    pub fn textbox(
        shape_id: ShapeId,
        name: impl Into<String>,
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
    ) -> Self {
        let mut shape = Self::new(shape_id, name, left, top, width, height);
        shape.is_textbox = true;
        shape.prst_geom = Some(PresetGeometry::Rect);
        shape
    }

    /// Create a shape with the given preset geometry type.
    pub fn with_geometry(
        shape_id: ShapeId,
        name: impl Into<String>,
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
        prst_geom: PresetGeometry,
    ) -> Self {
        let mut shape = Self::new(shape_id, name, left, top, width, height);
        shape.prst_geom = Some(prst_geom);
        shape
    }

    #[must_use]
    pub const fn has_text_frame(&self) -> bool {
        self.text_frame.is_some() || self.tx_body_xml.is_some()
    }

    /// Set the fill formatting.
    pub fn set_fill(&mut self, fill: FillFormat) {
        self.fill = Some(fill);
    }

    /// Set the line (outline) formatting.
    pub fn set_line(&mut self, line: LineFormat) {
        self.line = Some(line);
    }

    /// Get a reference to the text frame, if present.
    #[must_use]
    pub const fn text_frame(&self) -> Option<&TextFrame> {
        self.text_frame.as_ref()
    }

    /// Get a mutable reference to the text frame, if present.
    pub fn text_frame_mut(&mut self) -> Option<&mut TextFrame> {
        self.text_frame.as_mut()
    }

    /// Set a click action on this shape.
    pub fn set_click_action(&mut self, action: ActionSetting) {
        self.click_action = Some(action);
    }

    /// Set a hover action on this shape.
    pub fn set_hover_action(&mut self, action: ActionSetting) {
        self.hover_action = Some(action);
    }

    /// Set shadow effect.
    pub fn set_shadow(&mut self, shadow: ShadowFormat) {
        self.shadow = Some(shadow);
    }

    /// Set custom geometry from a `FreeformBuilder` (overrides preset geometry).
    pub fn set_custom_geometry(&mut self, geom: FreeformBuilder) {
        self.custom_geometry = Some(geom);
    }

    /// Get a reference to the 3D scene, if present.
    #[must_use]
    pub const fn scene_3d(&self) -> Option<&Scene3D> {
        self.scene_3d.as_ref()
    }

    /// Set the 3D scene (camera, lighting).
    pub fn set_scene_3d(&mut self, scene: Scene3D) {
        self.scene_3d = Some(scene);
    }

    /// Get a reference to the 3D shape properties, if present.
    #[must_use]
    pub const fn shape_3d(&self) -> Option<&Shape3D> {
        self.shape_3d.as_ref()
    }

    /// Set the 3D shape properties (bevel, extrusion, material).
    pub fn set_shape_3d(&mut self, shape3d: Shape3D) {
        self.shape_3d = Some(shape3d);
    }
}

impl std::fmt::Display for AutoShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AutoShape(\"{}\")", self.name)
    }
}
