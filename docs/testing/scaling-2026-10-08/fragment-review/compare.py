#!/usr/bin/env python3
"""Serial, interleaved comparison of immutable saved Pötyi probes."""
import datetime as dt
import importlib.util
import json
import os
from pathlib import Path
import tempfile

ROOT = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("saved_scaling", ROOT / "harness/scaling.py")
scaling = importlib.util.module_from_spec(spec)
spec.loader.exec_module(scaling)
provenance = json.loads((ROOT / "provenance.json").read_text())
scaling.ROOT = Path(provenance["repository"])
report = dict(schema_version=1, measured_at=dt.datetime.now(dt.timezone.utc).isoformat(),
              provenance=provenance,
              settings=dict(samples=3, warmups=1, iterations=4, timeout_seconds=60,
                            render=False, memory_sampling=False, interleaved=True),
              execution_order=[], cases=[])


def save():
    for case in report["cases"]:
        for variant in case["variants"].values():
            variant["summary"] = scaling.summarize(variant["runs"])
    (ROOT / "results.json").write_text(json.dumps(report, indent=2) + "\n")


for case_index, config in enumerate(scaling.matrix("fragments", 4, False)):
    case = dict(config=config, variants={label: dict(runs=[], summary=[]) for label in ("fixed64", "adaptive")})
    report["cases"].append(case)
    stopped = set()
    for repetition in range(4):
        order = ["fixed64", "adaptive"] if (repetition + case_index) % 2 == 0 else ["adaptive", "fixed64"]
        for label in order:
            if label in stopped:
                continue
            with tempfile.TemporaryDirectory(prefix="potyi-fragment-fixture-", dir="/private/tmp") as temp:
                current = {**config, "fixture_dir": str(Path(temp) / "fixture")}
                env = {**os.environ, "SDL_VIDEODRIVER": "dummy", "POTYI_SCALING_CASE": json.dumps(current)}
                logfile = ROOT / label / f"fragments{config['fragmented_edits']}-run{repetition:02}.log"
                row = scaling.bounded([provenance["variants"][label]["binary_path"], scaling.PROBE,
                                       "--exact", "--ignored", "--nocapture", "--test-threads=1"],
                                      env, logfile, 60, memory=False)
            row.update(warmup=repetition == 0, repetition=repetition,
                       log=str(logfile.relative_to(ROOT)), refill_variant=label)
            case["variants"][label]["runs"].append(row)
            report["execution_order"].append(dict(fragmented_edits=config["fragmented_edits"],
                                                 repetition=repetition, variant=label))
            save()
            print(f"fragments={config['fragmented_edits']} variant={label} run={repetition+1} "
                  f"status={row['status']} seconds={row['seconds']:.3f} phase={row['last_phase']}", flush=True)
            if row["status"] != "complete":
                stopped.add(label)
save()
print(f"Saved {ROOT / 'results.json'}", flush=True)
