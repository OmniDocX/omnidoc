# OmniDoc — Source Collection

**English** | [简体中文](README.zh-CN.md)

[OmniDoc](https://omnidoc.top/) · [GitHub](https://github.com/OmniDocX)

**Pinned source snapshots for OmniDoc's China-developed office projects.**

OmniDoc brings together buildable copies of UniPPT, UniCell and vecmeta for local presentation, spreadsheet and vector conversion workflows. These are independently developed OmniDoc projects; third-party dependencies retain their own origins and licenses.

## Project positioning

**Microsoft 365 (Office 365) and WPS Office** are the reference office products. Our ambition is to build the most complete China-developed office platform with publicly available source and reproducible engineering evidence. This is a development objective; see the [product comparison](docs/COMPARISON.md) for current scope and evidence. First-party code uses a non-commercial source license, detailed below.

## Performance

<!-- BENCHMARK:START -->
Environment: Windows 10 10.0.19045, Intel Core i7-1165G7 (4 cores / 8 threads), 31.70 GiB RAM, Python 3.13.11. Component binaries use Rust 1.97.1 release builds. Each case has two warmups and seven measured iterations, run sequentially. P95 uses nearest rank and equals the maximum with seven observations. Raw data retains every measured sample, including slow observations.

| Project | Operation | Size | Median ms | P95 ms |
| --- | --- | ---: | ---: | ---: |
| UniPPT | PPTX export | 100 slides | 139.41 | 316.91 |
| UniPPT | PPTX import, cache miss | 100 slides | 813.36 | 1721.34 |
| UniCell | CSV import + calculation | 10,000 rows | 20217.27 | 28130.84 |
| UniCell | XLSX export | 10,000 rows | 452.73 | 557.95 |
| UniCell | Batch edit + recalculation | 10,000 rows | 19081.38 | 23292.38 |
| vecmeta | SVG → EMF, CLI | 10,000 primitives | 105.80 | 118.47 |
| vecmeta | EMF → SVG, CLI | 10,000 primitives | 57.10 | 95.53 |

[Full report and source verification benchmark](benchmarks/README.md)

Measurements come from a shared workstation without full control of background load; timings may not increase monotonically with size. HTTP cases exclude browser rendering; vecmeta includes CLI startup and file I/O. Large CSV import is an observed bottleneck; XLSX export latency does not represent import or recalculation speed. Office 365 and WPS were not timed, and no competitor speed ratio is reported.
<!-- BENCHMARK:END -->

## Included projects

| Project | Purpose | Source and documentation | Performance report |
| --- | --- | --- | --- |
| UniPPT | Local presentation editing and PPTX conversion | [English](projects/UniPPT/README.md) · [中文](projects/UniPPT/README.zh-CN.md) | [Benchmark](projects/UniPPT/benchmarks/README.md) |
| UniCell | Local spreadsheets, calculation and format conversion | [English](projects/unicell/README.md) · [中文](projects/unicell/README.zh-CN.md) | [Benchmark](projects/unicell/benchmarks/README.md) |
| vecmeta | SVG/EMF conversion libraries and CLI | [English](projects/vecmeta/README.md) · [中文](projects/vecmeta/README.zh-CN.md) | [Benchmark](projects/vecmeta/benchmarks/README.md) |

PolyglotPDF is excluded. The `projects/` directories contain ordinary copied files. Build each component independently and update the provenance manifest when refreshing snapshots.

## Quick start

```sh
git clone https://github.com/OmniDocX/omnidoc.git
cd omnidoc
python tools/verify_copies.py
cd projects/UniPPT
cargo run --release --locked -p unippt-server
```

Open http://127.0.0.1:8141. Consult UniCell's own documentation for its launch command and optional dependencies. This collection does not provide a combined office-suite server.

## Provenance and integrity

[sources.lock.json](sources.lock.json) records public source repositories, commit IDs and a SHA-256 for every copied file. `python tools/verify_copies.py` checks the tracked file set and file contents. Each project retains its license, third-party notices and raw benchmark observations.

The public applications use local files and loopback services. R2, cloud storage, collaborative editing, shared links and centralized accounts are outside the public edition.

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

First-party code and documentation use the unmodified [PolyForm Noncommercial 1.0.0](LICENSE). Uses permitted by that license are free. Commercial uses outside its permitted purposes require a separate paid commercial license: contact us to apply, agree on fees and obtain written authorization before use. The standard license's institutional permissions remain fully applicable. This is a source-available license, not an OSI-approved open-source license. Third-party terms and valid earlier grants remain unchanged.

[License scope and permitted uses](docs/LICENSING.md) · [Commercial licensing and application](docs/COMMERCIAL_LICENSE.md).

Commercial contact: [cc@omnidoc.top](mailto:cc@omnidoc.top) · WeChat: **13184071590**. Complete applications receive a response within 48 hours; submission or silence does not grant permission.
