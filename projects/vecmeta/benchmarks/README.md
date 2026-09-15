# vecmeta — Performance benchmark

**English** | [简体中文](README.zh-CN.md)

Measured on 2026-09-16: Windows 10, Intel Core i7-1165G7, 31.70 GiB RAM. Each case uses two warmups and seven measured iterations, executed sequentially.

## Environment

| Field | Value |
| --- | --- |
| OS | Microsoft Windows 10 专业版 (10.0.19045, AMD64) |
| CPU | 11th Gen Intel(R) Core(TM) i7-1165G7 @ 2.80GHz (4 cores / 8 threads) |
| RAM | 31.70 GiB |
| Rust | `rustc 1.97.1 (8bab26f4f 2026-07-14)` |
| Python | 3.13.11 |
| Runtime source commit | `96acc4608d644d8137fd334412e82bb2ba7c5a0a` |
| Build profile | `cargo build --release --locked -p emfsvg-cli; default opt-level=3, lto=true` |
| Executable size | 1,203,712 bytes |
| Measured at UTC | 2026-09-15T23:03:36.508770+00:00 |

## Workload and timing boundary

Alternating solid rectangles and triangle paths, with deterministic positions and colors. No text, font lookup, gradients, images or source encapsulation. Conversion uses the default scale and no `--lossless` flag. Checks cover EMF signature/declared length, SVG path counts and the CLI geometry fixed-point test at tolerance 1e-6. Geometry checks are not pixel-level rendering verification.

Timing includes the complete CLI process: startup, file I/O, conversion and exit. Output validation is outside the timed region. The OS file cache is warm.

## Complete results

| Operation | Size (primitives) | Median ms | P95 ms |
| --- | ---: | ---: | ---: |
| SVG → EMF, CLI | 100 | 25.10 | 25.85 |
| EMF → SVG, CLI | 100 | 22.36 | 24.29 |
| Geometry round trip, CLI | 100 | 23.83 | 25.95 |
| SVG → EMF, CLI | 1,000 | 26.62 | 46.84 |
| EMF → SVG, CLI | 1,000 | 25.75 | 26.39 |
| Geometry round trip, CLI | 1,000 | 30.40 | 47.85 |
| SVG → EMF, CLI | 10,000 | 105.80 | 118.47 |
| EMF → SVG, CLI | 10,000 | 57.10 | 95.53 |
| Geometry round trip, CLI | 10,000 | 206.04 | 224.29 |

[Raw observations](results/2026-09-16-windows-x64.json) · [Harness](run.py)

## Method and limitations

Warmup observations are excluded. The median is the fourth ordered sample. P95 uses nearest rank `ceil(0.95 × n)`; with n=7 it equals the observed maximum and is not a stable tail-latency estimate. Raw JSON retains every measured duration, input/output sizes or hashes, correctness results, binary SHA-256 and harness SHA-256. The source commit identifies the runtime code used for the build; this documentation/benchmark update does not modify that runtime code.

This was a shared workstation run without full control of background load, power management or file-cache state. Slow observations were not discarded. No concurrent load test was performed. Results apply to these synthetic workloads only; peak memory, full application startup, browser frame rate, complex real documents, external AI and cloud services were not measured. Microsoft 365 (Office 365) and WPS Office were not timed, so these results do not establish a relative speed ranking.

## Reproduce

```sh
cargo build --release --locked -p emfsvg-cli
python benchmarks/run.py --binary target/release/emfsvg --sizes 100,1000,10000 --warmups 2 --samples 7 --output benchmarks/results/local.json
python benchmarks/verify_results.py
```

On Windows, append `.exe` to the executable path. Adjust `--binary` when building with `--target-dir`. Only the Python standard library is required; all inputs are generated. Server benchmarks start and stop their own temporary loopback process. Reproduce with the recorded runtime source, lockfile, compiler and build profile; timings vary with hardware and load.

[Office 365 / WPS comparison and controlled-test protocol](../docs/COMPARISON.md)
