"""Check published benchmark statistics, provenance and documentation coverage."""
from pathlib import Path
import hashlib
import json
import math
import statistics

root = Path(__file__).resolve().parents[1]
reports = sorted((root / "benchmarks/results").glob("20*.json"))
assert reports, "No dated benchmark report"
assert (root / "benchmarks/results/2026-09-16-polyform-snapshot-windows-x64.json").exists(), "Missing current snapshot measurement"
for path in reports:
    report = json.loads(path.read_text(encoding="utf-8"))
    assert report["schema_version"] == 1
    assert report["concurrency"] == 1
    assert report["samples_per_case"] > 0
    assert report["warmups_per_case"] >= 0
    assert len(report["environment"]["source_commit"]) == 40
    if path.name == "2026-09-16-polyform-snapshot-windows-x64.json":
        assert report["environment"]["harness_sha256"] == hashlib.sha256((root / "benchmarks/run.py").read_bytes()).hexdigest()
    manifest_path = root / "sources.lock.json"
    if manifest_path.exists() and path.name == "2026-09-16-polyform-snapshot-windows-x64.json":
        assert report["manifest_sha256"] == hashlib.sha256(manifest_path.read_bytes()).hexdigest()
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        assert report["source_snapshot_commits"] == {p["name"]: p["commit"] for p in manifest["projects"]}
        assert report["files"] == sum(len(p["files"]) for p in manifest["projects"])
    keys = set()
    for case in report["cases"]:
        key = (case["operation"], case["size"])
        assert key not in keys, key
        keys.add(key)
        samples = case["samples_ms"]
        assert len(samples) == report["samples_per_case"]
        assert all(isinstance(v, (float, int)) and math.isfinite(v) and v > 0 for v in samples)
        assert case["median_ms"] == statistics.median(samples)
        assert case["p95_ms"] == sorted(samples)[math.ceil(.95 * len(samples))-1]
        assert case["min_ms"] == min(samples) and case["max_ms"] == max(samples)
        assert len(case["validation"]) == len(samples) and all(case["validation"])
        row_values = f"| {case['size']:,} | {case['median_ms']:.2f} | {case['p95_ms']:.2f} |"
        for language in ["README.md", "README.zh-CN.md"]:
            assert row_values in (root / "benchmarks" / language).read_text(encoding="utf-8"), (key, language)
    print(f"{report['project']}: {len(keys)} cases, statistics and bilingual reports verified")
for name in ["README.md", "README.zh-CN.md"]:
    text = (root / name).read_text(encoding="utf-8")
    assert "Microsoft 365" in text and "WPS Office" in text
    assert "benchmarks/README" in text
    assert "<!-- BENCHMARK:START -->\n<!-- BENCHMARK:END -->" not in text
