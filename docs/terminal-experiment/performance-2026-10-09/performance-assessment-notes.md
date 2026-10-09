# Independent assessment

`assess-terminal-performance.py` reads existing evidence and frozen inputs only. It never builds or executes a terminal workload. Default resource locations are `baseline-resources`, `layout-stage1-resources`, and `optimized-resources` under this experiment's parent directory. Missing final evidence stays pending.

Before/counting assessment is available at `review/assessment-before-counting/assessment.{json,md}`. Both source/build/binary/helper identities are consistent. Counting-only median matched new/old app CPU ratios are 1.63×, 1.67×, and 1.74× over the three streams, versus 1.94×, 2.07×, and 2.17× before. Resize CPU is 1.51 seconds versus 2.33 before; within-batch matched new/old ratio is 0.67×. The unchanged old stream wall times also changed between batches, so raw wall-time improvements must not all be attributed to counting changes. The counting-only resource first-command outlier remains 2958.948 ms.

The script preserves per-run matching phases and ratios, then aggregates them; it does not divide unpaired pooled samples. Combined RSS is calculated per raw simultaneous app/descendant snapshot before per-run medians and between-run aggregation. It recomputes complete-observer-interval phase classification and boundary exclusion counts. Settled memory continues growing across the three streams; no flat plateau is claimed. CPU deltas exclude the unsampled beginning/end of phases and short-lived descendants may be undercounted.

For final evidence, save a resource run context containing the three exact result identities, both trace variables explicitly absent, and the exact runner command:

```json
{
  "source_sha256": "FINAL_RESOURCE_SOURCE_SHA",
  "binary_sha256": "FINAL_RESOURCE_BINARY_SHA",
  "helper_sha256": "FINAL_RESOURCE_HELPER_SHA",
  "trace_environment": {
    "POTYI_TERM_TRACE": null,
    "POTYI_TERM_TRACE_FILE": null
  },
  "command": ["THE", "EXACT", "RESOURCE", "RUNNER", "ARGUMENTS"]
}
```

The frozen before/counting bridge source predates trace controls. Final optimized resource tracing is marked unproven until this matching context exists. Startup collector explicitly clears both trace controls and records tracing disabled per process.

Root chooses the actual startup output directory and variant label; use those options when regenerating the final report:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 review/assess-terminal-performance.py \
  --startup /ABSOLUTE/FINAL-STARTUP/results.json \
  --optimized-startup-label optimized \
  --trace-context /ABSOLUTE/optimized-resource-run-context.json \
  --output review/final-assessment \
  --require-final
```

`--require-final` fails if any input is pending/incomplete, identities differ, or trace-off evidence is missing. It does not fail when aspirational performance targets are missed: those misses are reported honestly. Startup first-command outliers and all five warmed commands per Session remain in distributions. First p95 uses nearest rank; with ten fresh processes it includes the maximum. Fixture/window/font setup precedes timing, and neither first-use artifact copies nor file hashing prove cold OS-cache launches. Fresh copied helper/test paths and their compiled-artifact build manifests stay recorded so the symmetric first-use scenario is auditable.

# Session and literal-output review

Reviewed final production `session.rs` and `literal_output.rs` against frozen baseline. Root's final source identity changed from `72d2e659…` to `d6f0a4f7…` only for lifecycle test assertions; these reviewed production changes stayed identical. No correctness defect found by source inspection. Root owns execution and full acceptance claims.

- Bulk copying is enabled only in Ground with no pending CR or partial UTF-8 sequence. ASCII printable bytes, tabs, and DEL follow the exact previous acceptance rule; every other control and non-ASCII byte continues through the original parser.
- A run is capped by remaining partial-row space. At a full row, continuation is emitted before consuming the next ASCII byte; failed emission leaves the unwritten partial row intact. A newline at exact capacity follows the previous non-continuation newline path. No partial String grows beyond its existing 60 KiB bound.
- Existing UTF-8 decoder and character/row emitter are unchanged except formatting. Split malformed sequences, replacement placement, CR ordering, CSI/OSC suppression and CAN recovery use the same paths as baseline.
- Differential tests include the exact old byte-processing loop and compare emitted rows, continuation flags, internal parser/CR/UTF-8 state and capacity across mixed Unicode/control splits, cap boundaries, deterministic malformed sequences and callback/finish failures. They share only unchanged decoder/emitter routines.
- Session acquires a transcript guard lazily on first emitted row, then reuses it within the existing 1 KiB parse slice. Tail finish can emit only bounded retained partial text. It adds no new queue or aggregate output buffer.
- The guard is explicitly dropped before error handling and boundary/seal processing. `append_plain` and its history/style mutation have no reentrant acquisition of the same transcript mutex. Output ordering, provenance, error latching and final-row sealing remain unchanged. Disk operations can still take variable wall time; batching does not convert the existing 2 ms scheduling budget into a hard filesystem latency guarantee.
