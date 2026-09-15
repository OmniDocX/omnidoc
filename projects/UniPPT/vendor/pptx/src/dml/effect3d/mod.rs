//! `DrawingML` 3D effect types (bevel, extrusion, 3D rotation, scene, lighting).

pub mod bevel;
pub mod rotation;
pub mod scene3d;
pub mod shape3d;

pub use bevel::Bevel;
pub use rotation::Rotation3D;
pub use scene3d::{Camera, LightRig, Scene3D};
pub use shape3d::Shape3D;
