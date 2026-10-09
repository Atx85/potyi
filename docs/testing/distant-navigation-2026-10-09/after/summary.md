# Sustained editing baseline

Recorded: 2026-10-09T14:37:41.440652+00:00.

Profile: **release**. OS: macOS-15.8.1-arm64-arm-64bit-Mach-O.
Source stable during build: **True**.

Times are milliseconds. First and later edits are separate; setup includes navigation and initial drawing.
Only complete measured runs contribute to medians. Failed and timed-out runs retain partial events in JSON.

| Case | Completed measured runs | Unfinished runs | Last unfinished phase | Sampled RSS maximum, MiB |
| --- | ---: | --- | --- | ---: |
| 1048576b-line64-pos0 | 3 | none | — | unavailable |
| 1048576b-line64-pos50 | 3 | none | — | unavailable |
| 1048576b-line64-pos99 | 3 | none | — | unavailable |
| 1048576b-line1048576-pos0 | 3 | none | — | unavailable |
| 1048576b-line1048576-pos50 | 3 | none | — | unavailable |
| 1048576b-line1048576-pos99 | 3 | none | — | unavailable |
| 104857600b-line64-pos0 | 3 | none | — | unavailable |
| 104857600b-line64-pos50 | 3 | none | — | unavailable |
| 104857600b-line64-pos99 | 3 | none | — | unavailable |
| 104857600b-line1048576-pos0 | 3 | none | — | unavailable |
| 104857600b-line1048576-pos50 | 3 | none | — | unavailable |
| 104857600b-line1048576-pos99 | 3 | none | — | unavailable |
| 1000000000b-line64-pos0 | 3 | none | — | unavailable |
| 1000000000b-line64-pos50 | 3 | none | — | unavailable |
| 1000000000b-line64-pos99 | 3 | none | — | unavailable |
| 1000000000b-line1048576-pos0 | 3 | none | — | unavailable |
| 1000000000b-line1048576-pos50 | 3 | none | — | unavailable |
| 1000000000b-line1048576-pos99 | 3 | none | — | unavailable |
| 1mib-short-middle-variant1 | 3 | none | — | unavailable |
| 1mib-short-middle-variant2 | 3 | none | — | unavailable |
| 1mib-short-middle-variant3 | 3 | none | — | unavailable |
| 1mib-short-middle-variant4 | 3 | none | — | unavailable |
| 1mib-short-middle-variant5 | 3 | none | — | unavailable |

## 1048576b-line64-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.008 | 0.009 | 0.009 | 259 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.005 | 0.007 | 0.007 | 259 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.068 | 4.110 | 4.110 | 1701 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 0.079 | 0.089 | 0.089 | 322 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.010 | 0.018 | 0.018 | 322 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 0.061 | 0.080 | 0.080 | 65536 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 6.208 | 11.050 | 11.050 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.001 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.001 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.240 | 2.505 | 2.505 | 3303 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 2.202 | 2.615 | 2.615 | 3303 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.060 | 0.060 | 0.060 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.004 | 0.010 | 0.010 | 1 | 0 | 0 / 0 / 0 |

## 1048576b-line64-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.011 | 0.012 | 0.012 | 266 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.009 | 0.012 | 0.012 | 266 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.123 | 4.363 | 4.363 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.077 | 0.080 | 0.080 | 325 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.014 | 0.022 | 0.022 | 325 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 0.559 | 0.559 | 0.559 | 589826 | 8193 | 0 / 0 / 0 |
| open | setup | 3 | 13.934 | 14.280 | 14.280 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.007 | 0.007 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.368 | 2.467 | 2.467 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.320 | 2.580 | 2.580 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.057 | 0.064 | 0.064 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.005 | 0.007 | 0.007 | 2 | 0 | 0 / 0 / 0 |

## 1048576b-line64-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.015 | 0.015 | 0.015 | 329 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.012 | 0.013 | 0.013 | 329 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 3.937 | 4.310 | 4.310 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.084 | 0.086 | 0.086 | 325 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.018 | 0.023 | 0.023 | 325 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 0.946 | 0.949 | 0.949 | 1048641 | 16221 | 0 / 0 / 0 |
| open | setup | 3 | 8.767 | 12.214 | 12.214 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.530 | 2.632 | 2.632 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.344 | 2.892 | 2.892 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.063 | 0.068 | 0.068 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.005 | 0.006 | 0.006 | 2 | 0 | 0 / 0 / 0 |

## 1048576b-line1048576-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.177 | 0.180 | 0.180 | 1048579 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.154 | 0.167 | 0.167 | 1048579 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 1.853 | 1.857 | 1.857 | 8192 | 1 | 0 / 0 / 0 |
| insert | first | 3 | 0.241 | 0.289 | 0.289 | 1052675 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.157 | 0.199 | 0.199 | 1052675 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 0.275 | 0.275 | 0.275 | 1048576 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 7.322 | 10.378 | 10.378 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.001 | 0.001 | 0.001 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 0.286 | 0.347 | 0.347 | 12288 | 1 | 0 / 0 / 0 |
| redraw | later | 9 | 0.195 | 0.290 | 0.290 | 12288 | 1 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.052 | 0.055 | 0.055 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.002 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |

## 1048576b-line1048576-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 2.195 | 2.281 | 2.281 | 1572874 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 2.244 | 2.285 | 2.285 | 1572874 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 2.037 | 2.087 | 2.087 | 40960 | 1 | 0 / 0 / 0 |
| insert | first | 3 | 2.333 | 2.378 | 2.378 | 1576966 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 2.221 | 2.316 | 2.316 | 1576966 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 2.361 | 2.444 | 2.444 | 1572866 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 4.235 | 15.288 | 15.288 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.005 | 0.011 | 0.011 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.329 | 2.367 | 2.367 | 569344 | 1 | 0 / 0 / 0 |
| redraw | later | 9 | 2.344 | 2.410 | 2.410 | 569344 | 1 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.076 | 0.199 | 0.199 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.005 | 0.006 | 0.006 | 2 | 0 | 0 / 0 / 0 |

## 1048576b-line1048576-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 4.227 | 4.334 | 4.334 | 2088970 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 4.264 | 4.322 | 4.322 | 2088970 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 2.054 | 2.249 | 2.249 | 28672 | 1 | 0 / 0 / 0 |
| insert | first | 3 | 4.302 | 4.353 | 4.353 | 2088966 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 4.239 | 4.381 | 4.381 | 2088966 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 4.465 | 5.237 | 5.237 | 2088962 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 4.678 | 8.511 | 8.511 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 4.091 | 4.138 | 4.138 | 1069056 | 1 | 0 / 0 / 0 |
| redraw | later | 9 | 4.114 | 4.248 | 4.248 | 1069056 | 1 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.060 | 0.068 | 0.068 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.006 | 0.007 | 0.007 | 2 | 0 | 0 / 0 / 0 |

## 104857600b-line64-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.007 | 0.008 | 0.008 | 259 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.006 | 0.007 | 0.007 | 259 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.223 | 4.275 | 4.275 | 1701 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 0.073 | 0.073 | 0.073 | 322 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.011 | 0.018 | 0.018 | 322 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 0.084 | 0.094 | 0.094 | 65536 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 395.456 | 470.155 | 470.155 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.001 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.343 | 2.362 | 2.362 | 3303 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 2.315 | 2.580 | 2.580 | 3303 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.058 | 0.060 | 0.060 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.005 | 0.007 | 0.007 | 1 | 0 | 0 / 0 / 0 |

## 104857600b-line64-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.014 | 0.016 | 0.016 | 266 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.012 | 0.017 | 0.017 | 266 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.244 | 4.392 | 4.392 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.087 | 0.087 | 0.087 | 325 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.018 | 0.024 | 0.024 | 325 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 42.530 | 45.523 | 45.523 | 52494338 | 819201 | 0 / 0 / 0 |
| open | setup | 3 | 184.791 | 202.096 | 202.096 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.004 | 0.008 | 0.008 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.446 | 2.529 | 2.529 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.305 | 2.482 | 2.482 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.062 | 0.080 | 0.080 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.007 | 0.029 | 0.029 | 2 | 0 | 0 / 0 / 0 |

## 104857600b-line64-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.015 | 0.023 | 0.023 | 266 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.013 | 0.024 | 0.024 | 266 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.177 | 4.536 | 4.536 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.084 | 0.114 | 0.114 | 325 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.019 | 0.031 | 0.031 | 325 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 85.923 | 86.200 | 86.200 | 103874562 | 1622017 | 0 / 0 / 0 |
| open | setup | 3 | 185.756 | 218.644 | 218.644 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.005 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.004 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.605 | 2.773 | 2.773 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.373 | 2.529 | 2.529 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.065 | 0.066 | 0.066 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.006 | 0.009 | 0.009 | 2 | 0 | 0 / 0 / 0 |

## 104857600b-line1048576-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.200 | 0.211 | 0.211 | 1113859 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.206 | 0.217 | 0.217 | 1113859 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 9.295 | 9.910 | 9.910 | 26324992 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 0.306 | 0.320 | 0.320 | 1117954 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.166 | 0.210 | 0.210 | 1117954 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 0.251 | 0.268 | 0.268 | 1048576 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 5.437 | 7.369 | 7.369 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.003 | 0.003 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 7.389 | 7.475 | 7.475 | 26329088 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 7.240 | 8.139 | 8.139 | 26329088 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.075 | 0.082 | 0.082 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.008 | 0.009 | 0.009 | 1 | 0 | 0 / 0 / 0 |

## 104857600b-line1048576-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.191 | 0.195 | 0.195 | 1113866 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.179 | 0.205 | 0.205 | 1113866 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.396 | 4.668 | 4.668 | 110592 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.247 | 0.254 | 0.254 | 1117957 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.166 | 0.216 | 0.216 | 1117957 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 9.677 | 9.712 | 9.712 | 53477378 | 51 | 0 / 0 / 0 |
| open | setup | 3 | 4.300 | 4.301 | 4.301 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.585 | 2.654 | 2.654 | 114688 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.526 | 2.740 | 2.740 | 114688 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.067 | 0.072 | 0.072 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.005 | 0.006 | 0.006 | 2 | 0 | 0 / 0 / 0 |

## 104857600b-line1048576-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.181 | 0.192 | 0.192 | 1048586 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.174 | 0.200 | 0.200 | 1048586 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.603 | 4.626 | 4.626 | 110592 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.268 | 0.302 | 0.302 | 1052678 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.166 | 0.172 | 0.172 | 1052678 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 18.610 | 18.765 | 18.765 | 104857602 | 100 | 0 / 0 / 0 |
| open | setup | 3 | 4.415 | 4.954 | 4.954 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.687 | 2.755 | 2.755 | 114688 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.465 | 2.592 | 2.592 | 114688 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.054 | 0.065 | 0.065 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.004 | 0.006 | 0.006 | 2 | 0 | 0 / 0 / 0 |

## 1000000000b-line64-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.010 | 0.011 | 0.011 | 259 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.008 | 0.009 | 0.009 | 259 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.045 | 4.096 | 4.096 | 1701 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 0.069 | 0.090 | 0.090 | 322 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.015 | 0.022 | 0.022 | 322 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 0.078 | 0.082 | 0.082 | 65536 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 1917.530 | 2088.057 | 2088.057 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.001 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.382 | 2.621 | 2.621 | 3303 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 2.311 | 2.730 | 2.730 | 3303 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.062 | 0.064 | 0.064 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.006 | 0.007 | 0.007 | 1 | 0 | 0 / 0 / 0 |

## 1000000000b-line64-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.016 | 0.016 | 0.016 | 266 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.014 | 0.017 | 0.017 | 266 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.195 | 4.377 | 4.377 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.086 | 0.094 | 0.094 | 325 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.019 | 0.025 | 0.025 | 325 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 407.599 | 412.684 | 412.684 | 500039682 | 7812501 | 0 / 0 / 0 |
| open | setup | 3 | 2031.284 | 5756.348 | 5756.348 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.005 | 0.006 | 0.006 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.004 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.393 | 2.633 | 2.633 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.378 | 2.596 | 2.596 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.072 | 0.078 | 0.078 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.007 | 0.009 | 0.009 | 2 | 0 | 0 / 0 / 0 |

## 1000000000b-line64-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.015 | 0.016 | 0.016 | 266 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.011 | 0.017 | 0.017 | 266 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.123 | 4.186 | 4.186 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.078 | 0.081 | 0.081 | 325 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.019 | 0.025 | 0.025 | 325 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 812.876 | 819.520 | 819.520 | 990052354 | 15468751 | 0 / 0 / 0 |
| open | setup | 3 | 1756.533 | 1894.499 | 1894.499 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.005 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.004 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.354 | 2.416 | 2.416 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.340 | 2.575 | 2.575 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.058 | 0.061 | 0.061 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.007 | 0.009 | 0.009 | 2 | 0 | 0 / 0 / 0 |

## 1000000000b-line1048576-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.208 | 0.210 | 0.210 | 1113859 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.211 | 0.218 | 0.218 | 1113859 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 9.451 | 9.714 | 9.714 | 26324992 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 0.310 | 0.328 | 0.328 | 1117954 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.169 | 0.570 | 0.570 | 1117954 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 0.263 | 0.300 | 0.300 | 1048576 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 4.030 | 5.879 | 5.879 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.003 | 0.003 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 7.523 | 7.783 | 7.783 | 26329088 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 7.449 | 8.508 | 8.508 | 26329088 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.075 | 0.083 | 0.083 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.008 | 0.010 | 0.010 | 1 | 0 | 0 / 0 / 0 |

## 1000000000b-line1048576-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 3.518 | 3.564 | 3.564 | 1994506 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 3.651 | 3.786 | 3.786 | 1994506 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 91.656 | 91.731 | 91.731 | 22048768 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 3.604 | 3.638 | 3.638 | 1994501 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 3.683 | 4.648 | 4.648 | 1994501 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 90.499 | 92.320 | 92.320 | 501051394 | 477 | 0 / 0 / 0 |
| open | setup | 3 | 18.051 | 19.646 | 19.646 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 5.750 | 5.928 | 5.928 | 1015808 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 5.773 | 6.375 | 6.375 | 1015808 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.003 | 0.003 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.071 | 0.077 | 0.077 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.005 | 0.007 | 0.007 | 2 | 0 | 0 / 0 / 0 |

## 1000000000b-line1048576-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.760 | 0.794 | 0.794 | 1261322 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.727 | 0.789 | 0.789 | 1261322 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 18.690 | 19.009 | 19.009 | 3698688 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.882 | 0.908 | 0.908 | 1261317 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.719 | 0.810 | 0.810 | 1261317 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 174.718 | 179.522 | 179.522 | 991051778 | 945 | 0 / 0 / 0 |
| open | setup | 3 | 9.091 | 30.981 | 30.981 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 3.194 | 3.234 | 3.234 | 262144 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 3.092 | 3.206 | 3.206 | 262144 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.055 | 0.061 | 0.061 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.005 | 0.006 | 0.006 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant1

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.011 | 0.144 | 0.144 | 266 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.008 | 0.010 | 0.010 | 266 | 1 | 0 / 0 / 0 |
| duplicate_view | setup | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.816 | 4.887 | 4.887 | 68875 | 26 | 0 / 0 / 0 |
| insert | first | 3 | 0.075 | 0.091 | 0.091 | 325 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.013 | 0.018 | 0.018 | 325 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 0.506 | 0.544 | 0.544 | 589826 | 8193 | 0 / 0 / 0 |
| open | setup | 3 | 5.782 | 5.802 | 5.802 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 3.138 | 3.171 | 3.171 | 19277 | 91 | 0 / 0 / 0 |
| redraw | later | 9 | 3.158 | 3.381 | 3.381 | 19277 | 91 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.003 | 0.003 | 0.003 | 2 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.002 | 0.003 | 0.003 | 2 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.002 | 0.003 | 0.003 | 2 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.002 | 0.003 | 0.003 | 2 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.004 | 0.005 | 0.005 | 258 | 1 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.004 | 0.004 | 0.004 | 258 | 1 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.006 | 0.008 | 0.008 | 258 | 1 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.003 | 0.004 | 0.004 | 258 | 1 | 0 / 0 / 0 |
| undo | first | 3 | 0.048 | 0.065 | 0.065 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.005 | 0.008 | 0.008 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant2

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.009 | 0.017 | 0.017 | 266 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.009 | 0.010 | 0.010 | 266 | 1 | 0 / 0 / 0 |
| duplicate_view | setup | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.767 | 5.054 | 5.054 | 68875 | 26 | 0 / 0 / 0 |
| insert | first | 3 | 0.024 | 0.027 | 0.027 | 325 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.014 | 0.018 | 0.018 | 325 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 0.460 | 0.498 | 0.498 | 589314 | 8192 | 0 / 0 / 0 |
| open | setup | 3 | 10.188 | 10.686 | 10.686 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 3.130 | 3.186 | 3.186 | 19277 | 91 | 0 / 0 / 0 |
| redraw | later | 9 | 3.103 | 3.196 | 3.196 | 19277 | 91 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.003 | 0.004 | 0.004 | 2 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.002 | 0.003 | 0.003 | 2 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.002 | 0.003 | 0.003 | 2 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.002 | 0.002 | 0.002 | 2 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.004 | 0.004 | 0.004 | 258 | 1 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.004 | 0.004 | 0.004 | 258 | 1 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.006 | 0.006 | 0.006 | 258 | 1 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.004 | 0.009 | 0.009 | 258 | 1 | 0 / 0 / 0 |
| undo | first | 3 | 0.056 | 0.057 | 0.057 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.006 | 0.011 | 0.011 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant3

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.022 | 0.023 | 0.023 | 266 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.019 | 0.022 | 0.022 | 266 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 3.947 | 4.025 | 4.025 | 1703 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.030 | 0.031 | 0.031 | 325 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.018 | 0.024 | 0.024 | 325 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 1.205 | 1.207 | 1.207 | 589570 | 8193 | 0 / 0 / 0 |
| open | setup | 3 | 7.752 | 16.169 | 16.169 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.005 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.004 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.342 | 2.411 | 2.411 | 1769 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.243 | 2.557 | 2.557 | 1769 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.055 | 0.059 | 0.059 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.009 | 0.013 | 0.013 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant4

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 8.699 | 9.517 | 9.517 | 266 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 9.348 | 10.623 | 10.623 | 266 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.166 | 4.318 | 4.318 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 50.940 | 62.209 | 62.209 | 325 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 8.767 | 11.071 | 11.071 | 325 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 0.535 | 0.546 | 0.546 | 589826 | 8193 | 0 / 0 / 0 |
| open | setup | 3 | 21.559 | 31.295 | 31.295 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 10.137 | 10.564 | 10.564 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 8.387 | 10.112 | 10.112 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.498 | 2.746 | 2.746 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.396 | 2.579 | 2.579 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 9.596 | 9.925 | 9.925 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 8.803 | 9.749 | 9.749 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant5

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 8.570 | 9.009 | 9.009 | 266 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 8.425 | 13.319 | 13.319 | 266 | 1 | 0 / 0 / 0 |
| duplicate_view | setup | 3 | 0.001 | 0.001 | 0.001 | 0 | 0 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 5.128 | 5.298 | 5.298 | 68878 | 26 | 0 / 0 / 0 |
| insert | first | 3 | 39.743 | 57.297 | 57.297 | 325 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 8.806 | 9.331 | 9.331 | 325 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 1.197 | 1.220 | 1.220 | 589570 | 8192 | 0 / 0 / 0 |
| open | setup | 3 | 5.920 | 23.707 | 23.707 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 8.960 | 9.400 | 9.400 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 8.498 | 13.495 | 13.495 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 3.319 | 3.374 | 3.374 | 19280 | 91 | 0 / 0 / 0 |
| redraw | later | 9 | 3.539 | 4.071 | 4.071 | 19280 | 91 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.006 | 0.006 | 0.006 | 2 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.005 | 0.007 | 0.007 | 2 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.005 | 0.005 | 0.005 | 2 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.005 | 0.006 | 0.006 | 2 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.017 | 0.017 | 0.017 | 258 | 1 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.014 | 0.019 | 0.019 | 258 | 1 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.019 | 0.021 | 0.021 | 258 | 1 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.015 | 0.018 | 0.018 | 258 | 1 | 0 / 0 / 0 |
| undo | first | 3 | 9.462 | 9.513 | 9.513 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 8.773 | 11.626 | 11.626 | 2 | 0 | 0 / 0 / 0 |

## Limits

- Local measurements with freshly written, warm filesystem fixtures; no cold-disk or other-editor comparison.
- Headless rendering measures SDL draw work, not native desktop presentation, input dispatch or LSP work.
- RSS is sampled about every 100ms across fixture/setup/editing, not true peak memory or an allocation budget; ps may be unavailable.
- Read counters count successful positional file reads in the piece table, including rereads; not physical disk I/O or recovery writes.
- Line discoveries count discovery calls, including reconstructed evicted entries and EOF handling; not unique lines.
- Copy counters count top-level records at view/history cloning sites, not bytes or deep allocations in undo snapshots.
- p95/p99 use nearest ranks; small sample counts and correlated operations limit tail claims.
- The first edit and later edits are reported separately. Timeout includes fixture and setup; partial timings remain in raw events.
- Unfinished warmups stop that case's remaining runs; no timing threshold determines correctness.
- Concurrent checkout changes after building cannot alter the saved probe. Different before/after build hashes make source provenance inconclusive.
