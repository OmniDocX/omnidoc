mod parse;
mod parse_accum;
mod patch;
mod xml_capture;
mod xml_gen;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_add_api;
#[cfg(test)]
mod tests_enhanced;

use crate::enums::shapes::MsoAutoShapeType;
use crate::enums::shapes::MsoConnectorType;
use crate::error::{PptxError, PptxResult};
use crate::shapes::Shape;
use crate::units::{Emu, ShapeId};

pub(crate) use xml_gen::shape_name_for_prst;

/// A collection of shapes parsed from a slide's `<p:spTree>` element.
#[derive(Debug, Clone, Default)]
pub struct ShapeTree {
    pub shapes: Vec<Shape>,
    /// When enabled, `insert_shape_xml()` skips re-parsing the full XML tree
    /// and directly appends before the closing `</p:spTree>` tag. This is a
    /// performance optimization for bulk shape additions.
    turbo_add_enabled: bool,
}

impl ShapeTree {
    /// Parse shapes from slide XML (the full `<p:sld>` or similar element).
    ///
    /// Extracts all shape elements from the `<p:spTree>` within `<p:cSld>`.
    ///
    /// # Errors
    ///
    /// Returns `PptxError` if the XML cannot be parsed.
    pub fn from_slide_xml(xml: &[u8]) -> PptxResult<Self> {
        let shapes = parse::parse_shapes_from_slide_xml(xml)?;
        Ok(Self {
            shapes,
            turbo_add_enabled: false,
        })
    }

    /// Iterate over all shapes.
    pub fn iter(&self) -> impl Iterator<Item = &Shape> {
        self.shapes.iter()
    }

    /// Number of shapes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.shapes.len()
    }

    /// Whether the shape tree is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.shapes.is_empty()
    }

    /// Get whether turbo add mode is enabled.
    #[must_use]
    pub const fn turbo_add_enabled(&self) -> bool {
        self.turbo_add_enabled
    }

    /// Set whether turbo add mode is enabled.
    ///
    /// When enabled, `insert_shape_xml()` skips validation/re-parsing and
    /// directly appends before the closing `</p:spTree>` tag.
    pub fn set_turbo_add_enabled(&mut self, enabled: bool) {
        self.turbo_add_enabled = enabled;
    }

    /// Find the maximum shape ID currently in use.
    #[must_use]
    pub fn max_shape_id(&self) -> ShapeId {
        self.shapes
            .iter()
            .map(super::Shape::shape_id)
            .max()
            .unwrap_or(ShapeId(0))
    }

    /// Count how many existing shapes have a name starting with the given prefix.
    ///
    /// Used to generate auto-incremented shape names like "Rectangle 3".
    fn count_shapes_with_prefix(&self, prefix: &str) -> u32 {
        let count = self
            .shapes
            .iter()
            .filter(|s| s.name().starts_with(prefix))
            .count();
        // usize→u32: shape count will never exceed u32::MAX in practice
        u32::try_from(count).unwrap_or(u32::MAX)
    }

    /// Add an auto shape with preset geometry to the slide XML.
    ///
    /// Auto-assigns `shape_id` and name. Returns the updated slide XML bytes.
    ///
    /// # Errors
    ///
    /// Returns `PptxError` if the slide XML cannot be parsed or modified.
    pub fn add_shape(
        slide_xml: &[u8],
        auto_shape_type: MsoAutoShapeType,
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
    ) -> PptxResult<Vec<u8>> {
        let tree = Self::from_slide_xml(slide_xml)?;
        let shape_id = ShapeId(tree.max_shape_id().0 + 1);
        let prst = auto_shape_type.to_xml_str();
        let base_name = shape_name_for_prst(prst);
        let count = tree.count_shapes_with_prefix(base_name) + 1;
        let name = format!("{base_name} {count}");

        let xml = Self::new_autoshape_xml(shape_id, &name, left, top, width, height, prst);
        Self::insert_shape_xml(slide_xml, &xml)
    }

    /// Add a textbox shape to the slide XML.
    ///
    /// Auto-assigns `shape_id` and name. Returns the updated slide XML bytes.
    ///
    /// # Errors
    ///
    /// Returns `PptxError` if the slide XML cannot be parsed or modified.
    pub fn add_textbox(
        slide_xml: &[u8],
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
    ) -> PptxResult<Vec<u8>> {
        let tree = Self::from_slide_xml(slide_xml)?;
        let shape_id = ShapeId(tree.max_shape_id().0 + 1);
        let count = tree.count_shapes_with_prefix("TextBox") + 1;
        let name = format!("TextBox {count}");

        let xml = Self::new_textbox_xml(shape_id, &name, left, top, width, height);
        Self::insert_shape_xml(slide_xml, &xml)
    }

    /// Add a picture shape to the slide XML.
    ///
    /// `image_r_id` is the relationship ID linking the slide to the image part.
    /// Auto-assigns `shape_id` and name. Returns the updated slide XML bytes.
    ///
    /// # Errors
    ///
    /// Returns `PptxError` if the slide XML cannot be parsed or modified.
    pub fn add_picture(
        slide_xml: &[u8],
        image_r_id: &str,
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
    ) -> PptxResult<Vec<u8>> {
        let tree = Self::from_slide_xml(slide_xml)?;
        let shape_id = ShapeId(tree.max_shape_id().0 + 1);
        let count = tree.count_shapes_with_prefix("Picture") + 1;
        let name = format!("Picture {count}");

        let xml = Self::new_picture_xml(shape_id, &name, "", image_r_id, left, top, width, height);
        Self::insert_shape_xml(slide_xml, &xml)
    }

    /// Add a table to the slide XML.
    ///
    /// Creates a `<p:graphicFrame>` containing a table with the given number of rows and columns.
    /// Auto-assigns `shape_id` and name. Returns the updated slide XML bytes.
    ///
    /// # Errors
    ///
    /// Returns `PptxError` if the slide XML cannot be parsed or modified.
    pub fn add_table(
        slide_xml: &[u8],
        rows: u32,
        cols: u32,
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
    ) -> PptxResult<Vec<u8>> {
        let tree = Self::from_slide_xml(slide_xml)?;
        let shape_id = ShapeId(tree.max_shape_id().0 + 1);
        let count = tree.count_shapes_with_prefix("Table") + 1;
        let name = format!("Table {count}");

        let xml = Self::new_table_xml(shape_id, &name, rows, cols, left, top, width, height);
        Self::insert_shape_xml(slide_xml, &xml)
    }

    /// Add a connector shape to the slide XML.
    ///
    /// The connector is placed using begin/end coordinates. The method computes
    /// the bounding box (left, top, width, height) and flip flags from the coordinates.
    /// Auto-assigns `shape_id` and name. Returns the updated slide XML bytes.
    ///
    /// # Errors
    ///
    /// Returns `PptxError` if the slide XML cannot be parsed or modified.
    pub fn add_connector(
        slide_xml: &[u8],
        connector_type: MsoConnectorType,
        begin_x: Emu,
        begin_y: Emu,
        end_x: Emu,
        end_y: Emu,
    ) -> PptxResult<Vec<u8>> {
        let tree = Self::from_slide_xml(slide_xml)?;
        let shape_id = ShapeId(tree.max_shape_id().0 + 1);
        let count = tree.count_shapes_with_prefix("Connector") + 1;
        let name = format!("Connector {count}");
        let prst = connector_type.to_xml_str();

        // Compute bounding box and flip flags from begin/end coordinates
        let (left, width, flip_h) = if begin_x.0 <= end_x.0 {
            (begin_x, Emu(end_x.0 - begin_x.0), false)
        } else {
            (end_x, Emu(begin_x.0 - end_x.0), true)
        };
        let (top, height, flip_v) = if begin_y.0 <= end_y.0 {
            (begin_y, Emu(end_y.0 - begin_y.0), false)
        } else {
            (end_y, Emu(begin_y.0 - end_y.0), true)
        };

        let xml = Self::new_connector_xml_with_flip(
            shape_id, &name, left, top, width, height, prst, flip_h, flip_v,
        );
        Self::insert_shape_xml(slide_xml, &xml)
    }

    /// Add an empty group shape to the slide XML.
    ///
    /// Auto-assigns `shape_id` and name. Returns the updated slide XML bytes.
    ///
    /// # Errors
    ///
    /// Returns `PptxError` if the slide XML cannot be parsed or modified.
    pub fn add_group_shape(
        slide_xml: &[u8],
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
    ) -> PptxResult<Vec<u8>> {
        let tree = Self::from_slide_xml(slide_xml)?;
        let shape_id = ShapeId(tree.max_shape_id().0 + 1);
        let count = tree.count_shapes_with_prefix("Group") + 1;
        let name = format!("Group {count}");

        let xml = Self::new_group_shape_xml(shape_id, &name, left, top, width, height);
        Self::insert_shape_xml(slide_xml, &xml)
    }

    /// Insert a shape XML fragment into an existing slide XML blob.
    ///
    /// Inserts the given shape XML as a child of `<p:spTree>`, just before
    /// the closing `</p:spTree>` tag.
    pub fn insert_shape_xml(slide_xml: &[u8], shape_xml: &str) -> PptxResult<Vec<u8>> {
        let slide_str = String::from_utf8_lossy(slide_xml).to_string();
        slide_str.find("</p:spTree>").map_or_else(
            || {
                Err(PptxError::InvalidXml(
                    "slide XML does not contain </p:spTree>".to_string(),
                ))
            },
            |pos| {
                let mut updated = String::with_capacity(slide_str.len() + shape_xml.len());
                updated.push_str(&slide_str[..pos]);
                updated.push_str(shape_xml);
                updated.push_str(&slide_str[pos..]);
                Ok(updated.into_bytes())
            },
        )
    }

    /// Add a movie (video) shape to the slide XML.
    ///
    /// `video_r_id` is the relationship ID for the external video link.
    /// `poster_r_id` is the relationship ID for the embedded poster frame image.
    /// Auto-assigns `shape_id` and name. Returns the updated slide XML bytes.
    ///
    /// # Errors
    ///
    /// Returns `PptxError` if the slide XML cannot be parsed or modified.
    pub fn add_movie(
        slide_xml: &[u8],
        video_r_id: &str,
        poster_r_id: &str,
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
    ) -> PptxResult<Vec<u8>> {
        let tree = Self::from_slide_xml(slide_xml)?;
        let shape_id = ShapeId(tree.max_shape_id().0 + 1);
        let count = tree.count_shapes_with_prefix("Movie") + 1;
        let name = format!("Movie {count}");

        let xml = Self::new_movie_xml(
            shape_id,
            &name,
            video_r_id,
            poster_r_id,
            left,
            top,
            width,
            height,
        );
        Self::insert_shape_xml(slide_xml, &xml)
    }

    /// Return the first shape that is a title placeholder.
    ///
    /// Looks for placeholders with type "title" or `"ctrTitle"`.
    #[must_use]
    pub fn title(&self) -> Option<&Shape> {
        self.shapes.iter().find(|s| match s {
            Shape::AutoShape(a) => a
                .placeholder
                .as_ref()
                .is_some_and(super::placeholder::PlaceholderFormat::is_title),
            Shape::Picture(p) => p
                .placeholder
                .as_ref()
                .is_some_and(super::placeholder::PlaceholderFormat::is_title),
            Shape::GraphicFrame(g) => g
                .placeholder
                .as_ref()
                .is_some_and(super::placeholder::PlaceholderFormat::is_title),
            _ => false,
        })
    }

    /// Return all shapes that are placeholders.
    #[must_use]
    pub fn placeholders(&self) -> Vec<&Shape> {
        self.shapes.iter().filter(|s| s.is_placeholder()).collect()
    }
}
