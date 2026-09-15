//! Text formatting: `TextFrame`, `Paragraph`, `Run`, and `Font`.

pub mod bullet;
pub mod font;
pub mod paragraph;
pub mod run;
pub mod text_frame;

pub use bullet::BulletFormat;
pub use font::{Font, RgbColor};
pub use paragraph::Paragraph;
pub use run::Run;
pub use text_frame::TextFrame;
