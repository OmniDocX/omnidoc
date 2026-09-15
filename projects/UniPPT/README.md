# UniPPT

**English** | [简体中文](README.zh-CN.md)

[OmniDoc](https://omnidoc.top/) · [GitHub](https://github.com/OmniDocX)

**Native presentation editing, PPTX conversion and local AI automation.**

UniPPT is a China-developed presentation application from OmniDoc. It combines a Rust document engine with an HTML/SVG browser editor. Text, shapes, images, equations and supported animations are represented as structured objects and exported to native PPTX elements.

## Project positioning

**Microsoft 365 (Office 365) and WPS Office** are the reference office products. Our ambition is to build the most complete China-developed office platform with publicly available source and reproducible engineering evidence. This is a development objective; see the [product comparison](docs/COMPARISON.md) for current scope and evidence. First-party code uses a non-commercial source license, detailed below.

## Performance

<!-- BENCHMARK:START -->
Measured on 2026-09-16: Windows 10, Intel Core i7-1165G7, 31.70 GiB RAM. Each case uses two warmups and seven measured iterations, executed sequentially.

| Operation | Size (slides) | Median ms | P95 ms |
| --- | ---: | ---: | ---: |
| PPTX export | 100 | 139.41 | 316.91 |
| PPTX import, cache miss | 100 | 813.36 | 1721.34 |
| UDOC export | 100 | 669.36 | 1189.64 |
| UDOC import, cache miss | 100 | 72.49 | 85.58 |
| HTML export | 100 | 325.71 | 369.41 |
| HTML import, cache miss | 100 | 83.82 | 278.04 |

[All sizes, methodology and limitations](benchmarks/README.md) · [Raw observations](benchmarks/results/2026-09-16-windows-x64.json)

Timings exclude browser rendering. Office 365 and WPS were not timed in this campaign.
<!-- BENCHMARK:END -->

## Capabilities

| Area | Public local edition |
| --- | --- |
| Editing | Text and shapes, images, connectors, equations, object transforms, slide notes, undo/redo |
| Presenting | Slideshow, supported animation effects, timing and motion paths |
| Document formats | PPTX import/export; portable UDOC and re-importable HTML |
| Format preservation | Retain untouched OPC parts and supported unknown extensions during round trips |
| Automation | Local stdio MCP, object-level operations, editable-slides workflow and optional configurable U AI |
| Optional delivery | Preview, PDF, images and video through the documented browser/rendering toolchain |

This repository contains the single-user local edition. R2, cloud storage, shared editing, sharing links, remote accounts and hosted collaboration are outside its scope.

## Quick start

Requirements: Git, Rust 1.88+ and a modern browser. Frontend assets are served directly; no frontend build is required.

```sh
git clone https://github.com/OmniDocX/UniPPT.git
cd UniPPT
cargo run --release --locked -p unippt-server
```

Open **http://127.0.0.1:8141**. Set `UNIPPT_PORT` to change the port. The service binds to the loopback interface. Save documents to local files.

For LaTeX/Office equation conversion, install Python 3.10+ and `python -m pip install -r tools/math/requirements.txt`; use `UNIPPT_PYTHON` to select the interpreter. Node.js 22+ and `npm ci` are required for MCP image processing and the complete JavaScript checks. Chromium and FFmpeg are optional dependencies for specific export workflows. See [local runtime](docs/LOCAL_RUNTIME.md).

## AI and local MCP

Start `node /absolute/path/to/UniPPT/tools/unippt_mcp.cjs` from a stdio-capable MCP client and connect to an open local document. See the [integration guide](docs/AI_MCP_INTEGRATION.md), [protocol](docs/MCP_PROTOCOL.md) and [editable-slides workflow](skills/unippt-editable-slides/SKILL.md).

U AI accepts a configurable provider through `UNIPPT_AI_BASE`, `UNIPPT_AI_MODEL` and `UNIPPT_AI_KEY`, or the local settings interface. Store keys locally. Selected context is sent to the configured provider; basic editing works without AI. Coordinate OCR requires a DashScope-compatible native protocol, independently of MCP host configuration.

## Compatibility

Native editing and source preservation are distinct capabilities. Charts, SmartArt, OLE, masters and advanced animation have object-specific limits; successful export does not establish complete PowerPoint compatibility. Validate object semantics and rendered output for production documents. See [compatibility](docs/COMPATIBILITY.md), [portable formats](docs/PORTABLE_FORMATS.md) and the [user guide](docs/USER_GUIDE.md).

## Architecture and verification

| Directory | Responsibility |
| --- | --- |
| `crates/unippt-core` | Document model, formats, equations and animation |
| `crates/unippt-server` | Local HTTP service, rendering and AI integration |
| `web` | Browser editor and presentation UI |
| `vendor/pptx`, `vendor/vecmeta` | Attributed third-party fork and versioned vector component |
| `benchmarks` | Synthetic workloads, measured results and reproduction instructions |

```sh
cargo test --workspace --locked
npm ci
node --test web/*.test.js tools/*.test.cjs
python tools/test_public_boundary.py -v
python tools/check_public_boundary.py
```

The publication check inspects indexed files; stage intended changes before running it. [Architecture](docs/ARCHITECTURE.md) · [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md) · [Licensing details](docs/LICENSING.md).

## OmniDoc ecosystem

| Project | Purpose | Website / source |
| --- | --- | --- |
| OmniDoc | Main product portal | [omnidoc.top](https://omnidoc.top/) |
| UniDoc | Document authoring | [app.unidoc.top](https://app.unidoc.top/) |
| UniPPT | Presentations | [Editor](https://unippt.unidoc.top/) · [Source](https://github.com/OmniDocX/UniPPT) |
| UniCell | Spreadsheets | [Editor](https://unicell.unidoc.top/) · [Source](https://github.com/OmniDocX/unicell) |
| UniMail | Email, calendar and contacts | [unimail.omnidoc.top](https://unimail.omnidoc.top/) |
| UniPic | Image and vector editing | [pic.unidoc.top](https://pic.unidoc.top/) |
| vecmeta | SVG ↔ EMF conversion | [Source](https://github.com/OmniDocX/vecmeta) |
| Source collections | Pinned copies of the three published components | [omnidoc](https://github.com/OmniDocX/omnidoc) · [omnidocx](https://github.com/OmniDocX/omnidocx) |

Hosted products may offer features beyond the public local editions. Their availability and terms are defined by each product.

## License and commercial use

First-party code and documentation use the [OmniDoc Non-Commercial Source License 1.0](LICENSE). Qualifying non-commercial use is free. Commercial use, including internal business use by companies in China or elsewhere, requires prior written permission. This is a source-available license, not an OSI-approved open-source license. Third-party components retain their own terms; prior lawful grants for earlier releases remain unaffected.

Commercial contact: [cc@omnidoc.top](mailto:cc@omnidoc.top) · WeChat: **13184071590**. Complete applications receive a response within 48 hours; submission or silence does not grant permission.
