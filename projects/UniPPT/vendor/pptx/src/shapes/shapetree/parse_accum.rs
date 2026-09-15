use crate::enums::shapes::PresetGeometry;
use crate::shapes::autoshape::AutoShape;
use crate::shapes::connector::Connector;
use crate::shapes::graphfrm::{self, GraphicFrame};
use crate::shapes::group::GroupShape;
use crate::shapes::picture::Picture;
use crate::shapes::placeholder::PlaceholderFormat;
use crate::shapes::Shape;
use crate::units::{Emu, ShapeId};

#[derive(Debug, Clone, Copy)]
pub(super) enum ShapeKind {
    Sp,
    Pic,
    GraphicFrame,
    CxnSp,
    GrpSp,
}

/// Accumulator for building a shape during parsing.
#[allow(clippy::struct_excessive_bools)]
pub(super) struct ShapeAccum {
    pub(super) kind: ShapeKind,
    pub(super) shape_id: ShapeId,
    pub(super) name: String,
    pub(super) left: i64,
    pub(super) top: i64,
    pub(super) width: i64,
    pub(super) height: i64,
    pub(super) rotation: f64,
    // sp-specific
    pub(super) prst_geom: Option<String>,
    pub(super) is_textbox: bool,
    pub(super) has_tx_body: bool,
    // pic-specific
    pub(super) image_r_id: Option<String>,
    pub(super) description: Option<String>,
    pub(super) crop_left: f64,
    pub(super) crop_top: f64,
    pub(super) crop_right: f64,
    pub(super) crop_bottom: f64,
    pub(super) duotone_xml: Option<Vec<u8>>,
    // graphicFrame-specific
    pub(super) graphic_data_uri: Option<String>,
    pub(super) smartart_r_id: Option<String>,
    // connector-specific
    pub(super) flip_h: bool,
    pub(super) flip_v: bool,
    // placeholder
    pub(super) placeholder: Option<PlaceholderFormat>,
    // Raw captured XML for enhanced parsing
    pub(super) sp_pr_xml: Option<Vec<u8>>,
    pub(super) tx_body_xml_bytes: Option<Vec<u8>>,
}

impl ShapeAccum {
    pub(super) const fn new(kind: ShapeKind) -> Self {
        Self {
            kind,
            shape_id: ShapeId(0),
            name: String::new(),
            left: 0,
            top: 0,
            width: 0,
            height: 0,
            rotation: 0.0,
            prst_geom: None,
            is_textbox: false,
            has_tx_body: false,
            image_r_id: None,
            description: None,
            crop_left: 0.0,
            crop_top: 0.0,
            crop_right: 0.0,
            crop_bottom: 0.0,
            duotone_xml: None,
            graphic_data_uri: None,
            smartart_r_id: None,
            flip_h: false,
            flip_v: false,
            placeholder: None,
            sp_pr_xml: None,
            tx_body_xml_bytes: None,
        }
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn into_shape(self) -> Shape {
        match self.kind {
            ShapeKind::Sp => {
                // Parse fill and line from captured spPr XML
                let (fill, line) = self
                    .sp_pr_xml
                    .as_ref()
                    .and_then(|xml| crate::shapes::parser::parse_sp_pr(xml).ok())
                    .unwrap_or((None, None));
                let shadow = self
                    .sp_pr_xml
                    .as_ref()
                    .and_then(|xml| crate::shapes::parser::parse_shadow_from_xml(xml).ok())
                    .flatten();
                let custom_geometry = self.sp_pr_xml.as_ref().and_then(|xml| {
                    crate::shapes::parser::parse_custom_geometry_from_xml(xml)
                        .ok()
                        .flatten()
                });
                // Parse text frame from captured txBody XML
                let text_frame = self.tx_body_xml_bytes.as_ref().and_then(|xml| {
                    crate::shapes::parser::parse_text_frame_from_xml(xml)
                        .ok()
                        .flatten()
                });
                Shape::AutoShape(Box::new(AutoShape {
                    shape_id: self.shape_id,
                    name: self.name,
                    left: Emu(self.left),
                    top: Emu(self.top),
                    width: Emu(self.width),
                    height: Emu(self.height),
                    rotation: self.rotation,
                    prst_geom: self.prst_geom.map(|s| PresetGeometry::from_xml_str(&s)),
                    is_textbox: self.is_textbox,
                    placeholder: self.placeholder,
                    tx_body_xml: self.tx_body_xml_bytes,
                    fill,
                    line,
                    text_frame,
                    click_action: None,
                    hover_action: None,
                    adjustments: Vec::new(),
                    shadow,
                    custom_geometry,
                    scene_3d: None,
                    shape_3d: None,
                }))
            }
            ShapeKind::Pic => {
                // Parse line from captured spPr XML
                let line = self
                    .sp_pr_xml
                    .as_ref()
                    .and_then(|xml| crate::shapes::parser::parse_sp_pr(xml).ok())
                    .and_then(|(_, line)| line);
                let shadow = self
                    .sp_pr_xml
                    .as_ref()
                    .and_then(|xml| crate::shapes::parser::parse_shadow_from_xml(xml).ok())
                    .flatten();
                let soft_edge_radius = self
                    .sp_pr_xml
                    .as_ref()
                    .and_then(|xml| {
                        crate::shapes::parser::parse_soft_edge_radius_from_xml(xml).ok()
                    })
                    .flatten();
                Shape::Picture(Box::new(Picture {
                    shape_id: self.shape_id,
                    name: self.name,
                    left: Emu(self.left),
                    top: Emu(self.top),
                    width: Emu(self.width),
                    height: Emu(self.height),
                    rotation: self.rotation,
                    flip_h: self.flip_h,
                    flip_v: self.flip_v,
                    image_r_id: self.image_r_id,
                    description: self.description,
                    placeholder: self.placeholder,
                    crop_left: self.crop_left,
                    crop_right: self.crop_right,
                    crop_top: self.crop_top,
                    crop_bottom: self.crop_bottom,
                    duotone: self.duotone_xml.as_deref().and_then(|xml| {
                        crate::shapes::parser::parse_duotone_from_xml(xml)
                            .ok()
                            .flatten()
                    }),
                    line,
                    click_action: None,
                    hover_action: None,
                    shadow,
                    soft_edge_radius,
                    auto_shape_type: None,
                    image_data: None,
                    image_content_type: None,
                    scene_3d: None,
                    shape_3d: None,
                }))
            }
            ShapeKind::GraphicFrame => {
                let has_table =
                    self.graphic_data_uri.as_deref() == Some(graphfrm::graphic_data_uri::TABLE);
                let has_chart =
                    self.graphic_data_uri.as_deref() == Some(graphfrm::graphic_data_uri::CHART);
                Shape::GraphicFrame(Box::new(GraphicFrame {
                    shape_id: self.shape_id,
                    name: self.name,
                    left: Emu(self.left),
                    top: Emu(self.top),
                    width: Emu(self.width),
                    height: Emu(self.height),
                    rotation: self.rotation,
                    has_table,
                    has_chart,
                    graphic_data_uri: self.graphic_data_uri,
                    placeholder: self.placeholder,
                    smartart_r_id: self.smartart_r_id,
                }))
            }
            ShapeKind::CxnSp => {
                // Parse line from captured spPr XML
                let line = self
                    .sp_pr_xml
                    .as_ref()
                    .and_then(|xml| crate::shapes::parser::parse_sp_pr(xml).ok())
                    .and_then(|(_, line)| line);
                Shape::Connector(Connector {
                    shape_id: self.shape_id,
                    name: self.name,
                    left: Emu(self.left),
                    top: Emu(self.top),
                    width: Emu(self.width),
                    height: Emu(self.height),
                    rotation: self.rotation,
                    flip_h: self.flip_h,
                    flip_v: self.flip_v,
                    prst_geom: self.prst_geom.map(|s| PresetGeometry::from_xml_str(&s)),
                    line,
                    begin_shape_id: None,
                    begin_cxn_idx: None,
                    end_shape_id: None,
                    end_cxn_idx: None,
                })
            }
            ShapeKind::GrpSp => Shape::GroupShape(Box::new(GroupShape {
                shape_id: self.shape_id,
                name: self.name,
                left: Emu(self.left),
                top: Emu(self.top),
                width: Emu(self.width),
                height: Emu(self.height),
                rotation: self.rotation,
                child_left: Emu(self.left),
                child_top: Emu(self.top),
                child_width: Emu(self.width),
                child_height: Emu(self.height),
                fill: None,
                line: None,
                shapes: Vec::new(),
            })),
        }
    }
}
