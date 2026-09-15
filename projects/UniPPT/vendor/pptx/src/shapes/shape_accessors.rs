use super::{AutoShape, Connector, GraphicFrame, GroupShape, OleObject, Picture, Shape};

impl Shape {
    /// Get a reference to the inner `AutoShape`, if this is one.
    #[inline]
    #[must_use]
    pub const fn as_autoshape(&self) -> Option<&AutoShape> {
        match self {
            Self::AutoShape(s) => Some(s),
            _ => None,
        }
    }

    /// Get a mutable reference to the inner `AutoShape`, if this is one.
    #[inline]
    #[must_use]
    pub fn as_autoshape_mut(&mut self) -> Option<&mut AutoShape> {
        match self {
            Self::AutoShape(s) => Some(s),
            _ => None,
        }
    }

    /// Get a reference to the inner `Picture`, if this is one.
    #[inline]
    #[must_use]
    pub const fn as_picture(&self) -> Option<&Picture> {
        match self {
            Self::Picture(s) => Some(s),
            _ => None,
        }
    }

    /// Get a mutable reference to the inner `Picture`, if this is one.
    #[inline]
    #[must_use]
    pub fn as_picture_mut(&mut self) -> Option<&mut Picture> {
        match self {
            Self::Picture(s) => Some(s),
            _ => None,
        }
    }

    /// Get a reference to the inner `GraphicFrame`, if this is one.
    #[inline]
    #[must_use]
    pub const fn as_graphic_frame(&self) -> Option<&GraphicFrame> {
        match self {
            Self::GraphicFrame(s) => Some(s),
            _ => None,
        }
    }

    /// Get a reference to the inner `GroupShape`, if this is one.
    #[inline]
    #[must_use]
    pub const fn as_group(&self) -> Option<&GroupShape> {
        match self {
            Self::GroupShape(s) => Some(s),
            _ => None,
        }
    }

    /// Get a mutable reference to the inner `GroupShape`, if this is one.
    #[inline]
    #[must_use]
    pub fn as_group_mut(&mut self) -> Option<&mut GroupShape> {
        match self {
            Self::GroupShape(s) => Some(s),
            _ => None,
        }
    }

    /// Get a reference to the inner `Connector`, if this is one.
    #[inline]
    #[must_use]
    pub const fn as_connector(&self) -> Option<&Connector> {
        match self {
            Self::Connector(s) => Some(s),
            _ => None,
        }
    }

    /// Get a mutable reference to the inner `Connector`, if this is one.
    #[inline]
    #[must_use]
    pub fn as_connector_mut(&mut self) -> Option<&mut Connector> {
        match self {
            Self::Connector(s) => Some(s),
            _ => None,
        }
    }

    /// Get a reference to the inner `OleObject`, if this is one.
    #[inline]
    #[must_use]
    pub const fn as_ole_object(&self) -> Option<&OleObject> {
        match self {
            Self::OleObject(s) => Some(s),
            _ => None,
        }
    }

    /// Get a mutable reference to the inner `OleObject`, if this is one.
    #[inline]
    #[must_use]
    pub fn as_ole_object_mut(&mut self) -> Option<&mut OleObject> {
        match self {
            Self::OleObject(s) => Some(s),
            _ => None,
        }
    }
}
