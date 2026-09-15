#![forbid(unsafe_code)]
use std::process;

use clap::{Parser, Subcommand};
use pptx::repair::{PptxRepairer, PptxValidator, Severity};
use pptx::shapes::ShapeTree;
use pptx::Presentation;

#[derive(Parser)]
#[command(
    name = "pptx-cli",
    about = "CLI tool for inspecting and manipulating PPTX files"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show presentation metadata (slide count, title, author, created date)
    Info {
        /// Path to the PPTX file
        file: String,
    },
    /// List all slides with their shapes
    Slides {
        /// Path to the PPTX file
        file: String,
    },
    /// Export the presentation to HTML
    ExportHtml {
        /// Path to the PPTX file
        file: String,
        /// Output HTML file path (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Validate the PPTX file and show issues
    Validate {
        /// Path to the PPTX file
        file: String,
    },
    /// Repair the PPTX file and save the result
    Repair {
        /// Path to the PPTX file
        file: String,
        /// Output file path (defaults to `<file>_repaired.pptx`)
        #[arg(short, long)]
        output: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Command::Info { file } => cmd_info(&file),
        Command::Slides { file } => cmd_slides(&file),
        Command::ExportHtml { file, output } => cmd_export_html(&file, output.as_deref()),
        Command::Validate { file } => cmd_validate(&file),
        Command::Repair { file, output } => cmd_repair(&file, output.as_deref()),
    }
}

fn cmd_info(file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let prs = Presentation::open(file)?;
    let slide_count = prs.slide_count()?;
    let props = prs.core_properties()?;

    println!("File: {file}");
    println!("Slides: {slide_count}");

    let title = props.title();
    if !title.is_empty() {
        println!("Title: {title}");
    }
    let author = props.author();
    if !author.is_empty() {
        println!("Author: {author}");
    }
    let subject = props.subject();
    if !subject.is_empty() {
        println!("Subject: {subject}");
    }
    let created = props.created();
    if !created.is_empty() {
        println!("Created: {created}");
    }
    let modified = props.modified();
    if !modified.is_empty() {
        println!("Modified: {modified}");
    }
    let last_modified_by = props.last_modified_by();
    if !last_modified_by.is_empty() {
        println!("Last modified by: {last_modified_by}");
    }

    Ok(())
}

fn cmd_slides(file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let prs = Presentation::open(file)?;
    let slides = prs.slides()?;

    if slides.is_empty() {
        println!("No slides.");
        return Ok(());
    }

    for (i, slide_ref) in slides.iter().enumerate() {
        let name = prs.slide_name(slide_ref)?.unwrap_or_default();
        if name.is_empty() {
            println!("Slide {}:", i + 1);
        } else {
            println!("Slide {} ({name}):", i + 1);
        }

        let xml = prs.slide_xml(slide_ref)?;
        let tree = ShapeTree::from_slide_xml(xml)?;

        if tree.is_empty() {
            println!("  (no shapes)");
        } else {
            for shape in tree.iter() {
                let kind = match shape {
                    pptx::Shape::AutoShape(_) => "AutoShape",
                    pptx::Shape::Picture(_) => "Picture",
                    pptx::Shape::GraphicFrame(_) => "GraphicFrame",
                    pptx::Shape::Connector(_) => "Connector",
                    pptx::Shape::GroupShape(_) => "GroupShape",
                    pptx::Shape::OleObject(_) => "OleObject",
                    _ => "Unknown",
                };
                println!(
                    "  - \"{}\" [{}] pos=({},{}) size={}x{}",
                    shape.name(),
                    kind,
                    shape.left().0,
                    shape.top().0,
                    shape.width().0,
                    shape.height().0,
                );
            }
        }
    }

    Ok(())
}

fn cmd_export_html(file: &str, output: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let prs = Presentation::open(file)?;
    let html = prs.export_html()?;

    if let Some(path) = output {
        std::fs::write(path, &html)?;
        println!("HTML exported to {path}");
    } else {
        print!("{html}");
    }

    Ok(())
}

fn cmd_validate(file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let prs = Presentation::open(file)?;
    let issues = PptxValidator::validate(&prs);

    if issues.is_empty() {
        println!("No issues found. The file is valid.");
        return Ok(());
    }

    println!("Found {} issue(s):", issues.len());
    for issue in &issues {
        let severity = match issue.severity {
            Severity::Critical => "CRITICAL",
            Severity::High => "HIGH",
            Severity::Medium => "MEDIUM",
            Severity::Low => "LOW",
            _ => "UNKNOWN",
        };
        let location = issue.location.as_deref().unwrap_or("-");
        println!(
            "  [{severity}] {desc} (at {location})",
            desc = issue.description
        );
    }

    process::exit(1);
}

fn cmd_repair(file: &str, output: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let mut prs = Presentation::open(file)?;
    let report = PptxRepairer::repair(&mut prs);

    if report.issues_found.is_empty() {
        println!("No issues found. No repair needed.");
        return Ok(());
    }

    println!("Issues found: {}", report.issues_found.len());
    println!("Issues fixed: {}", report.issues_fixed.len());

    for issue in &report.issues_fixed {
        println!("  Fixed: {}", issue.description);
    }

    if report.is_valid {
        println!("Result: valid after repair");
    } else {
        println!("Result: some issues remain after repair");
    }

    let output_path = output.map_or_else(
        || {
            let stem = file.strip_suffix(".pptx").unwrap_or(file);
            format!("{stem}_repaired.pptx")
        },
        str::to_string,
    );

    prs.save(&output_path)?;
    println!("Saved to {output_path}");

    Ok(())
}
