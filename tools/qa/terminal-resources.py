#!/usr/bin/env python3
"""Sample matched release old/new terminal fixtures in fresh paired processes."""
import argparse
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import platform
import queue
import re
import statistics
import subprocess
import sys
import tempfile
import threading
import time

ROOT = Path(__file__).resolve().parents[2]
PROBE = "app::input::tests::terminal_resource_probe::matched_terminal_resource_probe"
PREFIX = "POTYI_TERM_RESOURCE "
SOURCE_ALGORITHM = "sha256-relative-path-nul-file-v1"
PRODUCER = '''import os, sys, time
total, pace_ms = int(sys.argv[1]), int(sys.argv[2])
sequence = 0
remaining = total
while remaining:
    rows = []
    for _ in range(256):
        prefix = (f"RESOURCE row {sequence:08d} alpha beta 東京 ").encode()
        rows.append(prefix + b"." * (127 - len(prefix)) + b"\\n")
        sequence += 1
    block = b"".join(rows)
    chunk = memoryview(block)[:min(len(block), remaining)]
    while chunk:
        written = os.write(1, chunk)
        chunk = chunk[written:]
        remaining -= written
    if pace_ms:
        time.sleep(pace_ms / 1000)
os.write(1, b"RESOURCE_DONE\\n")
'''


def cpu_seconds(value):
    """POSIX ps cumulative TIME: [days-][hours:]minutes:seconds[.fraction]."""
    days = 0
    if "-" in value:
        day, value = value.split("-", 1)
        days = int(day)
    fields = value.split(":")
    if len(fields) == 3:
        hours, minutes, seconds = fields
    elif len(fields) == 2:
        hours, (minutes, seconds) = "0", fields
    else:
        raise ValueError(f"unsupported ps CPU time: {value!r}")
    return days * 86400 + int(hours) * 3600 + int(minutes) * 60 + float(seconds)


def parse_ps(text):
    result = {}
    for line in text.splitlines():
        words = line.split(None, 4)
        if len(words) != 5:
            continue
        try:
            pid, ppid, rss = (int(word) for word in words[:3])
            result[pid] = {"pid": pid, "ppid": ppid, "rss_bytes": rss * 1024,
                           "cpu_seconds": cpu_seconds(words[3]), "command": words[4]}
        except ValueError:
            continue  # Header or a process that vanished during ps enumeration.
    return result


def tree(processes, root):
    children = {}
    for row in processes.values():
        children.setdefault(row["ppid"], []).append(row["pid"])
    found, todo = set(), list(children.get(root, ()))
    while todo:
        pid = todo.pop()
        if pid == root or pid in found:
            continue
        found.add(pid)
        todo.extend(children.get(pid, ()))
    return [processes[pid] for pid in sorted(found)]


def snapshot(pid):
    before = time.monotonic()
    result = subprocess.run(["ps", "-A", "-o", "pid=,ppid=,rss=,time=,comm="],
                            text=True, capture_output=True, check=True, timeout=10)
    after = time.monotonic()
    processes = parse_ps(result.stdout)
    return {"sample_monotonic": after, "ps_started_monotonic": before,
            "ps_finished_monotonic": after, "ps_seconds": after - before,
            "app": processes.get(pid), "descendants": tree(processes, pid)}


def phase_intervals(events):
    intervals, active = [], None
    for event in events:
        if event["event"] == "phase":
            if active is not None:
                raise RuntimeError("Probe started a phase before ending the previous phase")
            active = {"phase": event["phase"], "start": event["observed_monotonic"]}
        elif event["event"] == "end":
            if active is None or active["phase"] != event["phase"]:
                raise RuntimeError("Probe phase end does not match its start")
            intervals.append(dict(active, end=event["observed_monotonic"]))
            active = None
    if active is not None:
        raise RuntimeError("Probe exited with an unfinished measurement phase")
    return intervals


def classify_samples(samples, intervals):
    """A ps snapshot is usable only wholly inside one observed marker interval."""
    for sample in samples:
        start, end = sample["ps_started_monotonic"], sample["ps_finished_monotonic"]
        matching = [interval for interval in intervals
                    if interval["start"] <= start <= end <= interval["end"]]
        sample["phase"] = matching[0]["phase"] if len(matching) == 1 else None
        if sample["phase"] is None:
            sample["phase_exclusion"] = "ps interval crosses or falls outside observed phase boundaries"
    return samples


def source_provenance(root=ROOT):
    """Exported frozen-build input identity; call before and after Cargo build."""
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
    return {"algorithm": SOURCE_ALGORITHM, "source_sha256": digest.hexdigest(), "files": files}


def validated_build_provenance(path, source, binary_hash, helper, helper_hash):
    if path is None:
        return None
    manifest = json.loads(Path(path).read_text())
    if manifest.get("algorithm") != SOURCE_ALGORITHM:
        raise RuntimeError("Build provenance uses a different source identity algorithm")
    for key in ("source_sha256", "source_before_sha256", "source_after_sha256"):
        if manifest.get(key) != source["source_sha256"]:
            raise RuntimeError(f"Build provenance {key} does not match the sampled checkout")
    if manifest.get("binary_sha256") != binary_hash:
        raise RuntimeError("Build provenance does not match the sampled test binary")
    if manifest.get("helper_sha256") != helper_hash or not manifest.get("helper_path"):
        raise RuntimeError("Build provenance does not match the explicit release helper")
    if Path(manifest["helper_path"]).resolve() != helper:
        raise RuntimeError("Build provenance helper path does not match the explicit release helper")
    if manifest.get("profile") != "release" or not manifest.get("target") or not manifest.get("command"):
        raise RuntimeError("Build provenance must record release profile, target and exact build command")
    return {"path": str(Path(path).resolve()), "sha256": hashlib.sha256(Path(path).read_bytes()).hexdigest(),
            "manifest": manifest}


def summarize_phase(samples):
    valid = [row for row in samples if row.get("app")]
    if not valid:
        return {"samples": 0, "app": None, "descendants": None,
                "reason": "No process sample landed in this short phase; operation timing remains available."}
    seconds = valid[-1]["sample_monotonic"] - valid[0]["sample_monotonic"]
    app_cpu = max(0., valid[-1]["app"]["cpu_seconds"] - valid[0]["app"]["cpu_seconds"]) if len(valid) > 1 else None
    first_pids = {row["pid"]: row["cpu_seconds"] for row in valid[0]["descendants"]}
    last_cpu = {}
    for sample in valid:
        for row in sample["descendants"]:
            last_cpu[row["pid"]] = max(last_cpu.get(row["pid"], 0.), row["cpu_seconds"])
    # A child born after the first sample starts at zero; vanished children
    # contribute their last observed cumulative counter rather than disappearing.
    descendant_cpu = sum(max(0., value - first_pids.get(pid, 0.)) for pid, value in last_cpu.items()) if len(valid) > 1 else None
    app_rss = [row["app"]["rss_bytes"] for row in valid]
    child_rss = [sum(child["rss_bytes"] for child in row["descendants"]) for row in valid]
    stats = lambda values: {"median_rss_bytes": statistics.median(values),
                            "max_rss_bytes": max(values), "last_rss_bytes": values[-1]}
    app = stats(app_rss)
    descendants = stats(child_rss)
    app.update(cpu_seconds=app_cpu, cpu_percent_one_core=100 * app_cpu / seconds if seconds > 0 and app_cpu is not None else None)
    descendants.update(cpu_seconds_observed=descendant_cpu,
                       cpu_percent_one_core_observed=100 * descendant_cpu / seconds if seconds > 0 and descendant_cpu is not None else None,
                       pids_observed=len(last_cpu))
    return {"samples": len(valid), "sample_window_seconds": seconds, "app": app,
            "descendants": descendants, "ps_observer_seconds": sum(row["ps_seconds"] for row in valid)}


def architecture(binary):
    result = {"platform": platform.platform(), "sampler_machine": platform.machine(),
              "binary_file": subprocess.check_output(["file", "-b", str(binary)], text=True).strip()}
    if platform.system() == "Darwin":
        def sysctl(name):
            value = subprocess.run(["sysctl", "-n", name], text=True, capture_output=True)
            return value.stdout.strip() if value.returncode == 0 else None
        result.update(hardware_machine=sysctl("hw.machine"),
                      arm_hardware=sysctl("hw.optional.arm64") == "1",
                      sampler_rosetta=sysctl("sysctl.proc_translated") == "1")
        description = result["binary_file"]
        if result["arm_hardware"] and "x86_64" in description and "arm64" not in description:
            result["test_execution"] = "x86_64 release binary under Rosetta on ARM hardware"
        elif "universal binary" in description:
            result["test_execution"] = "universal binary; process ISA is not established by file alone"
        else:
            result["test_execution"] = "binary ISA from file description; no translation inferred"
    else:
        result["test_execution"] = "binary ISA from file description"
    return result


def run_case(binary, kind, repetition, fixture, args):
    name = f"{repetition:02d}-{kind}"
    log_path = args.output / (name + ".log")
    samples_path = args.output / (name + "-samples.jsonl")
    events, operations, samples = [], [], []
    inbox = queue.Queue()
    environment = dict(os.environ, SDL_VIDEODRIVER="dummy", SDL_AUDIODRIVER="dummy",
                       POTYI_TERM_TEST_CLIENT=str(args.helper),
                       POTYI_TERM_RESOURCE_KIND=kind, POTYI_TERM_RESOURCE_ROOT=str(fixture),
                       POTYI_TERM_RESOURCE_PYTHON=str(Path(sys.executable).resolve()),
                       POTYI_TERM_RESOURCE_IDLE_SECONDS=str(args.idle_seconds),
                       POTYI_TERM_RESOURCE_LARGE_BYTES=str(args.large_mib * 1024 * 1024))
    command = [str(binary), PROBE, "--exact", "--ignored", "--nocapture", "--test-threads=1"]
    binary_hash = hashlib.sha256(binary.read_bytes()).hexdigest()
    helper_hash = hashlib.sha256(args.helper.read_bytes()).hexdigest()
    start = time.monotonic()
    process = subprocess.Popen(command, cwd=ROOT, env=environment, stdout=subprocess.PIPE,
                               stderr=subprocess.STDOUT, text=True, bufsize=1, start_new_session=True)
    phase = None
    with log_path.open("w") as log, samples_path.open("w") as raw:
        def read_output():
            try:
                for line in process.stdout:
                    log.write(line)
                    log.flush()
                    offset = line.find(PREFIX)
                    if offset >= 0:
                        data = json.loads(line[offset + len(PREFIX):])
                        data["observed_monotonic"] = time.monotonic()
                        inbox.put(data)
            except Exception as error:
                inbox.put({"event": "reader_error", "error": repr(error)})
            finally:
                inbox.put({"event": "reader_closed"})
        reader = threading.Thread(target=read_output, daemon=True)
        reader.start()
        closed = False
        try:
            while process.poll() is None or not closed or not inbox.empty():
                if time.monotonic() - start > args.timeout:
                    raise TimeoutError(f"{name} exceeded {args.timeout}s; logs retained")
                wait = args.interval
                try:
                    data = inbox.get(timeout=wait)
                    pending = [data]
                    while not inbox.empty():
                        pending.append(inbox.get_nowait())
                except queue.Empty:
                    pending = []
                for data in pending:
                    if data["event"] == "reader_closed":
                        closed = True
                        continue
                    if data["event"] == "reader_error":
                        raise RuntimeError(data["error"])
                    if data.get("kind") != kind:
                        raise RuntimeError("probe kind does not match the paired process")
                    events.append(data)
                    if data["event"] == "phase":
                        phase = data["phase"]
                    elif data["event"] == "end" and phase == data["phase"]:
                        phase = None
                    elif data["event"] == "operation":
                        operations.append(data)
                if phase and process.poll() is None:
                    row = snapshot(process.pid)
                    row.update(observed_phase=phase, elapsed_seconds=time.monotonic() - start)
                    samples.append(row)
                    raw.write(json.dumps(row) + "\n")
                    raw.flush()
            process.wait()
        except BaseException:
            # The test is a fresh private process group, including its fixtures.
            # Do not leave producers running when a sample fails or is interrupted.
            import signal
            # PTY shells may own another process group/session. Capture their
            # private descendant PIDs while parentage is still attributable.
            try:
                descendants = snapshot(process.pid)["descendants"]
            except (subprocess.SubprocessError, OSError):
                descendants = []
            for child in reversed(descendants):
                try:
                    os.kill(child["pid"], signal.SIGKILL)
                except ProcessLookupError:
                    pass
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait()
            raise
        finally:
            reader.join(timeout=2)
    text = log_path.read_text(errors="replace")
    if hashlib.sha256(binary.read_bytes()).hexdigest() != binary_hash:
        raise RuntimeError("Release binary changed during sampling; do not compare different builds")
    if hashlib.sha256(args.helper.read_bytes()).hexdigest() != helper_hash:
        raise RuntimeError("Release helper changed during sampling; do not compare different builds")
    if process.returncode != 0 or not any(row["event"] == "done" for row in events) or "1 passed; 0 failed" not in text:
        raise RuntimeError(f"{name} probe failed; inspect {log_path}")
    intervals = phase_intervals(events)
    classify_samples(samples, intervals)
    # Keep boundary exclusions and both observer timestamps in the raw evidence.
    with samples_path.open("w") as raw:
        for row in samples:
            raw.write(json.dumps(row) + "\n")
    phases = sorted({row["phase"] for row in events if row["event"] == "phase"})
    return {"kind": kind, "repetition": repetition, "pid": process.pid,
            "wall_seconds": time.monotonic() - start, "binary_sha256": binary_hash,
            "helper_sha256": helper_hash, "log": log_path.name,
            "samples_file": samples_path.name, "events": events, "operations": operations,
            "phase_intervals": intervals, "excluded_boundary_samples": sum(row["phase"] is None for row in samples),
            "phases": {phase: summarize_phase([row for row in samples if row["phase"] == phase]) for phase in phases}}


def aggregate(report):
    groups = {}
    for run in report["runs"]:
        for phase, data in run["phases"].items():
            if not data["app"]:
                continue
            groups.setdefault((run["kind"], phase), []).append(data)
    summary = []
    for (kind, phase), rows in sorted(groups.items()):
        data = {"kind": kind, "phase": phase, "runs_with_samples": len(rows)}
        for scope, metrics in [("app", ["median_rss_bytes", "max_rss_bytes", "cpu_seconds", "cpu_percent_one_core"]),
                               ("descendants", ["median_rss_bytes", "max_rss_bytes", "cpu_seconds_observed", "cpu_percent_one_core_observed"])]:
            data[scope] = {}
            for metric in metrics:
                values = [row[scope][metric] for row in rows if row[scope][metric] is not None]
                if values:
                    data[scope][metric] = {"median": statistics.median(values), "min": min(values), "max": max(values)}
        summary.append(data)
    return summary


def write_report(report, output):
    report["summary"] = aggregate(report)
    (output / "results.json").write_text(json.dumps(report, indent=2) + "\n")
    lines = ["# Matched terminal resource measurements\n\n",
             f"{report['utc']} · release test binary · {report['repetitions']} fresh paired samples, alternating order.\n\n",
             f"Execution: **{report['environment']['test_execution']}**. SDL uses a hidden 800×600 dummy-video fixture, embedded fonts and the app's poll/frame schedule. This is not a native window idle measurement.\n\n",
             "App and descendant processes are separate scopes. CPU is a percentage of one core from cumulative process counters; small zero values can be below `ps` time resolution. Descendant CPU covers observed process lifetimes and can miss short-lived children.\n\n",
             "Only snapshots whose whole `ps` interval is within an observed phase are aggregated. Boundary exclusions and timestamps remain in the raw JSONL evidence. The source digest identifies the sampled checkout; compiled-source identity requires the matching frozen-build manifest.\n\n",
             "| Phase | Integration | App RSS median MiB | App CPU s | App CPU % | Descendant RSS median MiB | Observed descendant CPU s | Observed descendant CPU % |\n",
             "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |\n"]
    value = lambda row, scope, metric: row[scope].get(metric, {}).get("median")
    number = lambda val, divisor=1: "—" if val is None else f"{val/divisor:.2f}"
    large_commands = report["workload"].get("large_commands", 2)
    stream_phases = ["stream_first", "plateau_first", "stream_second", "plateau_second"]
    operation_phases = ["small_command", "stream_first", "stream_second"]
    if large_commands >= 3:
        stream_phases.extend(["stream_third", "plateau_third"])
        operation_phases.append("stream_third")
    for phase in ["idle", "small_command", "small_idle"] + stream_phases + ["resize", "copy_selected", "copy_all", "copied_idle"]:
        for kind in ["old", "new"]:
            row = next((row for row in report["summary"] if row["kind"] == kind and row["phase"] == phase), None)
            if row:
                lines.append(f"| {phase} | {kind} | {number(value(row,'app','median_rss_bytes'),1048576)} | {number(value(row,'app','cpu_seconds'))} | {number(value(row,'app','cpu_percent_one_core'))} | {number(value(row,'descendants','median_rss_bytes'),1048576)} | {number(value(row,'descendants','cpu_seconds_observed'))} | {number(value(row,'descendants','cpu_percent_one_core_observed'))} |\n")
            else:
                lines.append(f"| {phase} | {kind} | — | — | — | — | — | — |\n")
    lines.extend(["\n| Operation | Integration | Median ms | Min–max ms | Median copied MiB |\n",
                  "| --- | --- | ---: | ---: | ---: |\n"])
    for phase in operation_phases + ["resize", "copy_selected", "copy_all"]:
        for kind in ["old", "new"]:
            rows = [event for run in report["runs"] if run["kind"] == kind
                    for event in run["operations"] if event["phase"] == phase]
            if rows:
                durations = [row["duration_ms"] for row in rows]
                lengths = [row["copied_bytes"] for row in rows if "copied_bytes" in row]
                copied = number(statistics.median(lengths), 1048576) if lengths else "—"
                lines.append(f"| {phase} | {kind} | {statistics.median(durations):.2f} | {min(durations):.2f}–{max(durations):.2f} | {copied} |\n")
    large_mib = report["workload"]["large_bytes_each"] // 1048576
    count = {2: "two", 3: "three"}.get(large_commands, str(large_commands))
    lines.extend([f"\nEach integration receives the same 64KiB small producer and {count} paced {large_mib}MiB producers ({large_commands*large_mib}MiB cumulative cap-crossing output), then 9 width/height changes and selected ≤4KiB / all-retained clipboard copies. Commands, cwd, fonts and test binary are shared. Later settled phases check continued eviction and memory growth after the first cap crossing; a flat RSS plateau is not asserted.\n\n",
                  "RSS includes allocator-retained memory, headless renderer/texture caches, terminal grids and clipboard storage. Clipboard phases include verification through SDL. Binary/test fixture setup and producer creation are outside the measured settled phases.\n\n"])
    lines.append("CopyAll is each integration's retained plaintext, whose size is recorded above: the new 8MiB budget includes record/index metadata, while legacy caps its output text. CopyAll costs therefore cover different retained byte counts; the selected ≤4KiB scope is matched. Producer rows contain changing serial numbers and Unicode, rather than repeating one cache-identical row.\n\n")
    if report.get("helper"):
        lines.append("The command helper path is explicit and overrides inherited test-helper settings; its SHA-256 is checked before and after each run and against the frozen-build manifest when supplied.\n\n")
    for limit in report["limitations"]:
        lines.append("- " + limit + "\n")
    lines.append("\n[Raw metrics, phase events, source/binary hashes and log names](results.json). Per-run JSONL files retain every app/descendant sample.\n")
    (output / "summary.md").write_text("".join(lines))


def self_check():
    assert cpu_seconds("00:01.25") == 1.25
    assert cpu_seconds("01:02:03.50") == 3723.5
    assert cpu_seconds("2-01:02:03") == 176523
    processes = parse_ps("1 0 1024 00:01.00 app\n2 1 512 00:02.00 shell\n3 2 256 00:03.00 producer\n4 0 999 00:04.00 unrelated\n")
    assert [row["pid"] for row in tree(processes, 1)] == [2, 3]
    first = {"sample_monotonic": 1., "ps_seconds": .01, "app": processes[1], "descendants": [processes[2]]}
    second = {"sample_monotonic": 3., "ps_seconds": .01,
              "app": dict(processes[1], cpu_seconds=2.),
              "descendants": [dict(processes[2], cpu_seconds=2.5), processes[3]]}
    last = {"sample_monotonic": 5., "ps_seconds": .01,
            "app": dict(processes[1], cpu_seconds=3.), "descendants": []}
    result = summarize_phase([first, second, last])
    assert result["app"]["cpu_percent_one_core"] == 50.
    assert result["descendants"]["cpu_seconds_observed"] == 3.5
    assert result["descendants"]["max_rss_bytes"] == 768 * 1024
    assert result["descendants"]["last_rss_bytes"] == 0
    assert summarize_phase([])["app"] is None
    assert summarize_phase([first])["app"]["cpu_seconds"] is None
    assert summarize_phase([first])["descendants"]["cpu_seconds_observed"] is None
    events = [{"event": "phase", "phase": "first", "observed_monotonic": 1.},
              {"event": "end", "phase": "first", "observed_monotonic": 2.},
              {"event": "phase", "phase": "second", "observed_monotonic": 2.1},
              {"event": "end", "phase": "second", "observed_monotonic": 3.}]
    rows = [{"ps_started_monotonic": 1.1, "ps_finished_monotonic": 1.2},
            {"ps_started_monotonic": 1.9, "ps_finished_monotonic": 2.2},
            {"ps_started_monotonic": 2.2, "ps_finished_monotonic": 2.3, "observed_phase": "first"}]
    assert [row["phase"] for row in classify_samples(rows, phase_intervals(events))] == ["first", None, "second"]
    assert "phase_exclusion" in rows[1]
    with tempfile.TemporaryDirectory(prefix="potyi-resource-provenance-") as directory:
        root = Path(directory)
        for name in ("Cargo.toml", "Cargo.lock", "build.rs", "src/asset.dat", "fonts/test.ttf", "vendor/vt100/src/lib.rs"):
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(name.encode())
        source = source_provenance(root)
        assert len(source["files"]) == 6
        (root / "fonts/test.ttf").write_bytes(b"changed font")
        assert source_provenance(root)["source_sha256"] != source["source_sha256"]
        helper = (root / "release-helper").resolve()
        manifest = dict(algorithm=SOURCE_ALGORITHM, source_sha256=source["source_sha256"],
                        source_before_sha256=source["source_sha256"], source_after_sha256=source["source_sha256"],
                        binary_sha256="binary", helper_path=str(helper), helper_sha256="helper",
                        profile="release", target="fixture-target", command=["cargo", "test", "--release", "--no-run"])
        path = root / "manifest.json"
        path.write_text(json.dumps(manifest))
        assert validated_build_provenance(path, source, "binary", helper, "helper")["manifest"] == manifest
        for binary_hash, helper_path, helper_hash in [("different binary", helper, "helper"),
                                                     ("binary", helper, "stale helper"),
                                                     ("binary", root / "debug-helper", "helper")]:
            try:
                validated_build_provenance(path, source, binary_hash, helper_path, helper_hash)
            except RuntimeError:
                pass
            else:
                raise AssertionError("Mismatched build/helper identity must be rejected")
        output = root / "report"
        output.mkdir()
        report = {"utc": "fixture", "repetitions": 3,
                  "environment": {"test_execution": "fixture"},
                  "workload": {"large_bytes_each": 12 * 1048576, "large_commands": 3},
                  "runs": [{"kind": "new", "phases": {"plateau_third": result},
                            "operations": [{"phase": "stream_third", "duration_ms": 12.5}]}],
                  "limitations": []}
        write_report(report, output)
        text = (output / "summary.md").read_text()
        assert "three paced 12MiB producers (36MiB" in text
        assert "| plateau_third | new |" in text and "| stream_third | new | 12.50 |" in text
        # Historical two-stream evidence keeps its actual workload label and
        # does not invent third-phase rows when regenerated by this runner.
        report["workload"]["large_commands"] = 2
        report["runs"] = []
        write_report(report, output)
        text = (output / "summary.md").read_text()
        assert "two paced 12MiB producers (24MiB" in text
        assert "stream_third" not in text and "plateau_third" not in text
    print("Sampler parsing, lifetime accounting, phase boundaries, frozen-build provenance and workload reporting checks passed.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, help="Existing release test binary; build/registration are root-owned")
    parser.add_argument("--helper", type=Path, help="Explicit release app/helper from the same frozen source build")
    parser.add_argument("--build-provenance", type=Path, help="Frozen release build manifest with matching source and binary hashes")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--idle-seconds", type=float, default=2.)
    parser.add_argument("--large-mib", type=int, default=12)
    parser.add_argument("--interval", type=float, default=.1)
    parser.add_argument("--timeout", type=float, default=600.)
    parser.add_argument("--report-only", action="store_true")
    parser.add_argument("--self-check", action="store_true")
    args = parser.parse_args()
    if args.self_check:
        self_check()
        return
    if not args.output:
        parser.error("--output is required")
    args.output = args.output.resolve()
    if args.report_only:
        write_report(json.loads((args.output / "results.json").read_text()), args.output)
        return
    if os.name != "posix":
        parser.error("This ps sampler is POSIX-only; Windows command lifecycle has separate CI tests")
    if not args.binary or not args.binary.is_file():
        parser.error("--binary must name the registered release test binary")
    if not args.helper or not args.helper.is_file():
        parser.error("--helper must name the explicit release app/helper from the frozen build")
    if args.repetitions < 3 or not .5 <= args.idle_seconds <= 10 or not 9 <= args.large_mib <= 64 or not .05 <= args.interval <= 1:
        parser.error("Use ≥3 pairs, idle 0.5–10s, each large producer 9–64MiB, ps interval 0.05–1s")
    binary = args.binary.resolve()
    args.helper = args.helper.resolve()
    listing = subprocess.check_output([str(binary), "--list", "--ignored"], cwd=ROOT, text=True)
    if PROBE + ": test" not in listing:
        parser.error("Resource probe is not registered in this binary")
    args.output.mkdir(parents=True, exist_ok=True)
    source = source_provenance()
    binary_hash = hashlib.sha256(binary.read_bytes()).hexdigest()
    helper_hash = hashlib.sha256(args.helper.read_bytes()).hexdigest()
    build = validated_build_provenance(args.build_provenance, source, binary_hash, args.helper, helper_hash)
    report = {"utc": dt.datetime.now(dt.timezone.utc).isoformat(),
              "profile": "release (matching frozen-build manifest)" if build else "release (caller supplied)",
              "binary": str(binary), "binary_sha256": binary_hash,
              "helper": str(args.helper), "helper_sha256": helper_hash,
              "sampler_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              "source_sha256": source["source_sha256"], "source_identity": source,
              "source_identity_scope": "sampled checkout build inputs", "build_provenance": build,
              "repetitions": args.repetitions,
              "environment": architecture(binary), "workload": {"small_bytes": 64 * 1024,
              "shell": os.environ.get("SHELL", "/bin/sh"), "python": str(Path(sys.executable).resolve()),
              "large_bytes_each": args.large_mib * 1024 * 1024, "large_commands": 3,
              "idle_seconds": args.idle_seconds, "ps_interval_seconds": args.interval,
              "producer_sha256": hashlib.sha256(PRODUCER.encode()).hexdigest()}, "runs": [],
              "limitations": [
                  "Headless hidden SDL fixture; no native compositor, OS keyboard delivery or visible clipboard UX is measured.",
                  "The persistent new terminal shell contributes to descendant idle RSS; it is separate from app RSS.",
                  "Polling ps is an external observer with scheduling overhead, retained in raw sample timings.",
                  "Whole observer intervals crossing observed phase markers are excluded; marker timestamps include log reader scheduling delay.",
                  "CPU counters have platform-dependent resolution; vanished/short-lived descendants can be undercounted.",
                  "Fast phases may have no ps sample; operation latency remains in events rather than inventing zero resource usage.",
                  "Three pairs describe this machine and warm local temporary storage, not universal hardware/OS guarantees.",
                  "RSS sums child address spaces and may double-count shared pages; it is not proportional/private memory.",
                  "The prebuilt binary hash is recorded; release profile is validated against the supplied frozen-build manifest when present, otherwise caller supplied.",
              ]}
    (args.output / "producer.py").write_text(PRODUCER)
    with tempfile.TemporaryDirectory(prefix="potyi-terminal-matched-", dir="/tmp") as directory:
        fixture = Path(directory).resolve()
        (fixture / "producer.py").write_text(PRODUCER)
        for number in range(12):
            (fixture / f"fixture-{number:02d}.txt").write_text("same browsing fixture\n")
        (fixture / ".hidden").write_text("dotfile\n")
        report["fixture_root"] = str(fixture)
        for repetition in range(1, args.repetitions + 1):
            for kind in (["old", "new"] if repetition % 2 else ["new", "old"]):
                print(f"Measuring pair {repetition}/{args.repetitions}: {kind}", flush=True)
                run = run_case(binary, kind, repetition, fixture, args)
                if run["binary_sha256"] != report["binary_sha256"]:
                    raise RuntimeError("Release binary differs between paired samples; comparison cancelled")
                if run["helper_sha256"] != report["helper_sha256"]:
                    raise RuntimeError("Release helper differs between paired samples; comparison cancelled")
                if source_provenance()["source_sha256"] != source["source_sha256"]:
                    raise RuntimeError("Sampled checkout build inputs changed during paired measurements")
                report["runs"].append(run)
                write_report(report, args.output)
    print(f"Results saved to {args.output / 'summary.md'}")


if __name__ == "__main__":
    main()
