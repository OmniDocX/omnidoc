//! `DrawingML` formatting types: color, fill, line, and effects.

pub mod color;
pub mod effect;
pub mod effect3d;
pub mod fill;
pub mod line;

pub use color::{ColorFormat, HslColor, PresetColor, SystemColor, ThemeColor};
pub use effect::{DuotoneColor, DuotoneColorTransform, DuotoneEffect, ShadowFormat};
pub use effect3d::{Bevel, Camera, LightRig, Rotation3D, Scene3D, Shape3D};
pub use fill::{
    FillFormat, GradientFill, GradientStop, PatternFill, PictureFill, PictureSourceRect, SolidFill,
};
pub use line::{LineCap, LineFormat, LineJoin};
