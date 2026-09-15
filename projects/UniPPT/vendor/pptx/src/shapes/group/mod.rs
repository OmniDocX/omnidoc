mod builders;
mod xml;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_builders;

use crate::dml::fill::FillFormat;
use crate::dml::line::LineFormat;
use crate::shapes::Shape;
use crate::units::{Emu, ShapeId};

/// A group shape (`<p:grpSp>`) that contains other shapes.
///
/// Group shapes have their own coordinate space defined by
/// child offset/extent (`chOff`, `chExt`).
#[derive(Debug, Clone, PartialEq)]
pub struct GroupShape {
    pub shape_id: ShapeId,
    pub name: String,
    pub left: Emu,
    pub top: Emu,
    pub width: Emu,
    pub height: Emu,
    pub rotation: f64,
    /// Origin of the child coordinate space (`a:chOff`).
    pub child_left: Emu,
    pub child_top: Emu,
    /// Extent of the child coordinate space (`a:chExt`).
    pub child_width: Emu,
    pub child_height: Emu,
    /// Fill defined by `<p:grpSpPr>`. A child `<a:grpFill/>` inherits it.
    pub fill: Option<FillFormat>,
    /// Line defined by `<p:grpSpPr>`, retained for loss-aware round trips.
    pub line: Option<LineFormat>,
    /// The child shapes within this group.
    pub shapes: Vec<Shape>,
}

impl GroupShape {
    /// Add a shape to this group.
    pub fn add_shape(&mut self, shape: Shape) {
        self.shapes.push(shape);
    }

    /// Get the number of child shapes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.shapes.len()
    }

    /// Check if the group is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.shapes.is_empty()
    }

    /// Iterate over child shapes.
    pub fn iter(&self) -> impl Iterator<Item = &Shape> {
        self.shapes.iter()
    }

    /// Find the maximum shape ID among all child shapes in this group.
    ///
    /// Returns `ShapeId(0)` if the group has no children.
    #[must_use]
    pub fn max_shape_id(&self) -> ShapeId {
        self.shapes
            .iter()
            .map(|s| match s {
                Shape::GroupShape(g) => g.max_shape_id().max(g.shape_id),
                other => other.shape_id(),
            })
            .max()
            .unwrap_or(ShapeId(0))
    }

    /// Return the next available shape ID within this group.
    pub(crate) fn next_shape_id(&self) -> ShapeId {
        let max = self.max_shape_id().max(self.shape_id);
        ShapeId(max.0 + 1)
    }

    /// Count how many existing child shapes have a name starting with the given prefix.
    pub(crate) fn count_shapes_with_prefix(&self, prefix: &str) -> u32 {
        let count = self
            .shapes
            .iter()
            .filter(|s| s.name().starts_with(prefix))
            .count();
        // usize→u32: shape count will never exceed u32::MAX in practice
        u32::try_from(count).unwrap_or(u32::MAX)
    }
}

impl std::fmt::Display for GroupShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GroupShape(\"{}\")", self.name)
    }
}
