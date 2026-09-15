#!/usr/bin/env python3
"""Measure full source-copy verification, including Python/Git startup and disk reads."""
import argparse
import datetime
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import statistics
import subprocess
import sys
import time

PROJECT = "OmniDoc"

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--samples", type=int, default=7)
    parser.add_argument("--warmups", type=int, default=2)
    parser.add_argument("--output", default="benchmarks/results/local.json")
    args = parser.parse_args()
    if args.samples < 1 or args.warmups < 0:
        parser.error("samples must be positive and warmups nonnegative")
    manifest = json.loads((ROOT/"sources.lock.json").read_text(encoding="utf-8"))
    files = sum(len(p["files"]) for p in manifest["projects"])
    size = sum((ROOT/p["directory"]/f).stat().st_size for p in manifest["projects"] for f in p["files"])
    values, validations = [], []
    flags = subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0
    for i in range(args.warmups + args.samples):
        start = time.perf_counter_ns()
        result = subprocess.run([sys.executable, "tools/verify_copies.py"], cwd=ROOT,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True, creationflags=flags)
        elapsed = (time.perf_counter_ns()-start)/1e6
        assert b"All copied sources verified" in result.stdout
        if i >= args.warmups:
            values.append(elapsed)
            validations.append({"files_checked": files, "bytes_checked": size, "exit_code": result.returncode})
    env = {"os": platform.system(), "os_version": platform.version(), "architecture": platform.machine(),
        "python": platform.python_version(), "logical_cpus": os.cpu_count(),
        "source_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "harness_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    if os.name == "nt":
        query = "$c=Get-CimInstance Win32_Processor; $o=Get-CimInstance Win32_OperatingSystem; @{cpu=$c.Name; physical_cores=$c.NumberOfCores; ram_gib=[math]::Round($o.TotalVisibleMemorySize/1MB,2); os_caption=$o.Caption} | ConvertTo-Json -Compress"
        env.update(json.loads(subprocess.check_output(["powershell", "-NoProfile", "-Command", query], text=True, encoding="utf-8")))
    record = {"schema_version": 1, "project": PROJECT, "environment": env,
        "measured_at_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "source_snapshot_commits": {p["name"]: p["commit"] for p in manifest["projects"]},
        "manifest_sha256": hashlib.sha256((ROOT/"sources.lock.json").read_bytes()).hexdigest(),
        "files": files, "source_bytes": size, "warmups_per_case": args.warmups,
        "samples_per_case": args.samples, "concurrency": 1,
        "p95_method": "nearest rank: sorted[ceil(0.95*n)-1]",
        "timing_boundary": "Python and Git subprocess startup, indexed file inventory, file reads and SHA-256 verification; warm OS cache; excludes network and app runtime",
        "cases": [{"operation": "verify_copies", "size": files, "samples_ms": values,
            "median_ms": statistics.median(values), "p95_ms": sorted(values)[math.ceil(.95*len(values))-1],
            "min_ms": min(values), "max_ms": max(values), "validation": validations}]}
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(record, indent=2, ensure_ascii=False)+"\n", encoding="utf-8", newline="\n")
    print(f"{PROJECT}: {files} files / {size} bytes, median {statistics.median(values):.2f} ms, all checks passed")


if __name__ == "__main__":
    main()
