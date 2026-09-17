<div align="center">

# vecmeta

**A Rust vector engine connecting browser SVG with Office EMF.**

**English** | [简体中文](README.zh-CN.md)

[Website](https://omnidoc.top/) · [Quick start](#quick-start) · [Documentation](#documentation) · [Benchmarks](benchmarks/README.md)

[![CI](https://github.com/OmniDocX/vecmeta/actions/workflows/verify.yml/badge.svg)](https://github.com/OmniDocX/vecmeta/actions/workflows/verify.yml)
[![License: PolyForm Noncommercial](https://img.shields.io/badge/license-PolyForm_Noncommercial-315EFB?style=flat-square)](LICENSE)
[![GitHub issues](https://img.shields.io/github/issues/OmniDocX/vecmeta?style=flat-square)](https://github.com/OmniDocX/vecmeta/issues)

</div>

vecmeta provides Rust libraries and a CLI for bidirectional SVG ↔ EMF conversion. A shared scene model handles paths, shapes, transforms, paint and glyph outlines for document converters, vector asset pipelines and office applications. Conversion runs without Microsoft Office or LibreOffice installed.

An independently developed OmniDoc component from China, supplying vector conversion to its application projects.

![vecmeta: SVG and EMF connected through a shared vector scene](docs/images/pipeline.svg)

## Highlights

- **Two-way conversion** — Emit EMF from SVG or turn EMF drawing records into browser-readable SVG.
- **Rust libraries and CLI** — Embed conversion in Rust applications or use the command line in batch jobs, build scripts and local tools.
- **Shared vector model** — Use consistent paths, transforms, fills, strokes and supported gradient handling.
- **Glyph outlines** — Find fonts and extract glyph geometry through fontdb and ttf-parser.
- **Conversion diagnostics** — Receive warnings about skipped content and compatibility losses for downstream handling.
- **Optional source recovery** — Use `--lossless` to encapsulate original data and recover the embedded source on reverse conversion.

## Quick start

Install Rust and Cargo; CI uses Rust 1.88:

```sh
git clone https://github.com/OmniDocX/vecmeta.git
cd vecmeta
cargo build --release --locked -p emfsvg-cli

# SVG → EMF
cargo run --release --locked -p emfsvg-cli -- to-emf input.svg -o output.emf

# EMF → SVG
cargo run --release --locked -p emfsvg-cli -- to-svg input.emf -o output.svg
```

The executable is `target/release/emfsvg` (`emfsvg.exe` on Windows). Use `--help` for arguments. Text-to-outline conversion requires the corresponding fonts.

## Rust integration

Add the `svg2emf` / `emf2svg` crates as path or Git dependencies to convert directly and inspect diagnostics:

```rust
use svg2emf::{svg_to_emf_with_report, EmitOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
      <circle cx="50" cy="50" r="25" fill="#315efb"/>
    </svg>"#;

    let result = svg_to_emf_with_report(svg, EmitOptions::default())?;
    std::fs::write("output.emf", &result.bytes)?;
    for warning in &result.warnings {
        eprintln!("{warning}");
    }
    Ok(())
}

```

Use `emf_to_svg_with_report` for the reverse path. UniPPT embeds a separately versioned snapshot; use the APIs available in your target component.

## Performance

<!-- BENCHMARK:START -->
| Operation | Workload | Median |
| --- | --- | --- |
| SVG → EMF | 10,000 primitives | **105.80 ms** |
| EMF → SVG | 10,000 primitives | **57.10 ms** |
| Geometry round trip | 10,000 primitives | **206.04 ms** |

2026-09-16 · Windows 10 · Intel i7-1165G7 · 31.7 GiB · Rust release · 2 warmups / 7 measurements.

[Full results, raw observations and reproduction](benchmarks/README.md) — Synthetic local workloads; browser rendering is excluded. Other office products were not timed.
<!-- BENCHMARK:END -->

## Conversion modes and compatibility

Default conversion translates supported vector semantics. `--lossless` additionally embeds the source file. Exact source-byte recovery and matching rendering in another application are separate capabilities; rendering needs validation in the target application.

Outlined text becomes geometry. Bitmap, filter and clipping support is limited; radial gradients are approximated. Inspect conversion warnings and check important output in its destination application.

## Documentation

| Component | Responsibility |
| --- | --- |
| [`vector-ir`](crates/vector-ir) | Scenes, paths, transforms and paint |
| [`emf-core`](crates/emf-core) | EMF binary records and I/O |
| [`svg2emf`](crates/svg2emf) / [`emf2svg`](crates/emf2svg) | Parsing, conversion and diagnostics |
| [`glyph2path`](crates/glyph2path) | Font lookup and glyph outlines |
| [`emfsvg-cli`](crates/emfsvg-cli) | CLI and geometry round-trip checks |

[Provenance](docs/PROVENANCE.md) · [Contributing](CONTRIBUTING.md) · [Full benchmarks](benchmarks/README.md)

## OmniDoc and community

[OmniDoc website](https://omnidoc.top/) · [UniPPT](https://github.com/OmniDocX/UniPPT) · [UniCell](https://github.com/OmniDocX/unicell) · [vecmeta](https://github.com/OmniDocX/vecmeta)

Share reproducible bugs and feature requests through [GitHub Issues](https://github.com/OmniDocX/vecmeta/issues). Contributions to features, format compatibility and documentation are welcome.

Microsoft 365 (Office 365) and WPS Office inform our office workflows; ONLYOFFICE and Univer are reference projects in the public office ecosystem. See [project positioning and capabilities](docs/COMPARISON.md).

## License and commercial licensing

First-party code uses [PolyForm Noncommercial 1.0.0](LICENSE). Noncommercial and specified institutional uses are free under its terms. Commercial uses outside those permissions require a [paid commercial license](docs/COMMERCIAL_LICENSE.md) and written authorization.

**Commercial contact: [cc@omnidoc.top](mailto:cc@omnidoc.top) · WeChat: 13184071590**

This is a source-available license, not an OSI-approved open-source license. Third-party terms and valid earlier grants remain independent. See [license scope](docs/LICENSING.md).
