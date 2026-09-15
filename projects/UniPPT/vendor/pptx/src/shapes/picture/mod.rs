mod xml;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use crate::dml::effect::{DuotoneEffect, ShadowFormat};
use crate::dml::effect3d::{Scene3D, Shape3D};
use crate::dml::line::LineFormat;
use crate::enums::shapes::MsoAutoShapeType;
use crate::error::PptxError;
use crate::media::Image;
use crate::shapes::action::ActionSetting;
use crate::shapes::placeholder::PlaceholderFormat;
use crate::units::{Emu, ShapeId};

/// A picture shape (`<p:pic>`).
///
/// Contains a reference to an image via a relationship ID.
#[derive(Debug, Clone, PartialEq)]
pub struct Picture {
    pub shape_id: ShapeId,
    pub name: String,
    pub left: Emu,
    pub top: Emu,
    pub width: Emu,
    pub height: Emu,
    pub rotation: f64,
    /// Whether the picture is mirrored horizontally by `<a:xfrm flipH="1">`.
    pub flip_h: bool,
    /// Whether the picture is mirrored vertically by `<a:xfrm flipV="1">`.
    pub flip_v: bool,
    /// The relationship ID referencing the image part (from `<a:blip r:embed="...">`).
    pub image_r_id: Option<String>,
    /// Description text for accessibility.
    pub description: Option<String>,
    /// Placeholder information, if this shape is a placeholder.
    pub placeholder: Option<PlaceholderFormat>,
    /// Crop left (0.0 to 1.0).
    pub crop_left: f64,
    /// Crop right (0.0 to 1.0).
    pub crop_right: f64,
    /// Crop top (0.0 to 1.0).
    pub crop_top: f64,
    /// Crop bottom (0.0 to 1.0).
    pub crop_bottom: f64,
    /// Optional native bitmap duotone mapping declared below `<a:blip>`.
    pub duotone: Option<DuotoneEffect>,
    /// Native DrawingML soft-edge radius declared in `<a:effectLst>`, in EMU.
    pub soft_edge_radius: Option<Emu>,
    /// Line (outline) formatting for the picture.
    pub line: Option<LineFormat>,
    /// Click action (hyperlink, navigation, etc.).
    pub click_action: Option<ActionSetting>,
    /// Hover action (hyperlink, navigation, etc.) triggered on mouse hover.
    pub hover_action: Option<ActionSetting>,
    /// Shadow effect.
    pub shadow: Option<ShadowFormat>,
    /// Auto shape type for clipping the picture to a shape geometry.
    /// When set, emits `<a:prstGeom prst="X">` instead of the default "rect".
    pub auto_shape_type: Option<MsoAutoShapeType>,
    /// Cached image data (not serialized in XML; populated when reading from a package).
    ///
    /// The bytes are reference-counted so repeated references to one OPC image part
    /// share a single allocation across slides, layouts, masters, and groups.
    pub image_data: Option<Arc<[u8]>>,
    /// Cached image content type (not serialized in XML; populated when reading from a package).
    pub image_content_type: Option<String>,
    /// 3D scene properties (camera, lighting).
    pub scene_3d: Option<Scene3D>,
    /// 3D shape properties (bevel, extrusion, material).
    pub shape_3d: Option<Shape3D>,
}

impl Picture {
    /// Create a new `Picture` with the given position, size, and image relationship ID.
    ///
    /// All optional fields default to `None` or their zero-value equivalents.
    pub fn new(
        shape_id: ShapeId,
        name: impl Into<String>,
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
        image_r_id: impl Into<String>,
    ) -> Self {
        Self {
            shape_id,
            name: name.into(),
            left,
            top,
            width,
            height,
            rotation: 0.0,
            flip_h: false,
            flip_v: false,
            image_r_id: Some(image_r_id.into()),
            description: None,
            placeholder: None,
            crop_left: 0.0,
            crop_right: 0.0,
            crop_top: 0.0,
            crop_bottom: 0.0,
            duotone: None,
            soft_edge_radius: None,
            line: None,
            click_action: None,
            hover_action: None,
            shadow: None,
            auto_shape_type: None,
            image_data: None,
            image_content_type: None,
            scene_3d: None,
            shape_3d: None,
        }
    }

    /// Set the line (outline) formatting.
    pub fn set_line(&mut self, line: LineFormat) {
        self.line = Some(line);
    }

    /// Set crop values (each 0.0 to 1.0).
    ///
    /// # Errors
    ///
    /// Returns `Err(PptxError::InvalidValue)` if any value is outside the range 0.0 to 1.0.
    pub fn set_crop(
        &mut self,
        left: f64,
        top: f64,
        right: f64,
        bottom: f64,
    ) -> Result<(), PptxError> {
        Self::check_crop("Picture.crop_left", left)?;
        Self::check_crop("Picture.crop_top", top)?;
        Self::check_crop("Picture.crop_right", right)?;
        Self::check_crop("Picture.crop_bottom", bottom)?;
        self.crop_left = left;
        self.crop_top = top;
        self.crop_right = right;
        self.crop_bottom = bottom;
        Ok(())
    }

    /// Set the left crop (0.0 to 1.0).
    ///
    /// # Errors
    ///
    /// Returns `Err(PptxError::InvalidValue)` if value is outside 0.0 to 1.0.
    pub fn set_crop_left(&mut self, value: f64) -> Result<(), PptxError> {
        Self::check_crop("Picture.crop_left", value)?;
        self.crop_left = value;
        Ok(())
    }

    /// Set the top crop (0.0 to 1.0).
    ///
    /// # Errors
    ///
    /// Returns `Err(PptxError::InvalidValue)` if value is outside 0.0 to 1.0.
    pub fn set_crop_top(&mut self, value: f64) -> Result<(), PptxError> {
        Self::check_crop("Picture.crop_top", value)?;
        self.crop_top = value;
        Ok(())
    }

    /// Set the right crop (0.0 to 1.0).
    ///
    /// # Errors
    ///
    /// Returns `Err(PptxError::InvalidValue)` if value is outside 0.0 to 1.0.
    pub fn set_crop_right(&mut self, value: f64) -> Result<(), PptxError> {
        Self::check_crop("Picture.crop_right", value)?;
        self.crop_right = value;
        Ok(())
    }

    /// Set the bottom crop (0.0 to 1.0).
    ///
    /// # Errors
    ///
    /// Returns `Err(PptxError::InvalidValue)` if value is outside 0.0 to 1.0.
    pub fn set_crop_bottom(&mut self, value: f64) -> Result<(), PptxError> {
        Self::check_crop("Picture.crop_bottom", value)?;
        self.crop_bottom = value;
        Ok(())
    }

    fn check_crop(field: &'static str, val: f64) -> Result<(), PptxError> {
        if !(0.0..=1.0).contains(&val) {
            return Err(PptxError::InvalidValue {
                field,
                value: val.to_string(),
                expected: "0.0 to 1.0",
            });
        }
        Ok(())
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

    /// Set the auto shape type for picture clipping.
    /// The picture will be clipped to the specified shape geometry.
    pub fn set_auto_shape_type(&mut self, shape_type: MsoAutoShapeType) {
        self.auto_shape_type = Some(shape_type);
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

    /// Get the image associated with this picture, if image data has been cached.
    ///
    /// Returns `None` if no image data was populated (e.g., if the `Picture` was
    /// constructed programmatically without reading from a package).
    #[must_use]
    pub fn image(&self) -> Option<Image> {
        let data = self.image_data.as_ref()?;
        let ct = self.image_content_type.as_deref()?;
        Some(Image::from_bytes(data.to_vec(), ct))
    }
}

impl std::fmt::Display for Picture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Picture(\"{}\")", self.name)
    }
}
