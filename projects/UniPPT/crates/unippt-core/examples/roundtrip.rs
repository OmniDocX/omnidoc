use std::path::PathBuf;

use pptx::{PptxValidator, Presentation, Severity};
use unippt_core::{export_pptx, import_pptx};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let source = PathBuf::from(
        arguments
            .next()
            .ok_or("usage: roundtrip SOURCE.pptx OUTPUT.pptx")?,
    );
    let output = PathBuf::from(
        arguments
            .next()
            .ok_or("usage: roundtrip SOURCE.pptx OUTPUT.pptx")?,
    );
    if arguments.next().is_some() {
        return Err("usage: roundtrip SOURCE.pptx OUTPUT.pptx".into());
    }

    let source_bytes = std::fs::read(&source)?;
    let source_presentation = Presentation::from_bytes(&source_bytes)?;
    let source_severe = severe_issue_count(&source_presentation);
    let deck = import_pptx(&source_bytes)?;
    let output_bytes = export_pptx(&deck, Some(&source_bytes))?;
    let output_presentation = Presentation::from_bytes(&output_bytes)?;
    let output_severe = severe_issue_count(&output_presentation);
    if output_severe > source_severe {
        return Err(format!(
            "export introduced severe validation issues: source={source_severe}, output={output_severe}"
        )
        .into());
    }
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&output, &output_bytes)?;
    println!(
        "{} slides, {} bytes -> {}",
        deck.slides.len(),
        output_bytes.len(),
        output.display()
    );
    Ok(())
}

fn severe_issue_count(presentation: &Presentation) -> usize {
    PptxValidator::validate(presentation)
        .iter()
        .filter(|issue| matches!(issue.severity, Severity::Critical | Severity::High))
        .count()
}
