# Crash-recovery typing latency

First-edit timings include creation of the immutable recovery backup and initial checkpoint. Later edits pay synchronous journal costs without copying the original again.

## First edit

Medians across fresh editing sessions; parentheses show the recovery-on minimum–maximum.

| File | Recovery off | Recovery on | Later insert p99, recovery on |
| --- | ---: | ---: | ---: |
| Empty | 0.09 ms | 28.75 ms (25.51–33.02) | 12.87 ms |
| 1 MiB | 0.09 ms | 31.91 ms (26.75–35.27) | 14.10 ms |
| 100 MiB | 0.09 ms | 27.58 ms (26.90–29.66) | 13.67 ms |
| 1024 MiB | 0.12 ms | 46.49 ms (36.48–47.08) | 18.52 ms |

## Subsequent edits

Pooled across file sizes and repetitions; p99 is the nearest-rank 99th percentile.

| Operation | Off median | On median | On p99 | On worst | On >50 ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| Burst insert | 0.014 ms | 9.33 ms | 15.63 ms | 31.13 ms | 0/2400 |
| Backspace | 0.012 ms | 4.51 ms | 11.54 ms | 15.01 ms | 0/2400 |
| Replace selection | 0.013 ms | 13.89 ms | 21.68 ms | 25.66 ms | 0/600 |
| Paced typing | 0.113 ms | 10.35 ms | 20.23 ms | 23.17 ms | 0/360 |

The first edit after Save took a median **8.86 ms**, worst **11.02 ms** with recovery. It did not repeat the initial large-file backup. Save itself was timed separately in the raw data.

For the largest file, median peak RSS was **7.20 MiB off** and **7.53 MiB on**: **0.33 MiB extra** in the isolated test process. This is not graphical-app RSS.

## Interpretation and method

The first-edit delays are synchronous periods during which the main event loop cannot proceed to rendering. Filesystem copy-on-write support can avoid streaming the original; unsupported filesystems still require the bounded copy. Later journal synchronization is a separate roughly per-edit cost; selection replacement currently records deletion and insertion separately.

Run: 2026-09-15T14:12:52.103236+00:00; macOS-15.7.3-arm64-arm-64bit-Mach-O; Rust host `x86_64-apple-darwin`; release build; 3 repetitions per size/mode, alternating on/off order. All probe assertions passed, including text lengths, saved output length, undo/redo and absence of recovery warnings.

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
