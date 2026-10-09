# Matched terminal resource measurements

2026-10-09T12:28:44.141493+00:00 · release test binary · 3 fresh paired samples, alternating order.

Execution: **x86_64 release binary under Rosetta on ARM hardware**. SDL uses a hidden 800×600 dummy-video fixture, embedded fonts and the app's poll/frame schedule. This is not a native window idle measurement.

App and descendant processes are separate scopes. CPU is a percentage of one core from cumulative process counters; small zero values can be below `ps` time resolution. Descendant CPU covers observed process lifetimes and can miss short-lived children.

Only snapshots whose whole `ps` interval is within an observed phase are aggregated. Boundary exclusions and timestamps remain in the raw JSONL evidence. The source digest identifies the sampled checkout; compiled-source identity requires the matching frozen-build manifest.

| Phase | Integration | App RSS median MiB | App CPU s | App CPU % | Descendant RSS median MiB | Observed descendant CPU s | Observed descendant CPU % |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| idle | old | 14.11 | 0.01 | 0.53 | 0.00 | 0.00 | 0.00 |
| idle | new | 15.08 | 0.01 | 0.54 | 3.95 | 0.00 | 0.00 |
| small_command | old | 14.79 | — | — | 1.41 | — | — |
| small_command | new | 15.70 | — | — | 4.16 | — | — |
| small_idle | old | 15.38 | 0.01 | 0.54 | 0.00 | 0.00 | 0.00 |
| small_idle | new | 16.59 | 0.01 | 0.55 | 4.20 | 0.00 | 0.00 |
| stream_first | old | 32.08 | 0.59 | 26.03 | 12.06 | 0.06 | 2.69 |
| stream_first | new | 34.00 | 0.88 | 34.86 | 16.03 | 0.15 | 5.94 |
| plateau_first | old | 40.35 | 0.01 | 0.54 | 0.00 | 0.00 | 0.00 |
| plateau_first | new | 42.37 | 0.01 | 0.53 | 4.05 | 0.00 | 0.00 |
| stream_second | old | 43.23 | 0.60 | 25.18 | 12.08 | 0.09 | 4.03 |
| stream_second | new | 45.43 | 0.94 | 37.55 | 15.59 | 0.14 | 5.60 |
| plateau_second | old | 46.52 | 0.01 | 0.54 | 0.00 | 0.00 | 0.00 |
| plateau_second | new | 48.20 | 0.01 | 0.55 | 4.06 | 0.00 | 0.00 |
| stream_third | old | 47.82 | 0.60 | 25.73 | 12.11 | 0.09 | 3.75 |
| stream_third | new | 49.55 | 0.99 | 40.01 | 15.66 | 0.14 | 5.84 |
| plateau_third | old | 49.89 | 0.01 | 0.54 | 0.00 | 0.00 | 0.00 |
| plateau_third | new | 50.45 | 0.01 | 0.54 | 4.06 | 0.00 | 0.00 |
| resize | old | 78.99 | 2.21 | 99.74 | 0.00 | 0.00 | 0.00 |
| resize | new | 56.88 | 1.64 | 100.15 | 4.06 | 0.00 | 0.00 |
| copy_selected | old | 108.28 | 0.01 | 1.57 | 0.00 | 0.00 | 0.00 |
| copy_selected | new | 60.18 | 0.01 | 1.50 | 4.06 | 0.00 | 0.00 |
| copy_all | old | 135.21 | 0.01 | 1.41 | 0.00 | 0.00 | 0.00 |
| copy_all | new | 81.73 | 0.05 | 7.59 | 4.06 | 0.00 | 0.00 |
| copied_idle | old | 135.21 | 0.01 | 0.54 | 0.00 | 0.00 | 0.00 |
| copied_idle | new | 81.73 | 0.01 | 0.54 | 4.06 | 0.00 | 0.00 |

| Operation | Integration | Median ms | Min–max ms | Median copied MiB |
| --- | --- | ---: | ---: | ---: |
| small_command | old | 77.55 | 59.23–80.71 | — |
| small_command | new | 193.16 | 182.48–196.10 | — |
| stream_first | old | 2450.03 | 2447.27–2452.18 | — |
| stream_first | new | 2634.68 | 2587.74–2774.86 | — |
| stream_second | old | 2446.84 | 2443.59–2464.38 | — |
| stream_second | new | 2588.89 | 2573.54–2616.41 | — |
| stream_third | old | 2453.83 | 2440.65–2470.41 | — |
| stream_third | new | 2603.66 | 2595.40–2610.86 | — |
| resize | old | 2300.51 | 2286.78–2309.15 | — |
| resize | new | 1709.60 | 1701.42–1711.71 | — |
| copy_selected | old | 0.01 | 0.01–0.01 | 0.00 |
| copy_selected | new | 1.41 | 1.41–1.42 | 0.00 |
| copy_all | old | 14.25 | 12.16–17.38 | 8.00 |
| copy_all | new | 49.97 | 46.53–50.08 | 4.56 |

Each integration receives the same 64KiB small producer and three paced 12MiB producers (36MiB cumulative cap-crossing output), then 9 width/height changes and selected ≤4KiB / all-retained clipboard copies. Commands, cwd, fonts and test binary are shared. Later settled phases check continued eviction and memory growth after the first cap crossing; a flat RSS plateau is not asserted.

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
