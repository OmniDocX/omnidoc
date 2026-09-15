//! Fill struct types: `SolidFill`, `GradientFill`, `GradientStop`, `PatternFill`, `PictureFill`.

use std::sync::Arc;

use crate::dml::color::ColorFormat;
use crate::enums::dml::MsoPatternType;
use crate::error::PptxError;
use crate::units::RelationshipId;

/// A solid color fill.
#[derive(Debug, Clone, PartialEq)]
pub struct SolidFill {
    /// The fill color.
    pub color: ColorFormat,
    /// Effective DrawingML alpha, normalized to `0.0..=1.0`.
    ///
    /// This is stored on the fill rather than on the whole shape so text and
    /// stroke content remain fully opaque when only the shape fill is
    /// translucent.
    pub opacity: f64,
}

/// A gradient fill with a sequence of gradient stops.
#[derive(Debug, Clone, PartialEq)]
pub struct GradientFill {
    /// The gradient stops, defining colors and positions along the gradient.
    pub stops: Vec<GradientStop>,
    /// The angle of a linear gradient in degrees (0..360).
    /// `None` means the angle is inherited or the gradient is non-linear.
    pub angle: Option<f64>,
}

/// A single stop along a gradient.
#[derive(Debug, Clone, PartialEq)]
pub struct GradientStop {
    /// Position of this stop, from 0.0 (start) to 1.0 (end).
    pub position: f64,
    /// The color at this stop.
    pub color: ColorFormat,
    /// Effective DrawingML alpha for this stop, normalized to `0.0..=1.0`.
    pub opacity: f64,
}

impl GradientStop {
    /// Create a validated gradient stop.
    ///
    /// `position` must be in the range 0.0 to 1.0 (inclusive).
    ///
    /// # Errors
    ///
    /// Returns `Err(PptxError::InvalidValue)` if the position is out of range.
    pub fn new(position: f64, color: ColorFormat) -> Result<Self, PptxError> {
        if !(0.0..=1.0).contains(&position) {
            return Err(PptxError::InvalidValue {
                field: "GradientStop.position",
                value: position.to_string(),
                expected: "0.0 to 1.0",
            });
        }
        Ok(Self {
            position,
            color,
            opacity: 1.0,
        })
    }

    /// Create a validated gradient stop with an explicit alpha value.
    ///
    /// # Errors
    ///
    /// Returns `Err(PptxError::InvalidValue)` when either normalized value is
    /// outside `0.0..=1.0`.
    pub fn with_opacity(
        position: f64,
        color: ColorFormat,
        opacity: f64,
    ) -> Result<Self, PptxError> {
        if !(0.0..=1.0).contains(&opacity) {
            return Err(PptxError::InvalidValue {
                field: "GradientStop.opacity",
                value: opacity.to_string(),
                expected: "0.0 to 1.0",
            });
        }
        let mut stop = Self::new(position, color)?;
        stop.opacity = opacity;
        Ok(stop)
    }
}

/// A pattern fill.
#[derive(Debug, Clone, PartialEq)]
pub struct PatternFill {
    /// The pattern preset type (e.g. Cross, Percent50).
    pub preset: Option<MsoPatternType>,
    /// The foreground color of the pattern.
    pub fore_color: Option<ColorFormat>,
    /// The background color of the pattern.
    pub back_color: Option<ColorFormat>,
}

/// A picture (bitmap image) fill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureFill {
    /// The relationship ID referencing the image part.
    pub image_r_id: RelationshipId,
    /// Optional crop rectangle from `<a:srcRect>`, in OOXML percentage units.
    pub source_rect: Option<PictureSourceRect>,
    /// Destination rectangle inside `<a:stretch>`. Signed offsets are legal;
    /// negative values expand the image beyond the shape and are clipped by
    /// the shape geometry (PowerPoint commonly uses this for picture-filled
    /// circles).
    pub fill_rect: Option<PictureSourceRect>,
    /// Whether to stretch the image to fill the shape.
    pub stretch: bool,
    /// Whether to tile the image instead of stretching.
    /// When true, emits `<a:tile/>` instead of `<a:stretch>`.
    pub tile: bool,
    /// Resolved image bytes populated by `Presentation::slide_shape_tree`.
    /// The relationship and original XML remain authoritative. Repeated OPC
    /// references share the same allocation through `Arc`.
    pub image_data: Option<Arc<[u8]>>,
    /// MIME type for `image_data`, resolved from the related OPC part.
    pub image_content_type: Option<String>,
}

/// Crop offsets used by a DrawingML picture fill.
///
/// Values are the raw OOXML integer percentages (100_000 is 100%). Keeping
/// them raw avoids rounding while projecting an imported shape to HTML.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PictureSourceRect {
    pub left: i64,
    pub top: i64,
    pub right: i64,
    pub bottom: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gradient_stops() {
        let gf = GradientFill {
            stops: vec![
                GradientStop {
                    position: 0.0,
                    color: ColorFormat::rgb(255, 0, 0),
                    opacity: 1.0,
                },
                GradientStop {
                    position: 0.5,
                    color: ColorFormat::rgb(0, 255, 0),
                    opacity: 1.0,
                },
                GradientStop {
                    position: 1.0,
                    color: ColorFormat::rgb(0, 0, 255),
                    opacity: 1.0,
                },
            ],
            angle: Some(45.0),
        };
        assert_eq!(gf.stops.len(), 3);
        assert_eq!(gf.stops[1].position, 0.5);
    }

    #[test]
    fn test_picture_fill_tile_field() {
        // EXCEPTION(unwrap): test-only code with known-valid input
        let pf = PictureFill {
            image_r_id: RelationshipId::try_from("rId3").unwrap(),
            source_rect: None,
            fill_rect: None,
            stretch: false,
            tile: true,
            image_data: None,
            image_content_type: None,
        };
        assert!(pf.tile);
        assert!(!pf.stretch);
    }

    #[test]
    fn test_gradient_stop_new_valid() {
        let stop = GradientStop::new(0.5, ColorFormat::rgb(255, 0, 0)).unwrap(); // EXCEPTION(test-only)
        assert_eq!(stop.position, 0.5);
    }

    #[test]
    fn test_gradient_stop_new_boundary_values() {
        assert!(GradientStop::new(0.0, ColorFormat::rgb(0, 0, 0)).is_ok());
        assert!(GradientStop::new(1.0, ColorFormat::rgb(0, 0, 0)).is_ok());
    }

    #[test]
    fn test_gradient_stop_new_out_of_range() {
        assert!(GradientStop::new(-0.1, ColorFormat::rgb(0, 0, 0)).is_err());
        assert!(GradientStop::new(1.1, ColorFormat::rgb(0, 0, 0)).is_err());
    }
}
