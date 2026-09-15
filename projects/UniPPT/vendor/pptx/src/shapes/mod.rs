pub mod action;
pub mod autoshape;
pub mod connector;
pub mod freeform;
pub mod graphfrm;
pub mod group;
pub mod ole;
pub mod parser;
pub mod picture;
pub mod placeholder;
mod shape_accessors;
pub mod shapetree;

pub use action::{ActionSetting, Hyperlink};
pub use autoshape::AutoShape;
pub use connector::Connector;
pub use freeform::FreeformBuilder;
pub use graphfrm::GraphicFrame;
pub use group::GroupShape;
pub use ole::OleObject;
pub use picture::Picture;
pub use placeholder::PlaceholderFormat;
pub use shapetree::ShapeTree;

use std::fmt;

use crate::units::{Emu, ShapeId};
use crate::xml_util::WriteXml;

/// Common geometric and identity properties shared by all shape types.
pub trait ShapeProperties {
    /// Returns the unique shape identifier.
    fn shape_id(&self) -> ShapeId;
    /// Returns the shape name.
    fn name(&self) -> &str;
    /// Returns the left position in EMU (English Metric Units).
    fn left(&self) -> Emu;
    /// Returns the top position in EMU (English Metric Units).
    fn top(&self) -> Emu;
    /// Returns the width in EMU (English Metric Units).
    fn width(&self) -> Emu;
    /// Returns the height in EMU (English Metric Units).
    fn height(&self) -> Emu;
    /// Returns the rotation angle in degrees.
    fn rotation(&self) -> f64;
}

/// Implement `ShapeProperties` for a struct whose fields are named identically.
macro_rules! impl_shape_properties {
    ($ty:ty) => {
        impl ShapeProperties for $ty {
            #[inline]
            fn shape_id(&self) -> ShapeId {
                self.shape_id
            }
            #[inline]
            fn name(&self) -> &str {
                &self.name
            }
            #[inline]
            fn left(&self) -> Emu {
                self.left
            }
            #[inline]
            fn top(&self) -> Emu {
                self.top
            }
            #[inline]
            fn width(&self) -> Emu {
                self.width
            }
            #[inline]
            fn height(&self) -> Emu {
                self.height
            }
            #[inline]
            fn rotation(&self) -> f64 {
                self.rotation
            }
        }
    };
}

impl_shape_properties!(AutoShape);
impl_shape_properties!(Picture);
impl_shape_properties!(GraphicFrame);
impl_shape_properties!(GroupShape);
impl_shape_properties!(Connector);
impl_shape_properties!(OleObject);

/// A shape on a slide.
///
/// This enum wraps the different kinds of shape that can appear on a slide.
// Newer clippy toolchains flag the size gap between `Connector` and the boxed
// variants.  Boxing the inline variants would ripple through every match site,
// so the fork allows the lint instead of changing the upstream API surface.
#[allow(clippy::large_enum_variant)]
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    AutoShape(Box<AutoShape>),
    Picture(Box<Picture>),
    GraphicFrame(Box<GraphicFrame>),
    GroupShape(Box<GroupShape>),
    Connector(Connector),
    OleObject(OleObject),
}

/// Dispatch a method call to the inner shape type for all variants.
macro_rules! dispatch_shape {
    ($self:expr, $method:ident) => {
        match $self {
            Shape::AutoShape(s) => s.$method(),
            Shape::Picture(s) => s.$method(),
            Shape::GraphicFrame(s) => s.$method(),
            Shape::GroupShape(s) => s.$method(),
            Shape::Connector(s) => s.$method(),
            Shape::OleObject(s) => s.$method(),
        }
    };
}

impl Shape {
    /// Returns the unique shape identifier.
    #[inline]
    #[must_use]
    pub fn shape_id(&self) -> ShapeId {
        dispatch_shape!(self, shape_id)
    }
    /// Returns the shape name.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        dispatch_shape!(self, name)
    }
    /// Returns the left position in EMU (English Metric Units).
    #[inline]
    pub fn left(&self) -> Emu {
        dispatch_shape!(self, left)
    }
    /// Returns the top position in EMU (English Metric Units).
    #[inline]
    pub fn top(&self) -> Emu {
        dispatch_shape!(self, top)
    }
    /// Returns the width in EMU (English Metric Units).
    #[inline]
    pub fn width(&self) -> Emu {
        dispatch_shape!(self, width)
    }
    /// Returns the height in EMU (English Metric Units).
    #[inline]
    pub fn height(&self) -> Emu {
        dispatch_shape!(self, height)
    }
    /// Returns the rotation angle in degrees.
    #[inline]
    #[must_use]
    pub fn rotation(&self) -> f64 {
        dispatch_shape!(self, rotation)
    }

    /// Returns `true` if this shape contains a text frame.
    #[inline]
    #[must_use]
    pub const fn has_text_frame(&self) -> bool {
        match self {
            Self::AutoShape(a) => a.has_text_frame(),
            _ => false,
        }
    }

    /// Returns `true` if this shape contains a table.
    #[inline]
    #[must_use]
    pub fn has_table(&self) -> bool {
        matches!(self, Self::GraphicFrame(g) if g.has_table)
    }

    /// Returns `true` if this shape is a placeholder.
    #[inline]
    #[must_use]
    pub const fn is_placeholder(&self) -> bool {
        match self {
            Self::AutoShape(s) => s.placeholder.is_some(),
            Self::Picture(s) => s.placeholder.is_some(),
            Self::GraphicFrame(s) => s.placeholder.is_some(),
            Self::GroupShape(_) | Self::Connector(_) | Self::OleObject(_) => false,
        }
    }

    /// Get the placeholder format for this shape, if it is a placeholder.
    #[inline]
    #[must_use]
    pub const fn placeholder(&self) -> Option<&PlaceholderFormat> {
        match self {
            Self::AutoShape(s) => s.placeholder.as_ref(),
            Self::Picture(s) => s.placeholder.as_ref(),
            Self::GraphicFrame(s) => s.placeholder.as_ref(),
            Self::GroupShape(_) | Self::Connector(_) | Self::OleObject(_) => None,
        }
    }
}

impl WriteXml for Shape {
    fn write_xml<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        match self {
            Self::AutoShape(s) => s.write_xml(w),
            Self::Picture(s) => s.write_xml(w),
            Self::GroupShape(s) => s.write_xml(w),
            Self::Connector(s) => s.write_xml(w),
            Self::OleObject(s) => s.write_xml(w),
            Self::GraphicFrame(_) => {
                // GraphicFrame (tables, charts, SmartArt) is serialized through
                // specialized paths in ShapeTree that preserve the original XML.
                // Direct WriteXml serialization is not supported for this shape type.
                Ok(())
            }
        }
    }
}

impl fmt::Display for Shape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self {
            Self::AutoShape(_) => "AutoShape",
            Self::Picture(_) => "Picture",
            Self::GraphicFrame(_) => "GraphicFrame",
            Self::GroupShape(_) => "GroupShape",
            Self::Connector(_) => "Connector",
            Self::OleObject(_) => "OleObject",
        };
        write!(f, "{}(\"{}\")", kind, self.name())
    }
}
