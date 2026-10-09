# Sustained editing baseline

Recorded: 2026-10-08T15:37:46.357986+00:00.

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
| 1000000000b-line64-pos99 | 0 | timed_out | navigation | unavailable |
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
| backspace | first | 3 | 0.015 | 0.016 | 0.016 | 65539 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.012 | 0.015 | 0.015 | 65539 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.160 | 4.261 | 4.261 | 1701 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 0.089 | 0.092 | 0.092 | 65602 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.020 | 0.022 | 0.022 | 65602 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 0.063 | 0.063 | 0.063 | 65536 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 6.942 | 8.549 | 8.549 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.001 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.541 | 2.592 | 2.592 | 1767 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 2.324 | 2.503 | 2.503 | 1767 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.001 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.069 | 0.076 | 0.076 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.006 | 0.008 | 0.008 | 1 | 0 | 0 / 0 / 0 |

## 1048576b-line64-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.020 | 0.020 | 0.020 | 65546 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.015 | 0.021 | 0.021 | 65546 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.170 | 7.937 | 7.937 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.097 | 0.121 | 0.121 | 65605 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.019 | 0.026 | 0.026 | 65605 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 2.579 | 2.852 | 2.852 | 589826 | 8193 | 0 / 0 / 0 |
| open | setup | 3 | 16.405 | 17.495 | 17.495 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.408 | 2.920 | 2.920 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.281 | 3.231 | 3.231 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.001 | 0.006 | 0.006 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.073 | 0.073 | 0.073 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.006 | 0.007 | 0.007 | 2 | 0 | 0 / 0 / 0 |

## 1048576b-line64-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.016 | 0.021 | 0.021 | 10569 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.019 | 3.781 | 3.781 | 10569 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.230 | 7.117 | 7.117 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.094 | 0.098 | 0.098 | 10566 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.024 | 0.048 | 0.048 | 10566 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 7.376 | 9.041 | 9.041 | 1048641 | 16221 | 0 / 0 / 0 |
| open | setup | 3 | 21.055 | 46.917 | 46.917 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.009 | 0.009 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.599 | 4.930 | 4.930 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 3.428 | 14.563 | 14.563 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.001 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.001 | 0.001 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.092 | 0.137 | 0.137 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.012 | 0.127 | 0.127 | 2 | 0 | 0 / 0 / 0 |

## 1048576b-line1048576-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 4.248 | 4.318 | 4.318 | 1048579 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 4.239 | 7.687 | 7.687 | 1048579 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 1.910 | 2.154 | 2.154 | 8192 | 1 | 0 / 0 / 0 |
| insert | first | 3 | 4.222 | 4.506 | 4.506 | 1052676 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 4.130 | 5.239 | 5.239 | 1052676 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 4.322 | 4.700 | 4.700 | 1048576 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 10.350 | 18.174 | 18.174 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.003 | 0.003 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 0.467 | 0.561 | 0.561 | 12288 | 1 | 0 / 0 / 0 |
| redraw | later | 9 | 0.335 | 0.746 | 0.746 | 12288 | 1 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.002 | 0.008 | 0.008 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.064 | 0.074 | 0.074 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.005 | 0.007 | 0.007 | 1 | 0 | 0 / 0 / 0 |

## 1048576b-line1048576-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 6.068 | 6.201 | 6.201 | 1572874 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 6.162 | 6.520 | 6.520 | 1572874 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 2.258 | 2.356 | 2.356 | 40960 | 1 | 0 / 0 / 0 |
| insert | first | 3 | 6.099 | 6.785 | 6.785 | 1576967 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 6.321 | 7.039 | 7.039 | 1576967 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 6.440 | 6.938 | 6.938 | 1572866 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 8.600 | 14.668 | 14.668 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.251 | 2.347 | 2.347 | 569344 | 1 | 0 / 0 / 0 |
| redraw | later | 9 | 2.184 | 2.231 | 2.231 | 569344 | 1 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.003 | 0.003 | 0.003 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.075 | 0.132 | 0.132 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.007 | 0.016 | 0.016 | 2 | 0 | 0 / 0 / 0 |

## 1048576b-line1048576-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 8.009 | 45.293 | 45.293 | 2088970 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 8.059 | 10.676 | 10.676 | 2088970 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 2.093 | 2.414 | 2.414 | 28672 | 1 | 0 / 0 / 0 |
| insert | first | 3 | 8.194 | 9.226 | 9.226 | 2088967 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 8.155 | 15.266 | 15.266 | 2088967 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 7.910 | 8.015 | 8.015 | 2088962 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 4.286 | 10.623 | 10.623 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 4.535 | 7.994 | 7.994 | 1069056 | 1 | 0 / 0 / 0 |
| redraw | later | 9 | 4.190 | 6.231 | 6.231 | 1069056 | 1 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.082 | 0.089 | 0.089 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.007 | 0.010 | 0.010 | 2 | 0 | 0 / 0 / 0 |

## 104857600b-line64-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.014 | 0.016 | 0.016 | 65539 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.014 | 0.041 | 0.041 | 65539 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.228 | 4.319 | 4.319 | 1701 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 0.087 | 0.105 | 0.105 | 65602 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.019 | 0.025 | 0.025 | 65602 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 0.075 | 0.098 | 0.098 | 65536 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 182.157 | 186.809 | 186.809 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.001 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.432 | 2.475 | 2.475 | 1767 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 2.270 | 2.484 | 2.484 | 1767 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.001 | 0.001 | 0.001 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.071 | 0.075 | 0.075 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.006 | 0.012 | 0.012 | 1 | 0 | 0 / 0 / 0 |

## 104857600b-line64-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.016 | 0.019 | 0.019 | 65546 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.015 | 0.018 | 0.018 | 65546 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 3.937 | 4.233 | 4.233 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.091 | 0.101 | 0.101 | 65605 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.020 | 0.025 | 0.025 | 65605 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 215.218 | 215.431 | 215.431 | 52494338 | 819201 | 0 / 0 / 0 |
| open | setup | 3 | 207.878 | 359.878 | 359.878 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.309 | 2.336 | 2.336 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.244 | 2.351 | 2.351 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.001 | 0.001 | 0.001 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.061 | 0.063 | 0.063 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.005 | 0.007 | 0.007 | 2 | 0 | 0 / 0 / 0 |

## 104857600b-line64-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.016 | 0.018 | 0.018 | 65546 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.016 | 0.019 | 0.019 | 65546 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.104 | 4.354 | 4.354 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.100 | 0.105 | 0.105 | 65605 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.021 | 0.027 | 0.027 | 65605 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 427.194 | 744.250 | 744.250 | 103874562 | 1622017 | 0 / 0 / 0 |
| open | setup | 3 | 240.557 | 652.143 | 652.143 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.382 | 2.411 | 2.411 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.361 | 2.476 | 2.476 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.001 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.069 | 0.071 | 0.071 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.006 | 0.016 | 0.016 | 2 | 0 | 0 / 0 / 0 |

## 104857600b-line1048576-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 3.996 | 4.195 | 4.195 | 1048579 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 3.953 | 4.463 | 4.463 | 1048579 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 105.491 | 111.540 | 111.540 | 26324992 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 4.133 | 4.216 | 4.216 | 1118211 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 4.106 | 4.204 | 4.204 | 1118211 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 4.271 | 4.877 | 4.877 | 1048576 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 4.048 | 18.560 | 18.560 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.006 | 0.006 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 103.190 | 107.416 | 107.416 | 26329113 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 103.250 | 108.125 | 108.125 | 26329113 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.090 | 0.186 | 0.186 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.008 | 0.011 | 0.011 | 1 | 0 | 0 / 0 / 0 |

## 104857600b-line1048576-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 3.997 | 4.003 | 4.003 | 1048586 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 3.938 | 4.038 | 4.038 | 1048586 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.114 | 4.160 | 4.160 | 110592 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 4.156 | 4.180 | 4.180 | 1118214 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 3.996 | 4.076 | 4.076 | 1118214 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 202.602 | 203.707 | 203.707 | 53477378 | 51 | 0 / 0 / 0 |
| open | setup | 3 | 10.806 | 10.987 | 10.987 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.007 | 0.007 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.006 | 0.006 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.582 | 2.775 | 2.775 | 114688 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.647 | 2.719 | 2.719 | 114688 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.067 | 0.081 | 0.081 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.005 | 0.009 | 0.009 | 2 | 0 | 0 / 0 / 0 |

## 104857600b-line1048576-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 4.012 | 4.220 | 4.220 | 1048586 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 4.041 | 4.285 | 4.285 | 1048586 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.094 | 4.250 | 4.250 | 110592 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 4.083 | 4.221 | 4.221 | 1052679 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 3.962 | 4.111 | 4.111 | 1052679 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 403.840 | 404.590 | 404.590 | 104857602 | 100 | 0 / 0 / 0 |
| open | setup | 3 | 19.756 | 35.322 | 35.322 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.536 | 2.637 | 2.637 | 114688 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.504 | 2.709 | 2.709 | 114688 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.072 | 0.078 | 0.078 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.005 | 0.008 | 0.008 | 2 | 0 | 0 / 0 / 0 |

## 1000000000b-line64-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.013 | 0.013 | 0.013 | 65539 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.013 | 0.020 | 0.020 | 65539 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.295 | 4.723 | 4.723 | 1701 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 0.090 | 0.102 | 0.102 | 65602 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.019 | 0.031 | 0.031 | 65602 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 0.068 | 0.071 | 0.071 | 65536 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 1800.242 | 2559.540 | 2559.540 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.001 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.359 | 2.387 | 2.387 | 1767 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 2.335 | 2.818 | 2.818 | 1767 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.001 | 0.001 | 0.001 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.066 | 0.066 | 0.066 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.005 | 0.010 | 0.010 | 1 | 0 | 0 / 0 / 0 |

## 1000000000b-line64-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.015 | 0.018 | 0.018 | 65546 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.013 | 0.017 | 0.017 | 65546 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 3.976 | 4.408 | 4.408 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.092 | 0.112 | 0.112 | 65605 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.017 | 0.022 | 0.022 | 65605 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 2024.898 | 2025.692 | 2025.692 | 500039682 | 7812501 | 0 / 0 / 0 |
| open | setup | 3 | 1658.855 | 1731.656 | 1731.656 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.336 | 2.463 | 2.463 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.248 | 2.314 | 2.314 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.001 | 0.001 | 0.001 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.061 | 0.135 | 0.135 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.004 | 0.015 | 0.015 | 2 | 0 | 0 / 0 / 0 |

## 1000000000b-line1048576-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 3.883 | 4.125 | 4.125 | 1048579 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 3.870 | 3.960 | 3.960 | 1048579 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 102.880 | 104.324 | 104.324 | 26324992 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 4.026 | 4.221 | 4.221 | 1118211 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 3.953 | 4.229 | 4.229 | 1118211 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 4.377 | 4.795 | 4.795 | 1048576 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 17.369 | 28.254 | 28.254 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 100.496 | 101.026 | 101.026 | 26329113 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 101.374 | 102.043 | 102.043 | 26329113 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.084 | 0.087 | 0.087 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.008 | 0.010 | 0.010 | 1 | 0 | 0 / 0 / 0 |

## 1000000000b-line1048576-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 6.921 | 7.157 | 7.157 | 1929226 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 7.069 | 7.395 | 7.395 | 1929226 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 81.129 | 84.621 | 84.621 | 22048768 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 7.015 | 7.156 | 7.156 | 1994758 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 7.179 | 7.330 | 7.330 | 1994758 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 1885.273 | 1998.037 | 1998.037 | 501051394 | 477 | 0 / 0 / 0 |
| open | setup | 3 | 17.529 | 50.365 | 50.365 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.011 | 0.011 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 5.418 | 5.580 | 5.580 | 1015808 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 5.434 | 5.545 | 5.545 | 1015808 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.083 | 0.135 | 0.135 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.006 | 0.006 | 0.006 | 2 | 0 | 0 / 0 / 0 |

## 1000000000b-line1048576-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 4.566 | 4.702 | 4.702 | 1196042 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 4.588 | 4.662 | 4.662 | 1196042 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 17.811 | 17.825 | 17.825 | 3698688 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 4.667 | 4.885 | 4.885 | 1261574 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 4.577 | 4.720 | 4.720 | 1261574 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 3820.514 | 4332.193 | 4332.193 | 991051778 | 945 | 0 / 0 / 0 |
| open | setup | 3 | 20.366 | 39.242 | 39.242 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 3.126 | 3.187 | 3.187 | 262144 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 3.053 | 3.110 | 3.110 | 262144 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.002 | 0.005 | 0.005 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.088 | 0.091 | 0.091 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.006 | 0.008 | 0.008 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant1

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.014 | 0.014 | 0.014 | 65546 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.013 | 0.034 | 0.034 | 65546 | 1 | 0 / 0 / 0 |
| duplicate_view | setup | 3 | 0.003 | 0.003 | 0.003 | 0 | 0 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 5.309 | 6.181 | 6.181 | 68875 | 26 | 0 / 0 / 0 |
| insert | first | 3 | 0.105 | 0.115 | 0.115 | 65605 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.020 | 0.025 | 0.025 | 65605 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 2.354 | 2.367 | 2.367 | 589826 | 8193 | 0 / 0 / 0 |
| open | setup | 3 | 6.756 | 8.182 | 8.182 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.006 | 0.006 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 3.342 | 3.400 | 3.400 | 134477 | 91 | 0 / 0 / 0 |
| redraw | later | 9 | 3.099 | 3.991 | 3.991 | 134477 | 91 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.004 | 0.005 | 0.005 | 2 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.002 | 0.003 | 0.003 | 2 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.002 | 0.002 | 0.002 | 2 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.002 | 0.002 | 0.002 | 2 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.010 | 0.024 | 0.024 | 65538 | 1 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.008 | 0.025 | 0.025 | 65538 | 1 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.029 | 0.044 | 0.044 | 65538 | 1 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.010 | 0.108 | 0.108 | 65538 | 1 | 0 / 0 / 0 |
| undo | first | 3 | 0.084 | 0.086 | 0.086 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.007 | 0.015 | 0.015 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant2

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.013 | 0.013 | 0.013 | 65546 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.013 | 0.015 | 0.015 | 65546 | 1 | 0 / 0 / 0 |
| duplicate_view | setup | 3 | 0.001 | 0.001 | 0.001 | 0 | 0 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 5.020 | 5.530 | 5.530 | 68875 | 26 | 0 / 0 / 0 |
| insert | first | 3 | 0.032 | 0.033 | 0.033 | 65605 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.018 | 0.024 | 0.024 | 65605 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 2.222 | 2.424 | 2.424 | 524290 | 8192 | 0 / 0 / 0 |
| open | setup | 3 | 7.315 | 7.731 | 7.731 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.006 | 0.006 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 3.134 | 3.593 | 3.593 | 134477 | 91 | 0 / 0 / 0 |
| redraw | later | 9 | 3.157 | 4.634 | 4.634 | 134477 | 91 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.004 | 0.005 | 0.005 | 2 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.002 | 0.003 | 0.003 | 2 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.002 | 0.002 | 0.002 | 2 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.002 | 0.002 | 0.002 | 2 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.008 | 0.011 | 0.011 | 65538 | 1 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.008 | 0.011 | 0.011 | 65538 | 1 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.010 | 0.012 | 0.012 | 65538 | 1 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.011 | 0.024 | 0.024 | 65538 | 1 | 0 / 0 / 0 |
| undo | first | 3 | 0.073 | 0.086 | 0.086 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.008 | 0.019 | 0.019 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant3

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.103 | 0.104 | 0.104 | 65546 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.101 | 0.117 | 0.117 | 65546 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.229 | 4.292 | 4.292 | 1703 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.126 | 0.126 | 0.126 | 65605 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.100 | 0.177 | 0.177 | 65605 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 3.130 | 3.132 | 3.132 | 589826 | 8193 | 0 / 0 / 0 |
| open | setup | 3 | 6.123 | 6.547 | 6.547 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.005 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.004 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.361 | 2.397 | 2.397 | 1769 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.355 | 2.530 | 2.530 | 1769 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.001 | 0.001 | 0.001 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.067 | 0.069 | 0.069 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.012 | 0.015 | 0.015 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant4

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 8.239 | 8.742 | 8.742 | 65546 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 8.209 | 9.189 | 9.189 | 65546 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.069 | 4.384 | 4.384 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 52.193 | 56.077 | 56.077 | 65605 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 9.170 | 9.763 | 9.763 | 65605 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 2.357 | 2.371 | 2.371 | 589826 | 8193 | 0 / 0 / 0 |
| open | setup | 3 | 16.627 | 20.214 | 20.214 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 8.456 | 9.584 | 9.584 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 8.273 | 9.765 | 9.765 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.573 | 2.575 | 2.575 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.416 | 2.502 | 2.502 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 8.991 | 9.511 | 9.511 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 8.388 | 12.335 | 12.335 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant5

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 9.604 | 9.927 | 9.927 | 65546 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 8.577 | 11.070 | 11.070 | 65546 | 1 | 0 / 0 / 0 |
| duplicate_view | setup | 3 | 0.001 | 0.001 | 0.001 | 0 | 0 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 5.169 | 5.352 | 5.352 | 68878 | 26 | 0 / 0 / 0 |
| insert | first | 3 | 58.690 | 97.803 | 97.803 | 65605 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 8.750 | 10.241 | 10.241 | 65605 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 2.898 | 3.015 | 3.015 | 589826 | 8192 | 0 / 0 / 0 |
| open | setup | 3 | 20.433 | 23.691 | 23.691 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 9.800 | 12.988 | 12.988 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 8.763 | 12.287 | 12.287 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 3.935 | 4.885 | 4.885 | 134480 | 91 | 0 / 0 / 0 |
| redraw | later | 9 | 3.508 | 4.694 | 4.694 | 134480 | 91 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.013 | 0.014 | 0.014 | 2 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.006 | 0.023 | 0.023 | 2 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.006 | 0.011 | 0.011 | 2 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.005 | 0.006 | 0.006 | 2 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.108 | 0.136 | 0.136 | 65538 | 1 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.106 | 0.245 | 0.245 | 65538 | 1 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.127 | 0.138 | 0.138 | 65538 | 1 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.120 | 0.467 | 0.467 | 65538 | 1 | 0 / 0 / 0 |
| undo | first | 3 | 10.893 | 12.723 | 12.723 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 9.055 | 13.794 | 13.794 | 2 | 0 | 0 / 0 / 0 |

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
