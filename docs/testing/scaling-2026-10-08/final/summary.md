# Sustained editing baseline

Recorded: 2026-10-08T15:41:08.364254+00:00.

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
| backspace | first | 3 | 0.005 | 0.014 | 0.014 | 259 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.005 | 0.010 | 0.010 | 259 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.570 | 4.993 | 4.993 | 1701 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 0.088 | 0.091 | 0.091 | 322 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.012 | 0.021 | 0.021 | 322 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 0.068 | 0.089 | 0.089 | 65536 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 14.440 | 16.794 | 16.794 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.001 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.377 | 2.779 | 2.779 | 3305 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 2.360 | 2.653 | 2.653 | 3305 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.052 | 0.073 | 0.073 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.005 | 0.007 | 0.007 | 1 | 0 | 0 / 0 / 0 |

## 1048576b-line64-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.009 | 0.011 | 0.011 | 266 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.010 | 0.017 | 0.017 | 266 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.368 | 4.642 | 4.642 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.083 | 0.094 | 0.094 | 325 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.015 | 0.023 | 0.023 | 325 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 2.340 | 2.379 | 2.379 | 589826 | 8193 | 0 / 0 / 0 |
| open | setup | 3 | 13.367 | 17.243 | 17.243 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.017 | 0.017 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.257 | 2.380 | 2.380 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.278 | 2.453 | 2.453 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.043 | 0.064 | 0.064 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.005 | 0.007 | 0.007 | 2 | 0 | 0 / 0 / 0 |

## 1048576b-line64-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.011 | 0.016 | 0.016 | 329 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.010 | 0.015 | 0.015 | 329 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.208 | 4.361 | 4.361 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.092 | 0.092 | 0.092 | 325 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.013 | 0.018 | 0.018 | 325 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 4.439 | 4.459 | 4.459 | 1048641 | 16221 | 0 / 0 / 0 |
| open | setup | 3 | 14.326 | 23.390 | 23.390 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.556 | 2.641 | 2.641 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.198 | 2.385 | 2.385 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.063 | 0.067 | 0.067 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.003 | 0.006 | 0.006 | 2 | 0 | 0 / 0 / 0 |

## 1048576b-line1048576-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 4.006 | 4.021 | 4.021 | 1048579 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 3.986 | 4.156 | 4.156 | 1048579 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 1.806 | 1.914 | 1.914 | 8192 | 1 | 0 / 0 / 0 |
| insert | first | 3 | 4.066 | 4.096 | 4.096 | 1052675 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 3.960 | 4.035 | 4.035 | 1052675 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 4.048 | 4.099 | 4.099 | 1048576 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 11.160 | 12.660 | 12.660 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.001 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 0.344 | 0.403 | 0.403 | 12288 | 1 | 0 / 0 / 0 |
| redraw | later | 9 | 0.281 | 0.374 | 0.374 | 12288 | 1 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.056 | 0.073 | 0.073 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |

## 1048576b-line1048576-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 5.777 | 5.779 | 5.779 | 1572874 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 5.784 | 11.124 | 11.124 | 1572874 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 2.066 | 2.257 | 2.257 | 40960 | 1 | 0 / 0 / 0 |
| insert | first | 3 | 5.958 | 5.995 | 5.995 | 1576966 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 5.856 | 6.026 | 6.026 | 1576966 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 5.945 | 9.397 | 9.397 | 1572866 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 10.830 | 12.396 | 12.396 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.112 | 2.256 | 2.256 | 569344 | 1 | 0 / 0 / 0 |
| redraw | later | 9 | 2.175 | 2.256 | 2.256 | 569344 | 1 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.001 | 0.001 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.059 | 0.071 | 0.071 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.005 | 0.011 | 0.011 | 2 | 0 | 0 / 0 / 0 |

## 1048576b-line1048576-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 7.595 | 7.791 | 7.791 | 2088970 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 7.588 | 7.943 | 7.943 | 2088970 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 1.906 | 2.052 | 2.052 | 28672 | 1 | 0 / 0 / 0 |
| insert | first | 3 | 7.760 | 7.988 | 7.988 | 2088966 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 7.635 | 8.465 | 8.465 | 2088966 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 8.008 | 8.032 | 8.032 | 2088962 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 11.730 | 14.970 | 14.970 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.006 | 0.006 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 3.777 | 3.943 | 3.943 | 1069056 | 1 | 0 / 0 / 0 |
| redraw | later | 9 | 3.732 | 3.985 | 3.985 | 1069056 | 1 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.071 | 0.081 | 0.081 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.004 | 0.009 | 0.009 | 2 | 0 | 0 / 0 / 0 |

## 104857600b-line64-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.006 | 0.013 | 0.013 | 259 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.006 | 0.008 | 0.008 | 259 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.240 | 4.277 | 4.277 | 1701 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 0.090 | 0.091 | 0.091 | 322 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.013 | 0.018 | 0.018 | 322 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 0.062 | 0.069 | 0.069 | 65536 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 201.843 | 205.742 | 205.742 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.001 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.210 | 2.720 | 2.720 | 3305 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 2.757 | 3.463 | 3.463 | 3305 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.048 | 0.061 | 0.061 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.005 | 0.007 | 0.007 | 1 | 0 | 0 / 0 / 0 |

## 104857600b-line64-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.010 | 0.010 | 0.010 | 266 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.009 | 0.014 | 0.014 | 266 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.107 | 4.226 | 4.226 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.088 | 0.129 | 0.129 | 325 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.014 | 0.017 | 0.017 | 325 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 219.260 | 221.431 | 221.431 | 52494338 | 819201 | 0 / 0 / 0 |
| open | setup | 3 | 200.645 | 215.313 | 215.313 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.008 | 0.008 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.283 | 2.347 | 2.347 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.184 | 2.495 | 2.495 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.003 | 0.003 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.059 | 0.079 | 0.079 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.004 | 0.008 | 0.008 | 2 | 0 | 0 / 0 / 0 |

## 104857600b-line64-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.015 | 0.017 | 0.017 | 266 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.012 | 0.016 | 0.016 | 266 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.050 | 4.179 | 4.179 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.095 | 0.101 | 0.101 | 325 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.014 | 0.022 | 0.022 | 325 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 423.484 | 424.160 | 424.160 | 103874562 | 1622017 | 0 / 0 / 0 |
| open | setup | 3 | 202.367 | 205.487 | 205.487 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.005 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.321 | 2.701 | 2.701 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.220 | 2.768 | 2.768 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.062 | 0.087 | 0.087 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.004 | 0.008 | 0.008 | 2 | 0 | 0 / 0 / 0 |

## 104857600b-line1048576-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 3.919 | 3.928 | 3.928 | 1113859 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 3.917 | 4.146 | 4.146 | 1113859 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 102.165 | 102.194 | 102.194 | 26324992 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 3.984 | 4.010 | 4.010 | 1117954 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 3.864 | 4.072 | 4.072 | 1117954 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 4.129 | 4.270 | 4.270 | 1048576 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 16.678 | 16.889 | 16.889 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.003 | 0.003 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 100.852 | 101.057 | 101.057 | 26329088 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 100.523 | 101.904 | 101.904 | 26329088 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.076 | 0.080 | 0.080 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.007 | 0.011 | 0.011 | 1 | 0 | 0 / 0 / 0 |

## 104857600b-line1048576-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 3.923 | 4.121 | 4.121 | 1113866 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 3.922 | 4.185 | 4.185 | 1113866 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.119 | 4.366 | 4.366 | 110592 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 3.989 | 4.117 | 4.117 | 1117957 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 3.908 | 4.085 | 4.085 | 1117957 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 202.742 | 204.766 | 204.766 | 53477378 | 51 | 0 / 0 / 0 |
| open | setup | 3 | 12.581 | 12.674 | 12.674 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.008 | 0.008 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.014 | 0.014 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.500 | 2.607 | 2.607 | 114688 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.454 | 2.736 | 2.736 | 114688 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.049 | 0.069 | 0.069 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.005 | 0.006 | 0.006 | 2 | 0 | 0 / 0 / 0 |

## 104857600b-line1048576-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 3.869 | 3.878 | 3.878 | 1048586 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 3.848 | 3.961 | 3.961 | 1048586 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.294 | 4.350 | 4.350 | 110592 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 3.943 | 3.952 | 3.952 | 1052678 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 3.882 | 4.001 | 4.001 | 1052678 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 390.235 | 391.461 | 391.461 | 104857602 | 100 | 0 / 0 / 0 |
| open | setup | 3 | 15.974 | 16.933 | 16.933 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.371 | 2.548 | 2.548 | 114688 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.362 | 2.513 | 2.513 | 114688 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.054 | 0.064 | 0.064 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.004 | 0.005 | 0.005 | 2 | 0 | 0 / 0 / 0 |

## 1000000000b-line64-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.007 | 0.009 | 0.009 | 259 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.007 | 0.012 | 0.012 | 259 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.070 | 4.260 | 4.260 | 1701 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 0.087 | 0.096 | 0.096 | 322 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.011 | 0.022 | 0.022 | 322 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 0.058 | 0.074 | 0.074 | 65536 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 1627.388 | 1659.931 | 1659.931 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.002 | 0.005 | 0.005 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.001 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.387 | 2.405 | 2.405 | 3305 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 2.237 | 2.450 | 2.450 | 3305 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.058 | 0.073 | 0.073 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.004 | 0.009 | 0.009 | 1 | 0 | 0 / 0 / 0 |

## 1000000000b-line64-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.010 | 0.010 | 0.010 | 266 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.008 | 0.009 | 0.009 | 266 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.031 | 4.388 | 4.388 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.096 | 0.116 | 0.116 | 325 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.013 | 0.015 | 0.015 | 325 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 2011.449 | 2115.735 | 2115.735 | 500039682 | 7812501 | 0 / 0 / 0 |
| open | setup | 3 | 1629.789 | 3746.200 | 3746.200 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.006 | 0.006 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.397 | 2.523 | 2.523 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.176 | 2.398 | 2.398 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.054 | 0.061 | 0.061 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.003 | 0.005 | 0.005 | 2 | 0 | 0 / 0 / 0 |

## 1000000000b-line64-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.009 | 0.011 | 0.011 | 266 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.008 | 0.011 | 0.011 | 266 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 3.875 | 4.184 | 4.184 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.085 | 0.087 | 0.087 | 325 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.012 | 0.018 | 0.018 | 325 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 3955.098 | 4131.141 | 4131.141 | 990052354 | 15468751 | 0 / 0 / 0 |
| open | setup | 3 | 1643.693 | 1690.265 | 1690.265 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.191 | 2.328 | 2.328 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.202 | 2.684 | 2.684 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.040 | 0.063 | 0.063 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.004 | 0.006 | 0.006 | 2 | 0 | 0 / 0 / 0 |

## 1000000000b-line1048576-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 3.982 | 4.423 | 4.423 | 1113859 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 4.026 | 4.151 | 4.151 | 1113859 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 103.121 | 104.402 | 104.402 | 26324992 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 4.045 | 4.186 | 4.186 | 1117954 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 4.051 | 4.823 | 4.823 | 1117954 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 4.198 | 4.269 | 4.269 | 1048576 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 14.691 | 25.352 | 25.352 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.003 | 0.003 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 101.190 | 102.734 | 102.734 | 26329088 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 101.260 | 113.327 | 113.327 | 26329088 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.084 | 0.157 | 0.157 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.008 | 0.009 | 0.009 | 1 | 0 | 0 / 0 / 0 |

## 1000000000b-line1048576-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 6.882 | 7.376 | 7.376 | 1994506 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 6.998 | 7.193 | 7.193 | 1994506 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 81.910 | 82.218 | 82.218 | 22048768 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 6.972 | 7.226 | 7.226 | 1994501 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 6.988 | 7.341 | 7.341 | 1994501 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 1887.722 | 1975.710 | 1975.710 | 501051394 | 477 | 0 / 0 / 0 |
| open | setup | 3 | 12.854 | 15.651 | 15.651 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 5.721 | 6.185 | 6.185 | 1015808 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 5.443 | 5.620 | 5.620 | 1015808 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.080 | 0.081 | 0.081 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.005 | 0.008 | 0.008 | 2 | 0 | 0 / 0 / 0 |

## 1000000000b-line1048576-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 4.715 | 4.821 | 4.821 | 1261322 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 4.637 | 4.720 | 4.720 | 1261322 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 18.617 | 19.297 | 19.297 | 3698688 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 4.736 | 4.787 | 4.787 | 1261317 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 4.583 | 5.012 | 5.012 | 1261317 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 3888.074 | 3917.854 | 3917.854 | 991051778 | 945 | 0 / 0 / 0 |
| open | setup | 3 | 9.743 | 13.129 | 13.129 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 3.458 | 3.642 | 3.642 | 262144 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 3.303 | 3.480 | 3.480 | 262144 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.077 | 0.084 | 0.084 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.006 | 0.008 | 0.008 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant1

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.010 | 0.011 | 0.011 | 266 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.010 | 0.012 | 0.012 | 266 | 1 | 0 / 0 / 0 |
| duplicate_view | setup | 3 | 0.005 | 0.005 | 0.005 | 0 | 0 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 5.664 | 6.169 | 6.169 | 68875 | 26 | 0 / 0 / 0 |
| insert | first | 3 | 0.094 | 0.096 | 0.096 | 325 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.015 | 0.023 | 0.023 | 325 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 2.319 | 2.439 | 2.439 | 589826 | 8193 | 0 / 0 / 0 |
| open | setup | 3 | 11.124 | 13.307 | 13.307 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 3.153 | 3.297 | 3.297 | 19277 | 91 | 0 / 0 / 0 |
| redraw | later | 9 | 3.199 | 3.356 | 3.356 | 19277 | 91 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.003 | 0.003 | 0.003 | 2 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.002 | 0.003 | 0.003 | 2 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.002 | 0.003 | 0.003 | 2 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.002 | 0.006 | 0.006 | 2 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.005 | 0.005 | 0.005 | 258 | 1 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.004 | 0.005 | 0.005 | 258 | 1 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.005 | 0.016 | 0.016 | 258 | 1 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.005 | 0.007 | 0.007 | 258 | 1 | 0 / 0 / 0 |
| undo | first | 3 | 0.075 | 0.077 | 0.077 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.008 | 0.015 | 0.015 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant2

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.010 | 0.011 | 0.011 | 266 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.009 | 0.012 | 0.012 | 266 | 1 | 0 / 0 / 0 |
| duplicate_view | setup | 3 | 0.003 | 0.003 | 0.003 | 0 | 0 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 5.025 | 5.068 | 5.068 | 68875 | 26 | 0 / 0 / 0 |
| insert | first | 3 | 0.023 | 0.023 | 0.023 | 325 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.014 | 0.018 | 0.018 | 325 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 2.232 | 2.369 | 2.369 | 589314 | 8192 | 0 / 0 / 0 |
| open | setup | 3 | 11.225 | 20.738 | 20.738 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 3.144 | 3.149 | 3.149 | 19277 | 91 | 0 / 0 / 0 |
| redraw | later | 9 | 3.216 | 3.433 | 3.433 | 19277 | 91 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.003 | 0.003 | 0.003 | 2 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.002 | 0.003 | 0.003 | 2 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.002 | 0.002 | 0.002 | 2 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.002 | 0.004 | 0.004 | 2 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.004 | 0.005 | 0.005 | 258 | 1 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.004 | 0.005 | 0.005 | 258 | 1 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.005 | 0.007 | 0.007 | 258 | 1 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.004 | 0.007 | 0.007 | 258 | 1 | 0 / 0 / 0 |
| undo | first | 3 | 0.070 | 0.150 | 0.150 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.007 | 0.009 | 0.009 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant3

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.020 | 0.023 | 0.023 | 266 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.021 | 0.061 | 0.061 | 266 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.494 | 4.610 | 4.610 | 1703 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.038 | 0.043 | 0.043 | 325 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.019 | 0.031 | 0.031 | 325 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 2.953 | 3.015 | 3.015 | 589571 | 8193 | 0 / 0 / 0 |
| open | setup | 3 | 9.676 | 22.633 | 22.633 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.005 | 0.006 | 0.006 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.004 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.335 | 2.340 | 2.340 | 1769 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.332 | 2.495 | 2.495 | 1769 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.071 | 0.078 | 0.078 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.011 | 0.013 | 0.013 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant4

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 9.534 | 9.725 | 9.725 | 266 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 8.792 | 9.768 | 9.768 | 266 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.096 | 4.125 | 4.125 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 45.540 | 76.260 | 76.260 | 325 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 9.982 | 10.337 | 10.337 | 325 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 2.295 | 2.396 | 2.396 | 589826 | 8193 | 0 / 0 / 0 |
| open | setup | 3 | 5.818 | 6.991 | 6.991 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 9.506 | 10.199 | 10.199 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 10.085 | 10.211 | 10.211 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.560 | 2.820 | 2.820 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.497 | 2.645 | 2.645 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.003 | 0.003 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 9.737 | 10.212 | 10.212 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 8.311 | 10.171 | 10.171 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant5

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 8.268 | 8.554 | 8.554 | 266 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 9.138 | 12.166 | 12.166 | 266 | 1 | 0 / 0 / 0 |
| duplicate_view | setup | 3 | 0.002 | 0.003 | 0.003 | 0 | 0 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 5.120 | 5.269 | 5.269 | 68878 | 26 | 0 / 0 / 0 |
| insert | first | 3 | 48.925 | 52.347 | 52.347 | 325 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 8.667 | 11.602 | 11.602 | 325 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 2.839 | 2.963 | 2.963 | 589571 | 8192 | 0 / 0 / 0 |
| open | setup | 3 | 6.611 | 21.522 | 21.522 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 8.713 | 9.230 | 9.230 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 8.207 | 9.260 | 9.260 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 3.452 | 4.139 | 4.139 | 19281 | 91 | 0 / 0 / 0 |
| redraw | later | 9 | 3.326 | 3.960 | 3.960 | 19281 | 91 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.005 | 0.007 | 0.007 | 2 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.005 | 0.008 | 0.008 | 2 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.005 | 0.005 | 0.005 | 2 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.005 | 0.008 | 0.008 | 2 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.014 | 0.017 | 0.017 | 258 | 1 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.014 | 0.019 | 0.019 | 258 | 1 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.013 | 0.014 | 0.014 | 258 | 1 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.015 | 0.029 | 0.029 | 258 | 1 | 0 / 0 / 0 |
| undo | first | 3 | 9.859 | 10.955 | 10.955 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 8.628 | 9.329 | 9.329 | 2 | 0 | 0 / 0 / 0 |

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
