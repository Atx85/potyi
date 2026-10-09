#!/usr/bin/env python3
"""Paired macOS CPU/RSS checks of frozen release scaling probes."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import statistics
import subprocess
import tempfile

PROBE = "benchmarks::scaling::sustained_editing_probe"
MARKER = "POTYI_SCALING "


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("before", type=Path)
    parser.add_argument("after", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--no-render", action="store_true", help="Isolate text indexing from SDL/font allocations")
    args = parser.parse_args()
    if platform.system() != "Darwin":
        parser.error("This evidence runner uses macOS /usr/bin/time -l")
    binaries = {"before": args.before.resolve(), "after": args.after.resolve()}
    identities = {label: digest(binary) for label, binary in binaries.items()}
    destination = args.output.resolve()
    destination.mkdir(parents=True, exist_ok=False)
    report = {"platform": platform.platform(), "binary_sha256": identities,
              "case": {"bytes": 1_000_000_000, "line_bytes": 64,
                       "position_percent": 99, "panes": 1, "history_entries": 0,
                       "fragmented_edits": 0, "recovery": False, "render": not args.no_render,
                       "iterations": 4}, "runs": [],
              "limits": ["Freshly written warm fixtures; headless SDL, not native input/display latency.",
                         "Whole-process CPU and peak RSS include fixture creation, SDL initialization, navigation and four edit cycles.",
                         "One discarded pair and three measured pairs, alternating order; no concurrent builds or tests.",
                         "RSS differences near a few pages are noise, not an allocation budget or universal guarantee."]}
    for repetition in range(4):
        order = ("before", "after") if repetition % 2 == 0 else ("after", "before")
        for label in order:
            assert digest(binaries[label]) == identities[label]
            logfile = destination / f"pair{repetition}-{label}.log"
            with tempfile.TemporaryDirectory(prefix="potyi-distant-resources-") as folder:
                case = {**report["case"], "fixture_dir": str(Path(folder) / "fixture")}
                env = {**os.environ, "SDL_VIDEODRIVER": "dummy",
                       "POTYI_SCALING_CASE": json.dumps(case)}
                with logfile.open("w") as log:
                    completed = subprocess.run(
                        ["/usr/bin/time", "-l", str(binaries[label]), PROBE,
                         "--exact", "--ignored", "--nocapture", "--test-threads=1"],
                        env=env, stdout=log, stderr=subprocess.STDOUT, timeout=30)
            raw = logfile.read_text()
            rows = [json.loads(line.split(MARKER, 1)[1]) for line in raw.splitlines() if MARKER in line]
            stats = re.search(r"([\d.]+) real\s+([\d.]+) user\s+([\d.]+) sys", raw)
            rss = re.search(r"(\d+)\s+maximum resident set size", raw)
            assert completed.returncode == 0 and any(r.get("type") == "complete" for r in rows), raw
            assert stats and rss, "macOS CPU/RSS statistics unavailable; retain raw log"
            navigation = next(r for r in rows if r.get("stage") == "navigation")
            record = {"variant": label, "repetition": repetition, "warmup": repetition == 0,
                      "navigation_ms": navigation["milliseconds"],
                      "cpu_seconds": float(stats[2]) + float(stats[3]),
                      "wall_seconds": float(stats[1]), "peak_rss_bytes": int(rss[1]),
                      "counters": navigation["counters"],
                      "index_metadata": next(r["line_index_metadata"] for r in rows if r.get("type") == "complete"),
                      "log": logfile.name}
            report["runs"].append(record)
            (destination / "results.json").write_text(json.dumps(report, indent=2) + "\n")
            print(f"{label} pair {repetition}: {record['navigation_ms']:.1f} ms jump, "
                  f"{record['cpu_seconds']:.2f} s process CPU, "
                  f"{record['peak_rss_bytes'] / 1048576:.2f} MiB peak RSS", flush=True)
    report["medians"] = {}
    for label in binaries:
        samples = [r for r in report["runs"] if r["variant"] == label and not r["warmup"]]
        report["medians"][label] = {key: statistics.median(r[key] for r in samples)
                                    for key in ("navigation_ms", "cpu_seconds", "wall_seconds", "peak_rss_bytes")}
        assert digest(binaries[label]) == identities[label]
    (destination / "results.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report["medians"], indent=2))


if __name__ == "__main__":
    main()
