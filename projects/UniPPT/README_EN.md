<h1 align="center">UniPPT</h1>
<p align="center"><strong>Native Presentation Engine and AI Automation Platform</strong></p>
<p align="center">An OmniDoc product for native PPTX editing, MCP integration, Skills workflows, and configurable AI models.</p>

<p align="center">
  <a href="https://github.com/OmniDocX/UniPPT/actions/workflows/verify.yml"><img alt="Verify" src="https://github.com/OmniDocX/UniPPT/actions/workflows/verify.yml/badge.svg"></a>
  <img alt="Rust 1.88+" src="https://img.shields.io/badge/Rust-1.88%2B-222222">
  <img alt="MCP and Skills" src="https://img.shields.io/badge/AI-MCP%20%2B%20Skills-2563eb">
  <a href="LICENSE"><img alt="Non-commercial license" src="https://img.shields.io/badge/License-Non--Commercial%20%2B%20Commercial-orange"></a>
</p>

<p align="center"><a href="README.md">简体中文</a> · <strong>English</strong></p>
<p align="center"><a href="https://unippt.unidoc.top/">Live editor</a> · <a href="docs/USER_GUIDE.md">Documentation</a> · <a href="docs/MCP_PROTOCOL.md">MCP protocol</a> · <a href="https://omnidoc.top/">OmniDoc</a> · <a href="docs/COMMERCIAL_LICENSE.md">Commercial licensing</a></p>

---

## Local edition scope

This repository contains the local single-user edition: editing, animation, slideshow, format conversion, import/export, configurable U AI and local stdio MCP. R2, cloud storage, cloud documents, shared editing, sharing links, collaboration and account gateways are excluded.

## Overview

UniPPT is OmniDoc's presentation editing and automation platform, providing browser-based and locally deployed alternatives for PowerPoint document workflows. It combines a Rust document engine, an HTML/SVG editor, and MCP services for import, authoring, object editing, equations, animation, presentation, and multi-format export.

Text, shapes, connectors, equations, images, and animations use a shared structured document model. The browser editor and external AI clients operate on the same objects and properties, with export to native objects in PPTX files.

| Capability | Technical scope |
| --- | --- |
| **Native PPTX objects** | Map text, shapes, images, connectors, equations, and supported animations to native OOXML structures |
| **Preservation-first round trips** | Patch the original OPC package while retaining untouched parts, unknown extensions, and original timing trees where applicable |
| **LaTeX ↔ Office equations** | Preserve LaTeX and OMML to connect browser previews with native Office Math |
| **Programmable animation** | Edit entrance, emphasis, exit, timing, ordering, motion paths, and media nodes |
| **MCP + Skills** | Object inspection, batch module composition, regional updates, animation, preview, validation, and export |
| **Configurable models and deployment** | Model-neutral MCP and OpenAI-compatible provider configuration in U AI |
| **Multiple delivery formats** | PPTX, UDOC, lossless HTML, PDF, image packages, and video |

## Contents

[Capabilities](#capabilities-and-compatibility) · [AI, MCP, and Skills](#ai-mcp-and-skills) · [Quick start](#quick-start) · [OmniDoc Products](#omnidoc-products) · [Development](#architecture-and-development) · [License](#license-and-commercial-use)

## Capabilities and compatibility

### Editor features

The editor provides a ribbon, thumbnails, outline, notes, formatting and animation panes, and a presentation view. Object operations include selection, movement, resizing, rotation, stacking order, undo, and redo. Supported content includes text, shapes, images, and equations. The contextual text toolbar appears for character selections and applies formatting to the selected range.

### Native engines and portable formats

- **PPTX:** Rust-based OOXML/OPC parsing and export combine object edits with source-package preservation.
- **Equations:** LaTeX / MathML / OMML conversion, KaTeX previews, and Office Math output.
- **Animation:** edit supported effects and regenerate native `p:timing`; preserve unmodeled custom animation where applicable.
- **UDOC / lossless HTML:** carry re-importable native data; HTML also supports portable presentation.
- **SVG / EMF:** powered by OmniDoc's native Rust [vecmeta](https://github.com/OmniDocX/vecmeta) components.
- **Video:** browser recording or Chromium + FFmpeg frame-by-frame rendering for fixed-frame-rate delivery.

The native implementation covers document models, OOXML processing, and vector conversion. Equation conversion depends on Python/XSLT; preview and video processing depend on browsers and supporting tools.

| Content | Handling |
| --- | --- |
| Text, basic shapes, images, OMML, supported animation | Native editing and export |
| Untouched OPC parts and unknown Office extensions | Preservation-first handling with byte-level regressions |
| Charts, SmartArt, OLE, masters, complex objects | Import/preservation and structured editing are assessed separately |
| Advanced animation, protected files, specialized Office features | Subject to format and object support |

Supported formats and object types are documented in the [compatibility specification](docs/COMPATIBILITY.md). Compatibility validation assesses file structure, visual rendering, and object semantics separately from export completion.

## AI, MCP, and Skills

```text
Host model analyzes input → Skill defines the workflow
        ↓
MCP queries objects and capabilities → batch composition / atomic edits / animation
        ↓
Preview and validation → targeted revision → native PPTX / other export formats
```

- **Model-neutral integration:** supports appropriately capable ChatGPT, Codex, Claude, and other MCP clients, with model selection managed by the host.
- **Module-level batches:** expands descriptions of repeated layers, arrows, and labels into document objects within a batch submission.
- **Revision and concurrency guards:** document writes follow each tool's `sessionId`, `expectedRevision`, and stable `requestId` contract.
- **Targeted revisions:** updates specified regions by object ID, with preview, structural validation, and export.
- **Image-to-editable slides:** U AI provides image reconstruction; external models can perform visual analysis and create native document objects through MCP. Reconstructed text, equations, and geometry require comparison with the source image.

**Local MCP: `node /absolute/path/to/UniPPT/tools/unippt_mcp.cjs`**

Connect a stdio-capable client to an open local document. The editor settings menu controls MCP auto-connect. Runtime tools/list and schemas define the contract.

Use the [editable-slides Skill](skills/unippt-editable-slides/SKILL.md) for native editing and image reconstruction.

[Protocol](docs/MCP_PROTOCOL.md) · [Local integration](docs/AI_MCP_INTEGRATION.md)

### Model configuration

Configure an OpenAI-compatible service through U AI settings or local `.env.local`:

```ini
UNIPPT_AI_BASE=https://api.example.com/v1
UNIPPT_AI_MODEL=your-model-name
UNIPPT_AI_KEY=<your-local-key>
```

Model credentials must remain in local configuration or the deployment environment and must not be committed to the repository. The coordinate OCR setting `UNIPPT_OCR_MODEL` requires compatibility with the native DashScope protocol. MCP host models and U AI use independent configurations.

## Quick start

Requirements: **Rust 1.88+ and Git**. Equations also require **Python 3.10+**. Tests use Node.js 22; audio/video regressions use FFmpeg/ffprobe.

### Windows / PowerShell

```powershell
git clone https://github.com/OmniDocX/UniPPT.git
cd UniPPT
python -m venv .venv
.venv\Scripts\python.exe -m pip install -r tools/math/requirements.txt
$env:UNIPPT_PYTHON = (Resolve-Path .venv\Scripts\python.exe).Path
cargo run -p unippt-server --locked
```

### Linux / macOS

```sh
git clone https://github.com/OmniDocX/UniPPT.git
cd UniPPT
python3 -m venv .venv
.venv/bin/python -m pip install -r tools/math/requirements.txt
UNIPPT_PYTHON="$PWD/.venv/bin/python" cargo run -p unippt-server --locked
```

Open **http://127.0.0.1:8141**. Set process environment variable `UNIPPT_PORT` to change the port. No npm frontend build is required.

These commands start the local single-user edition. Save documents as local files; the server listens on loopback.

Local MCP image handling and the full test suite require Node.js 22+ and `npm ci` at the repository root. Optional native preview and delivery verification dependencies are listed in [Local runtime setup](docs/LOCAL_RUNTIME.md).

## OmniDoc Products

OmniDoc provides products for document processing, content authoring, presentations, spreadsheets, email management, and image editing, together with format-conversion components.

| Product / component | Business focus | Official website / source |
| --- | --- | --- |
| OmniDoc | Main site, document services, identity | [omnidoc.top](https://omnidoc.top/) |
| UniDoc | Document authoring and editing | [app.unidoc.top](https://app.unidoc.top/) |
| UniPPT | Native presentations and AI orchestration | [unippt.unidoc.top](https://unippt.unidoc.top/) · [Source](https://github.com/OmniDocX/UniPPT) |
| UniCell | Browser spreadsheets | [unicell.unidoc.top](https://unicell.unidoc.top/) |
| UniMail | Multi-account email, calendars, contacts, and AI email assistance | [unimail.omnidoc.top](https://unimail.omnidoc.top/) |
| UniPic | Image and vector editing | [pic.unidoc.top](https://pic.unidoc.top/) |
| vecmeta | Native Rust SVG ↔ EMF engine | [Source and documentation](https://github.com/OmniDocX/vecmeta) |
| PolyglotPDF | Multilingual ebook and PDF translation | [Source](https://github.com/OmniDocX/PolyglotPDF) |

Deployment, account, and licensing requirements are defined in each product's official documentation. vecmeta is distributed as Rust libraries and a command-line tool.

## Architecture and development

```text
crates/unippt-core/      Document model, import/export, equations, animation
crates/unippt-server/    Local service, rendering, AI host
web/                    HTML/SVG editing and presentation
vendor/pptx/            Attributed rust-pptx fork
vendor/vecmeta/         OmniDoc vector conversion
skills/                 Local MCP workflows
docs/                   Protocols, compatibility, formats, integrations
```

```sh
cargo test --workspace --locked
node --check web/app.js
npm ci
node --test web/*.test.js tools/*.test.cjs
python tools/test_public_boundary.py -v
python tools/check_public_boundary.py
```

Publication checks inspect the Git index and require intended changes to be staged before execution. Tests cover document round trips, native objects, animation, equations, web interactions, audio/video, authorization, and publication boundaries. Changes affecting visual output additionally require rendering and object-level validation.

[Architecture](docs/ARCHITECTURE.md) · [Portable formats](docs/PORTABLE_FORMATS.md) · [Maintenance](docs/OPEN_SOURCE.md) · [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md)

## License and commercial use

**Free for non-commercial use. Prior written permission is required for commercial use.**

Current first-party source uses the [OmniDoc Non-Commercial Source License 1.0](LICENSE). It is source-available, not MIT or an OSI-approved open-source license. Personal learning, non-commercial research, and teaching may use, modify, and redistribute it non-commercially under its terms.

**Companies in China and all other countries or regions require a commercial license for internal business use, commercial products, SaaS, APIs, customer delivery, or commercial integration.** Modifications, private hosting, and lack of a direct usage fee do not automatically exempt commercial activity.

- **Email:** [cc@omnidoc.top](mailto:cc@omnidoc.top)
- **WeChat:** add mobile number **13184071590**
- **Review:** complete applications are reviewed and answered within **48 hours** of receipt. Submission or silence does not grant permission; the written commercial agreement defines the actual rights.

[Apply for a license](docs/COMMERCIAL_LICENSE.md) · [License scope and earlier releases](docs/LICENSING.md). Third-party licenses remain in force; earlier MIT/GPL grants are not retroactively revoked.

PowerPoint is a Microsoft trademark. UniPPT is independent and is not a Microsoft product, certification, or endorsement.
