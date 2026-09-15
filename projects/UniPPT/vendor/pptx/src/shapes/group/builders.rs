use crate::enums::shapes::PresetGeometry;
use crate::shapes::autoshape::AutoShape;
use crate::shapes::connector::Connector;
use crate::shapes::graphfrm::{self, GraphicFrame};
use crate::shapes::picture::Picture;
use crate::shapes::Shape;
use crate::units::Emu;

use super::GroupShape;

impl GroupShape {
    /// Add a textbox shape to this group.
    ///
    /// Returns a mutable reference to the newly added `Shape`.
    pub fn add_textbox(&mut self, left: Emu, top: Emu, width: Emu, height: Emu) -> &mut Shape {
        let shape_id = self.next_shape_id();
        let count = self.count_shapes_with_prefix("TextBox") + 1;
        let name = format!("TextBox {count}");

        let shape = Shape::AutoShape(Box::new(AutoShape {
            shape_id,
            name,
            left,
            top,
            width,
            height,
            rotation: 0.0,
            prst_geom: Some(PresetGeometry::Rect),
            is_textbox: true,
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
        }));

        self.shapes.push(shape);
        let idx = self.shapes.len() - 1;
        &mut self.shapes[idx]
    }

    /// Add an autoshape with preset geometry to this group.
    ///
    /// `shape_type` is the OOXML preset geometry name (e.g. "rect", "ellipse", "roundRect").
    /// Returns a mutable reference to the newly added `Shape`.
    pub fn add_autoshape(
        &mut self,
        shape_type: &str,
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
    ) -> &mut Shape {
        let shape_id = self.next_shape_id();
        let base_name = shape_name_for_prst(shape_type);
        let count = self.count_shapes_with_prefix(base_name) + 1;
        let name = format!("{base_name} {count}");

        let shape = Shape::AutoShape(Box::new(AutoShape {
            shape_id,
            name,
            left,
            top,
            width,
            height,
            rotation: 0.0,
            prst_geom: Some(PresetGeometry::from_xml_str(shape_type)),
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
        }));

        self.shapes.push(shape);
        let idx = self.shapes.len() - 1;
        &mut self.shapes[idx]
    }

    /// Add a picture shape to this group.
    ///
    /// `image_r_id` is the relationship ID referencing the image part.
    /// Returns a mutable reference to the newly added `Shape`.
    pub fn add_picture(
        &mut self,
        image_r_id: &str,
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
    ) -> &mut Shape {
        let shape_id = self.next_shape_id();
        let count = self.count_shapes_with_prefix("Picture") + 1;
        let name = format!("Picture {count}");

        let shape = Shape::Picture(Box::new(Picture {
            shape_id,
            name,
            left,
            top,
            width,
            height,
            rotation: 0.0,
            flip_h: false,
            flip_v: false,
            image_r_id: Some(image_r_id.to_string()),
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
        }));

        self.shapes.push(shape);
        let idx = self.shapes.len() - 1;
        &mut self.shapes[idx]
    }

    /// Add a connector shape to this group.
    ///
    /// `connector_type` is the OOXML preset geometry name (e.g. "line", "bentConnector3", "curvedConnector3").
    /// The bounding box and flip flags are computed from the begin/end coordinates.
    /// Returns a mutable reference to the newly added `Shape`.
    pub fn add_connector(
        &mut self,
        connector_type: &str,
        begin_x: Emu,
        begin_y: Emu,
        end_x: Emu,
        end_y: Emu,
    ) -> &mut Shape {
        let shape_id = self.next_shape_id();
        let count = self.count_shapes_with_prefix("Connector") + 1;
        let name = format!("Connector {count}");

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

        let shape = Shape::Connector(Connector {
            shape_id,
            name,
            left,
            top,
            width,
            height,
            rotation: 0.0,
            flip_h,
            flip_v,
            prst_geom: Some(PresetGeometry::from_xml_str(connector_type)),
            line: None,
            begin_shape_id: None,
            begin_cxn_idx: None,
            end_shape_id: None,
            end_cxn_idx: None,
        });

        self.shapes.push(shape);
        let idx = self.shapes.len() - 1;
        &mut self.shapes[idx]
    }

    /// Add a table (as a `GraphicFrame`) to this group.
    ///
    /// Creates a `GraphicFrame` with table metadata. The actual table data
    /// is not embedded in the `GraphicFrame` struct; use the returned shape
    /// to further configure as needed.
    /// Returns a mutable reference to the newly added `Shape`.
    pub fn add_table(
        &mut self,
        rows: usize,
        _cols: usize,
        left: Emu,
        top: Emu,
        width: Emu,
        row_height: Emu,
    ) -> &mut Shape {
        let shape_id = self.next_shape_id();
        let count = self.count_shapes_with_prefix("Table") + 1;
        let name = format!("Table {count}");

        // usize→i64: row count is always small
        let height = Emu(row_height.0 * i64::try_from(rows).unwrap_or(i64::MAX));

        let shape = Shape::GraphicFrame(Box::new(GraphicFrame {
            shape_id,
            name,
            left,
            top,
            width,
            height,
            rotation: 0.0,
            has_table: true,
            has_chart: false,
            graphic_data_uri: Some(graphfrm::graphic_data_uri::TABLE.to_string()),
            placeholder: None,
            smartart_r_id: None,
        }));

        self.shapes.push(shape);
        let idx = self.shapes.len() - 1;
        &mut self.shapes[idx]
    }

    /// Add an empty nested group shape to this group.
    ///
    /// The new group shape has zero dimensions by default. Configure the
    /// returned shape's dimensions as needed.
    /// Returns a mutable reference to the newly added `Shape`.
    pub fn add_group_shape(&mut self) -> &mut Shape {
        let shape_id = self.next_shape_id();
        let count = self.count_shapes_with_prefix("Group") + 1;
        let name = format!("Group {count}");

        let shape = Shape::GroupShape(Box::new(Self {
            shape_id,
            name,
            left: Emu(0),
            top: Emu(0),
            width: Emu(0),
            height: Emu(0),
            rotation: 0.0,
            child_left: Emu(0),
            child_top: Emu(0),
            child_width: Emu(0),
            child_height: Emu(0),
            fill: None,
            line: None,
            shapes: Vec::new(),
        }));

        self.shapes.push(shape);
        let idx = self.shapes.len() - 1;
        &mut self.shapes[idx]
    }
}

/// Map a preset geometry name to a human-readable base name for auto-naming.
fn shape_name_for_prst(prst: &str) -> &'static str {
    match prst {
        "rect" => "Rectangle",
        "roundRect" => "Rounded Rectangle",
        "ellipse" => "Oval",
        "diamond" => "Diamond",
        "triangle" => "Isosceles Triangle",
        "rtTriangle" => "Right Triangle",
        "parallelogram" => "Parallelogram",
        "trapezoid" => "Trapezoid",
        "pentagon" => "Regular Pentagon",
        "hexagon" => "Hexagon",
        "octagon" => "Octagon",
        "star4" => "4-Point Star",
        "star5" => "5-Point Star",
        "star6" => "6-Point Star",
        "cloud" => "Cloud",
        "heart" => "Heart",
        "sun" => "Sun",
        "moon" => "Moon",
        "arc" => "Arc",
        "donut" => "Donut",
        "plus" => "Cross",
        "can" => "Can",
        "cube" => "Cube",
        "frame" => "Frame",
        "leftArrow" => "Left Arrow",
        "rightArrow" => "Right Arrow",
        "upArrow" => "Up Arrow",
        "downArrow" => "Down Arrow",
        _ => "Freeform",
    }
}
