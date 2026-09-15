# pptx

[![Crates.io](https://img.shields.io/crates/v/pptx.svg)](https://crates.io/crates/pptx)
[![docs.rs](https://docs.rs/pptx/badge.svg)](https://docs.rs/pptx)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

A pure-Rust library for **reading and writing** PowerPoint (.pptx) files. Open existing presentations, inspect every element, modify them, and save — or build new ones from scratch. Only 4 dependencies, no unsafe code.

## Why pptx?

- **Read and write** — Not just generation. Open an existing `.pptx`, parse its shapes, text, tables, and charts, modify them, and round-trip back to a valid file. Most Rust PPTX crates are write-only.
- **Feature-rich** — Animations, 3D effects, SmartArt parsing, freeform shapes, group shapes — things other Rust PPTX crates don't cover.
- **Lightweight** — 4 runtime dependencies (`zip`, `quick-xml`, `thiserror`, `sha1`). No regex, no HTTP client, no syntax highlighter bundled in.
- **Safe** — `#![forbid(unsafe_code)]` at the crate root. All key types are `Send + Sync`.

The API design draws from [python-pptx](https://github.com/scanny/python-pptx), so concepts and naming will feel familiar if you've used it. See [COMPATIBILITY.md](COMPATIBILITY.md) for the detailed feature matrix.

## Features

### Slides & Layout
- Create, delete, reorder slides
- 11 built-in slide layouts with placeholder inheritance
- Slide backgrounds (solid, gradient, image)
- Speaker notes, sections, comments

### Shapes & Text
- 180+ preset geometries (AutoShape)
- Freeform shapes with custom paths (`line_to`, `curve_to`, `close`)
- Group shapes with nesting
- Rich text: bold, italic, color, size, alignment, bullets, strikethrough, sub/superscript
- Right-to-left text direction (`TextDirection` enum)
- Hyperlinks and action settings on shapes and text

### Tables & Charts
- Tables with cell merging, borders, and per-cell formatting
- 20+ chart types (bar, line, pie, scatter, bubble, area, radar, stock, surface, 3D variants)
- Combo charts with dual axes (bar + line via `ComboChartData`)
- Chart axes, legends, data labels, markers
- Embedded Excel data for charts

### Media
- Images with SHA1 deduplication and SVG support
- Image cropping (LTRB crop via `set_crop`, `set_crop_left`, etc.)
- Video and audio embedding

### Effects & Animations
- Shadow effects (outer, inner, perspective)
- 3D effects: Bevel, Camera, LightRig, Scene3D, Shape3D
- Slide transitions (11 types)
- Animations: entrance, exit, emphasis effects with trigger and sequence control

### Export
- PPTX to self-contained HTML (`export_html()`)

### Advanced
- SmartArt reading and node-tree parsing
- Theme color scheme reading and modification
- Core properties (Dublin Core metadata)
- Print/handout settings (`PrintSettings`: color mode, page orientation, handout layout)
- PPTX validation and repair (`PptxValidator`, `PptxRepairer`)
- Digital signature metadata (structural XML; cryptographic signing not included)
- OLE object support
- Custom XML parts
- VBA macro support (`.pptm` save)
- Embedded fonts

## Installation

```toml
[dependencies]
pptx = "0.1"
```

## CLI Tool

A command-line interface for working with PPTX files is available:

```sh
cargo install pptx --features cli
```

Subcommands:
- `pptx-cli info` — Show presentation metadata and slide count
- `pptx-cli slides` — List slides with titles and layouts
- `pptx-cli export-html` — Export a presentation to self-contained HTML
- `pptx-cli validate` — Check a PPTX file for structural issues
- `pptx-cli repair` — Attempt to fix common PPTX problems

## Quick Start

### Create a new presentation

```rust
use pptx::Presentation;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut prs = Presentation::new()?;
    let layouts = prs.slide_layouts()?;
    let slide = prs.add_slide(&layouts[0])?;
    prs.save("hello.pptx")?;
    Ok(())
}
```

### Open and inspect an existing file

```rust
use pptx::{Presentation, ShapeTree};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let prs = Presentation::open("existing.pptx")?;

    for slide_ref in prs.slides()? {
        let xml = prs.slide_xml(&slide_ref)?;
        let tree = ShapeTree::from_slide_xml(xml)?;
        for shape in &tree.shapes {
            println!("{}: {}x{}", shape.name(), shape.width(), shape.height());
        }
    }
    Ok(())
}
```

### Add a chart

```rust
use pptx::Presentation;
use pptx::chart::data::CategoryChartData;
use pptx::enums::chart::XlChartType;
use pptx::units::{Emu, Inches};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut prs = Presentation::new()?;
    let layouts = prs.slide_layouts()?;
    let slide = prs.add_slide(&layouts[0])?;

    let mut data = CategoryChartData::new();
    data.add_category("Q1");
    data.add_category("Q2");
    data.add_series("Revenue", &[100.0, 150.0]);

    prs.add_chart_to_slide(
        &slide, &data, XlChartType::ColumnClustered,
        Inches(1.0).into(), Inches(1.5).into(),
        Inches(6.0).into(), Inches(4.0).into(),
    )?;

    prs.save("chart.pptx")?;
    Ok(())
}
```

## Minimum Supported Rust Version (MSRV)

Rust **1.85** or later.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Licensed under the [MIT License](./LICENSE).
