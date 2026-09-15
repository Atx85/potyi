#!/usr/bin/env python3
"""Measure first-edit and subsequent edit-handler latency with recovery on/off."""
import argparse
import datetime as dt
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import re
import statistics
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
PROBE = "benchmarks::recovery_latency::typing_latency_probe"


def stats(values):
    ordered = sorted(values)
    percentile = lambda q: ordered[max(0, math.ceil(q * len(ordered))-1)]
    return {"n": len(values), "min_ms": min(values), "median_ms": statistics.median(values),
            "p95_ms": percentile(.95), "p99_ms": percentile(.99), "max_ms": max(values),
            "over_16_7_ms": sum(v > 1000/60 for v in values),
            "over_50_ms": sum(v > 50 for v in values), "over_100_ms": sum(v > 100 for v in values)}


def write_summary(report, output):
    summary = report["summary"]
    rust_host = next((line.removeprefix("host: ") for line in report["rustc"].splitlines() if line.startswith("host: ")), "unknown")
    sizes = sorted({row["bytes"] for row in summary})
    lines = ["# Crash-recovery typing latency\n\n",
             "First-edit timings include creation of the immutable recovery backup and initial checkpoint. Later edits pay synchronous journal costs without copying the original again.\n\n",
             "## First edit\n\n",
             "Medians across fresh editing sessions; parentheses show the recovery-on minimum–maximum.\n\n",
             "| File | Recovery off | Recovery on | Later insert p99, recovery on |\n| --- | ---: | ---: | ---: |\n"]
    for size in sizes:
        off = next(r for r in summary if r["bytes"] == size and not r["recovery"])
        on = next(r for r in summary if r["bytes"] == size and r["recovery"])
        first = on["stages"]["first_ms"]
        label = "Empty" if size == 0 else f"{size//1048576} MiB"
        lines.append(f"| {label} | {off['stages']['first_ms']['median_ms']:.2f} ms | {first['median_ms']:.2f} ms ({first['min_ms']:.2f}–{first['max_ms']:.2f}) | {on['stages']['insert_ms']['p99_ms']:.2f} ms |\n")
    lines.extend(["\n## Subsequent edits\n\n",
                  "Pooled across file sizes and repetitions; p99 is the nearest-rank 99th percentile.\n\n",
                  "| Operation | Off median | On median | On p99 | On worst | On >50 ms |\n| --- | ---: | ---: | ---: | ---: | ---: |\n"])
    for key, label in [("insert_ms", "Burst insert"), ("backspace_ms", "Backspace"),
                       ("replace_ms", "Replace selection"), ("paced_ms", "Paced typing")]:
        off = stats([v for row in report["raw"] if not row["recovery"] for v in row[key]])
        on = stats([v for row in report["raw"] if row["recovery"] for v in row[key]])
        lines.append(f"| {label} | {off['median_ms']:.3f} ms | {on['median_ms']:.2f} ms | {on['p99_ms']:.2f} ms | {on['max_ms']:.2f} ms | {on['over_50_ms']}/{on['n']} |\n")
    after = stats([r["after_save_ms"] for r in report["raw"] if r["recovery"]])
    lines.append(f"\nThe first edit after Save took a median **{after['median_ms']:.2f} ms**, worst **{after['max_ms']:.2f} ms** with recovery. It did not repeat the initial large-file backup. Save itself was timed separately in the raw data.\n\n")
    if all(r.get("peak_rss_bytes") for r in report["raw"]):
        largest = max(sizes)
        off = statistics.median(r["peak_rss_bytes"] for r in report["raw"] if r["bytes"] == largest and not r["recovery"])
        on = statistics.median(r["peak_rss_bytes"] for r in report["raw"] if r["bytes"] == largest and r["recovery"])
        lines.append(f"For the largest file, median peak RSS was **{off/1048576:.2f} MiB off** and **{on/1048576:.2f} MiB on**: **{(on-off)/1048576:.2f} MiB extra** in the isolated test process. This is not graphical-app RSS.\n\n")
    lines.extend(["## Interpretation and method\n\n",
                  "The first-edit delays are synchronous periods during which the main event loop cannot proceed to rendering. Filesystem copy-on-write support can avoid streaming the original; unsupported filesystems still require the bounded copy. Later journal synchronization is a separate roughly per-edit cost; selection replacement currently records deletion and insertion separately.\n\n",
                  f"Run: {report['utc']}; {report['host']}; Rust host `{rust_host}`; release build; {report['repetitions']} repetitions per size/mode, alternating on/off order. All probe assertions passed, including text lengths, saved output length, undo/redo and absence of recovery warnings.\n\n"])
    lines.extend("- "+limit+"\n" for limit in report["limitations"])
    lines.extend(["\n[Raw samples, distributions, source fingerprint and log filenames](results.json).\n\n",
                  "Reproduce from the repository root: `python3 tools/qa/recovery_latency.py --output docs/testing/NEW_RECOVERY_RUN`. On macOS the timing tool may require permission to read process resource statistics.\n"])
    (output/"summary.md").write_text("".join(lines))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--sizes-mib", default="0,1,100,1024")
    parser.add_argument("--report-only", action="store_true", help="Regenerate summary.md from existing results.json without running probes")
    args = parser.parse_args()
    if args.report_only:
        write_summary(json.loads((args.output/"results.json").read_text()), args.output)
        return
    sizes = [int(n) * 1024 * 1024 for n in args.sizes_mib.split(",")]
    if args.repetitions < 3 or any(n < 0 or n > 1024**3 for n in sizes):
        parser.error("Use at least three repetitions and sizes between 0 and 1024 MiB")
    args.output.mkdir(parents=True, exist_ok=True)
    build = subprocess.run(["cargo", "test", "--release", "--locked", "--bin", "potyi", "--no-run", "--message-format=json"],
                           cwd=ROOT, text=True, capture_output=True, check=True)
    (args.output/"build.log").write_text(build.stdout+build.stderr)
    binary = next(d["executable"] for line in build.stdout.splitlines() if line.startswith("{")
                  for d in [json.loads(line)] if d.get("executable"))
    fingerprint = hashlib.sha256()
    for path in sorted([ROOT/"Cargo.toml", ROOT/"Cargo.lock", *ROOT.glob("src/**/*.rs")]):
        fingerprint.update(str(path.relative_to(ROOT)).encode()+b"\0"+path.read_bytes())
    report = {"utc": dt.datetime.now(dt.timezone.utc).isoformat(), "host": platform.platform(),
              "rustc": subprocess.check_output(["rustc", "-vV"], text=True), "profile": "release",
              "source_sha256": fingerprint.hexdigest(), "repetitions": args.repetitions,
              "raw": [], "limitations": [
                  "Synchronous Editor edit handlers, including history and recovery; no OS key events or rendering.",
                  "Physically written UTF-8 fixtures on the local temporary filesystem, warm OS cache.",
                  "Cursor near the start; first 40 lines accessed before timing; no full line indexing.",
                  "Fresh editor and recovery session per case; recovery order alternates per repetition.",
                  "Paced edits wait 50 ms after the previous dispatch; delay excluded from measured time.",
                  "First-edit statistics contain three samples by default; tail percentiles pool subsequent edits.",
                  "No claim about cold files, slow/network disks, Windows/Linux or native display latency.",
                  "16.7/50/100 ms are inspection thresholds; timings are not universal perception guarantees.",
              ]}
    for repetition in range(args.repetitions):
        for size in sizes:
            for mode in (["off", "on"] if repetition % 2 == 0 else ["on", "off"]):
                name = f"{size//1048576}mib-{mode}-{repetition+1}"
                env = dict(os.environ, POTYI_RECOVERY_LATENCY_BYTES=str(size), POTYI_RECOVERY_LATENCY_MODE=mode)
                command = [binary, PROBE, "--exact", "--ignored", "--nocapture", "--test-threads=1"]
                if platform.system() == "Darwin": command = ["/usr/bin/time", "-l", *command]
                start = time.monotonic()
                with (args.output/(name+".log")).open("w") as log:
                    result = subprocess.run(command, cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT, timeout=180)
                text = (args.output/(name+".log")).read_text(errors="replace")
                records = re.findall(r"^POTYI_RECOVERY_LATENCY (.+)$", text, re.M)
                if result.returncode != 0 or len(records) != 1:
                    raise RuntimeError(f"Probe failed: {name}; inspect its retained log")
                data = json.loads(records[0])
                data.update(repetition=repetition+1, log=name+".log", wall_seconds=time.monotonic()-start)
                rss = re.search(r"(\d+)\s+maximum resident set size", text)
                if rss: data["peak_rss_bytes"] = int(rss.group(1))
                report["raw"].append(data)
                (args.output/"results.json").write_text(json.dumps(report, indent=2)+"\n")
                print(f"{name}: first {data['first_ms']:.2f} ms, insert p99 {stats(data['insert_ms'])['p99_ms']:.2f} ms", flush=True)
    summary = []
    for size in sizes:
        for mode in [False, True]:
            rows = [r for r in report["raw"] if r["bytes"] == size and r["recovery"] == mode]
            stages = {stage: stats([v for row in rows for v in row[stage]])
                      for stage in ["insert_ms", "backspace_ms", "replace_ms", "paced_ms"]}
            stages.update({stage: stats([row[stage] for row in rows]) for stage in ["first_ms", "save_ms", "after_save_ms"]})
            summary.append({"bytes": size, "recovery": mode, "stages": stages,
                            "peak_rss_bytes": [r.get("peak_rss_bytes") for r in rows]})
    report["summary"] = summary
    (args.output/"results.json").write_text(json.dumps(report, indent=2)+"\n")
    write_summary(report, args.output)
    print("Completed all cases; raw samples and per-stage distributions saved.", flush=True)


if __name__ == "__main__":
    main()
