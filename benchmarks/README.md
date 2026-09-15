# OmniDoc — Performance benchmarks

**English** | [简体中文](README.zh-CN.md)

This collection reports the measured performance of its three components and a separate measurement of source integrity verification. It has no independent office runtime; verification latency is not editing or calculation latency.

## Environment and method

Environment: Windows 10 10.0.19045, Intel Core i7-1165G7 (4 cores / 8 threads), 31.70 GiB RAM, Python 3.13.11. Component binaries use Rust 1.97.1 release builds. Each case has two warmups and seven measured iterations, run sequentially. P95 uses nearest rank and equals the maximum with seven observations. Raw data retains every measured sample, including slow observations.

## Component results

| Project | Operation | Size | Median ms | P95 ms |
| --- | --- | ---: | ---: | ---: |
| UniPPT | PPTX export | 100 slides | 139.41 | 316.91 |
| UniPPT | PPTX import, cache miss | 100 slides | 813.36 | 1721.34 |
| UniCell | CSV import + calculation | 10,000 rows | 20217.27 | 28130.84 |
| UniCell | XLSX export | 10,000 rows | 452.73 | 557.95 |
| UniCell | Batch edit + recalculation | 10,000 rows | 19081.38 | 23292.38 |
| vecmeta | SVG → EMF, CLI | 10,000 primitives | 105.80 | 118.47 |
| vecmeta | EMF → SVG, CLI | 10,000 primitives | 57.10 | 95.53 |

Measurements come from a shared workstation without full control of background load; timings may not increase monotonically with size. HTTP cases exclude browser rendering; vecmeta includes CLI startup and file I/O. Large CSV import is an observed bottleneck; XLSX export latency does not represent import or recalculation speed. Office 365 and WPS were not timed, and no competitor speed ratio is reported.

| Project | Included commit | Runtime build commit | Report and raw data |
| --- | --- | --- | --- |
| UniPPT | `6ff38470e22de19dd822f4ff616e06cbb13148b5` | `e66c1e0eab10a93afd776a755e0973149dedcd73` | [Report](../projects/UniPPT/benchmarks/README.md) · [JSON](../projects/UniPPT/benchmarks/results/2026-09-16-windows-x64.json) |
| unicell | `44bfc75e3c49eda4ac408f50c711ce70b0e34e7a` | `51e007388d702f23290bb44df7270d164daa0e2d` | [Report](../projects/unicell/benchmarks/README.md) · [JSON](../projects/unicell/benchmarks/results/2026-09-16-windows-x64.json) |
| vecmeta | `2d2ce09aad1778b52a5af4bbc1902d166c286d08` | `96acc4608d644d8137fd334412e82bb2ba7c5a0a` | [Report](../projects/vecmeta/benchmarks/README.md) · [JSON](../projects/vecmeta/benchmarks/results/2026-09-16-windows-x64.json) |

## Collection integrity verification

Timing includes Python/Git process startup, indexed file inventory, reads of every copied source file and SHA-256 verification. OS caches are warm. Network and application startup are excluded. An overlapping preflight execution was rejected; the published data comes from a subsequent sequential rerun.

1,321 files · 18,463,820 bytes · SHA-256 manifest: `f12679d3c36f3e9da0f3104c210fd0b307316869ef0edc3ed76f4edc2b88fb79`

| Operation | Size (files) | Median ms | P95 ms |
| --- | ---: | ---: | ---: |
| Source integrity verification | 1,321 | 1557.28 | 3363.75 |

[Raw observations](results/2026-09-16-windows-x64.json). The manifest hash and component commits identify the source collection actually measured; the collection commit in environment metadata is the baseline at execution time.

## Reproduce

```sh
python tools/verify_copies.py
python benchmarks/run.py --warmups 2 --samples 7 --output benchmarks/results/local.json
python benchmarks/verify_results.py
```

Run component benchmarks from their respective `projects/` directories; each report includes its build profile and reproduction commands.
