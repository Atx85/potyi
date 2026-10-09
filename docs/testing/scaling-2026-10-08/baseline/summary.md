# Sustained editing baseline

Recorded: 2026-10-08T14:26:41.395277+00:00.

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
| 104857600b-line64-pos50 | 0 | timed_out | editing | unavailable |
| 104857600b-line64-pos99 | 0 | timed_out | editing | unavailable |
| 104857600b-line1048576-pos0 | 3 | none | — | unavailable |
| 104857600b-line1048576-pos50 | 3 | none | — | unavailable |
| 104857600b-line1048576-pos99 | 3 | none | — | unavailable |
| 1000000000b-line64-pos0 | 3 | none | — | unavailable |
| 1000000000b-line64-pos50 | 0 | timed_out | navigation | unavailable |
| 1000000000b-line64-pos99 | 0 | timed_out | navigation | unavailable |
| 1000000000b-line1048576-pos0 | 3 | none | — | unavailable |
| 1000000000b-line1048576-pos50 | 0 | timed_out | editing | unavailable |
| 1000000000b-line1048576-pos99 | 0 | timed_out | editing | unavailable |
| 1mib-short-middle-variant1 | 3 | none | — | unavailable |
| 1mib-short-middle-variant2 | 3 | none | — | unavailable |
| 1mib-short-middle-variant3 | 3 | none | — | unavailable |
| 1mib-short-middle-variant4 | 3 | none | — | unavailable |
| 1mib-short-middle-variant5 | 0 | timed_out | editing | unavailable |

## 1048576b-line64-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.016 | 0.016 | 0.016 | 65539 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.014 | 0.021 | 0.021 | 65539 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.690 | 5.137 | 5.137 | 1640101 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 0.082 | 0.108 | 0.108 | 65602 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.016 | 0.027 | 0.027 | 65602 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 0.074 | 0.085 | 0.085 | 65536 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 142.862 | 159.283 | 159.283 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.001 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.501 | 2.554 | 2.554 | 1640167 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 2.423 | 5.091 | 5.091 | 1640167 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.054 | 0.056 | 0.056 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.003 | 0.006 | 0.006 | 1 | 0 | 0 / 0 / 0 |

## 1048576b-line64-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 36.608 | 42.083 | 42.083 | 536936458 | 8193 | 0 / 0 / 0 |
| backspace | later | 9 | 37.757 | 53.326 | 53.326 | 536936458 | 8193 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.600 | 5.995 | 5.995 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 36.520 | 42.328 | 42.328 | 536936517 | 8193 | 0 / 0 / 0 |
| insert | later | 9 | 39.326 | 49.559 | 49.559 | 536936517 | 8193 | 0 / 0 / 0 |
| navigation | setup | 3 | 41.251 | 42.430 | 42.430 | 536936450 | 8193 | 0 / 0 / 0 |
| open | setup | 3 | 40.378 | 112.883 | 112.883 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.495 | 2.832 | 2.832 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.873 | 6.077 | 6.077 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.003 | 0.003 | 0.003 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.065 | 0.067 | 0.067 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.006 | 0.007 | 0.007 | 2 | 0 | 0 / 0 / 0 |

## 1048576b-line64-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 77.259 | 86.250 | 86.250 | 1039364809 | 16221 | 0 / 0 / 0 |
| backspace | later | 9 | 71.553 | 140.541 | 140.541 | 1039364809 | 16221 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.346 | 10.621 | 10.621 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 77.200 | 80.848 | 80.848 | 1039365665 | 16221 | 0 / 0 / 0 |
| insert | later | 9 | 72.556 | 116.372 | 116.372 | 1039365665 | 16221 | 0 / 0 / 0 |
| navigation | setup | 3 | 80.159 | 89.987 | 89.987 | 1039364801 | 16221 | 0 / 0 / 0 |
| open | setup | 3 | 63.363 | 149.097 | 149.097 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.717 | 3.501 | 3.501 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.592 | 4.432 | 4.432 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.003 | 0.003 | 0.003 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.072 | 0.076 | 0.076 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.005 | 0.006 | 0.006 | 2 | 0 | 0 / 0 / 0 |

## 1048576b-line1048576-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 3.971 | 7.790 | 7.790 | 1048579 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 3.930 | 4.847 | 4.847 | 1048579 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 2.168 | 2.384 | 2.384 | 8192 | 1 | 0 / 0 / 0 |
| insert | first | 3 | 4.246 | 4.273 | 4.273 | 1052676 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 4.050 | 5.626 | 5.626 | 1052676 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 4.803 | 5.266 | 5.266 | 1048576 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 371.710 | 619.825 | 619.825 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.001 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 0.337 | 0.400 | 0.400 | 12288 | 1 | 0 / 0 / 0 |
| redraw | later | 9 | 0.270 | 0.437 | 0.437 | 12288 | 1 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.002 | 0.003 | 0.003 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.056 | 0.074 | 0.074 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.003 | 0.066 | 0.066 | 1 | 0 | 0 / 0 / 0 |

## 1048576b-line1048576-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 8.898 | 9.434 | 9.434 | 1572874 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 6.486 | 10.328 | 10.328 | 1572874 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 2.350 | 2.397 | 2.397 | 40960 | 1 | 0 / 0 / 0 |
| insert | first | 3 | 7.648 | 7.985 | 7.985 | 1576967 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 6.294 | 8.857 | 8.857 | 1576967 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 8.010 | 9.906 | 9.906 | 1572866 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 119.653 | 168.947 | 168.947 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.007 | 0.007 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.007 | 0.007 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.403 | 2.410 | 2.410 | 569344 | 1 | 0 / 0 / 0 |
| redraw | later | 9 | 2.228 | 5.759 | 5.759 | 569344 | 1 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.002 | 0.003 | 0.003 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.075 | 0.190 | 0.190 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.005 | 0.029 | 0.029 | 2 | 0 | 0 / 0 / 0 |

## 1048576b-line1048576-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 7.695 | 8.575 | 8.575 | 2088970 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 7.555 | 8.015 | 8.015 | 2088970 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 2.206 | 2.433 | 2.433 | 28672 | 1 | 0 / 0 / 0 |
| insert | first | 3 | 7.850 | 7.857 | 7.857 | 2088967 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 7.949 | 9.188 | 9.188 | 2088967 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 7.792 | 7.805 | 7.805 | 2088962 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 4.712 | 5.927 | 5.927 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 3.980 | 4.135 | 4.135 | 1069056 | 1 | 0 / 0 / 0 |
| redraw | later | 9 | 4.005 | 4.188 | 4.188 | 1069056 | 1 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.057 | 0.078 | 0.078 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.005 | 0.006 | 0.006 | 2 | 0 | 0 / 0 / 0 |

## 104857600b-line64-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.015 | 0.017 | 0.017 | 65539 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.013 | 0.021 | 0.021 | 65539 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.320 | 4.403 | 4.403 | 1640101 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 0.079 | 0.081 | 0.081 | 65602 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.017 | 0.029 | 0.029 | 65602 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 0.065 | 0.067 | 0.067 | 65536 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 214.049 | 276.391 | 276.391 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.001 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.436 | 2.589 | 2.589 | 1640167 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 2.387 | 2.708 | 2.708 | 1640167 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.001 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.050 | 0.056 | 0.056 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.003 | 0.006 | 0.006 | 1 | 0 | 0 / 0 / 0 |

## 104857600b-line1048576-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 3.738 | 3.851 | 3.851 | 1048579 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 3.726 | 4.014 | 4.014 | 1048579 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 99.433 | 103.247 | 103.247 | 26324992 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 4.069 | 4.155 | 4.155 | 1118211 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 4.034 | 4.172 | 4.172 | 1118211 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 4.089 | 4.351 | 4.351 | 1048576 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 20.627 | 26.040 | 26.040 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.002 | 0.003 | 0.003 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 103.189 | 103.402 | 103.402 | 26329088 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 103.522 | 105.638 | 105.638 | 26329088 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.079 | 0.088 | 0.088 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.005 | 0.008 | 0.008 | 1 | 0 | 0 / 0 / 0 |

## 104857600b-line1048576-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 188.968 | 189.171 | 189.171 | 53477386 | 51 | 0 / 0 / 0 |
| backspace | later | 9 | 186.919 | 189.840 | 189.840 | 53477386 | 51 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.052 | 4.451 | 4.451 | 110592 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 201.733 | 202.327 | 202.327 | 53547014 | 51 | 0 / 0 / 0 |
| insert | later | 9 | 201.820 | 204.217 | 204.217 | 53547014 | 51 | 0 / 0 / 0 |
| navigation | setup | 3 | 191.068 | 193.030 | 193.030 | 53477378 | 51 | 0 / 0 / 0 |
| open | setup | 3 | 14.357 | 16.786 | 16.786 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.735 | 2.936 | 2.936 | 114688 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.569 | 2.814 | 2.814 | 114688 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.002 | 0.003 | 0.003 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.070 | 0.074 | 0.074 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.004 | 0.005 | 0.005 | 2 | 0 | 0 / 0 / 0 |

## 104857600b-line1048576-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 375.130 | 499.403 | 499.403 | 104857610 | 100 | 0 / 0 / 0 |
| backspace | later | 9 | 377.099 | 510.409 | 510.409 | 104857610 | 100 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.265 | 4.539 | 4.539 | 110592 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 404.208 | 650.983 | 650.983 | 104861703 | 100 | 0 / 0 / 0 |
| insert | later | 9 | 405.759 | 547.113 | 547.113 | 104861703 | 100 | 0 / 0 / 0 |
| navigation | setup | 3 | 386.288 | 531.979 | 531.979 | 104857602 | 100 | 0 / 0 / 0 |
| open | setup | 3 | 15.992 | 227.030 | 227.030 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.004 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.708 | 3.386 | 3.386 | 114688 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.588 | 2.880 | 2.880 | 114688 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.003 | 0.004 | 0.004 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.069 | 0.087 | 0.087 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.005 | 0.005 | 0.005 | 2 | 0 | 0 / 0 / 0 |

## 1000000000b-line64-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.017 | 0.018 | 0.018 | 65539 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.014 | 0.022 | 0.022 | 65539 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.482 | 6.828 | 6.828 | 1640101 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 0.082 | 0.113 | 0.113 | 65602 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.018 | 0.030 | 0.030 | 65602 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 0.065 | 0.112 | 0.112 | 65536 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 1699.004 | 1736.088 | 1736.088 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.001 | 0.001 | 0.001 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.417 | 2.501 | 2.501 | 1640167 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 2.416 | 2.557 | 2.557 | 1640167 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.001 | 0.001 | 0.001 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.062 | 0.070 | 0.070 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.003 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |

## 1000000000b-line1048576-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 3.985 | 4.039 | 4.039 | 1048579 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 3.721 | 8.354 | 8.354 | 1048579 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 111.605 | 140.317 | 140.317 | 26324992 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 4.142 | 4.197 | 4.197 | 1118211 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 4.066 | 7.648 | 7.648 | 1118211 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 3.861 | 12.598 | 12.598 | 1048576 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 92.679 | 95.914 | 95.914 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.002 | 0.003 | 0.003 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.003 | 0.003 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 113.341 | 157.231 | 157.231 | 26329088 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 149.446 | 178.055 | 178.055 | 26329088 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.083 | 0.086 | 0.086 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.006 | 0.007 | 0.007 | 1 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant1

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 30.936 | 31.277 | 31.277 | 536936458 | 8193 | 0 / 0 / 0 |
| backspace | later | 9 | 31.097 | 31.816 | 31.816 | 536936458 | 8193 | 0 / 0 / 0 |
| duplicate_view | setup | 3 | 0.066 | 0.070 | 0.070 | 0 | 0 | 1 / 8193 / 0 |
| initial_draw | setup | 3 | 4.705 | 5.192 | 5.192 | 3339 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 32.670 | 34.069 | 34.069 | 536936517 | 8193 | 0 / 0 / 0 |
| insert | later | 9 | 32.084 | 32.370 | 32.370 | 536936517 | 8193 | 0 / 0 / 0 |
| navigation | setup | 3 | 31.344 | 33.787 | 33.787 | 536936450 | 8193 | 0 / 0 / 0 |
| open | setup | 3 | 21.200 | 38.674 | 38.674 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.009 | 0.011 | 0.011 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.005 | 0.007 | 0.007 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 3.382 | 3.537 | 3.537 | 3405 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 3.163 | 3.347 | 3.347 | 3405 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.138 | 0.150 | 0.150 | 2 | 0 | 3 / 8193 / 1 |
| synchronize | later | 9 | 0.084 | 0.149 | 0.149 | 2 | 0 | 3 / 8193 / 5 |
| synchronize_backspace | first | 3 | 0.191 | 0.226 | 0.226 | 2 | 0 | 2 / 8193 / 2 |
| synchronize_backspace | later | 9 | 0.031 | 0.097 | 0.097 | 2 | 0 | 2 / 8193 / 6 |
| synchronize_redo | first | 3 | 32.185 | 32.844 | 32.844 | 536936450 | 8193 | 3 / 0 / 1 |
| synchronize_redo | later | 9 | 32.103 | 32.877 | 32.877 | 536936450 | 8193 | 3 / 0 / 5 |
| synchronize_undo | first | 3 | 31.024 | 32.386 | 32.386 | 536936450 | 8193 | 2 / 0 / 1 |
| synchronize_undo | later | 9 | 31.068 | 31.288 | 31.288 | 536936450 | 8193 | 2 / 0 / 5 |
| undo | first | 3 | 0.067 | 0.068 | 0.068 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.006 | 0.008 | 0.008 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant2

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 31.149 | 31.935 | 31.935 | 536936458 | 8193 | 0 / 0 / 0 |
| backspace | later | 9 | 31.213 | 33.149 | 33.149 | 536936458 | 8193 | 0 / 0 / 0 |
| duplicate_view | setup | 3 | 0.351 | 0.381 | 0.381 | 0 | 0 | 2 / 8193 / 1000 |
| initial_draw | setup | 3 | 4.610 | 4.893 | 4.893 | 3339 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 32.160 | 32.466 | 32.466 | 536936517 | 8193 | 0 / 0 / 0 |
| insert | later | 9 | 32.177 | 32.474 | 32.474 | 536936517 | 8193 | 0 / 0 / 0 |
| navigation | setup | 3 | 30.662 | 35.318 | 35.318 | 536870914 | 8192 | 0 / 0 / 0 |
| open | setup | 3 | 15.304 | 20.469 | 20.469 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.014 | 0.015 | 0.015 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.005 | 0.007 | 0.007 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 3.047 | 3.171 | 3.171 | 3405 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 3.073 | 3.390 | 3.390 | 3405 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.463 | 0.524 | 0.524 | 2 | 0 | 4 / 8193 / 1001 |
| synchronize | later | 9 | 0.328 | 0.386 | 0.386 | 2 | 0 | 4 / 8193 / 1005 |
| synchronize_backspace | first | 3 | 0.352 | 0.358 | 0.358 | 2 | 0 | 3 / 8193 / 1002 |
| synchronize_backspace | later | 9 | 0.293 | 0.357 | 0.357 | 2 | 0 | 3 / 8193 / 1006 |
| synchronize_redo | first | 3 | 33.516 | 36.511 | 36.511 | 536936450 | 8193 | 4 / 0 / 1001 |
| synchronize_redo | later | 9 | 32.393 | 34.407 | 34.407 | 536936450 | 8193 | 4 / 0 / 1005 |
| synchronize_undo | first | 3 | 31.524 | 32.080 | 32.080 | 536936450 | 8193 | 3 / 0 / 1001 |
| synchronize_undo | later | 9 | 31.289 | 33.350 | 33.350 | 536936450 | 8193 | 3 / 0 / 1005 |
| undo | first | 3 | 0.041 | 0.066 | 0.066 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.005 | 0.008 | 0.008 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant3

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 726.062 | 860.801 | 860.801 | 536936458 | 8193 | 0 / 0 / 0 |
| backspace | later | 9 | 690.206 | 936.993 | 936.993 | 536936458 | 8193 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 5.747 | 17.155 | 17.155 | 1703 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 782.827 | 823.858 | 823.858 | 536936517 | 8193 | 0 / 0 / 0 |
| insert | later | 9 | 702.792 | 823.838 | 823.838 | 536936517 | 8193 | 0 / 0 / 0 |
| navigation | setup | 3 | 864.692 | 1028.965 | 1028.965 | 536936450 | 8193 | 0 / 0 / 0 |
| open | setup | 3 | 40.655 | 61.796 | 61.796 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.006 | 0.006 | 0.006 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.005 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.693 | 2.698 | 2.698 | 1769 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.574 | 3.063 | 3.063 | 1769 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.002 | 0.003 | 0.003 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.083 | 0.085 | 0.085 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.012 | 0.012 | 0.012 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant4

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 44.670 | 45.278 | 45.278 | 536936458 | 8193 | 0 / 0 / 0 |
| backspace | later | 9 | 51.865 | 77.236 | 77.236 | 536936458 | 8193 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.409 | 4.959 | 4.959 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 366.355 | 399.835 | 399.835 | 536936517 | 8193 | 0 / 0 / 0 |
| insert | later | 9 | 49.243 | 75.550 | 75.550 | 536936517 | 8193 | 0 / 0 / 0 |
| navigation | setup | 3 | 36.453 | 39.238 | 39.238 | 536936450 | 8193 | 0 / 0 / 0 |
| open | setup | 3 | 63.532 | 245.906 | 245.906 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 8.606 | 9.577 | 9.577 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 9.659 | 12.421 | 12.421 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 3.035 | 4.265 | 4.265 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.456 | 2.985 | 2.985 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.003 | 0.003 | 0.003 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 11.028 | 12.114 | 12.114 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 9.516 | 12.820 | 12.820 | 2 | 0 | 0 / 0 / 0 |

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
