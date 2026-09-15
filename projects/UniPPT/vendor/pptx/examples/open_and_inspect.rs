//! Create a presentation, then re-open it and inspect shapes.
//!
//! Run with: `cargo run --example open_and_inspect`

use pptx::{Presentation, ShapeTree};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // First, create a file to inspect
    let mut prs = Presentation::new()?;
    let layouts = prs.slide_layouts()?;
    let _slide = prs.add_slide(&layouts[0])?;
    let bytes = prs.to_bytes()?;

    // Now re-open and inspect
    let prs = Presentation::from_bytes(&bytes)?;
    for slide_ref in prs.slides()? {
        let xml = prs.slide_xml(&slide_ref)?;
        let tree = ShapeTree::from_slide_xml(xml)?;
        println!("Slide has {} shape(s)", tree.shapes.len());
        for shape in &tree.shapes {
            println!("  - {}: {}x{}", shape.name(), shape.width(), shape.height());
        }
    }
    Ok(())
}
