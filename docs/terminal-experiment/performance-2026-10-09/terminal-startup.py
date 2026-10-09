#!/usr/bin/env python3
"""Collect one first and five warmed commands in alternating frozen old/new runs.

No Cargo, process sampler, or performance-threshold assertion runs here.
Every input variant needs a frozen release build-provenance manifest.
"""
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

PROBE = "app::input::tests::terminal_startup_probe::matched_terminal_startup_probe"
PREFIX = "POTYI_TERM_STARTUP "
ALGORITHM = "sha256-relative-path-nul-file-v1"
DEFAULT_COMMAND = "echo POTYI_STARTUP_DONE" if os.name == "nt" else "printf '%s\\n' POTYI_STARTUP_DONE"


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def source_provenance(root):
    # Exact algorithm/input set shared with tools/qa/terminal-resources.py.
    root = Path(root).resolve()
    paths = {root / name for name in ("Cargo.toml", "Cargo.lock", "build.rs")}
    for name in ("src", ".cargo", "assets", "config", "fonts", "linux", "macos",
                 "windows", "vendor/vt100", "vendor/sdl3-sys", "vendor/sdl3-ttf-sys"):
        directory = root / name
        if directory.is_dir():
            paths.update(path for path in directory.rglob("*") if path.is_file()
                         and not {"target", ".git", ".DS_Store"}.intersection(path.relative_to(directory).parts))
    digest, files = hashlib.sha256(), []
    for path in sorted(paths, key=lambda path: path.relative_to(root).as_posix()):
        name = path.relative_to(root).as_posix()
        content = path.read_bytes()
        digest.update(name.encode() + b"\0" + content)
        files.append({"path": name, "sha256": hashlib.sha256(content).hexdigest()})
    return {"algorithm": ALGORITHM, "source_sha256": digest.hexdigest(), "files": files}


def architecture(binary):
    def command(args):
        try:
            result = subprocess.run(args, text=True, capture_output=True, timeout=10)
        except (OSError, subprocess.TimeoutExpired):
            return None
        return result.stdout.strip() if result.returncode == 0 else None
    result = {"os": platform.platform(), "collector_machine": platform.machine(),
              "binary_file": command(["file", "-b", str(binary)])}
    if platform.system() == "Darwin":
        result.update(hardware_machine=command(["sysctl", "-n", "hw.machine"]),
                      arm_hardware=command(["sysctl", "-n", "hw.optional.arm64"]) == "1",
                      collector_rosetta=command(["sysctl", "-n", "sysctl.proc_translated"]) == "1")
        description = result["binary_file"] or ""
        if result["arm_hardware"] and "x86_64" in description and "arm64" not in description:
            result["test_execution"] = "x86_64 release binary under Rosetta on ARM hardware"
        else:
            result["test_execution"] = "binary ISA from file; universal process ISA not established"
    else:
        result["test_execution"] = "binary ISA from file description"
    return result


def identity(variant):
    return {"source_sha256": source_provenance(variant["source_root"])["source_sha256"],
            "binary_sha256": sha(variant["binary"]), "helper_sha256": sha(variant["helper"]),
            "build_provenance_sha256": sha(variant["build_provenance"]),
            "probe_sha256": sha(Path(variant["source_root"]) / "src/app/input/terminal_startup_probe.rs")}


def load_variants(path):
    data = json.loads(path.read_text(encoding="utf-8"))
    variants = data["variants"]
    if not isinstance(variants, list) or not variants:
        raise ValueError("Configuration must contain a nonempty variants list")
    names, probe_hashes = set(), set()
    for variant in variants:
        name = variant["label"]
        if not re.fullmatch(r"[A-Za-z0-9_-]{1,48}", name) or name in names:
            raise ValueError("Variant labels must be unique simple filename components")
        names.add(name)
        for key in ("source_root", "binary", "helper", "build_provenance"):
            value = Path(variant[key])
            if not value.is_absolute():
                raise ValueError(f"{name}.{key} must be an absolute frozen-input path")
            variant[key] = str(value.resolve(strict=True))
        manifest = json.loads(Path(variant["build_provenance"]).read_text(encoding="utf-8"))
        current = identity(variant)
        if manifest.get("algorithm") != ALGORITHM:
            raise ValueError("Build manifest uses a different source identity algorithm")
        for key in ("source_sha256", "source_before_sha256", "source_after_sha256"):
            if manifest.get(key) != current["source_sha256"]:
                raise ValueError(f"{name}: build {key} differs from frozen instrumented source")
        for item in ("binary", "helper"):
            if manifest.get(item + "_sha256") != current[item + "_sha256"]:
                raise ValueError(f"{name}: {item} does not match its release build manifest")
            if Path(manifest.get(item + "_path", "")).resolve() != Path(variant[item]):
                raise ValueError(f"{name}: manifest {item} path differs from its immutable copy")
        if Path(manifest.get("source_root", "")).resolve() != Path(variant["source_root"]):
            raise ValueError(f"{name}: manifest source root differs from instrumented checkout")
        if manifest.get("profile") != "release" or not manifest.get("target") or not manifest.get("command"):
            raise ValueError("Manifest must include release profile, target, and exact build command")
        registration = (Path(variant["source_root"]) / "src/app/input/tests.rs").read_text(encoding="utf-8")
        if 'mod terminal_startup_probe;' not in registration:
            raise ValueError(f"{name}: standalone startup probe is not registered")
        variant.update(identity=current, build_manifest=manifest,
                       architecture=architecture(Path(variant["binary"])))
        probe_hashes.add(current["probe_sha256"])
    if len(probe_hashes) != 1:
        raise ValueError("Every instrumented variant must contain identical startup probe source")
    return variants


def stats(values):
    if not values:
        return None
    ordered = sorted(values)
    return {"count": len(values), "median_ms": statistics.median(values),
            "min_ms": min(values), "max_ms": max(values),
            "p95_nearest_rank_ms": ordered[math.ceil(.95 * len(ordered)) - 1]}


def validate_events(events, kind):
    contexts = [event for event in events if event.get("event") == "context"]
    if len(contexts) != 1 or contexts[0].get("kind") != kind or contexts[0].get("warm_commands") != 5:
        raise RuntimeError("Startup probe context missing or inconsistent")
    if any(event.get("kind") != kind for event in events):
        raise RuntimeError("Mixed old/new markers in one process")
    phases = ["pane_create", "clear", "first_command"] + [f"warm_command_{n:02}" for n in range(1, 6)]
    starts = [event for event in events if event["event"] == "phase"]
    ends = [event for event in events if event["event"] == "end"]
    if [event["phase"] for event in starts] != phases or [event["phase"] for event in ends] != phases:
        raise RuntimeError("Missing, repeated, or out-of-order measurement phases")
    if len([event for event in events if event["event"] == "done"]) != 1:
        raise RuntimeError("Probe did not publish one successful done marker")
    durations = {}
    for event in events:
        duration = event.get("duration_ms")
        if duration is not None and (not isinstance(duration, (int, float)) or not math.isfinite(duration) or duration < 0):
            raise RuntimeError("Invalid duration in probe event")
        if event["event"] in ("open_return", "viewport_settled"):
            durations[event["event"]] = duration
        elif event["event"] == "operation":
            durations[event["phase"]] = duration
        elif event["event"] == "submitted":
            durations[event["phase"] + "_submit"] = duration
    expected = {"open_return", "viewport_settled", "clear", "first_command", "first_command_submit"}
    expected.update(f"warm_command_{n:02}{suffix}" for n in range(1, 6) for suffix in ("", "_submit"))
    if set(durations) != expected or any(value is None for value in durations.values()):
        raise RuntimeError("Missing duration stages")
    ready_name = "session_ready" if kind == "new" else "legacy_open_ready"
    ready = [event for event in events if event["event"] == ready_name]
    if len(ready) != 1:
        raise RuntimeError("Missing first-observed readiness event")
    durations[ready_name] = ready[0]["elapsed_ms"] - starts[0]["elapsed_ms"]
    if durations[ready_name] < 0:
        raise RuntimeError("Readiness preceded pane-create start")
    return durations


def write_json(path, data):
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def run_case(variant, kind, repetition, fixture, args):
    name = f"{repetition:02}-{variant['label']}-{kind}"
    before = identity(variant)
    if before != variant["identity"]:
        raise RuntimeError(f"Frozen inputs changed before {name}")
    # Do not inherit earlier terminal/profiling overrides, helper authentication,
    # trace paths, or workload choices. PATH and the selected shell are retained.
    environment = {key: value for key, value in os.environ.items()
                   if not key.startswith("POTYI_TERM_") and key != "POTYI_EMBEDDED_PREVIEW"}
    environment.update(SDL_AUDIODRIVER="dummy", POTYI_TERM_TEST_CLIENT=variant["helper"],
                       POTYI_TERM_STARTUP_KIND=kind, POTYI_TERM_STARTUP_ROOT=str(fixture),
                       POTYI_TERM_STARTUP_VISIBLE="1" if args.visible else "0",
                       POTYI_TERM_STARTUP_HOLD_MS=str(args.hold_ms))
    if args.visible:
        environment.pop("SDL_VIDEODRIVER", None)
        environment.pop("SDL_RENDER_DRIVER", None)
        if args.video_driver:
            environment["SDL_VIDEODRIVER"] = args.video_driver
    else:
        environment.update(SDL_VIDEODRIVER="dummy", SDL_RENDER_DRIVER="software")
    if args.command is not None:
        environment["POTYI_TERM_STARTUP_COMMAND"] = args.command
    if args.expect_line is not None:
        environment["POTYI_TERM_STARTUP_EXPECT_LINE"] = args.expect_line
    command = [variant["binary"], PROBE, "--exact", "--ignored", "--nocapture", "--test-threads=1"]
    started = time.monotonic()
    try:
        process = subprocess.run(command, cwd=variant["source_root"], env=environment,
                                 stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                 timeout=args.timeout, check=False)
    except subprocess.TimeoutExpired as error:
        (args.output / (name + ".log")).write_bytes(error.stdout or b"")
        raise RuntimeError(f"{name} exceeded operational {args.timeout}s timeout") from error
    whole_process_seconds = time.monotonic() - started
    (args.output / (name + ".log")).write_bytes(process.stdout)
    events = []
    for line in process.stdout.decode("utf-8", errors="replace").splitlines():
        if PREFIX in line:
            events.append(json.loads(line.split(PREFIX, 1)[1]))
    write_json(args.output / (name + "-events.json"), events)
    after = identity(variant)
    if after != before:
        raise RuntimeError(f"Frozen inputs changed during {name}")
    if process.returncode != 0:
        raise RuntimeError(f"{name} test failed ({process.returncode}); retain its raw log")
    durations = validate_events(events, kind)
    return {"name": name, "variant": variant["label"], "kind": kind, "repetition": repetition,
            "command": command, "cwd": variant["source_root"], "returncode": process.returncode,
            "identity_before": before, "identity_after": after,
            "whole_process_seconds": whole_process_seconds, "context": events[0],
            "helper_trace_enabled": False,
            "durations_ms": durations,
            "warm_commands_ms": [durations[f"warm_command_{n:02}"] for n in range(1, 6)],
            "environment": {key: environment.get(key) for key in
                            ("SHELL", "SDL_VIDEODRIVER", "SDL_AUDIODRIVER", "SDL_RENDER_DRIVER",
                             "POTYI_TERM_TEST_CLIENT", "POTYI_TERM_STARTUP_KIND", "POTYI_TERM_STARTUP_ROOT",
                             "POTYI_TERM_STARTUP_VISIBLE", "POTYI_TERM_STARTUP_HOLD_MS")}}


def summarize(runs):
    result = {}
    for label, kind in sorted({(run["variant"], run["kind"]) for run in runs}):
        subset = [run for run in runs if (run["variant"], run["kind"]) == (label, kind)]
        stages = {stage: stats([run["durations_ms"][stage] for run in subset])
                  for stage in subset[0]["durations_ms"]}
        stages["warm_pooled"] = stats([value for run in subset for value in run["warm_commands_ms"]])
        stages["per_session_warm_medians"] = stats([statistics.median(run["warm_commands_ms"]) for run in subset])
        result[label + "/" + kind] = {"fresh_processes": len(subset), "stages": stages,
                                      "first_commands_ms": [run["durations_ms"]["first_command"] for run in subset]}
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--repetitions", type=int, default=10)
    parser.add_argument("--timeout", type=float, default=120)
    parser.add_argument("--visible", action="store_true")
    parser.add_argument("--video-driver")
    parser.add_argument("--hold-ms", type=int, default=0)
    parser.add_argument("--command", help="Literal shell command; identical for first and all five warm operations")
    parser.add_argument("--expect-line", help="One exact output line to check after each measured command")
    parser.add_argument("--workload-file", type=Path, help="Optional external producer file to hash and freeze")
    args = parser.parse_args()
    if not 1 <= args.repetitions <= 100 or not 1 <= args.timeout <= 600 or not 0 <= args.hold_ms <= 10_000:
        parser.error("Repetitions 1..100, timeout 1..600s, hold 0..10000ms required")
    if args.video_driver and not args.visible:
        parser.error("--video-driver applies only to a visible run")
    if args.command is not None and not 0 < len(args.command.encode()) <= 64 * 1024:
        parser.error("Command must be nonempty and at most 64 KiB")
    if args.expect_line is not None and (not args.expect_line or any(c in args.expect_line for c in "\r\n")):
        parser.error("Expected output must be one nonempty line")
    args.config = args.config.resolve(strict=True)
    args.output = args.output.resolve()
    if args.output.exists() and any(args.output.iterdir()):
        parser.error("Output directory must be absent or empty; prior evidence is immutable")
    args.output.mkdir(parents=True, exist_ok=True)
    variants = load_variants(args.config)
    fixture = args.output / "fixture"
    fixture.mkdir()
    (fixture / "a.txt").write_text("first fixture file\n", encoding="utf-8")
    (fixture / "b.txt").write_text("second fixture file\n", encoding="utf-8")
    workload = {"command_sha256": hashlib.sha256((args.command or DEFAULT_COMMAND).encode()).hexdigest(),
                "command_source": "environment override" if args.command is not None else "built-in small command",
                "expected_line": args.expect_line or ("POTYI_STARTUP_DONE" if args.command is None else None)}
    if args.workload_file is not None:
        workload.update(producer_path=str(args.workload_file.resolve(strict=True)), producer_sha256=sha(args.workload_file))
    report = {"schema_version": 1, "started_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
              "collector_sha256": sha(Path(__file__)), "config_sha256": sha(args.config),
              "repetitions": args.repetitions, "warm_commands_per_process": 5,
              "workload": workload, "variants": variants, "runs": [],
              "limitations": ["No build, profiler, or process sampler may run concurrently.",
                              "Fresh process first commands are not proven cold OS-cache launches.",
                              "Fixture SDL/window/font setup precedes pane timing; whole-process wall time is separate.",
                              "Session readiness is first app poll observation, not backend event generation time.",
                              "Command timing includes output, layout and final draw settlement; no physical keyboard/scanout timing.",
                              "Warm pooled values are correlated within each Session; per-Session warm medians are also retained.",
                              "Nearest-rank p95 from fewer than ten fresh processes is weak evidence.",
                              "This collector provides no RSS/CPU results; use the separate matched resource runner."]}
    output = args.output / "results.json"
    write_json(output, report)
    try:
        plan = [(variant, kind) for variant in variants for kind in ("old", "new")]
        for repetition in range(1, args.repetitions + 1):
            for variant, kind in (plan if repetition % 2 else list(reversed(plan))):
                if args.workload_file is not None and sha(args.workload_file) != workload["producer_sha256"]:
                    raise RuntimeError("External workload file changed")
                run = run_case(variant, kind, repetition, fixture, args)
                if args.workload_file is not None and sha(args.workload_file) != workload["producer_sha256"]:
                    raise RuntimeError("External workload file changed during probe")
                report["runs"].append(run)
                report["summary"] = summarize(report["runs"])
                write_json(output, report)
                print(f"{run['name']}: first {run['durations_ms']['first_command']:.2f}ms, warm median {statistics.median(run['warm_commands_ms']):.2f}ms", flush=True)
    except Exception as error:
        report["failure"] = str(error)
        write_json(output, report)
        raise
    report["completed_utc"] = dt.datetime.now(dt.timezone.utc).isoformat()
    if sha(Path(__file__)) != report["collector_sha256"] or sha(args.config) != report["config_sha256"]:
        raise RuntimeError("Collector or run configuration changed during measurement")
    report["complete"] = True
    write_json(output, report)
    rows = ["# Startup comparison", "", "| Variant | Kind | Stage | Samples | Median ms | Min ms | Max ms | p95 ms |",
            "|---|---|---|---:|---:|---:|---:|---:|"]
    for name, entry in report["summary"].items():
        label, kind = name.split("/")
        for stage, values in entry["stages"].items():
            rows.append(f"| {label} | {kind} | {stage} | {values['count']} | {values['median_ms']:.2f} | {values['min_ms']:.2f} | {values['max_ms']:.2f} | {values['p95_nearest_rank_ms']:.2f} |")
    rows.extend(["", "All first-command values and per-process warm values remain in results.json.", "",
                 *["- " + note for note in report["limitations"]]])
    (args.output / "summary.md").write_text("\n".join(rows) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
