//! `DrawingML` fill types.

mod types;
mod xml;

pub use types::{
    GradientFill, GradientStop, PatternFill, PictureFill, PictureSourceRect, SolidFill,
};

use crate::dml::color::ColorFormat;
use crate::enums::dml::MsoFillType;
use crate::units::RelationshipId;

/// The fill formatting for a shape or other element.
///
/// Corresponds to the `EG_FillProperties` choice group in the OOXML schema,
/// which allows `<a:noFill>`, `<a:solidFill>`, `<a:gradFill>`, `<a:pattFill>`,
/// or `<a:blipFill>`.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum FillFormat {
    /// No fill (transparent).  Corresponds to `<a:noFill/>`.
    NoFill,
    /// A solid color fill.  Corresponds to `<a:solidFill>`.
    Solid(SolidFill),
    /// A gradient fill.  Corresponds to `<a:gradFill>`.
    Gradient(GradientFill),
    /// A pattern fill.  Corresponds to `<a:pattFill>`.
    Pattern(PatternFill),
    /// A picture (bitmap) fill.  Corresponds to `<a:blipFill>`.
    Picture(PictureFill),
    /// Background fill (inherits from slide background).  Corresponds to `<a:grpFill/>`.
    Background,
}

impl FillFormat {
    /// Create a solid fill from a color.
    #[must_use]
    pub const fn solid(color: ColorFormat) -> Self {
        Self::Solid(SolidFill {
            color,
            opacity: 1.0,
        })
    }

    /// Create a solid fill with explicit normalized DrawingML alpha.
    #[must_use]
    pub fn solid_with_opacity(color: ColorFormat, opacity: f64) -> Self {
        Self::Solid(SolidFill {
            color,
            opacity: opacity.clamp(0.0, 1.0),
        })
    }

    /// Create a no-fill (transparent).
    #[must_use]
    pub const fn no_fill() -> Self {
        Self::NoFill
    }

    /// Create a picture fill from a relationship ID.
    ///
    /// # Errors
    ///
    /// Returns an error if `image_r_id` is not a valid relationship ID (must match `rId<digits>`).
    pub fn picture(image_r_id: &str) -> Result<Self, crate::error::PptxError> {
        Ok(Self::Picture(PictureFill {
            image_r_id: RelationshipId::try_from(image_r_id)?,
            source_rect: None,
            fill_rect: None,
            stretch: true,
            tile: false,
            image_data: None,
            image_content_type: None,
        }))
    }

    /// Create a tiled picture fill from a relationship ID.
    ///
    /// # Errors
    ///
    /// Returns an error if `image_r_id` is not a valid relationship ID (must match `rId<digits>`).
    pub fn picture_tiled(image_r_id: &str) -> Result<Self, crate::error::PptxError> {
        Ok(Self::Picture(PictureFill {
            image_r_id: RelationshipId::try_from(image_r_id)?,
            source_rect: None,
            fill_rect: None,
            stretch: false,
            tile: true,
            image_data: None,
            image_content_type: None,
        }))
    }

    /// Create a background fill (inherits from slide background).
    #[must_use]
    pub const fn background() -> Self {
        Self::Background
    }

    /// Return the fill type classification for this fill.
    #[must_use]
    pub const fn fill_type(&self) -> MsoFillType {
        match self {
            Self::NoFill => MsoFillType::Background,
            Self::Solid(_) => MsoFillType::Solid,
            Self::Gradient(_) => MsoFillType::Gradient,
            Self::Pattern(_) => MsoFillType::Patterned,
            Self::Picture(_) => MsoFillType::Picture,
            Self::Background => MsoFillType::Group,
        }
    }

    /// Create a simple linear gradient between two colors.
    #[must_use]
    pub fn linear_gradient(start_color: ColorFormat, end_color: ColorFormat, angle: f64) -> Self {
        Self::Gradient(GradientFill {
            stops: vec![
                GradientStop {
                    position: 0.0,
                    color: start_color,
                    opacity: 1.0,
                },
                GradientStop {
                    position: 1.0,
                    color: end_color,
                    opacity: 1.0,
                },
            ],
            angle: Some(angle),
        })
    }
}
