#!/usr/bin/env python3
"""Run local feature verification and retain individually attributable evidence.

No downloads or server installations. Headless SDL tests run in separate
processes. Native UI, other operating systems and benchmarks are separate gates.
"""
import argparse
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import signal
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
DEFERRED = {
    "benchmarks::recovery_latency::typing_latency_probe": "Separate recovery typing latency investigation",
    "benchmarks::editor_performance_probe": "Separate resource measurements",
    "formatting::tests::resource_probe": "Separate resource measurements",
    "piece_table::recovery::tests::recovery_memory_probe": "Separate resource measurements",
    "renderer::terminal_selection_render_tests::long_line_navigation_probe": "Separate resource measurements",
    "renderer::terminal_selection_render_tests::terminal_command_latency_probe": "Separate resource measurements",
    "renderer::terminal_selection_render_tests::terminal_performance_probe": "Separate resource measurements",
    "piece_table::recovery::tests::crash_fixture": "Child fixture exercised by its parent test",
    "lsp_setup::tests::process_fixture": "Child fixture exercised by its parent tests",
    "lsp_setup::tests::live_install_and_initialize": "Separate opt-in network installation check",
    "lsp_setup::tests::live_java_distribution": "Separate opt-in network distribution check",
}
LIVE = {
    "formatting::tests::rustfmt_smoke_test_formats_buffer_without_saving": ["rustfmt"],
    "lsp::tests::real_clangd_hover_and_definition": ["clangd"],
    "lsp::tests::real_rust_analyzer_hover_and_definition": ["rust-analyzer"],
    "lsp::tests::real_rust_analyzer_extract_module_to_file": ["rust-analyzer"],
    "lsp::tests::real_servers_complete_member_access_and_prepare_insertion": ["clangd", "rust-analyzer"],
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--core-log", type=Path, help="Reuse a baseline log from this unchanged source tree")
    parser.add_argument("--skip-live", action="store_true")
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, SDL_VIDEODRIVER="dummy")
    results = []
    fingerprint = hashlib.sha256()
    for path in sorted([ROOT/"Cargo.toml", ROOT/"Cargo.lock", ROOT/"build.rs"] + list((ROOT/"src").rglob("*.rs")) + list((ROOT/"config").rglob("*.toml"))):
        fingerprint.update(str(path.relative_to(ROOT)).encode())
        fingerprint.update(path.read_bytes())

    def record():
        report = {"utc": dt.datetime.now(dt.timezone.utc).isoformat(), "host": platform.platform(),
                  "source_sha256": fingerprint.hexdigest(), "results": results}
        (args.output/"results.json").write_text(json.dumps(report, indent=2))

    def run(name, command, timeout=120):
        start = time.monotonic()
        logfile = args.output/(name.replace("::", "__") + ".log")
        with logfile.open("w") as log:
            proc = subprocess.Popen(command, cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT,
                                    start_new_session=os.name == "posix")
            timed_out = False
            try:
                code = proc.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                timed_out = True
                if os.name == "posix": os.killpg(proc.pid, signal.SIGKILL)
                else: proc.kill()
                code = proc.wait()
        text = logfile.read_text(errors="replace")
        matches = re.findall(r"test (\S+) \.\.\. (ok|FAILED|ignored)", text)
        # --nocapture may print diagnostics between the test name and its final
        # status. Exact single-test invocations can use the harness summary.
        if "--exact" in command and "test result: ok. 1 passed;" in text:
            matches = [(name, "ok")]
        row = {"name": name, "status": "PASS" if code == 0 else "FAIL", "exit_code": code,
               "seconds": round(time.monotonic()-start, 3), "timeout": timed_out,
               "log": logfile.name, "tests": dict(matches), "command": command}
        if not matches and name != "build-test-binary": row["status"] = "FAIL"
        results.append(row)
        record()
        print(f'{row["status"]}: {name} ({row["seconds"]} s)', flush=True)
        return row, text

    build, text = run("build-test-binary", ["cargo", "test", "--locked", "--bin", "potyi", "--no-run", "--message-format=json"], 600)
    if build["status"] != "PASS": return 1
    binaries = [item["executable"] for line in text.splitlines() if line.startswith("{")
                for item in [json.loads(line)] if item.get("reason") == "compiler-artifact" and item.get("executable")]
    if len(binaries) != 1: raise RuntimeError("Expected one Potyi test binary")
    binary = binaries[0]
    ignored_text = subprocess.check_output([binary, "--ignored", "--list"], cwd=ROOT, env=env, text=True)
    (args.output/"ignored-tests.txt").write_text(ignored_text)
    if args.core_log:
        text = args.core_log.read_text()
        (args.output/"core.log").write_text(text)
        results.append({"name": "core", "status": "PASS" if "test result: ok." in text and "FAILED" not in text else "FAIL",
                        "log": "core.log", "reused_from": str(args.core_log),
                        "tests": dict(re.findall(r"test (\S+) \.\.\. (ok|FAILED|ignored)", text))})
        record()
    else:
        run("core", [binary, "--test-threads=1"], 300)

    for name in re.findall(r"^(\S+): test$", ignored_text, re.M):
        if name in DEFERRED:
            results.append({"name": name, "status": "DEFERRED", "reason": DEFERRED[name]})
            record()
            continue
        if name in LIVE:
            reason = "Live checks disabled" if args.skip_live else None
            for tool in LIVE[name]:
                try:
                    subprocess.run([tool, "--version"], env=env, stdout=subprocess.DEVNULL,
                                   stderr=subprocess.DEVNULL, check=True, timeout=15)
                except (OSError, subprocess.SubprocessError): reason = f"Unavailable tool: {tool}"
            if reason:
                results.append({"name": name, "status": "DEFERRED", "reason": reason})
                record()
                continue
        run(name, [binary, name, "--exact", "--ignored", "--nocapture", "--test-threads=1"], 180)
    record()
    return int(any(row["status"] == "FAIL" for row in results))


if __name__ == "__main__":
    sys.exit(main())
