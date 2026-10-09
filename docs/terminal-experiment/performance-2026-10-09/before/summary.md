# Matched terminal resource measurements

2026-10-09T11:22:24.122793+00:00 · release test binary · 3 fresh paired samples, alternating order.

Execution: **x86_64 release binary under Rosetta on ARM hardware**. SDL uses a hidden 800×600 dummy-video fixture, embedded fonts and the app's poll/frame schedule. This is not a native window idle measurement.

App and descendant processes are separate scopes. CPU is a percentage of one core from cumulative process counters; small zero values can be below `ps` time resolution. Descendant CPU covers observed process lifetimes and can miss short-lived children.

Only snapshots whose whole `ps` interval is within an observed phase are aggregated. Boundary exclusions and timestamps remain in the raw JSONL evidence. The source digest identifies the sampled checkout; compiled-source identity requires the matching frozen-build manifest.

| Phase | Integration | App RSS median MiB | App CPU s | App CPU % | Descendant RSS median MiB | Observed descendant CPU s | Observed descendant CPU % |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| idle | old | 14.16 | 0.01 | 0.53 | 0.00 | 0.00 | 0.00 |
| idle | new | 14.97 | 0.01 | 0.51 | 3.96 | 0.00 | 0.00 |
| small_command | old | 14.84 | — | — | 1.31 | — | — |
| small_command | new | 15.74 | 0.01 | 3.70 | 4.18 | 0.02 | 7.05 |
| small_idle | old | 15.29 | 0.01 | 0.53 | 0.00 | 0.00 | 0.00 |
| small_idle | new | 16.50 | 0.01 | 0.53 | 4.21 | 0.00 | 0.00 |
| stream_first | old | 33.77 | 0.62 | 23.42 | 12.30 | 0.09 | 3.36 |
| stream_first | new | 35.22 | 1.20 | 42.72 | 19.74 | 0.15 | 5.34 |
| plateau_first | old | 41.01 | 0.01 | 0.53 | 0.00 | 0.00 | 0.00 |
| plateau_first | new | 42.82 | 0.01 | 0.51 | 4.07 | 0.00 | 0.00 |
| stream_second | old | 43.39 | 0.61 | 23.24 | 12.42 | 0.09 | 3.52 |
| stream_second | new | 45.40 | 1.24 | 44.89 | 19.74 | 0.15 | 5.43 |
| plateau_second | old | 47.41 | 0.01 | 0.56 | 0.00 | 0.00 | 0.00 |
| plateau_second | new | 48.78 | 0.01 | 0.56 | 4.07 | 0.00 | 0.00 |
| stream_third | old | 49.51 | 0.62 | 23.15 | 12.07 | 0.07 | 2.63 |
| stream_third | new | 49.68 | 1.36 | 48.53 | 19.52 | 0.15 | 5.31 |
| plateau_third | old | 50.73 | 0.01 | 0.51 | 0.00 | 0.00 | 0.00 |
| plateau_third | new | 50.39 | 0.01 | 0.52 | 4.08 | 0.00 | 0.00 |
| resize | old | 82.00 | 2.19 | 99.53 | 0.00 | 0.00 | 0.00 |
| resize | new | 53.28 | 2.33 | 99.52 | 4.08 | 0.00 | 0.00 |
| copy_selected | old | 100.83 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| copy_selected | new | 57.62 | 0.00 | 0.00 | 4.08 | 0.00 | 0.00 |
| copy_all | old | 116.83 | 0.01 | 1.44 | 0.00 | 0.00 | 0.00 |
| copy_all | new | 78.37 | 0.05 | 7.24 | 4.08 | 0.00 | 0.00 |
| copied_idle | old | 116.83 | 0.01 | 0.55 | 0.00 | 0.00 | 0.00 |
| copied_idle | new | 78.37 | 0.01 | 0.52 | 4.08 | 0.00 | 0.00 |

| Operation | Integration | Median ms | Min–max ms | Median copied MiB |
| --- | --- | ---: | ---: | ---: |
| small_command | old | 84.36 | 59.68–86.78 | — |
| small_command | new | 210.47 | 195.53–2971.14 | — |
| stream_first | old | 2800.86 | 2796.49–2818.43 | — |
| stream_first | new | 2921.23 | 2899.91–2944.33 | — |
| stream_second | old | 2812.18 | 2759.38–2821.49 | — |
| stream_second | new | 2940.47 | 2913.54–2966.80 | — |
| stream_third | old | 2826.99 | 2777.40–2840.05 | — |
| stream_third | new | 2958.47 | 2957.93–2972.41 | — |
| resize | old | 2314.48 | 2306.03–2316.12 | — |
| resize | new | 2411.53 | 2400.34–2426.86 | — |
| copy_selected | old | 0.01 | 0.01–0.04 | 0.00 |
| copy_selected | new | 1.42 | 1.40–1.53 | 0.00 |
| copy_all | old | 12.65 | 10.60–15.78 | 8.00 |
| copy_all | new | 47.34 | 46.15–50.62 | 4.56 |

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
