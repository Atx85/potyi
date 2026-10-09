# Sustained editing baseline

Recorded: 2026-10-08T14:37:32.653254+00:00.

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
| backspace | first | 3 | 0.010 | 0.012 | 0.012 | 65539 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.008 | 0.022 | 0.022 | 65539 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.260 | 4.277 | 4.277 | 1701 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 0.082 | 0.088 | 0.088 | 65602 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.014 | 0.021 | 0.021 | 65602 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 0.057 | 0.092 | 0.092 | 65536 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 5.933 | 6.740 | 6.740 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.001 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.001 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.301 | 2.322 | 2.322 | 1767 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 2.208 | 2.454 | 2.454 | 1767 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.058 | 0.061 | 0.061 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.004 | 0.008 | 0.008 | 1 | 0 | 0 / 0 / 0 |

## 1048576b-line64-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.013 | 0.018 | 0.018 | 65546 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.011 | 0.019 | 0.019 | 65546 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.141 | 4.173 | 4.173 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.074 | 0.854 | 0.854 | 65605 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.017 | 0.021 | 0.021 | 65605 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 2.171 | 2.321 | 2.321 | 589826 | 8193 | 0 / 0 / 0 |
| open | setup | 3 | 9.272 | 21.716 | 21.716 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.002 | 0.039 | 0.039 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.152 | 2.620 | 2.620 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.117 | 2.453 | 2.453 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.046 | 0.173 | 0.173 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.003 | 0.006 | 0.006 | 2 | 0 | 0 / 0 / 0 |

## 1048576b-line64-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.015 | 0.019 | 0.019 | 10569 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.011 | 0.020 | 0.020 | 10569 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.016 | 4.106 | 4.106 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.084 | 0.089 | 0.089 | 10566 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.017 | 0.023 | 0.023 | 10566 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 4.126 | 4.479 | 4.479 | 1048641 | 16221 | 0 / 0 / 0 |
| open | setup | 3 | 8.151 | 21.528 | 21.528 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.625 | 2.644 | 2.644 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.217 | 2.491 | 2.491 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.066 | 0.070 | 0.070 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.004 | 0.007 | 0.007 | 2 | 0 | 0 / 0 / 0 |

## 1048576b-line1048576-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 3.799 | 3.858 | 3.858 | 1048579 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 3.660 | 4.825 | 4.825 | 1048579 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 1.869 | 1.875 | 1.875 | 8192 | 1 | 0 / 0 / 0 |
| insert | first | 3 | 3.810 | 4.029 | 4.029 | 1052676 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 3.790 | 3.841 | 3.841 | 1052676 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 3.813 | 3.828 | 3.828 | 1048576 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 6.285 | 6.958 | 6.958 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.001 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 0.421 | 0.467 | 0.467 | 12288 | 1 | 0 / 0 / 0 |
| redraw | later | 9 | 0.261 | 0.408 | 0.408 | 12288 | 1 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.071 | 0.089 | 0.089 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |

## 1048576b-line1048576-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 5.750 | 5.832 | 5.832 | 1572874 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 5.668 | 6.538 | 6.538 | 1572874 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 2.266 | 2.450 | 2.450 | 40960 | 1 | 0 / 0 / 0 |
| insert | first | 3 | 5.793 | 6.663 | 6.663 | 1576967 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 5.690 | 6.121 | 6.121 | 1576967 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 5.895 | 6.111 | 6.111 | 1572866 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 4.573 | 6.224 | 6.224 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.243 | 2.295 | 2.295 | 569344 | 1 | 0 / 0 / 0 |
| redraw | later | 9 | 2.227 | 2.403 | 2.403 | 569344 | 1 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.088 | 0.093 | 0.093 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.006 | 0.009 | 0.009 | 2 | 0 | 0 / 0 / 0 |

## 1048576b-line1048576-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 7.483 | 7.538 | 7.538 | 2088970 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 7.443 | 7.671 | 7.671 | 2088970 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 2.276 | 2.297 | 2.297 | 28672 | 1 | 0 / 0 / 0 |
| insert | first | 3 | 7.607 | 7.658 | 7.658 | 2088967 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 7.498 | 7.804 | 7.804 | 2088967 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 7.657 | 7.832 | 7.832 | 2088962 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 4.705 | 9.992 | 9.992 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 3.897 | 3.899 | 3.899 | 1069056 | 1 | 0 / 0 / 0 |
| redraw | later | 9 | 3.847 | 4.154 | 4.154 | 1069056 | 1 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.073 | 0.080 | 0.080 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.006 | 0.009 | 0.009 | 2 | 0 | 0 / 0 / 0 |

## 104857600b-line64-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.019 | 0.042 | 0.042 | 65539 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.014 | 0.022 | 0.022 | 65539 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.755 | 5.113 | 5.113 | 1701 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 0.096 | 0.104 | 0.104 | 65602 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.019 | 0.088 | 0.088 | 65602 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 0.060 | 0.062 | 0.062 | 65536 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 180.720 | 185.385 | 185.385 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.001 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.365 | 2.369 | 2.369 | 1767 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 2.383 | 2.412 | 2.412 | 1767 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.071 | 0.079 | 0.079 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.007 | 0.009 | 0.009 | 1 | 0 | 0 / 0 / 0 |

## 104857600b-line64-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.016 | 0.020 | 0.020 | 65546 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.014 | 0.017 | 0.017 | 65546 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.004 | 4.231 | 4.231 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.101 | 0.120 | 0.120 | 65605 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.019 | 0.024 | 0.024 | 65605 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 203.966 | 208.486 | 208.486 | 52494338 | 819201 | 0 / 0 / 0 |
| open | setup | 3 | 180.062 | 201.819 | 201.819 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.445 | 2.449 | 2.449 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.239 | 2.306 | 2.306 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.069 | 0.070 | 0.070 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.005 | 0.008 | 0.008 | 2 | 0 | 0 / 0 / 0 |

## 104857600b-line64-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.017 | 0.019 | 0.019 | 65546 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.015 | 0.018 | 0.018 | 65546 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.215 | 4.399 | 4.399 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.081 | 0.094 | 0.094 | 65605 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.021 | 0.023 | 0.023 | 65605 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 403.530 | 420.560 | 420.560 | 103874562 | 1622017 | 0 / 0 / 0 |
| open | setup | 3 | 248.820 | 350.883 | 350.883 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.353 | 2.548 | 2.548 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.407 | 2.628 | 2.628 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.061 | 0.085 | 0.085 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.007 | 0.009 | 0.009 | 2 | 0 | 0 / 0 / 0 |

## 104857600b-line1048576-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 3.790 | 3.829 | 3.829 | 1048579 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 3.748 | 4.441 | 4.441 | 1048579 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 98.733 | 98.799 | 98.799 | 26324992 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 3.871 | 4.048 | 4.048 | 1118211 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 3.755 | 4.227 | 4.227 | 1118211 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 3.889 | 4.069 | 4.069 | 1048576 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 6.269 | 14.944 | 14.944 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.003 | 0.003 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 96.916 | 97.055 | 97.055 | 26329113 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 97.238 | 99.205 | 99.205 | 26329113 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.078 | 0.087 | 0.087 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.009 | 0.010 | 0.010 | 1 | 0 | 0 / 0 / 0 |

## 104857600b-line1048576-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 3.667 | 4.688 | 4.688 | 1048586 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 3.707 | 3.865 | 3.865 | 1048586 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.626 | 5.582 | 5.582 | 110592 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 3.886 | 4.778 | 4.778 | 1118214 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 3.718 | 3.923 | 3.923 | 1118214 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 193.711 | 228.883 | 228.883 | 53477378 | 51 | 0 / 0 / 0 |
| open | setup | 3 | 14.323 | 20.040 | 20.040 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.006 | 0.006 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.756 | 2.762 | 2.762 | 114688 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.612 | 2.822 | 2.822 | 114688 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.079 | 0.084 | 0.084 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.006 | 0.007 | 0.007 | 2 | 0 | 0 / 0 / 0 |

## 104857600b-line1048576-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 4.669 | 6.724 | 6.724 | 1048586 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 3.756 | 8.129 | 8.129 | 1048586 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.447 | 4.797 | 4.797 | 110592 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 3.980 | 4.807 | 4.807 | 1052679 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 4.070 | 17.190 | 17.190 | 1052679 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 594.170 | 599.265 | 599.265 | 104857602 | 100 | 0 / 0 / 0 |
| open | setup | 3 | 70.378 | 272.862 | 272.862 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.671 | 3.205 | 3.205 | 114688 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.607 | 12.082 | 12.082 | 114688 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.086 | 0.088 | 0.088 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.007 | 0.010 | 0.010 | 2 | 0 | 0 / 0 / 0 |

## 1000000000b-line64-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.018 | 0.020 | 0.020 | 65539 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.014 | 0.018 | 0.018 | 65539 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.596 | 5.130 | 5.130 | 1701 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 0.074 | 0.084 | 0.084 | 65602 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.018 | 0.026 | 0.026 | 65602 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 0.069 | 0.076 | 0.076 | 65536 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 1894.958 | 3626.242 | 3626.242 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.001 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.444 | 2.656 | 2.656 | 1767 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 2.417 | 2.813 | 2.813 | 1767 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.075 | 0.075 | 0.075 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.006 | 0.008 | 0.008 | 1 | 0 | 0 / 0 / 0 |

## 1000000000b-line64-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.013 | 0.014 | 0.014 | 65546 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.015 | 0.022 | 0.022 | 65546 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.066 | 4.411 | 4.411 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.093 | 0.096 | 0.096 | 65605 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.016 | 0.019 | 0.019 | 65605 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 1890.343 | 1907.842 | 1907.842 | 500039682 | 7812501 | 0 / 0 / 0 |
| open | setup | 3 | 1793.186 | 3416.924 | 3416.924 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.306 | 2.344 | 2.344 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.260 | 2.596 | 2.596 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.053 | 0.057 | 0.057 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.004 | 0.007 | 0.007 | 2 | 0 | 0 / 0 / 0 |

## 1000000000b-line64-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.014 | 0.016 | 0.016 | 65546 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.014 | 0.015 | 0.015 | 65546 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.034 | 4.231 | 4.231 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.087 | 0.094 | 0.094 | 65605 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.017 | 0.026 | 0.026 | 65605 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 3774.299 | 3883.160 | 3883.160 | 990052354 | 15468751 | 0 / 0 / 0 |
| open | setup | 3 | 1652.582 | 1654.556 | 1654.556 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.351 | 2.463 | 2.463 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.210 | 2.418 | 2.418 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.060 | 0.071 | 0.071 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.003 | 0.007 | 0.007 | 2 | 0 | 0 / 0 / 0 |

## 1000000000b-line1048576-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 3.620 | 3.641 | 3.641 | 1048579 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 3.624 | 3.737 | 3.737 | 1048579 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 94.918 | 95.508 | 95.508 | 26324992 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 3.661 | 3.755 | 3.755 | 1118211 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 3.657 | 3.855 | 3.855 | 1118211 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 3.902 | 3.934 | 3.934 | 1048576 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 10.700 | 14.297 | 14.297 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.002 | 0.003 | 0.003 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.005 | 0.005 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 93.259 | 94.213 | 94.213 | 26329113 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 93.799 | 94.918 | 94.918 | 26329113 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.091 | 0.093 | 0.093 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.007 | 0.011 | 0.011 | 1 | 0 | 0 / 0 / 0 |

## 1000000000b-line1048576-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 6.610 | 6.654 | 6.654 | 1929226 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 6.545 | 6.825 | 6.825 | 1929226 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 81.808 | 83.292 | 83.292 | 22048768 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 6.679 | 6.739 | 6.739 | 1994758 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 6.592 | 6.842 | 6.842 | 1994758 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 1722.130 | 1737.378 | 1737.378 | 501051394 | 477 | 0 / 0 / 0 |
| open | setup | 3 | 21.905 | 32.941 | 32.941 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.002 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 5.486 | 5.687 | 5.687 | 1015808 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 5.348 | 5.734 | 5.734 | 1015808 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.073 | 0.093 | 0.093 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.004 | 0.006 | 0.006 | 2 | 0 | 0 / 0 / 0 |

## 1000000000b-line1048576-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 4.233 | 4.258 | 4.258 | 1196042 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 4.248 | 4.404 | 4.404 | 1196042 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 17.022 | 18.212 | 18.212 | 3698688 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 4.338 | 4.485 | 4.485 | 1261574 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 4.292 | 4.406 | 4.406 | 1261574 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 3499.938 | 3599.781 | 3599.781 | 991051778 | 945 | 0 / 0 / 0 |
| open | setup | 3 | 13.444 | 17.479 | 17.479 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.002 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 3.113 | 3.413 | 3.413 | 262144 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 3.027 | 3.165 | 3.165 | 262144 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.072 | 0.087 | 0.087 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.004 | 0.008 | 0.008 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant1

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.014 | 0.018 | 0.018 | 65546 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.012 | 0.023 | 0.023 | 65546 | 1 | 0 / 0 / 0 |
| duplicate_view | setup | 3 | 0.070 | 0.073 | 0.073 | 0 | 0 | 1 / 8193 / 0 |
| initial_draw | setup | 3 | 4.900 | 5.411 | 5.411 | 3339 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.091 | 0.091 | 0.091 | 65605 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.018 | 0.020 | 0.020 | 65605 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 2.344 | 2.531 | 2.531 | 589826 | 8193 | 0 / 0 / 0 |
| open | setup | 3 | 13.142 | 18.182 | 18.182 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.008 | 0.008 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.002 | 0.002 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 3.185 | 3.340 | 3.340 | 3405 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 3.063 | 3.297 | 3.297 | 3405 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.110 | 0.128 | 0.128 | 2 | 0 | 3 / 8193 / 1 |
| synchronize | later | 9 | 0.013 | 0.052 | 0.052 | 2 | 0 | 3 / 8193 / 5 |
| synchronize_backspace | first | 3 | 0.093 | 0.112 | 0.112 | 2 | 0 | 2 / 8193 / 2 |
| synchronize_backspace | later | 9 | 0.011 | 0.068 | 0.068 | 2 | 0 | 2 / 8193 / 6 |
| synchronize_redo | first | 3 | 0.090 | 0.142 | 0.142 | 65538 | 1 | 3 / 8192 / 1 |
| synchronize_redo | later | 9 | 0.090 | 0.131 | 0.131 | 65538 | 1 | 3 / 8192 / 5 |
| synchronize_undo | first | 3 | 0.092 | 0.630 | 0.630 | 65538 | 1 | 2 / 8192 / 1 |
| synchronize_undo | later | 9 | 0.045 | 0.097 | 0.097 | 65538 | 1 | 2 / 8192 / 5 |
| undo | first | 3 | 0.048 | 0.051 | 0.051 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.004 | 0.009 | 0.009 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant2

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.014 | 0.021 | 0.021 | 65546 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.012 | 0.059 | 0.059 | 65546 | 1 | 0 / 0 / 0 |
| duplicate_view | setup | 3 | 0.321 | 0.323 | 0.323 | 0 | 0 | 2 / 8193 / 1000 |
| initial_draw | setup | 3 | 4.738 | 4.802 | 4.802 | 3339 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.022 | 0.032 | 0.032 | 65605 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.017 | 0.031 | 0.031 | 65605 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 2.003 | 2.075 | 2.075 | 524290 | 8192 | 0 / 0 / 0 |
| open | setup | 3 | 16.894 | 30.507 | 30.507 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.968 | 3.041 | 3.041 | 3405 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.872 | 3.241 | 3.241 | 3405 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.417 | 0.453 | 0.453 | 2 | 0 | 4 / 8193 / 1001 |
| synchronize | later | 9 | 0.216 | 0.269 | 0.269 | 2 | 0 | 4 / 8193 / 1005 |
| synchronize_backspace | first | 3 | 0.216 | 0.256 | 0.256 | 2 | 0 | 3 / 8193 / 1002 |
| synchronize_backspace | later | 9 | 0.235 | 0.270 | 0.270 | 2 | 0 | 3 / 8193 / 1006 |
| synchronize_redo | first | 3 | 0.275 | 0.328 | 0.328 | 65538 | 1 | 4 / 8192 / 1001 |
| synchronize_redo | later | 9 | 0.280 | 0.341 | 0.341 | 65538 | 1 | 4 / 8192 / 1005 |
| synchronize_undo | first | 3 | 0.276 | 0.350 | 0.350 | 65538 | 1 | 3 / 8192 / 1001 |
| synchronize_undo | later | 9 | 0.263 | 0.358 | 0.358 | 65538 | 1 | 3 / 8192 / 1005 |
| undo | first | 3 | 0.053 | 0.058 | 0.058 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.004 | 0.009 | 0.009 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant3

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.104 | 0.123 | 0.123 | 65546 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.099 | 0.127 | 0.127 | 65546 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.182 | 4.432 | 4.432 | 1703 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.116 | 0.120 | 0.120 | 65605 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.098 | 0.148 | 0.148 | 65605 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 2.779 | 3.073 | 3.073 | 589826 | 8193 | 0 / 0 / 0 |
| open | setup | 3 | 16.272 | 18.397 | 18.397 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.005 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.316 | 2.450 | 2.450 | 1769 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.187 | 2.749 | 2.749 | 1769 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.001 | 0.001 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.056 | 0.086 | 0.086 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.009 | 0.024 | 0.024 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant4

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 8.814 | 9.469 | 9.469 | 65546 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 9.525 | 12.557 | 12.557 | 65546 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.077 | 4.134 | 4.134 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 45.859 | 50.115 | 50.115 | 65605 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 8.591 | 9.465 | 9.465 | 65605 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 2.211 | 2.258 | 2.258 | 589826 | 8193 | 0 / 0 / 0 |
| open | setup | 3 | 13.743 | 19.432 | 19.432 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 9.569 | 10.085 | 10.085 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 7.799 | 9.596 | 9.596 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.460 | 2.908 | 2.908 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 3.150 | 3.623 | 3.623 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 8.789 | 9.219 | 9.219 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 8.771 | 10.894 | 10.894 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant5

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 8.133 | 8.397 | 8.397 | 65546 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 8.474 | 9.969 | 9.969 | 65546 | 1 | 0 / 0 / 0 |
| duplicate_view | setup | 3 | 0.301 | 0.319 | 0.319 | 0 | 0 | 2002 / 8193 / 1000 |
| initial_draw | setup | 3 | 4.866 | 4.929 | 4.929 | 3342 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 63.858 | 72.622 | 72.622 | 65605 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 8.737 | 9.835 | 9.835 | 65605 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 2.801 | 2.996 | 2.996 | 589826 | 8192 | 0 / 0 / 0 |
| open | setup | 3 | 18.112 | 24.251 | 24.251 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 8.873 | 9.726 | 9.726 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 8.798 | 9.130 | 9.130 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 3.941 | 4.104 | 4.104 | 3408 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 3.856 | 4.677 | 4.677 | 3408 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.628 | 0.881 | 0.881 | 2 | 0 | 2004 / 8193 / 1001 |
| synchronize | later | 9 | 0.309 | 0.434 | 0.434 | 2 | 0 | 2004 / 8193 / 1005 |
| synchronize_backspace | first | 3 | 0.279 | 0.281 | 0.281 | 2 | 0 | 2003 / 8193 / 1002 |
| synchronize_backspace | later | 9 | 0.294 | 0.491 | 0.491 | 2 | 0 | 2003 / 8193 / 1006 |
| synchronize_redo | first | 3 | 0.469 | 0.554 | 0.554 | 65538 | 1 | 2004 / 8192 / 1001 |
| synchronize_redo | later | 9 | 0.447 | 0.745 | 0.745 | 65538 | 1 | 2004 / 8192 / 1005 |
| synchronize_undo | first | 3 | 0.562 | 0.611 | 0.611 | 65538 | 1 | 2003 / 8192 / 1001 |
| synchronize_undo | later | 9 | 0.426 | 0.499 | 0.499 | 65538 | 1 | 2003 / 8192 / 1005 |
| undo | first | 3 | 7.964 | 9.202 | 9.202 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 8.264 | 9.372 | 9.372 | 2 | 0 | 0 / 0 / 0 |

## Limits

- Local measurements with freshly written, warm filesystem fixtures; no cold-disk or other-editor comparison.
- Headless rendering measures SDL draw work, not native desktop presentation, input dispatch or LSP work.
- RSS is sampled about every 100ms across fixture/setup/editing, not true peak memory or an allocation budget; ps may be unavailable.
- Read counters count successful positional file reads in the piece table, including rereads; not physical disk I/O or recovery writes.
- Line discoveries count discovery calls past the all-cached guard, including EOF handling; not unique lines.
- Copy counters count top-level records at view/history cloning sites, not bytes or deep allocations in undo snapshots.
- p95/p99 use nearest ranks; small sample counts and correlated operations limit tail claims.
- The first edit and later edits are reported separately. Timeout includes fixture and setup; partial timings remain in raw events.
- Unfinished warmups stop that case's remaining runs; no timing threshold determines correctness.
- Concurrent checkout changes after building cannot alter the saved probe. Different before/after build hashes make source provenance inconclusive.
