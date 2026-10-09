# Terminal performance assessment

| Variant | Phase | Old app CPU s | New app CPU s | Paired new/old CPU | New wall ms | New combined RSS MiB |
|---|---|---:|---:|---:|---:|---:|
| before | stream_first | 0.62 | 1.20 | 1.94 | 2921.23 | 54.99 |
| before | stream_second | 0.61 | 1.24 | 2.07 | 2940.47 | 65.19 |
| before | stream_third | 0.62 | 1.36 | 2.17 | 2958.47 | 69.20 |
| before | resize | 2.19 | 2.33 | 1.06 | 2411.53 | 57.37 |
| before | idle | 0.01 | 0.01 | 1.00 | — | 18.95 |
| before | plateau_first | 0.01 | 0.01 | 1.00 | — | 46.89 |
| before | plateau_second | 0.01 | 0.01 | 1.00 | — | 52.86 |
| before | plateau_third | 0.01 | 0.01 | 1.00 | — | 54.45 |
| before | copied_idle | 0.01 | 0.01 | 1.00 | — | 82.45 |
| counting | stream_first | 0.60 | 0.97 | 1.63 | 2627.81 | 52.34 |
| counting | stream_second | 0.60 | 1.01 | 1.67 | 2630.15 | 64.62 |
| counting | stream_third | 0.61 | 1.07 | 1.74 | 2625.10 | 67.75 |
| counting | resize | 2.24 | 1.51 | 0.67 | 1698.79 | 59.32 |
| counting | idle | 0.01 | 0.01 | 1.00 | — | 19.04 |
| counting | plateau_first | 0.01 | 0.01 | 1.00 | — | 45.98 |
| counting | plateau_second | 0.01 | 0.01 | 1.00 | — | 50.73 |
| counting | plateau_third | 0.01 | 0.01 | 1.00 | — | 53.19 |
| counting | copied_idle | 0.01 | 0.01 | 1.00 | — | 85.96 |
| optimized | stream_first | 0.59 | 0.88 | 1.51 | 2634.68 | 48.99 |
| optimized | stream_second | 0.60 | 0.94 | 1.57 | 2588.89 | 60.61 |
| optimized | stream_third | 0.60 | 0.99 | 1.63 | 2603.66 | 65.42 |
| optimized | resize | 2.21 | 1.64 | 0.74 | 1709.60 | 60.95 |
| optimized | idle | 0.01 | 0.01 | 1.00 | — | 19.04 |
| optimized | plateau_first | 0.01 | 0.01 | 1.00 | — | 46.42 |
| optimized | plateau_second | 0.01 | 0.01 | 1.00 | — | 52.25 |
| optimized | plateau_third | 0.01 | 0.01 | 1.00 | — | 54.52 |
| optimized | copied_idle | 0.01 | 0.01 | 1.00 | — | 85.79 |

| Target | Assessment |
|---|---|
| warm_median_100ms | missed |
| first_p95_250ms | missed |
| stream_app_cpu_1_3x_old | missed |
| settled_memory_vs_before | missed point estimate |
| idle_redraws_vs_before | met |
| idle_cpu_vs_before | same observed counter quantum |

| Startup | First median ms | First min–max ms | First p95 ms | Warm median ms | Warm min–max ms |
|---|---:|---:|---:|---:|---:|
| after/new | 142.10 | 128.21–2636.16 | 2636.16 | 134.30 | 120.86–159.02 |
| after/old | 19.20 | 17.77–23.29 | 23.29 | 16.66 | 14.75–34.44 |
| before/new | 141.98 | 134.04–2832.49 | 2832.49 | 141.87 | 125.91–198.99 |
| before/old | 18.10 | 17.80–24.95 | 24.95 | 16.71 | 14.88–34.61 |

before: source/binary consistency True; tracing: trace controls absent from frozen source.
old median combined settled RSS growth: first→second 5.76 MiB, second→third 3.32 MiB.
new median combined settled RSS growth: first→second 5.70 MiB, second→third 1.59 MiB.
Observer boundary exclusions (kind, repetition, excluded, total): [('old', 1, 9, 163), ('new', 1, 3, 183), ('new', 2, 5, 165), ('old', 2, 4, 161), ('old', 3, 4, 160), ('new', 3, 4, 167)].

counting: source/binary consistency True; tracing: trace controls absent from frozen source.
old median combined settled RSS growth: first→second 6.40 MiB, second→third 3.19 MiB.
new median combined settled RSS growth: first→second 4.75 MiB, second→third 3.14 MiB.
Observer boundary exclusions (kind, repetition, excluded, total): [('old', 1, 5, 159), ('new', 1, 1, 175), ('new', 2, 6, 159), ('old', 2, 4, 160), ('old', 3, 3, 159), ('new', 3, 1, 160)].

optimized: source/binary consistency True; tracing: explicit matching run context.
old median combined settled RSS growth: first→second 6.17 MiB, second→third 4.67 MiB.
new median combined settled RSS growth: first→second 5.38 MiB, second→third 2.57 MiB.
Observer boundary exclusions (kind, repetition, excluded, total): [('old', 1, 4, 160), ('new', 1, 8, 163), ('new', 2, 3, 161), ('old', 2, 5, 160), ('old', 3, 7, 161), ('new', 3, 3, 160)].

- CPU is cumulative app-counter change between valid process snapshots, excluding phase boundaries, not an exact whole-operation counter.
- Simultaneous RSS adds app and descendant address spaces within each single snapshot; shared pages may be counted twice.
- Per-run simultaneous RSS medians are aggregated across runs; independent app/child medians are never added.
- Wall timing changed in the unchanged old control across batches; use matched per-repetition old/new ratios as well as raw before/after values.
- Copy All retains different byte counts in old/new and is not equal-byte throughput; copied bytes remain recorded.
- Fresh-process startup timings follow SDL/font fixture setup and initial viewport/Clear; hashing and prior runs warm file caches.
- Cold-loader or first-artifact penalties remain visible as individual first-command values; fresh copied artifacts are not cold OS-cache proof.
- Aspirational targets are evidence assessments, not test assertions or universal hardware/platform guarantees.
