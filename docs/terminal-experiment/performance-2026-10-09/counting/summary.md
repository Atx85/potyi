# Matched terminal resource measurements

2026-10-09T11:31:21.413097+00:00 · release test binary · 3 fresh paired samples, alternating order.

Execution: **x86_64 release binary under Rosetta on ARM hardware**. SDL uses a hidden 800×600 dummy-video fixture, embedded fonts and the app's poll/frame schedule. This is not a native window idle measurement.

App and descendant processes are separate scopes. CPU is a percentage of one core from cumulative process counters; small zero values can be below `ps` time resolution. Descendant CPU covers observed process lifetimes and can miss short-lived children.

Only snapshots whose whole `ps` interval is within an observed phase are aggregated. Boundary exclusions and timestamps remain in the raw JSONL evidence. The source digest identifies the sampled checkout; compiled-source identity requires the matching frozen-build manifest.

| Phase | Integration | App RSS median MiB | App CPU s | App CPU % | Descendant RSS median MiB | Observed descendant CPU s | Observed descendant CPU % |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| idle | old | 14.18 | 0.01 | 0.53 | 0.00 | 0.00 | 0.00 |
| idle | new | 15.07 | 0.01 | 0.53 | 3.96 | 0.00 | 0.00 |
| small_command | old | 14.77 | — | — | 1.62 | — | — |
| small_command | new | 16.03 | 0.01 | 3.79 | 4.21 | 0.03 | 7.41 |
| small_idle | old | 15.36 | 0.01 | 0.54 | 0.00 | 0.00 | 0.00 |
| small_idle | new | 16.69 | 0.01 | 0.53 | 4.24 | 0.00 | 0.00 |
| stream_first | old | 31.66 | 0.60 | 26.32 | 12.08 | 0.08 | 3.39 |
| stream_first | new | 33.97 | 0.97 | 38.37 | 19.49 | 0.15 | 6.13 |
| plateau_first | old | 38.72 | 0.01 | 0.55 | 0.00 | 0.00 | 0.00 |
| plateau_first | new | 41.88 | 0.01 | 0.53 | 4.11 | 0.00 | 0.00 |
| stream_second | old | 42.93 | 0.60 | 26.64 | 12.05 | 0.07 | 3.11 |
| stream_second | new | 44.78 | 1.01 | 39.91 | 19.88 | 0.15 | 5.92 |
| plateau_second | old | 45.12 | 0.01 | 0.54 | 0.00 | 0.00 | 0.00 |
| plateau_second | new | 46.62 | 0.01 | 0.54 | 4.11 | 0.00 | 0.00 |
| stream_third | old | 47.64 | 0.61 | 26.07 | 12.09 | 0.09 | 3.99 |
| stream_third | new | 48.16 | 1.07 | 42.80 | 19.60 | 0.15 | 5.89 |
| plateau_third | old | 48.82 | 0.01 | 0.53 | 0.00 | 0.00 | 0.00 |
| plateau_third | new | 49.11 | 0.01 | 0.53 | 4.09 | 0.00 | 0.00 |
| resize | old | 83.67 | 2.24 | 99.63 | 0.00 | 0.00 | 0.00 |
| resize | new | 55.23 | 1.51 | 99.67 | 4.09 | 0.00 | 0.00 |
| copy_selected | old | 106.26 | 0.01 | 1.57 | 0.00 | 0.00 | 0.00 |
| copy_selected | new | 59.01 | 0.01 | 1.61 | 4.09 | 0.00 | 0.00 |
| copy_all | old | 122.27 | 0.01 | 1.48 | 0.00 | 0.00 | 0.00 |
| copy_all | new | 81.87 | 0.04 | 5.93 | 4.09 | 0.00 | 0.00 |
| copied_idle | old | 122.27 | 0.01 | 0.55 | 0.00 | 0.00 | 0.00 |
| copied_idle | new | 81.87 | 0.01 | 0.53 | 4.09 | 0.00 | 0.00 |

| Operation | Integration | Median ms | Min–max ms | Median copied MiB |
| --- | --- | ---: | ---: | ---: |
| small_command | old | 74.17 | 53.97–78.73 | — |
| small_command | new | 209.21 | 203.75–2958.95 | — |
| stream_first | old | 2448.79 | 2447.54–2454.28 | — |
| stream_first | new | 2627.81 | 2609.41–2651.67 | — |
| stream_second | old | 2456.31 | 2446.53–2458.80 | — |
| stream_second | new | 2630.15 | 2627.69–2655.70 | — |
| stream_third | old | 2449.39 | 2445.06–2459.65 | — |
| stream_third | new | 2625.10 | 2619.26–2663.50 | — |
| resize | old | 2317.38 | 2314.35–2324.05 | — |
| resize | new | 1698.79 | 1685.65–1700.04 | — |
| copy_selected | old | 0.02 | 0.01–0.02 | 0.00 |
| copy_selected | new | 1.48 | 1.40–1.48 | 0.00 |
| copy_all | old | 13.04 | 11.79–13.22 | 8.00 |
| copy_all | new | 52.67 | 48.70–54.24 | 4.56 |

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
