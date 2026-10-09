# Startup comparison

| Variant | Kind | Stage | Samples | Median ms | Min ms | Max ms | p95 ms |
|---|---|---|---:|---:|---:|---:|---:|
| after | new | open_return | 10 | 8.53 | 7.51 | 12.40 | 12.40 |
| after | new | viewport_settled | 10 | 44.29 | 41.83 | 52.39 | 52.39 |
| after | new | clear | 10 | 0.62 | 0.57 | 0.71 | 0.71 |
| after | new | first_command_submit | 10 | 1.33 | 1.17 | 1.45 | 1.45 |
| after | new | first_command | 10 | 142.10 | 128.21 | 2636.16 | 2636.16 |
| after | new | warm_command_01_submit | 10 | 1.27 | 0.92 | 7.30 | 7.30 |
| after | new | warm_command_01 | 10 | 133.53 | 120.86 | 159.02 | 159.02 |
| after | new | warm_command_02_submit | 10 | 1.14 | 0.87 | 2.86 | 2.86 |
| after | new | warm_command_02 | 10 | 132.93 | 128.34 | 144.36 | 144.36 |
| after | new | warm_command_03_submit | 10 | 1.28 | 0.98 | 5.65 | 5.65 |
| after | new | warm_command_03 | 10 | 136.49 | 128.79 | 151.14 | 151.14 |
| after | new | warm_command_04_submit | 10 | 1.28 | 1.04 | 3.63 | 3.63 |
| after | new | warm_command_04 | 10 | 136.86 | 126.46 | 155.97 | 155.97 |
| after | new | warm_command_05_submit | 10 | 1.28 | 0.99 | 5.41 | 5.41 |
| after | new | warm_command_05 | 10 | 130.92 | 126.79 | 147.73 | 147.73 |
| after | new | session_ready | 10 | 43.27 | 40.78 | 51.37 | 51.37 |
| after | new | warm_pooled | 50 | 134.30 | 120.86 | 159.02 | 155.97 |
| after | new | per_session_warm_medians | 10 | 133.82 | 127.28 | 139.89 | 139.89 |
| after | old | open_return | 10 | 0.95 | 0.86 | 2.99 | 2.99 |
| after | old | viewport_settled | 10 | 4.65 | 4.37 | 6.93 | 6.93 |
| after | old | clear | 10 | 1.00 | 0.74 | 2.97 | 2.97 |
| after | old | first_command_submit | 10 | 1.11 | 1.05 | 1.33 | 1.33 |
| after | old | first_command | 10 | 19.20 | 17.77 | 23.29 | 23.29 |
| after | old | warm_command_01_submit | 10 | 0.78 | 0.64 | 0.89 | 0.89 |
| after | old | warm_command_01 | 10 | 16.68 | 16.03 | 18.16 | 18.16 |
| after | old | warm_command_02_submit | 10 | 0.72 | 0.65 | 0.83 | 0.83 |
| after | old | warm_command_02 | 10 | 16.48 | 14.95 | 18.05 | 18.05 |
| after | old | warm_command_03_submit | 10 | 0.67 | 0.64 | 0.82 | 0.82 |
| after | old | warm_command_03 | 10 | 18.02 | 14.97 | 20.71 | 20.71 |
| after | old | warm_command_04_submit | 10 | 0.74 | 0.66 | 0.94 | 0.94 |
| after | old | warm_command_04 | 10 | 17.14 | 14.94 | 20.87 | 20.87 |
| after | old | warm_command_05_submit | 10 | 0.74 | 0.64 | 0.97 | 0.97 |
| after | old | warm_command_05 | 10 | 16.73 | 14.75 | 34.44 | 34.44 |
| after | old | legacy_open_ready | 10 | 1.06 | 0.96 | 3.14 | 3.14 |
| after | old | warm_pooled | 50 | 16.66 | 14.75 | 34.44 | 20.87 |
| after | old | per_session_warm_medians | 10 | 16.73 | 15.05 | 18.41 | 18.41 |
| before | new | open_return | 10 | 8.02 | 7.19 | 10.18 | 10.18 |
| before | new | viewport_settled | 10 | 43.44 | 41.45 | 141.67 | 141.67 |
| before | new | clear | 10 | 0.73 | 0.62 | 0.82 | 0.82 |
| before | new | first_command_submit | 10 | 1.30 | 1.17 | 1.62 | 1.62 |
| before | new | first_command | 10 | 141.98 | 134.04 | 2832.49 | 2832.49 |
| before | new | warm_command_01_submit | 10 | 1.25 | 0.82 | 2.29 | 2.29 |
| before | new | warm_command_01 | 10 | 144.17 | 128.75 | 173.94 | 173.94 |
| before | new | warm_command_02_submit | 10 | 1.71 | 0.87 | 3.82 | 3.82 |
| before | new | warm_command_02 | 10 | 144.32 | 130.12 | 198.99 | 198.99 |
| before | new | warm_command_03_submit | 10 | 1.34 | 0.91 | 2.25 | 2.25 |
| before | new | warm_command_03 | 10 | 140.72 | 125.91 | 157.19 | 157.19 |
| before | new | warm_command_04_submit | 10 | 1.73 | 0.90 | 4.41 | 4.41 |
| before | new | warm_command_04 | 10 | 141.21 | 126.05 | 149.71 | 149.71 |
| before | new | warm_command_05_submit | 10 | 1.12 | 0.93 | 2.83 | 2.83 |
| before | new | warm_command_05 | 10 | 139.28 | 125.95 | 145.96 | 145.96 |
| before | new | session_ready | 10 | 42.36 | 40.41 | 140.45 | 140.45 |
| before | new | warm_pooled | 50 | 141.87 | 125.91 | 198.99 | 170.40 |
| before | new | per_session_warm_medians | 10 | 140.98 | 137.16 | 148.07 | 148.07 |
| before | old | open_return | 10 | 1.07 | 0.84 | 4.69 | 4.69 |
| before | old | viewport_settled | 10 | 4.73 | 4.48 | 8.43 | 8.43 |
| before | old | clear | 10 | 1.29 | 0.80 | 2.85 | 2.85 |
| before | old | first_command_submit | 10 | 1.15 | 1.00 | 1.25 | 1.25 |
| before | old | first_command | 10 | 18.10 | 17.80 | 24.95 | 24.95 |
| before | old | warm_command_01_submit | 10 | 0.73 | 0.64 | 0.93 | 0.93 |
| before | old | warm_command_01 | 10 | 16.58 | 16.46 | 20.20 | 20.20 |
| before | old | warm_command_02_submit | 10 | 0.72 | 0.64 | 1.11 | 1.11 |
| before | old | warm_command_02 | 10 | 16.66 | 14.97 | 34.61 | 34.61 |
| before | old | warm_command_03_submit | 10 | 0.73 | 0.67 | 1.88 | 1.88 |
| before | old | warm_command_03 | 10 | 17.05 | 14.94 | 29.49 | 29.49 |
| before | old | warm_command_04_submit | 10 | 0.69 | 0.63 | 0.83 | 0.83 |
| before | old | warm_command_04 | 10 | 17.88 | 14.88 | 26.78 | 26.78 |
| before | old | warm_command_05_submit | 10 | 0.70 | 0.62 | 0.83 | 0.83 |
| before | old | warm_command_05 | 10 | 17.16 | 16.11 | 19.09 | 19.09 |
| before | old | legacy_open_ready | 10 | 1.19 | 0.95 | 4.81 | 4.81 |
| before | old | warm_pooled | 50 | 16.71 | 14.88 | 34.61 | 26.78 |
| before | old | per_session_warm_medians | 10 | 17.06 | 16.15 | 19.01 | 19.01 |

All first-command values and per-process warm values remain in results.json.

- No build, profiler, or process sampler may run concurrently.
- Fresh process first commands are not proven cold OS-cache launches.
- Fixture SDL/window/font setup precedes pane timing; whole-process wall time is separate.
- Session readiness is first app poll observation, not backend event generation time.
- Command timing includes output, layout and final draw settlement; no physical keyboard/scanout timing.
- Warm pooled values are correlated within each Session; per-Session warm medians are also retained.
- Nearest-rank p95 from fewer than ten fresh processes is weak evidence.
- This collector provides no RSS/CPU results; use the separate matched resource runner.
