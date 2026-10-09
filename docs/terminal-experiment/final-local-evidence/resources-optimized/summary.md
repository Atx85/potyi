# Matched terminal resource measurements

2026-10-09T09:53:54.481044+00:00 · release test binary · 3 fresh paired samples, alternating order.

Execution: **x86_64 release binary under Rosetta on ARM hardware**. SDL uses a hidden 800×600 dummy-video fixture, embedded fonts and the app's poll/frame schedule. This is not a native window idle measurement.

App and descendant processes are separate scopes. CPU is a percentage of one core from cumulative process counters; small zero values can be below `ps` time resolution. Descendant CPU covers observed process lifetimes and can miss short-lived children.

Only snapshots whose whole `ps` interval is within an observed phase are aggregated. Boundary exclusions and timestamps remain in the raw JSONL evidence. The source digest identifies the sampled checkout; compiled-source identity requires the matching frozen-build manifest.

| Phase | Integration | App RSS median MiB | App CPU s | App CPU % | Descendant RSS median MiB | Observed descendant CPU s | Observed descendant CPU % |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| idle | old | 14.28 | 0.01 | 0.52 | 0.00 | 0.00 | 0.00 |
| idle | new | 15.28 | 0.01 | 0.52 | 3.95 | 0.00 | 0.00 |
| small_command | old | 14.80 | — | — | 0.01 | — | — |
| small_command | new | 15.96 | 0.02 | 0.72 | 4.13 | 0.02 | 0.72 |
| small_idle | old | 15.38 | 0.01 | 0.53 | 0.00 | 0.00 | 0.00 |
| small_idle | new | 16.97 | 0.01 | 0.53 | 4.21 | 0.00 | 0.00 |
| stream_first | old | 31.59 | 0.59 | 25.46 | 12.33 | 0.06 | 2.63 |
| stream_first | new | 33.68 | 1.13 | 46.19 | 20.01 | 0.12 | 4.91 |
| plateau_first | old | 38.73 | 0.01 | 0.56 | 0.00 | 0.00 | 0.00 |
| plateau_first | new | 40.37 | 0.01 | 0.53 | 4.09 | 0.00 | 0.00 |
| stream_second | old | 42.25 | 0.57 | 25.26 | 12.55 | 0.05 | 2.23 |
| stream_second | new | 44.80 | 1.19 | 48.83 | 19.94 | 0.12 | 4.99 |
| plateau_second | old | 44.50 | 0.01 | 0.53 | 0.00 | 0.00 | 0.00 |
| plateau_second | new | 47.06 | 0.01 | 0.55 | 4.08 | 0.00 | 0.00 |
| stream_third | old | 45.43 | 0.59 | 25.49 | 12.34 | 0.07 | 2.97 |
| stream_third | new | 48.44 | 1.30 | 53.02 | 20.09 | 0.11 | 4.42 |
| plateau_third | old | 46.48 | 0.01 | 0.56 | 0.00 | 0.00 | 0.00 |
| plateau_third | new | 48.78 | 0.01 | 0.56 | 4.10 | 0.00 | 0.00 |
| resize | old | 82.52 | 2.17 | 99.71 | 0.00 | 0.00 | 0.00 |
| resize | new | 54.87 | 2.26 | 100.05 | 4.10 | 0.00 | 0.00 |
| copy_selected | old | 93.52 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| copy_selected | new | 57.76 | 0.00 | 0.00 | 4.10 | 0.00 | 0.00 |
| copy_all | old | 109.52 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| copy_all | new | 74.14 | 0.03 | 4.22 | 4.10 | 0.00 | 0.00 |
| copied_idle | old | 109.52 | 0.02 | 1.04 | 0.00 | 0.00 | 0.00 |
| copied_idle | new | 74.14 | 0.01 | 0.54 | 4.10 | 0.00 | 0.00 |

| Operation | Integration | Median ms | Min–max ms | Median copied MiB |
| --- | --- | ---: | ---: | ---: |
| small_command | old | 53.31 | 45.83–59.51 | — |
| small_command | new | 197.86 | 173.62–2908.28 | — |
| stream_first | old | 2411.75 | 2409.13–2431.66 | — |
| stream_first | new | 2611.20 | 2582.93–2613.10 | — |
| stream_second | old | 2421.38 | 2417.32–2428.51 | — |
| stream_second | new | 2611.33 | 2607.92–2616.74 | — |
| stream_third | old | 2418.85 | 2402.53–2427.22 | — |
| stream_third | new | 2623.17 | 2596.83–2686.19 | — |
| resize | old | 2305.19 | 2293.97–2308.02 | — |
| resize | new | 2400.09 | 2393.39–2403.15 | — |
| copy_selected | old | 0.01 | 0.01–0.02 | 0.00 |
| copy_selected | new | 1.47 | 1.44–1.51 | 0.00 |
| copy_all | old | 10.65 | 10.11–11.44 | 8.00 |
| copy_all | new | 45.83 | 45.47–45.90 | 4.56 |

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
