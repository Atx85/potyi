# Matched terminal resource measurements

2026-10-09T09:05:29.717357+00:00 · release test binary · 3 fresh paired samples, alternating order.

Execution: **x86_64 release binary under Rosetta on ARM hardware**. SDL uses a hidden 800×600 dummy-video fixture, embedded fonts and the app's poll/frame schedule. This is not a native window idle measurement.

App and descendant processes are separate scopes. CPU is a percentage of one core from cumulative process counters; small zero values can be below `ps` time resolution. Descendant CPU covers observed process lifetimes and can miss short-lived children.

Only snapshots whose whole `ps` interval is within an observed phase are aggregated. Boundary exclusions and timestamps remain in the raw JSONL evidence. The source digest identifies the sampled checkout; compiled-source identity requires the matching frozen-build manifest.

| Phase | Integration | App RSS median MiB | App CPU s | App CPU % | Descendant RSS median MiB | Observed descendant CPU s | Observed descendant CPU % |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| idle | old | 14.45 | 0.01 | 0.52 | 0.00 | 0.00 | 0.00 |
| idle | new | 15.45 | 0.01 | 0.55 | 3.93 | 0.00 | 0.00 |
| small_command | old | 15.07 | — | — | 1.92 | — | — |
| small_command | new | 16.45 | 0.01 | 3.72 | 4.11 | 0.02 | 6.89 |
| small_idle | old | 15.48 | 0.01 | 0.55 | 0.00 | 0.00 | 0.00 |
| small_idle | new | 16.86 | 0.01 | 0.53 | 4.19 | 0.00 | 0.00 |
| stream_first | old | 34.31 | 0.66 | 23.83 | 12.48 | 0.06 | 2.27 |
| stream_first | new | 33.64 | 2.89 | 85.57 | 20.02 | 0.15 | 4.36 |
| plateau_first | old | 45.07 | 0.01 | 0.56 | 0.00 | 0.00 | 0.00 |
| plateau_first | new | 42.01 | 0.01 | 0.53 | 4.06 | 0.00 | 0.00 |
| stream_second | old | 50.21 | 0.66 | 24.10 | 12.84 | 0.06 | 2.19 |
| stream_second | new | 45.46 | 3.32 | 98.19 | 20.06 | 0.15 | 4.45 |
| plateau_second | old | 54.78 | 0.01 | 0.55 | 0.00 | 0.00 | 0.00 |
| plateau_second | new | 47.82 | 0.01 | 0.53 | 4.08 | 0.00 | 0.00 |
| resize | old | 73.76 | 2.15 | 99.87 | 0.00 | 0.00 | 0.00 |
| resize | new | 59.34 | 3.59 | 97.33 | 4.08 | 0.00 | 0.00 |
| copy_selected | old | 86.95 | 0.01 | 1.59 | 0.00 | 0.00 | 0.00 |
| copy_selected | new | 68.38 | 0.00 | 0.00 | 4.08 | 0.00 | 0.00 |
| copy_all | old | 105.28 | 0.01 | 1.45 | 0.00 | 0.00 | 0.00 |
| copy_all | new | 87.79 | 0.20 | 23.51 | 4.08 | 0.00 | 0.00 |
| copied_idle | old | 105.28 | 0.01 | 0.52 | 0.00 | 0.00 | 0.00 |
| copied_idle | new | 87.79 | 0.01 | 0.54 | 4.08 | 0.00 | 0.00 |

| Operation | Integration | Median ms | Min–max ms | Median copied MiB |
| --- | --- | ---: | ---: | ---: |
| small_command | old | 60.51 | 55.09–73.76 | — |
| small_command | new | 215.97 | 189.79–2862.97 | — |
| stream_first | old | 2853.62 | 2838.59–2890.49 | — |
| stream_first | new | 3498.30 | 3472.46–3516.83 | — |
| stream_second | old | 2866.19 | 2829.36–2879.40 | — |
| stream_second | new | 3497.64 | 3496.10–3529.69 | — |
| resize | old | 2230.35 | 2227.03–2233.63 | — |
| resize | new | 3831.59 | 3808.07–3874.86 | — |
| copy_selected | old | 0.01 | 0.01–0.01 | 0.00 |
| copy_selected | new | 1.40 | 1.36–1.46 | 0.00 |
| copy_all | old | 12.03 | 9.83–13.46 | 7.66 |
| copy_all | new | 209.74 | 207.32–216.64 | 4.59 |

Each integration receives the same 64KiB small producer and two paced 12MiB producers (24MiB cumulative cap-crossing output), then 9 width/height changes and selected ≤4KiB / all-retained clipboard copies. Commands, cwd, fonts and test binary are shared. The second plateau observes continued eviction after the first cap crossing; no absolute memory or CPU budget is asserted.

RSS includes allocator-retained memory, headless renderer/texture caches, terminal grids and clipboard storage. Clipboard phases include verification through SDL. Binary/test fixture setup and producer creation are outside the measured settled phases.

CopyAll is each integration's retained plaintext, whose size is recorded above: the new 8MiB budget includes record/index metadata, while legacy caps its output text. CopyAll costs therefore cover different retained byte counts; the selected ≤4KiB scope is matched. Producer rows contain changing serial numbers and Unicode, rather than repeating one cache-identical row.

The command helper path is explicit and overrides inherited test-helper settings; its SHA-256 is checked before and after each run and against the frozen-build manifest when supplied.

- Headless hidden SDL fixture; no native compositor, OS keyboard delivery or visible clipboard UX is measured.
- The persistent new terminal shell contributes to descendant idle RSS; it is separate from app RSS.
- Polling ps is an external observer with scheduling overhead, retained in raw sample timings.
- Whole observer intervals crossing observed phase markers are excluded; marker timestamps include log reader scheduling delay.
- CPU counters have platform-dependent resolution; vanished/short-lived descendants can be undercounted.
- Fast phases may have no ps sample; operation latency remains in events rather than inventing zero resource usage.
- Three pairs describe this machine and warm local temporary storage, not universal hardware/OS guarantees.
- RSS sums child address spaces and may double-count shared pages; it is not proportional/private memory.
- The prebuilt binary hash is recorded; release profile is validated against the supplied frozen-build manifest when present, otherwise caller supplied.

[Raw metrics, phase events, source/binary hashes and log names](results.json). Per-run JSONL files retain every app/descendant sample.
