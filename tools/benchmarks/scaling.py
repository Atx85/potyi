#!/usr/bin/env python3
"""Measure real editor operations; retain bounded failures and raw samples."""
import argparse
import datetime as dt
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import shutil
import signal
import statistics
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
PROBE = "benchmarks::scaling::sustained_editing_probe"
MARKER = "POTYI_SCALING "


def output(command):
    return subprocess.check_output(command, cwd=ROOT, text=True).strip()


def source_hash():
    # Include native patches, configuration and embedded assets as well as Rust.
    files = {ROOT / name for name in ("Cargo.toml", "Cargo.lock", "build.rs")}
    for folder in ("src", "vendor", "config", "assets", "fonts"):
        files.update(p for p in (ROOT / folder).rglob("*") if p.is_file())
    digest = hashlib.sha256()
    for path in sorted(files):
        digest.update(str(path.relative_to(ROOT)).encode() + b"\0")
        digest.update(path.read_bytes())
    return digest.hexdigest()


def build(profile, destination):
    before = source_hash()
    command = ["cargo", "test", "--offline", "--locked", "--bin", "potyi",
               "--no-run", "--message-format=json"]
    if profile == "release":
        command.append("--release")
    with (destination / "build.jsonl").open("w") as stdout, (destination / "build.log").open("w") as stderr:
        subprocess.run(command, cwd=ROOT, stdout=stdout, stderr=stderr, check=True)
    artifacts = [json.loads(line) for line in (destination / "build.jsonl").read_text().splitlines()
                 if line.startswith("{")]
    binaries = [m["executable"] for m in artifacts if m.get("reason") == "compiler-artifact"
                and m.get("profile", {}).get("test") and m.get("executable")
                and m["target"]["name"] == "potyi"]
    if len(binaries) != 1:
        raise RuntimeError("Expected exactly one editor test executable")
    # Preserve this executable if another LLM builds in the same checkout.
    binary = destination / "probe"
    shutil.copy2(binaries[0], binary)
    after = source_hash()
    return binary, {"before_build": before, "after_build": after,
                    "stable_during_build": before == after,
                    "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest()}


def events(text):
    rows = []
    lines = text.splitlines()
    for index, line in enumerate(lines):
        if MARKER in line:
            # libtest can put its test-name prefix on the first output line.
            try:
                rows.append(json.loads(line.split(MARKER, 1)[1]))
            except json.JSONDecodeError:
                if index == len(lines) - 1 and not text.endswith("\n"):
                    continue  # A killed process can leave its final write incomplete.
                raise
    return rows


def kill_owned_group(process):
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass  # The child may have exited between polling and termination.
    except PermissionError:
        # Some sandboxes allow terminating our child but deny group signals.
        # The scaling probe does not spawn descendants.
        try:
            process.kill()
        except ProcessLookupError:
            pass


def bounded(command, env, logfile, timeout, memory=True):
    """Kill only our own process group; keep stdout after failure or timeout."""
    start = time.monotonic()
    peak = None
    memory_error = None
    timed_out = False
    with logfile.open("w") as log:
        process = subprocess.Popen(command, cwd=ROOT, env=env, stdout=log,
                                   stderr=subprocess.STDOUT, start_new_session=True)
        try:
            while process.poll() is None:
                if time.monotonic() - start >= timeout:
                    timed_out = True
                    kill_owned_group(process)
                    break
                if memory and memory_error is None:
                    try:
                        rss = subprocess.run(["ps", "-p", str(process.pid), "-o", "rss="],
                                             capture_output=True, text=True, timeout=1)
                        if rss.returncode == 0 and rss.stdout.strip():
                            peak = max(peak or 0, int(rss.stdout.strip()))
                        elif process.poll() is None:
                            memory_error = rss.stderr.strip() or "ps returned no resident-memory sample"
                    except (OSError, ValueError, subprocess.TimeoutExpired) as error:
                        memory_error = str(error)
                time.sleep(0.1)
        finally:
            if process.poll() is None:
                kill_owned_group(process)
            process.wait()
    rows = events(logfile.read_text())
    complete = any(row.get("type") == "complete" for row in rows)
    status = "timed_out" if timed_out else ("complete" if process.returncode == 0 and complete else "failed")
    phases = [row["name"] for row in rows if row.get("type") == "phase"]
    return {"status": status, "exit_code": process.returncode, "seconds": time.monotonic() - start,
            "last_phase": phases[-1] if phases else None, "events": rows,
            "sampled_peak_rss_kib": peak, "memory_unavailable": memory_error, "log": logfile.name}


def matrix(suite, iterations, render):
    base = dict(history_entries=0, fragmented_edits=0, panes=1, recovery=False,
                render=render, iterations=iterations)
    if suite == "fragments":
        return [{**base, "name": f"1mib-short-middle-fragments{edits}",
                 "bytes": 1024 * 1024, "line_bytes": 64, "position_percent": 50,
                 "fragmented_edits": edits} for edits in (1000, 10_000, 100_000)]
    cases = []
    sizes = [1024 * 1024] if suite == "quick" else [1024 * 1024, 100 * 1024 * 1024, 1_000_000_000]
    for size in sizes:
        for line in (64, 1024 * 1024):
            for position in (0, 50, 99):
                name = f"{size}b-line{line}-pos{position}"
                cases.append({**base, "name": name, "bytes": size, "line_bytes": line,
                              "position_percent": position})
    # Controlled comparisons: change one dimension at a time, then combine.
    variants = [dict(panes=2), dict(panes=2, history_entries=1000),
                dict(fragmented_edits=1000), dict(recovery=True),
                dict(panes=2, history_entries=1000, fragmented_edits=1000, recovery=True)]
    for index, variant in enumerate(variants):
        cases.append({**base, "name": f"1mib-short-middle-variant{index + 1}",
                      "bytes": 1024 * 1024, "line_bytes": 64, "position_percent": 50, **variant})
    return cases


def percentile(values, fraction):
    ordered = sorted(values)
    return ordered[max(0, math.ceil(len(ordered) * fraction) - 1)]


def summarize(runs):
    groups = {}
    for run in runs:
        if run["warmup"] or run["status"] != "complete":
            continue
        for row in run["events"]:
            if row.get("type") != "measurement":
                continue
            bucket = "setup" if row["iteration"] is None else ("first" if row["iteration"] == 0 else "later")
            groups.setdefault((row["stage"], bucket), []).append(row)
    result = []
    for (stage, bucket), rows in sorted(groups.items()):
        times = [row["milliseconds"] for row in rows]
        counters = {key: {"median": statistics.median(row["counters"][key] for row in rows),
                          "max": max(row["counters"][key] for row in rows)} for key in rows[0]["counters"]}
        result.append(dict(stage=stage, bucket=bucket, samples=len(times), median_ms=statistics.median(times),
                           p95_ms=percentile(times, .95), p99_ms=percentile(times, .99), counters=counters))
    return result


def markdown(report):
    lines = ["# Sustained editing baseline", "", f"Recorded: {report['measured_at']}.", "",
             f"Profile: **{report['environment']['profile']}**. OS: {report['environment']['os']}.",
             f"Source stable during build: **{report['source']['stable_during_build']}**.", "",
             "Times are milliseconds. First and later edits are separate; setup includes navigation and initial drawing.",
             "Only complete measured runs contribute to medians. Failed and timed-out runs retain partial events in JSON.", "",
             "| Case | Completed measured runs | Unfinished runs | Last unfinished phase | Sampled RSS maximum, MiB |",
             "| --- | ---: | --- | --- | ---: |"]
    for case in report["cases"]:
        runs = case["runs"]
        good = sum(r["status"] == "complete" and not r["warmup"] for r in runs)
        bad = [r for r in runs if r["status"] != "complete"]
        rss = [r["sampled_peak_rss_kib"] / 1024 for r in runs if r["sampled_peak_rss_kib"] is not None]
        lines.append(f"| {case['config']['name']} | {good} | {', '.join(r['status'] for r in bad) or 'none'} | "
                     f"{', '.join(r['last_phase'] or 'unknown' for r in bad) or '—'} | {max(rss):.1f} |" if rss else
                     f"| {case['config']['name']} | {good} | {', '.join(r['status'] for r in bad) or 'none'} | "
                     f"{', '.join(r['last_phase'] or 'unknown' for r in bad) or '—'} | unavailable |")
    for case in report["cases"]:
        if not case["summary"]:
            continue
        lines += ["", f"## {case['config']['name']}", "",
                  "| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |",
                  "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |"]
        for row in case["summary"]:
            c = row["counters"]
            copied = " / ".join(str(c[key]["median"]) for key in
                                ("copied_piece_records", "copied_line_records", "copied_history_entries"))
            lines.append(f"| {row['stage']} | {row['bucket']} | {row['samples']} | {row['median_ms']:.3f} | "
                         f"{row['p95_ms']:.3f} | {row['p99_ms']:.3f} | {c['read_bytes']['median']} | "
                         f"{c['line_discoveries']['median']} | {copied} |")
    lines += ["", "## Limits", ""] + [f"- {limit}" for limit in report["limitations"]]
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", choices=("dev", "release"), default="release")
    parser.add_argument("--suite", choices=("quick", "full", "fragments"), default="full")
    parser.add_argument("--samples", type=int, default=3)
    parser.add_argument("--warmups", type=int, default=1)
    parser.add_argument("--iterations", type=int, default=8)
    parser.add_argument("--timeout", type=float, default=30)
    parser.add_argument("--no-render", action="store_true")
    parser.add_argument("--no-memory", action="store_true")
    parser.add_argument("--only", help="Case-name substring filter")
    parser.add_argument("--output", type=Path, default=ROOT / "target/qa" /
                        ("scaling-" + dt.datetime.now(dt.timezone.utc).strftime("%Y%m%d-%H%M%S")))
    args = parser.parse_args()
    if os.name != "posix":
        parser.error("Runner currently supports macOS/Linux")
    if args.samples < 3 or args.warmups < 1 or not 2 <= args.iterations <= 1000 or args.timeout <= 0:
        parser.error("Use >=3 samples, >=1 warmup, 2..1000 iterations and a positive timeout")
    cases = [c for c in matrix(args.suite, args.iterations, not args.no_render)
             if not args.only or args.only in c["name"]]
    if not cases:
        parser.error("Case filter matched no workloads")
    destination = args.output.resolve()
    destination.mkdir(parents=True, exist_ok=False)
    print(f"Building {args.profile} probe; logs: {destination}", flush=True)
    binary, identity = build(args.profile, destination)
    if not identity["stable_during_build"]:
        print("Source changed during build; retaining hashes and flagging provenance as inconclusive.", flush=True)
    report = dict(schema_version=1, measured_at=dt.datetime.now(dt.timezone.utc).isoformat(), source=identity,
                  git_commit=output(["git", "rev-parse", "HEAD"]),
                  working_tree_dirty=bool(output(["git", "status", "--porcelain"])),
                  environment=dict(os=platform.platform(), cpu=platform.processor(), logical_cpus=os.cpu_count(),
                                   rustc=output(["rustc", "-vV"]), profile=args.profile,
                                   sdl_video_driver="dummy", window="800x600" if not args.no_render else None),
                  settings=dict(samples=args.samples, warmups=args.warmups, iterations=args.iterations,
                                timeout_seconds=args.timeout, suite=args.suite), cases=[],
                  limitations=["Local measurements with freshly written, warm filesystem fixtures; no cold-disk or other-editor comparison.",
                               "Headless rendering measures SDL draw work, not native desktop presentation, input dispatch or LSP work.",
                               "RSS is sampled about every 100ms across fixture/setup/editing, not true peak memory or an allocation budget; ps may be unavailable.",
                               "Read counters count successful positional file reads in the piece table, including rereads; not physical disk I/O or recovery writes.",
                               "Line discoveries count discovery calls, including reconstructed evicted entries and EOF handling; not unique lines.",
                               "Copy counters count top-level records at view/history cloning sites, not bytes or deep allocations in undo snapshots.",
                               "p95/p99 use nearest ranks; small sample counts and correlated operations limit tail claims.",
                               "The first edit and later edits are reported separately. Timeout includes fixture and setup; partial timings remain in raw events.",
                               "Unfinished warmups stop that case's remaining runs; no timing threshold determines correctness.",
                               "Concurrent checkout changes after building cannot alter the saved probe. Different before/after build hashes make source provenance inconclusive."])
    for index, config in enumerate(cases):
        runs = []
        for repetition in range(args.warmups + args.samples):
            with tempfile.TemporaryDirectory(prefix="potyi-scaling-") as folder:
                current = {**config, "fixture_dir": str(Path(folder) / "fixture")}
                env = {**os.environ, "SDL_VIDEODRIVER": "dummy", "POTYI_SCALING_CASE": json.dumps(current)}
                logfile = destination / f"case{index:02}-run{repetition:02}.log"
                row = bounded([str(binary), PROBE, "--exact", "--ignored", "--nocapture", "--test-threads=1"],
                              env, logfile, args.timeout, not args.no_memory)
            row.update(warmup=repetition < args.warmups, repetition=repetition)
            runs.append(row)
            print(f"{config['name']} run {repetition + 1}: {row['status']} ({row['seconds']:.2f}s)", flush=True)
            if row["status"] != "complete":
                break
        report["cases"].append(dict(config=config, runs=runs, summary=summarize(runs)))
        (destination / "results.json").write_text(json.dumps(report, indent=2) + "\n")
        (destination / "summary.md").write_text(markdown(report))
    print(f"Saved {destination / 'summary.md'}", flush=True)
    return 1 if any(r["status"] == "failed" for c in report["cases"] for r in c["runs"]) else 0


if __name__ == "__main__":
    raise SystemExit(main())
