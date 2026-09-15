# Crash-recovery typing latency

The initial recovery backup blocks the edit handler and its delay grows with file size. Later edits still pay synchronous journal costs, but do not copy the original again.

## First edit

Medians across fresh editing sessions; parentheses show the recovery-on minimum–maximum.

| File | Recovery off | Recovery on | Later insert p99, recovery on |
| --- | ---: | ---: | ---: |
| Empty | 0.09 ms | 27.35 ms (25.92–27.51) | 15.44 ms |
| 1 MiB | 0.09 ms | 34.65 ms (29.22–35.24) | 16.34 ms |
| 100 MiB | 0.10 ms | 78.94 ms (69.38–82.61) | 15.72 ms |
| 1024 MiB | 0.11 ms | 475.95 ms (456.84–564.26) | 15.85 ms |

## Subsequent edits

Pooled across file sizes and repetitions; p99 is the nearest-rank 99th percentile.

| Operation | Off median | On median | On p99 | On worst | On >50 ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| Burst insert | 0.015 ms | 9.48 ms | 16.00 ms | 43.99 ms | 0/2400 |
| Backspace | 0.012 ms | 4.91 ms | 12.48 ms | 15.88 ms | 0/2400 |
| Replace selection | 0.014 ms | 14.62 ms | 25.44 ms | 36.57 ms | 0/600 |
| Paced typing | 0.177 ms | 10.64 ms | 17.36 ms | 18.75 ms | 0/360 |

The first edit after Save took a median **9.42 ms**, worst **11.15 ms** with recovery. It did not repeat the initial large-file backup. Save itself was timed separately in the raw data.

For the largest file, median peak RSS was **7.04 MiB off** and **7.46 MiB on**: **0.42 MiB extra** in the isolated test process. This is not graphical-app RSS.

## Interpretation and method

The first-edit delays are synchronous periods during which the main event loop cannot proceed to rendering. Reducing the initial snapshot's blocking cost is the clearest first improvement. Later journal synchronization is a separate roughly per-edit cost; selection replacement currently records deletion and insertion separately. Any optimization must preserve the recovery guarantees and bounded memory.

Run: 2026-09-15T13:57:59.104516+00:00; macOS-15.7.3-arm64-arm-64bit-Mach-O; Rust host `x86_64-apple-darwin`; release build; 3 repetitions per size/mode, alternating on/off order. All probe assertions passed, including text lengths, saved output length, undo/redo and absence of recovery warnings. No production recovery behavior was changed.

- Synchronous Editor edit handlers, including history and recovery; no OS key events or rendering.
- Physically written UTF-8 fixtures on the local temporary filesystem, warm OS cache.
- Cursor near the start; first 40 lines accessed before timing; no full line indexing.
- Fresh editor and recovery session per case; recovery order alternates per repetition.
- Paced edits wait 50 ms after the previous dispatch; delay excluded from measured time.
- First-edit statistics contain three samples by default; tail percentiles pool subsequent edits.
- No claim about cold files, slow/network disks, Windows/Linux or native display latency.
- 16.7/50/100 ms are inspection thresholds; timings are not universal perception guarantees.

[Raw samples, distributions, source fingerprint and log filenames](results.json).

Reproduce from the repository root: `python3 tools/qa/recovery_latency.py --output docs/testing/NEW_RECOVERY_RUN`. On macOS the timing tool may require permission to read process resource statistics.
