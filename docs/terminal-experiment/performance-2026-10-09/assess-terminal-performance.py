#!/usr/bin/env python3
"""Assess existing frozen terminal evidence; never build or run a workload."""
import argparse
import datetime as dt
import hashlib
import json
import math
from pathlib import Path
import runpy
import statistics

PARENT = Path(__file__).resolve().parents[1]
STREAMS = ("stream_first", "stream_second", "stream_third")
SETTLED = ("idle", "small_idle", "plateau_first", "plateau_second", "plateau_third", "copied_idle")
SOURCE = runpy.run_path(str(Path(__file__).with_name("terminal-startup.py")))["source_provenance"]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def load(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def stats(values):
    values = [value for value in values if value is not None and math.isfinite(value)]
    if not values:
        return None
    ordered = sorted(values)
    return {"count": len(values), "median": statistics.median(values), "min": min(values), "max": max(values),
            "p95_nearest_rank": ordered[math.ceil(.95 * len(values)) - 1], "values": values}


def median(stat):
    return stat["median"] if stat else None


def ratio(numerator, denominator):
    return numerator / denominator if numerator is not None and denominator is not None and denominator > 0 else None


def resource_identity(data):
    failures = []
    def check(ok, message):
        if not ok:
            failures.append(message)
    build = data.get("build_provenance") or {}
    manifest = build.get("manifest") or {}
    check(manifest.get("profile") == "release", "No validated release manifest")
    check(bool(manifest.get("command")) and bool(manifest.get("target")), "Missing build command/target")
    check(data.get("source_identity", {}).get("source_sha256") == data["source_sha256"], "Recorded source identity differs")
    for key in ("source_sha256", "source_before_sha256", "source_after_sha256"):
        check(manifest.get(key) == data["source_sha256"], "Build " + key + " differs")
    for item in ("binary", "helper"):
        path = Path(data[item])
        check(manifest.get(item + "_sha256") == data[item + "_sha256"], "Manifest " + item + " SHA differs")
        check(Path(manifest.get(item + "_path", "")).resolve() == path.resolve(), "Manifest " + item + " path differs")
        check(path.is_file(), "Immutable " + item + " unavailable")
        if path.is_file():
            check(sha(path) == data[item + "_sha256"], "Current immutable " + item + " SHA differs")
        for run in data["runs"]:
            check(run.get(item + "_sha256") == data[item + "_sha256"], "Per-run " + item + " SHA differs")
    root = Path(manifest.get("source_root", ""))
    check(root.is_dir(), "Frozen source root unavailable")
    if root.is_dir():
        check(SOURCE(root)["source_sha256"] == data["source_sha256"], "Current frozen source differs from build/run identity")
    path = Path(build.get("path", ""))
    check(path.is_file(), "Build manifest file unavailable")
    if path.is_file():
        check(sha(path) == build.get("sha256"), "Build manifest file changed")
    return {"consistent": not failures, "failures": failures, "source_sha256": data["source_sha256"],
            "binary_sha256": data["binary_sha256"], "helper_sha256": data["helper_sha256"],
            "profile": manifest.get("profile"), "target": manifest.get("target"), "source_root": str(root)}


def phase_metrics(run, samples):
    result = {}
    for phase, existing in run["phases"].items():
        valid = [sample for sample in samples if sample.get("phase") == phase and sample.get("app")]
        simultaneous = [sample["app"]["rss_bytes"] + sum(child["rss_bytes"] for child in sample["descendants"]) for sample in valid]
        app = existing.get("app") or {}
        descendants = existing.get("descendants") or {}
        operations = [operation for operation in run["operations"] if operation["phase"] == phase]
        begins = [event for event in run["events"] if event["event"] == "phase" and event["phase"] == phase]
        ends = [event for event in run["events"] if event["event"] == "end" and event["phase"] == phase]
        redraws = ends[0]["draws"] - begins[0]["draws"] if len(begins) == len(ends) == 1 else None
        result[phase] = {
            "valid_samples": len(valid), "sample_window_seconds": existing.get("sample_window_seconds"),
            "app_cpu_seconds": app.get("cpu_seconds"), "app_cpu_percent_one_core": app.get("cpu_percent_one_core"),
            "descendant_cpu_seconds_observed": descendants.get("cpu_seconds_observed"),
            "app_rss_median_bytes": app.get("median_rss_bytes"),
            "descendant_rss_median_bytes": descendants.get("median_rss_bytes"),
            "simultaneous_rss_median_bytes": statistics.median(simultaneous) if simultaneous else None,
            "simultaneous_rss_max_bytes": max(simultaneous) if simultaneous else None,
            "simultaneous_rss_last_bytes": simultaneous[-1] if simultaneous else None,
            "operation_ms": operations[0]["duration_ms"] if len(operations) == 1 else None,
            "copied_bytes": operations[0].get("copied_bytes") if len(operations) == 1 else None,
            "draws": redraws,
            "ps_observer_seconds": sum(sample["ps_seconds"] for sample in valid)}
    return result


def assess_resource(label, path, trace_context=None):
    if not path.is_file():
        return {"label": label, "status": "pending", "path": str(path)}
    data = load(path)
    runs = []
    for run in data["runs"]:
        samples = [json.loads(line) for line in (path.parent / run["samples_file"]).read_text(encoding="utf-8").splitlines()]
        intervals = run["phase_intervals"]
        for sample in samples:
            matching = [interval["phase"] for interval in intervals
                        if interval["start"] <= sample["ps_started_monotonic"] <= sample["ps_finished_monotonic"] <= interval["end"]]
            expected = matching[0] if len(matching) == 1 else None
            if sample.get("phase") != expected:
                raise RuntimeError("Raw sample classification differs from complete-interval rule")
        excluded = sum(sample.get("phase") is None for sample in samples)
        if excluded != run["excluded_boundary_samples"]:
            raise RuntimeError("Recorded boundary exclusion count differs")
        phases = phase_metrics(run, samples)
        growth = {}
        for metric in ("app_rss_median_bytes", "simultaneous_rss_median_bytes"):
            plateaus = [phases[name][metric] for name in ("plateau_first", "plateau_second", "plateau_third")]
            growth[metric] = {"plateaus": plateaus, "second_minus_first": plateaus[1] - plateaus[0],
                              "third_minus_second": plateaus[2] - plateaus[1], "third_minus_first": plateaus[2] - plateaus[0]}
        total_stream_cpu = sum(phases[phase]["app_cpu_seconds"] for phase in STREAMS)
        runs.append({"kind": run["kind"], "repetition": run["repetition"], "phases": phases,
                     "total_stream_app_cpu_seconds_observed": total_stream_cpu, "plateau_growth": growth,
                     "observer": {"total_samples": len(samples), "excluded_boundary_samples": excluded,
                                  "excluded_fraction": excluded / len(samples) if samples else None,
                                  "total_ps_seconds": sum(sample["ps_seconds"] for sample in samples),
                                  "whole_run_wall_seconds": run["wall_seconds"]}})
    aggregate = {}
    for kind in ("old", "new"):
        subset = [run for run in runs if run["kind"] == kind]
        aggregate[kind] = {phase: {metric: stats([run["phases"][phase][metric] for run in subset])
                                   for metric in subset[0]["phases"][phase]}
                           for phase in subset[0]["phases"]}
        aggregate[kind]["total_stream_cpu"] = stats([run["total_stream_app_cpu_seconds_observed"] for run in subset])
        aggregate[kind]["plateau_growth"] = {
            metric: {difference: stats([run["plateau_growth"][metric][difference] for run in subset])
                     for difference in ("second_minus_first", "third_minus_second", "third_minus_first")}
            for metric in ("app_rss_median_bytes", "simultaneous_rss_median_bytes")}
    pairs = []
    for repetition in sorted({run["repetition"] for run in runs}):
        old = next(run for run in runs if run["repetition"] == repetition and run["kind"] == "old")
        new = next(run for run in runs if run["repetition"] == repetition and run["kind"] == "new")
        paired = {phase: {metric: ratio(new["phases"][phase][metric], old["phases"][phase][metric])
                          for metric in ("app_cpu_seconds", "operation_ms", "simultaneous_rss_median_bytes")}
                  for phase in old["phases"]}
        pairs.append({"repetition": repetition, "new_over_old": paired,
                      "total_stream_cpu_new_over_old": ratio(new["total_stream_app_cpu_seconds_observed"], old["total_stream_app_cpu_seconds_observed"])})
    paired_ratios = {phase: {metric: stats([pair["new_over_old"][phase][metric] for pair in pairs])
                             for metric in ("app_cpu_seconds", "operation_ms", "simultaneous_rss_median_bytes")}
                     for phase in pairs[0]["new_over_old"]}
    paired_ratios["total_stream_cpu"] = stats([pair["total_stream_cpu_new_over_old"] for pair in pairs])
    completed = len(runs) == data["repetitions"] * 2 and all(any(e["event"] == "done" for e in r["events"]) for r in data["runs"])
    trace = {"status": "not recorded", "disabled": None}
    bridge_path = Path(data.get("build_provenance", {}).get("manifest", {}).get("source_root", "")) / "src/experimental_terminal/bridge.rs"
    if bridge_path.is_file() and "POTYI_TERM_TRACE_FILE" not in bridge_path.read_text(encoding="utf-8"):
        trace = {"status": "trace controls absent from frozen source", "disabled": True}
    if trace_context is not None and trace_context.is_file():
        context = load(trace_context)
        controls = context.get("trace_environment") or {}
        explicit = all(key in controls and controls[key] is None for key in ("POTYI_TERM_TRACE", "POTYI_TERM_TRACE_FILE"))
        matching = all(context.get(key) == data[key] for key in ("source_sha256", "binary_sha256", "helper_sha256"))
        trace = {"status": "explicit matching run context" if explicit and matching else "incomplete or mismatched run context",
                 "disabled": explicit and matching and bool(context.get("command")),
                 "context_path": str(trace_context), "context_sha256": sha(trace_context), "record": context}
    return {"label": label, "status": "complete" if completed else "incomplete", "path": str(path), "result_sha256": sha(path),
            "identity": resource_identity(data), "environment": data["environment"], "workload": data["workload"],
            "trace": trace, "runs": runs, "aggregate": aggregate, "pairs": pairs, "paired_ratios": paired_ratios}


def assess_startup(path):
    if not path.is_file():
        return {"status": "pending", "path": str(path)}
    data = load(path)
    failures = []
    for variant in data["variants"]:
        identity = variant["identity"]
        for item in ("binary", "helper", "build_provenance"):
            filename = Path(variant[item])
            if not filename.is_file() or sha(filename) != identity[item + "_sha256"]:
                failures.append(variant["label"] + ": current " + item + " differs")
        if SOURCE(variant["source_root"])["source_sha256"] != identity["source_sha256"]:
            failures.append(variant["label"] + ": current source differs")
        for run in data["runs"]:
            if run["variant"] == variant["label"] and (run["identity_before"] != identity or run["identity_after"] != identity):
                failures.append(run["name"] + ": per-run identity differs")
    summaries = {}
    for variant, kind in sorted({(run["variant"], run["kind"]) for run in data["runs"]}):
        subset = [run for run in data["runs"] if (run["variant"], run["kind"]) == (variant, kind)]
        summaries[variant + "/" + kind] = {
            "first_commands_ms": stats([run["durations_ms"]["first_command"] for run in subset]),
            "warm_pooled_ms": stats([value for run in subset for value in run["warm_commands_ms"]]),
            "per_session_warm_medians_ms": stats([statistics.median(run["warm_commands_ms"]) for run in subset]),
            "stages_ms": {stage: stats([run["durations_ms"][stage] for run in subset]) for stage in subset[0]["durations_ms"]},
            "whole_process_seconds": stats([run["whole_process_seconds"] for run in subset])}
    tracing_disabled = all(run.get("helper_trace_enabled") is False for run in data["runs"])
    return {"status": "complete" if data.get("complete") else "incomplete", "path": str(path), "result_sha256": sha(path),
            "identity": {"consistent": not failures, "failures": failures}, "trace_disabled": tracing_disabled,
            "collector_sha256": data["collector_sha256"], "workload": data["workload"], "summary": summaries,
            "variants": [{"label": variant["label"], "identity": variant["identity"], "architecture": variant["architecture"],
                          "binary": variant["binary"], "helper": variant["helper"],
                          "build_manifest": variant["build_manifest"]} for variant in data["variants"]],
            "limitations": data["limitations"]}


def changes(before, after):
    if before.get("status") != "complete" or after.get("status") != "complete":
        return {"status": "pending"}
    if before["workload"] != after["workload"]:
        raise RuntimeError("Resource workloads differ; do not compare unmatched runs")
    result = {}
    for phase in before["aggregate"]["new"]:
        if phase in ("total_stream_cpu", "plateau_growth"):
            continue
        result[phase] = {}
        for metric in ("app_cpu_seconds", "operation_ms", "simultaneous_rss_median_bytes", "draws"):
            original = median(before["aggregate"]["new"][phase][metric])
            final = median(after["aggregate"]["new"][phase][metric])
            value = ratio(final, original)
            result[phase][metric] = {"before_median": original, "after_median": final,
                                    "after_over_before": value, "reduction_percent": 100 * (1 - value) if value is not None else None}
    result["control_normalized_total_stream_cpu"] = ratio(median(after["paired_ratios"]["total_stream_cpu"]), median(before["paired_ratios"]["total_stream_cpu"]))
    return {"status": "complete", "phases": result}


def targets(before, optimized, startup, startup_label):
    result = {"warm_median_100ms": {"status": "pending"}, "first_p95_250ms": {"status": "pending"},
              "stream_app_cpu_1_3x_old": {"status": "pending"}, "settled_memory_vs_before": {"status": "pending"},
              "idle_redraws_vs_before": {"status": "pending"}, "idle_cpu_vs_before": {"status": "pending"}}
    key = startup_label + "/new"
    if startup.get("status") == "complete" and startup.get("identity", {}).get("consistent") and key in startup.get("summary", {}):
        entry = startup["summary"][key]
        warm = median(entry["warm_pooled_ms"])
        first = entry["first_commands_ms"]["p95_nearest_rank"]
        result["warm_median_100ms"] = {"status": "met" if warm <= 100 else "missed", "measured_ms": warm, "limit_ms": 100,
                                      "per_session_warm_medians_ms": entry["per_session_warm_medians_ms"]}
        result["first_p95_250ms"] = {"status": "met" if first <= 250 else "missed", "measured_ms": first, "limit_ms": 250,
                                    "fresh_processes": entry["first_commands_ms"]["count"],
                                    "note": "First after Session ready and initial Clear; not cold OS-cache application launch."}
    if optimized.get("status") == "complete" and optimized.get("identity", {}).get("consistent"):
        ratios = {phase: median(optimized["paired_ratios"][phase]["app_cpu_seconds"]) for phase in STREAMS}
        total = median(optimized["paired_ratios"]["total_stream_cpu"])
        result["stream_app_cpu_1_3x_old"] = {"status": "met" if all(value <= 1.3 for value in ratios.values()) and total <= 1.3 else "missed",
                                          "phase_median_paired_ratios": ratios, "total_median_paired_ratio": total, "limit": 1.3}
    if before.get("status") == optimized.get("status") == "complete" and before.get("identity", {}).get("consistent") and optimized.get("identity", {}).get("consistent"):
        deltas = {phase: median(optimized["aggregate"]["new"][phase]["simultaneous_rss_median_bytes"]) - median(before["aggregate"]["new"][phase]["simultaneous_rss_median_bytes"]) for phase in SETTLED}
        draws = {phase: median(optimized["aggregate"]["new"][phase]["draws"]) - median(before["aggregate"]["new"][phase]["draws"]) for phase in SETTLED}
        result["settled_memory_vs_before"] = {"status": "met point estimate" if all(value <= 0 for value in deltas.values()) else "missed point estimate",
                                             "simultaneous_rss_delta_bytes": deltas, "note": "Three-pair medians and allocator variation; no statistical significance asserted."}
        result["idle_redraws_vs_before"] = {"status": "met" if all(value <= 0 for value in draws.values()) else "missed", "median_draw_delta": draws}
        previous_cpu = median(before["aggregate"]["new"]["idle"]["app_cpu_seconds"])
        final_cpu = median(optimized["aggregate"]["new"]["idle"]["app_cpu_seconds"])
        result["idle_cpu_vs_before"] = {
            "status": "same observed counter quantum" if math.isclose(previous_cpu, final_cpu, abs_tol=1e-9) else ("met point estimate" if final_cpu < previous_cpu else "missed point estimate"),
            "before_cpu_seconds": previous_cpu, "after_cpu_seconds": final_cpu,
            "before_percent_one_core": median(before["aggregate"]["new"]["idle"]["app_cpu_percent_one_core"]),
            "after_percent_one_core": median(optimized["aggregate"]["new"]["idle"]["app_cpu_percent_one_core"]),
            "note": "Short two-second idle windows and quantized counters do not establish a small percentage change."}
    return result


def number(value, scale=1):
    return "—" if value is None else f"{value / scale:.2f}"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--before", type=Path, default=PARENT / "baseline-resources/results.json")
    parser.add_argument("--counting", type=Path, default=PARENT / "layout-stage1-resources/results.json")
    parser.add_argument("--optimized", type=Path, default=PARENT / "optimized-resources/results.json")
    parser.add_argument("--startup", type=Path, default=PARENT / "startup-results/results.json")
    parser.add_argument("--optimized-startup-label", default="optimized")
    parser.add_argument("--trace-context", type=Path)
    parser.add_argument("--output", type=Path, default=Path(__file__).with_name("assessment"))
    parser.add_argument("--require-final", action="store_true")
    args = parser.parse_args()
    resources = {label: assess_resource(label, path, args.trace_context if label == "optimized" else None)
                 for label, path in (("before", args.before), ("counting", args.counting), ("optimized", args.optimized))}
    startup = assess_startup(args.startup)
    report = {"schema_version": 1, "assessed_utc": dt.datetime.now(dt.timezone.utc).isoformat(), "assessment_sha256": sha(__file__),
              "resources": resources, "startup": startup,
              "counting_change": changes(resources["before"], resources["counting"]),
              "optimized_change": changes(resources["before"], resources["optimized"]),
              "targets": targets(resources["before"], resources["optimized"], startup, args.optimized_startup_label),
              "limitations": ["CPU is cumulative app-counter change between valid process snapshots, excluding phase boundaries, not an exact whole-operation counter.",
                              "Simultaneous RSS adds app and descendant address spaces within each single snapshot; shared pages may be counted twice.",
                              "Per-run simultaneous RSS medians are aggregated across runs; independent app/child medians are never added.",
                              "Wall timing changed in the unchanged old control across batches; use matched per-repetition old/new ratios as well as raw before/after values.",
                              "Copy All retains different byte counts in old/new and is not equal-byte throughput; copied bytes remain recorded.",
                              "Fresh-process startup timings follow SDL/font fixture setup and initial viewport/Clear; hashing and prior runs warm file caches.",
                              "Cold-loader or first-artifact penalties remain visible as individual first-command values; fresh copied artifacts are not cold OS-cache proof.",
                              "Aspirational targets are evidence assessments, not test assertions or universal hardware/platform guarantees."]}
    args.output.mkdir(parents=True, exist_ok=True)
    (args.output / "assessment.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    lines = ["# Terminal performance assessment", "", "| Variant | Phase | Old app CPU s | New app CPU s | Paired new/old CPU | New wall ms | New combined RSS MiB |", "|---|---|---:|---:|---:|---:|---:|"]
    for label, resource in resources.items():
        if resource["status"] != "complete":
            lines.append(f"| {label} | Pending evidence | — | — | — | — | — |")
            continue
        for phase in (*STREAMS, "resize", "idle", "plateau_first", "plateau_second", "plateau_third", "copied_idle"):
            old, new = resource["aggregate"]["old"][phase], resource["aggregate"]["new"][phase]
            lines.append(f"| {label} | {phase} | {number(median(old['app_cpu_seconds']))} | {number(median(new['app_cpu_seconds']))} | {number(median(resource['paired_ratios'][phase]['app_cpu_seconds']))} | {number(median(new['operation_ms']))} | {number(median(new['simultaneous_rss_median_bytes']), 1048576)} |")
    lines.extend(["", "| Target | Assessment |", "|---|---|"])
    lines.extend(f"| {target} | {entry['status']} |" for target, entry in report["targets"].items())
    lines.extend(["", "| Startup | First median ms | First min–max ms | First p95 ms | Warm median ms | Warm min–max ms |", "|---|---:|---:|---:|---:|---:|"])
    for key, entry in startup.get("summary", {}).items():
        first, warm = entry["first_commands_ms"], entry["warm_pooled_ms"]
        lines.append(f"| {key} | {number(first['median'])} | {number(first['min'])}–{number(first['max'])} | {number(first['p95_nearest_rank'])} | {number(warm['median'])} | {number(warm['min'])}–{number(warm['max'])} |")
    for label, resource in resources.items():
        if resource["status"] != "complete":
            continue
        lines.extend(["", f"{label}: source/binary consistency {resource['identity']['consistent']}; tracing: {resource['trace']['status']}."])
        for kind in ("old", "new"):
            growth = resource["aggregate"][kind]["plateau_growth"]["simultaneous_rss_median_bytes"]
            lines.append(f"{kind} median combined settled RSS growth: first→second {number(median(growth['second_minus_first']), 1048576)} MiB, second→third {number(median(growth['third_minus_second']), 1048576)} MiB.")
        exclusions = [(run['kind'], run['repetition'], run['observer']['excluded_boundary_samples'], run['observer']['total_samples']) for run in resource['runs']]
        lines.append("Observer boundary exclusions (kind, repetition, excluded, total): " + str(exclusions) + ".")
    lines.extend(["", *["- " + note for note in report["limitations"]]])
    (args.output / "assessment.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(str(args.output / "assessment.md"))
    if args.require_final and (any(resource["status"] != "complete" or not resource.get("identity", {}).get("consistent") for resource in resources.values())
                               or startup["status"] != "complete" or not startup.get("identity", {}).get("consistent")
                               or not startup.get("trace_disabled") or resources["optimized"].get("trace", {}).get("disabled") is not True
                               or any(target["status"] == "pending" for target in report["targets"].values())):
        raise SystemExit("Final evidence incomplete, identities inconsistent, or final trace-off status unproven; see assessment.json")


if __name__ == "__main__":
    main()
