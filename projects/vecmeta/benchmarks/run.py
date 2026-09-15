#!/usr/bin/env python3
"""Reproducible, dependency-free local benchmark. See README.md in this folder."""
import argparse
import contextlib
import datetime
import hashlib
import http.cookiejar
import io
import json
import math
import os
from pathlib import Path
import platform
import socket
import statistics
import struct
import subprocess
import sys
import tempfile
import time
import urllib.request
import xml.etree.ElementTree as ET
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def digest(data):
    return hashlib.sha256(data).hexdigest()


def command(args, **kwargs):
    return subprocess.check_output(args, text=True, encoding="utf-8", **kwargs).strip()


def environment(binary):
    info = {"os": platform.system(), "os_release": platform.release(),
            "os_version": platform.version(), "architecture": platform.machine(),
            "logical_cpus": os.cpu_count(), "python": platform.python_version(),
            "rustc": command(["rustc", "--version"]),
            "source_commit": command(["git", "rev-parse", "HEAD"], cwd=ROOT),
            "binary": binary.name, "binary_bytes": binary.stat().st_size,
            "binary_sha256": digest(binary.read_bytes()),
            "harness_sha256": digest(Path(__file__).read_bytes())}
    if os.name == "nt":
        script = "$c=Get-CimInstance Win32_Processor; $o=Get-CimInstance Win32_OperatingSystem; @{cpu=$c.Name; physical_cores=$c.NumberOfCores; ram_gib=[math]::Round($o.TotalVisibleMemorySize/1MB,2); os_caption=$o.Caption} | ConvertTo-Json -Compress"
        info.update(json.loads(command(["powershell", "-NoProfile", "-Command", script])))
    else:
        info["cpu"] = platform.processor() or "not reported"
    return info


def summary(samples):
    ordered = sorted(samples)
    return {"samples_ms": samples, "median_ms": statistics.median(samples),
            "p95_ms": ordered[math.ceil(len(ordered) * .95) - 1],
            "min_ms": min(samples), "max_ms": max(samples)}


def measure(name, size, args, operation, validate, prepare=None):
    samples, metadata = [], []
    for i in range(args.warmups + args.samples):
        if prepare:
            prepare(i)
        start = time.perf_counter_ns()
        result = operation(i)
        elapsed = (time.perf_counter_ns() - start) / 1e6
        checked = validate(result, i)
        if i >= args.warmups:
            samples.append(elapsed)
            metadata.append(checked)
    record = {"operation": name, "size": size, **summary(samples), "validation": metadata}
    print(f"{name} size={size}: median={record['median_ms']:.3f} ms p95={record['p95_ms']:.3f} ms", flush=True)
    return record


class Client:
    def __init__(self, base):
        self.base = base
        self.opener = urllib.request.build_opener(urllib.request.ProxyHandler({}),
            urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))

    def request(self, path, data=None, content_type="application/json"):
        req = urllib.request.Request(self.base + path, data=data,
            headers={"Content-Type": content_type, "X-UniPPT-Filename": "benchmark.pptx"})
        with self.opener.open(req, timeout=180) as response:
            return response.read(), dict(response.headers)

    def json(self, path, obj=None):
        data = None if obj is None else json.dumps(obj, separators=(",", ":")).encode()
        return json.loads(self.request(path, data)[0])


@contextlib.contextmanager
def server(binary, variable, ready):
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        port = sock.getsockname()[1]
    env = dict(os.environ, **{variable: str(port)})
    flags = subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0
    with tempfile.TemporaryFile() as log:
        child = subprocess.Popen([str(binary)], cwd=ROOT, env=env,
            stdout=log, stderr=log, creationflags=flags)
        client = Client(f"http://127.0.0.1:{port}")
        try:
            deadline = time.monotonic() + 90
            while time.monotonic() < deadline:
                if child.poll() is not None:
                    raise RuntimeError("Benchmark server exited during startup")
                try:
                    client.request(ready)
                    break
                except OSError:
                    time.sleep(.1)
            else:
                raise RuntimeError("Benchmark server did not become ready")
            yield client
        finally:
            child.terminate()
            try:
                child.wait(timeout=10)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait()


def finish(args, binary, cases, **extra):
    report = {"schema_version": 1, "project": PROJECT,
              "measured_at_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
              "environment": environment(binary), "build_profile": PROFILE,
              "warmups_per_case": args.warmups, "samples_per_case": args.samples,
              "p95_method": "nearest rank: sorted[ceil(0.95*n)-1]",
              "concurrency": 1, "cases": cases, **extra}
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"Saved {output.name}; {len(cases)} cases passed", flush=True)


def arguments(default_sizes):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, help="Path to a release executable")
    parser.add_argument("--output", default="benchmarks/results/local.json")
    parser.add_argument("--samples", type=int, default=7)
    parser.add_argument("--warmups", type=int, default=2)
    parser.add_argument("--sizes", default=default_sizes)
    args = parser.parse_args()
    if args.samples < 1 or args.warmups < 0:
        parser.error("samples must be positive and warmups nonnegative")
    args.sizes = [int(n) for n in args.sizes.split(",")]
    if not args.sizes or any(n < 1 for n in args.sizes):
        parser.error("sizes must be positive integers")
    binary = Path(args.binary).resolve(strict=True)
    return args, binary

PROJECT = "vecmeta"
PROFILE = "cargo build --release --locked -p emfsvg-cli; default opt-level=3, lto=true"


def svg_fixture(size):
    shapes = []
    for i in range(size):
        x, y = i % 100 * 10, i // 100 * 10
        color = f"#{(i*7919) % 16777216:06x}"
        if i % 2 == 0:
            shapes.append(f'<rect x="{x}" y="{y}" width="8" height="8" fill="{color}"/>')
        else:
            shapes.append(f'<path d="M{x},{y} l8,0 l-4,8 Z" fill="{color}"/>')
    return (f'<svg xmlns="http://www.w3.org/2000/svg" width="1000" height="{max(10, math.ceil(size/100)*10)}">'
            + "".join(shapes) + "</svg>").encode()


def emf_check(path):
    data = path.read_bytes()
    assert len(data) >= 88
    assert struct.unpack_from("<I", data, 0)[0] == 1
    assert data[40:44] == b" EMF"
    assert struct.unpack_from("<I", data, 48)[0] == len(data)
    return {"bytes": len(data), "sha256": digest(data), "emf_header_checked": True}


def main():
    args, binary = arguments("100,1000,10000")
    cases = []
    flags = subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0
    with tempfile.TemporaryDirectory(prefix="vecmeta-benchmark-") as tmp:
        work = Path(tmp)
        source, emf, svg = work/"input.svg", work/"output.emf", work/"output.svg"
        def run(*parts):
            return subprocess.run([str(binary), *map(str, parts)], stdout=subprocess.PIPE,
                stderr=subprocess.PIPE, check=True, creationflags=flags).stdout
        for size in args.sizes:
            data = svg_fixture(size)
            source.write_bytes(data)
            cases.append(measure("svg_to_emf_cli", size, args,
                lambda i: run("to-emf", source, "-o", emf),
                lambda r, i: {**emf_check(emf), "input_bytes": len(data), "input_sha256": digest(data)}))
            def check_svg(result, i):
                output = svg.read_bytes()
                root = ET.fromstring(output)
                assert root.tag == "{http://www.w3.org/2000/svg}svg"
                count = len(root.findall(".//{http://www.w3.org/2000/svg}path"))
                assert count == size, (count, size)
                return {"bytes": len(output), "sha256": digest(output), "paths": count}
            cases.append(measure("emf_to_svg_cli", size, args,
                lambda i: run("to-svg", emf, "-o", svg), check_svg))
            def geometry(result, i):
                message = result.decode().strip()
                assert "fixed-point geometry: OK" in message, message
                return {"geometry_check": message, "tolerance": 1e-6}
            cases.append(measure("geometry_roundtrip_cli", size, args,
                lambda i: run("roundtrip", emf, "--tolerance", "1e-6"), geometry))
    finish(args, binary, cases, workload="Alternating solid rectangles and triangle paths; no text/fonts, images, gradients or source encapsulation",
           timing_boundary="Whole CLI process: startup, file I/O, conversion and process exit; output validation excluded; warm OS file cache",
           lossless_source_embedding=False)


if __name__ == "__main__":
    main()
