<div align="center">

<img src="web/unippt-logo.svg" width="72" height="72" alt="UniPPT" />

# UniPPT

**Create presentations in your browser. Export native, editable PPTX.**

**English** | [简体中文](README.zh-CN.md)

[Website](https://omnidoc.top/) · [Live app](https://unippt.unidoc.top/) · [Quick start](#quick-start) · [Documentation](#documentation) · [Benchmarks](benchmarks/README.md)

[![CI](https://github.com/OmniDocX/UniPPT/actions/workflows/verify.yml/badge.svg)](https://github.com/OmniDocX/UniPPT/actions/workflows/verify.yml)
[![License: PolyForm Noncommercial](https://img.shields.io/badge/license-PolyForm_Noncommercial-315EFB?style=flat-square)](LICENSE)
[![GitHub issues](https://img.shields.io/github/issues/OmniDocX/UniPPT?style=flat-square)](https://github.com/OmniDocX/UniPPT/issues)

</div>

UniPPT combines a browser editor, a Rust document engine and local AI automation for lessons, reports, technical talks and everyday presentations. Work with structured text, shapes, images, equations and animations; import a PPTX, keep editing, then export to PPTX, UDOC or HTML.

Developed in China as part of OmniDoc, with a public local edition you can build and run yourself.

![UniPPT presentation editor](docs/images/editor.png)

<sub>Actual public local edition: native text, shapes and Office math.</sub>

## Highlights

- **Native object editing** — Arrange text, shapes, images, connectors and equations, with alignment, grouping, undo and redo.
- **A PPTX workflow** — Import existing decks, edit supported objects and export native PPTX; the format engine retains untouched parts for round trips.
- **Present and animate** — Run slideshows in the browser with supported effects, motion paths, timing and speaker notes.
- **Math authoring** — Use LaTeX editing and Office equation conversion for educational and technical content.
- **AI and MCP** — Configure U AI in the editor or let an external AI client inspect, edit and preview decks through local stdio MCP.
- **Flexible delivery** — Save PPTX, UDOC or re-importable HTML. Add the documented toolchain for PDF, image and video exports.

## Quick start

With Git, Rust 1.88+ and a modern browser installed:

```sh
git clone https://github.com/OmniDocX/UniPPT.git
cd UniPPT
cargo run --release --locked -p unippt-server
```

Open **http://127.0.0.1:8141** and start editing. The server includes the frontend; no separate frontend build is needed. Set `UNIPPT_PORT` to change the port.

<details>
<summary>Optional dependencies</summary>

Equation conversion uses Python 3.10+: `python -m pip install -r tools/math/requirements.txt`. MCP image processing requires Node.js 22+ and `npm ci`. Chromium and FFmpeg support the corresponding rendering/video workflows. See [runtime setup](docs/LOCAL_RUNTIME.md).

</details>

## Connect your AI workflow

Start UniPPT, then add this configuration to a stdio-capable MCP client. Replace the path with your installation directory:

```json
{
  "mcpServers": {
    "unippt": {
      "command": "node",
      "args": ["/absolute/path/to/UniPPT/tools/unippt_mcp.cjs"]
    }
  }
}
```

Connect to the current local document to inspect objects, compose slides, modify layouts and check previews. [MCP guide](docs/AI_MCP_INTEGRATION.md) · [Editable-slides skill](skills/unippt-editable-slides/SKILL.md).

The editor’s **U AI** supports a configurable model provider through `UNIPPT_AI_BASE`, `UNIPPT_AI_MODEL` and `UNIPPT_AI_KEY`. When enabled, selected content is sent to that provider.

## Performance

<!-- BENCHMARK:START -->
| Operation | Workload | Median |
| --- | --- | --- |
| PPTX export | 100 slides | **139.41 ms** |
| PPTX import, cache miss | 100 slides | **813.36 ms** |

2026-09-16 · Windows 10 · Intel i7-1165G7 · 31.7 GiB · Rust release · 2 warmups / 7 measurements.

[Full results, raw observations and reproduction](benchmarks/README.md) — Synthetic local workloads; browser rendering is excluded. Other office products were not timed.
<!-- BENCHMARK:END -->

## Edition and file compatibility

This repository is the single-user local edition, with documents saved to local files. The live product is linked above; R2, cloud storage, sharing links, collaborative editing and hosted accounts are outside this edition.

PPTX editing and preservation depend on object type. See [compatibility](docs/COMPATIBILITY.md) for charts, SmartArt, OLE, masters and advanced animation.

## Documentation

| Guide | What it covers |
| --- | --- |
| [User guide](docs/USER_GUIDE.md) | Editing, presenting and exporting |
| [MCP](docs/MCP_PROTOCOL.md) | Tool protocol and integration |
| [Document formats](docs/PORTABLE_FORMATS.md) | PPTX, UDOC and HTML |
| [Architecture](docs/ARCHITECTURE.md) | Engine, server and editor |
| [Contributing](CONTRIBUTING.md) | Development, tests and contribution workflow |

## OmniDoc and community

[OmniDoc website](https://omnidoc.top/) · [UniPPT](https://github.com/OmniDocX/UniPPT) · [UniCell](https://github.com/OmniDocX/unicell) · [vecmeta](https://github.com/OmniDocX/vecmeta)

Share reproducible bugs and feature requests through [GitHub Issues](https://github.com/OmniDocX/UniPPT/issues). Contributions to features, format compatibility and documentation are welcome.

Microsoft 365 (Office 365) and WPS Office inform our office workflows; ONLYOFFICE and Univer are reference projects in the public office ecosystem. See [project positioning and capabilities](docs/COMPARISON.md).

## License and commercial licensing

First-party code uses [PolyForm Noncommercial 1.0.0](LICENSE). Noncommercial and specified institutional uses are free under its terms. Commercial uses outside those permissions require a [paid commercial license](docs/COMMERCIAL_LICENSE.md) and written authorization.

**Commercial contact: [cc@omnidoc.top](mailto:cc@omnidoc.top) · WeChat: 13184071590**

This is a source-available license, not an OSI-approved open-source license. Third-party terms and valid earlier grants remain independent. See [license scope](docs/LICENSING.md).
