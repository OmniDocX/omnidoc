//! Create a minimal PowerPoint presentation with a blank slide.
//!
//! Run with: `cargo run --example create_presentation`

use pptx::Presentation;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut prs = Presentation::new()?;
    let layouts = prs.slide_layouts()?;
    let _slide = prs.add_slide(&layouts[0])?;
    prs.save("example_output.pptx")?;
    println!("Created example_output.pptx with 1 slide");
    Ok(())
}
