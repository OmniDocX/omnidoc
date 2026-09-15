# OmniDoc — Performance benchmarks

**English** | [简体中文](README.zh-CN.md)

This collection provides independent component measurements and a separate measurement of source integrity verification. It has no independent office runtime; source verification latency does not represent document editing or calculation speed.

Environment: Windows 10 10.0.19045, Intel Core i7-1165G7 (4 cores / 8 threads), 31.70 GiB RAM, Python 3.13.11; components use Rust 1.97.1 release builds. Each case has two warmups and seven measured iterations, run sequentially. Nearest-rank P95 equals the maximum with seven samples. All measured observations are retained.

## Component benchmarks

| Project | Operation | Size | Median ms | P95 ms |
| --- | --- | ---: | ---: | ---: |
| UniPPT | PPTX export | 100 slides | 139.41 | 316.91 |
| UniPPT | PPTX import, cache miss | 100 slides | 813.36 | 1721.34 |
| UniCell | CSV import + calculation | 10,000 rows | 660.92 | 707.79 |
| UniCell | XLSX export | 10,000 rows | 562.30 | 618.65 |
| UniCell | Batch edit + recalculation | 10,000 rows | 120.68 | 135.50 |
| vecmeta | SVG → EMF, CLI | 10,000 primitives | 105.80 | 118.47 |
| vecmeta | EMF → SVG, CLI | 10,000 primitives | 57.10 | 95.53 |

UniCell includes the SUM optimization: public-edition import plus calculation of the same 10,000-row CSV improved from 20.217 s to 0.661 s, approximately 31×; all 20,000 formulas and results passed on every new iteration. The earlier full-application experiment at 0.368 s (approximately 55×) and the old baseline are retained in the [UniCell report](../projects/unicell/benchmarks/README.md). The optimized measurements use different builds and must not be conflated.

Background load on the shared workstation was not fully controlled; results apply only to the listed synthetic workloads. HTTP measurements exclude browser rendering; vecmeta includes CLI startup and file I/O. Office 365, Excel and WPS were not timed, so no competitor speed ranking is reported.

| Project | Included commit | Runtime build commit | Report and data |
| --- | --- | --- | --- |
| UniPPT | `14521e76459ac6236c484a0092e84da6e7b71f8b` | `e66c1e0eab10a93afd776a755e0973149dedcd73` | [Report](../projects/UniPPT/benchmarks/README.md) · [JSON](../projects/UniPPT/benchmarks/results/2026-09-16-windows-x64.json) |
| unicell | `9a45c5f94da22cc78cd9654258c334ebef8ff15c` | `dc6f6c6a5c5de532e93b3adbf6a00336562a1a9f` | [Report](../projects/unicell/benchmarks/README.md) · [JSON](../projects/unicell/benchmarks/results/2026-09-16-sum-optimized-windows-x64.json) |
| vecmeta | `5e186640fc943720635c2bc69e068078bc6d42a7` | `96acc4608d644d8137fd334412e82bb2ba7c5a0a` | [Report](../projects/vecmeta/benchmarks/README.md) · [JSON](../projects/vecmeta/benchmarks/results/2026-09-16-windows-x64.json) |

## Current source integrity verification

Current snapshot verification includes Python/Git startup, indexed file inventory, all source-file reads and SHA-256 checks. OS caches are warm; network and application runtime are excluded. The two collections were measured sequentially.

1,333 files · 18,518,350 bytes · manifest SHA-256: `892fdc03696104800d2c2a3227c3e81598fd0596d174ca9b02531a2033d3e44d`

| Operation | Size (files) | Median ms | P95 ms |
| --- | ---: | ---: | ---: |
| Source integrity verification | 1,333 | 375.54 | 408.44 |

[JSON](results/2026-09-16-polyform-snapshot-windows-x64.json)

## Preserved historical snapshot

The following measurement belongs to the preceding source snapshot, with its own file count and hashes. It does not describe the current collection, and the latency change alone does not establish an optimization benefit.

| Operation | Size (files) | Median ms | P95 ms |
| --- | ---: | ---: | ---: |
| Source integrity verification | 1,321 | 1557.28 | 3363.75 |

[JSON](results/2026-09-16-windows-x64.json)

## Reproduce

```sh
python tools/verify_copies.py
python benchmarks/run.py --warmups 2 --samples 7 --output benchmarks/results/local.json
python benchmarks/verify_results.py
```
Run component benchmarks from their respective `projects/` directories; build profiles and commands are in the linked reports.
