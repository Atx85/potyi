# Matched terminal resource measurements

2026-10-09T09:38:34.994537+00:00 · release test binary · 3 fresh paired samples, alternating order.

Execution: **x86_64 release binary under Rosetta on ARM hardware**. SDL uses a hidden 800×600 dummy-video fixture, embedded fonts and the app's poll/frame schedule. This is not a native window idle measurement.

App and descendant processes are separate scopes. CPU is a percentage of one core from cumulative process counters; small zero values can be below `ps` time resolution. Descendant CPU covers observed process lifetimes and can miss short-lived children.

Only snapshots whose whole `ps` interval is within an observed phase are aggregated. Boundary exclusions and timestamps remain in the raw JSONL evidence. The source digest identifies the sampled checkout; compiled-source identity requires the matching frozen-build manifest.

| Phase | Integration | App RSS median MiB | App CPU s | App CPU % | Descendant RSS median MiB | Observed descendant CPU s | Observed descendant CPU % |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| idle | old | 14.41 | 0.01 | 0.55 | 0.00 | 0.00 | 0.00 |
| idle | new | 15.09 | 0.01 | 0.55 | 3.93 | 0.00 | 0.00 |
| small_command | old | — | — | — | — | — | — |
| small_command | new | 15.88 | 0.02 | 0.69 | 5.54 | 0.05 | 1.72 |
| small_idle | old | 15.62 | 0.01 | 0.53 | 0.00 | 0.00 | 0.00 |
| small_idle | new | 16.57 | 0.01 | 0.55 | 4.20 | 0.00 | 0.00 |
| stream_first | old | 34.20 | 0.65 | 24.17 | 12.62 | 0.06 | 2.16 |
| stream_first | new | 33.67 | 1.19 | 42.87 | 20.20 | 0.13 | 4.65 |
| plateau_first | old | 42.15 | 0.01 | 0.56 | 0.00 | 0.00 | 0.00 |
| plateau_first | new | 42.27 | 0.01 | 0.56 | 4.08 | 0.00 | 0.00 |
| stream_second | old | 47.95 | 0.63 | 23.84 | 12.44 | 0.05 | 1.89 |
| stream_second | new | 46.63 | 1.26 | 45.69 | 20.02 | 0.12 | 4.34 |
| plateau_second | old | 52.75 | 0.01 | 0.55 | 0.00 | 0.00 | 0.00 |
| plateau_second | new | 48.00 | 0.01 | 0.53 | 4.08 | 0.00 | 0.00 |
| stream_third | old | 56.52 | 0.64 | 24.03 | 12.52 | 0.06 | 2.15 |
| stream_third | new | 50.06 | 1.33 | 47.52 | 19.92 | 0.12 | 4.29 |
| plateau_third | old | 57.44 | 0.01 | 0.56 | 0.00 | 0.00 | 0.00 |
| plateau_third | new | 50.88 | 0.01 | 0.52 | 4.08 | 0.00 | 0.00 |
| resize | old | 82.96 | 2.18 | 99.64 | 0.00 | 0.00 | 0.00 |
| resize | new | 59.78 | 2.30 | 99.89 | 4.08 | 0.00 | 0.00 |
| copy_selected | old | 91.12 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| copy_selected | new | 63.61 | 0.01 | 1.46 | 4.08 | 0.00 | 0.00 |
| copy_all | old | 107.12 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| copy_all | new | 79.59 | 0.03 | 4.39 | 4.08 | 0.00 | 0.00 |
| copied_idle | old | 107.12 | 0.01 | 0.56 | 0.00 | 0.00 | 0.00 |
| copied_idle | new | 79.59 | 0.01 | 0.56 | 4.08 | 0.00 | 0.00 |

| Operation | Integration | Median ms | Min–max ms | Median copied MiB |
| --- | --- | ---: | ---: | ---: |
| small_command | old | 52.06 | 51.84–55.09 | — |
| small_command | new | 202.11 | 166.34–3007.59 | — |
| stream_first | old | 2844.08 | 2835.72–2866.89 | — |
| stream_first | new | 2908.01 | 2876.26–2932.65 | — |
| stream_second | old | 2883.09 | 2847.62–2895.51 | — |
| stream_second | new | 2899.60 | 2880.81–2900.22 | — |
| stream_third | old | 2861.75 | 2836.35–2873.57 | — |
| stream_third | new | 2910.18 | 2906.97–2943.61 | — |
| resize | old | 2336.06 | 2324.10–2349.83 | — |
| resize | new | 2413.52 | 2410.51–2453.10 | — |
| copy_selected | old | 0.01 | 0.01–0.01 | 0.00 |
| copy_selected | new | 1.44 | 1.42–1.50 | 0.00 |
| copy_all | old | 11.28 | 10.06–11.51 | 8.00 |
| copy_all | new | 46.39 | 45.10–46.49 | 4.56 |

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
