# Sustained editing baseline

Recorded: 2026-10-08T15:25:02.777499+00:00.

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
| 1mib-short-middle-variant1 | 3 | none | — | unavailable |
| 1mib-short-middle-variant2 | 3 | none | — | unavailable |
| 1mib-short-middle-variant3 | 3 | none | — | unavailable |
| 1mib-short-middle-variant4 | 3 | none | — | unavailable |
| 1mib-short-middle-variant5 | 3 | none | — | unavailable |

## 1048576b-line64-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.015 | 0.018 | 0.018 | 65539 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.013 | 0.074 | 0.074 | 65539 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.545 | 5.223 | 5.223 | 1701 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 0.101 | 0.106 | 0.106 | 65602 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.022 | 0.046 | 0.046 | 65602 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 0.075 | 0.075 | 0.075 | 65536 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 11.043 | 12.470 | 12.470 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.021 | 0.021 | 0.021 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.683 | 2.684 | 2.684 | 1767 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 2.410 | 2.808 | 2.808 | 1767 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.057 | 0.075 | 0.075 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.006 | 0.009 | 0.009 | 1 | 0 | 0 / 0 / 0 |

## 1048576b-line64-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.018 | 0.019 | 0.019 | 65546 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.014 | 0.019 | 0.019 | 65546 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.239 | 4.277 | 4.277 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.098 | 0.100 | 0.100 | 65605 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.021 | 0.027 | 0.027 | 65605 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 2.135 | 2.167 | 2.167 | 589826 | 8193 | 0 / 0 / 0 |
| open | setup | 3 | 10.936 | 16.962 | 16.962 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.021 | 0.021 | 0.021 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.301 | 2.320 | 2.320 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.229 | 2.551 | 2.551 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.047 | 0.056 | 0.056 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.004 | 0.006 | 0.006 | 2 | 0 | 0 / 0 / 0 |

## 1048576b-line64-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.017 | 0.019 | 0.019 | 10569 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.013 | 0.023 | 0.023 | 10569 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.203 | 4.444 | 4.444 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.099 | 0.147 | 0.147 | 10566 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.016 | 0.022 | 0.022 | 10566 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 4.553 | 4.986 | 4.986 | 1048641 | 16221 | 0 / 0 / 0 |
| open | setup | 3 | 7.368 | 12.936 | 12.936 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.024 | 0.025 | 0.025 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.505 | 2.781 | 2.781 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.291 | 2.439 | 2.439 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.064 | 0.098 | 0.098 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.004 | 0.006 | 0.006 | 2 | 0 | 0 / 0 / 0 |

## 1048576b-line1048576-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 3.706 | 3.781 | 3.781 | 1048579 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 3.731 | 3.823 | 3.823 | 1048579 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 2.029 | 2.123 | 2.123 | 8192 | 1 | 0 / 0 / 0 |
| insert | first | 3 | 4.010 | 4.327 | 4.327 | 1052676 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 3.759 | 3.878 | 3.878 | 1052676 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 3.929 | 3.963 | 3.963 | 1048576 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 3.944 | 4.850 | 4.850 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.024 | 0.025 | 0.025 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 0.365 | 0.421 | 0.421 | 12288 | 1 | 0 / 0 / 0 |
| redraw | later | 9 | 0.310 | 0.358 | 0.358 | 12288 | 1 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.065 | 0.074 | 0.074 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.004 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |

## 1048576b-line1048576-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 5.865 | 5.883 | 5.883 | 1572874 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 5.828 | 7.057 | 7.057 | 1572874 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 2.247 | 2.379 | 2.379 | 40960 | 1 | 0 / 0 / 0 |
| insert | first | 3 | 5.869 | 6.210 | 6.210 | 1576967 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 5.794 | 6.021 | 6.021 | 1576967 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 6.101 | 6.170 | 6.170 | 1572866 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 5.141 | 5.939 | 5.939 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.023 | 0.023 | 0.023 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.347 | 2.423 | 2.423 | 569344 | 1 | 0 / 0 / 0 |
| redraw | later | 9 | 2.324 | 2.535 | 2.535 | 569344 | 1 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.062 | 0.071 | 0.071 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.006 | 0.008 | 0.008 | 2 | 0 | 0 / 0 / 0 |

## 1048576b-line1048576-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 7.796 | 7.825 | 7.825 | 2088970 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 7.793 | 7.896 | 7.896 | 2088970 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 1.993 | 2.067 | 2.067 | 28672 | 1 | 0 / 0 / 0 |
| insert | first | 3 | 7.763 | 7.940 | 7.940 | 2088967 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 7.796 | 8.037 | 8.037 | 2088967 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 7.984 | 8.255 | 8.255 | 2088962 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 3.730 | 3.974 | 3.974 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.023 | 0.023 | 0.023 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 4.180 | 4.181 | 4.181 | 1069056 | 1 | 0 / 0 / 0 |
| redraw | later | 9 | 4.173 | 4.260 | 4.260 | 1069056 | 1 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.053 | 0.066 | 0.066 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.005 | 0.009 | 0.009 | 2 | 0 | 0 / 0 / 0 |

## 104857600b-line64-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.014 | 0.016 | 0.016 | 65539 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.014 | 0.019 | 0.019 | 65539 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.376 | 4.558 | 4.558 | 1701 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 0.086 | 0.091 | 0.091 | 65602 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.019 | 0.023 | 0.023 | 65602 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 0.064 | 0.064 | 0.064 | 65536 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 198.990 | 372.126 | 372.126 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.019 | 0.023 | 0.023 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.002 | 0.002 | 0.002 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.355 | 2.365 | 2.365 | 1767 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 2.304 | 2.564 | 2.564 | 1767 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.054 | 0.065 | 0.065 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.006 | 0.008 | 0.008 | 1 | 0 | 0 / 0 / 0 |

## 104857600b-line64-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.018 | 0.023 | 0.023 | 65546 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.015 | 0.039 | 0.039 | 65546 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.246 | 4.851 | 4.851 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.085 | 0.098 | 0.098 | 65605 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.018 | 0.027 | 0.027 | 65605 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 209.811 | 224.627 | 224.627 | 52494338 | 819201 | 0 / 0 / 0 |
| open | setup | 3 | 190.709 | 191.497 | 191.497 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.022 | 0.028 | 0.028 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.051 | 0.051 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.323 | 2.425 | 2.425 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.371 | 3.207 | 3.207 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.059 | 0.069 | 0.069 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.006 | 0.085 | 0.085 | 2 | 0 | 0 / 0 / 0 |

## 104857600b-line64-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.021 | 0.034 | 0.034 | 65546 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.018 | 0.063 | 0.063 | 65546 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.806 | 5.461 | 5.461 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.107 | 0.110 | 0.110 | 65605 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.025 | 0.074 | 0.074 | 65605 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 464.826 | 486.658 | 486.658 | 103874562 | 1622017 | 0 / 0 / 0 |
| open | setup | 3 | 215.046 | 367.914 | 367.914 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.024 | 0.033 | 0.033 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.705 | 2.930 | 2.930 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.605 | 4.881 | 4.881 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.064 | 0.071 | 0.071 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.007 | 0.010 | 0.010 | 2 | 0 | 0 / 0 / 0 |

## 104857600b-line1048576-pos0

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 3.719 | 3.796 | 3.796 | 1048579 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 3.743 | 35.794 | 35.794 | 1048579 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 101.617 | 104.793 | 104.793 | 26324992 | 25 | 0 / 0 / 0 |
| insert | first | 3 | 3.865 | 3.870 | 3.870 | 1118211 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 3.704 | 5.554 | 5.554 | 1118211 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 3.903 | 3.994 | 3.994 | 1048576 | 1 | 0 / 0 / 0 |
| open | setup | 3 | 10.106 | 215.853 | 215.853 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.021 | 0.022 | 0.022 | 0 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.003 | 0.003 | 0 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 99.441 | 120.359 | 120.359 | 26329113 | 25 | 0 / 0 / 0 |
| redraw | later | 9 | 134.316 | 188.350 | 188.350 | 26329113 | 25 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.077 | 0.101 | 0.101 | 1 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.009 | 0.010 | 0.010 | 1 | 0 | 0 / 0 / 0 |

## 104857600b-line1048576-pos50

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 3.741 | 3.881 | 3.881 | 1048586 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 3.769 | 3.879 | 3.879 | 1048586 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.658 | 4.689 | 4.689 | 110592 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 3.774 | 3.891 | 3.891 | 1118214 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 3.685 | 3.901 | 3.901 | 1118214 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 194.805 | 195.527 | 195.527 | 53477378 | 51 | 0 / 0 / 0 |
| open | setup | 3 | 4.155 | 14.638 | 14.638 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.022 | 0.023 | 0.023 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.582 | 2.601 | 2.601 | 114688 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.639 | 3.355 | 3.355 | 114688 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.064 | 0.065 | 0.065 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.006 | 0.013 | 0.013 | 2 | 0 | 0 / 0 / 0 |

## 104857600b-line1048576-pos99

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 3.783 | 4.347 | 4.347 | 1048586 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 3.733 | 3.784 | 3.784 | 1048586 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.802 | 4.931 | 4.931 | 110592 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 3.954 | 4.510 | 4.510 | 1052679 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 3.804 | 3.933 | 3.933 | 1052679 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 389.342 | 413.214 | 413.214 | 104857602 | 100 | 0 / 0 / 0 |
| open | setup | 3 | 9.339 | 79.230 | 79.230 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.026 | 0.026 | 0.026 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.004 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 3.076 | 3.655 | 3.655 | 114688 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.562 | 4.074 | 4.074 | 114688 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.072 | 0.075 | 0.075 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.007 | 0.008 | 0.008 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant1

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.016 | 0.017 | 0.017 | 65546 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.012 | 0.019 | 0.019 | 65546 | 1 | 0 / 0 / 0 |
| duplicate_view | setup | 3 | 0.003 | 0.003 | 0.003 | 0 | 0 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 5.180 | 5.415 | 5.415 | 3339 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.118 | 0.210 | 0.210 | 65605 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.018 | 0.022 | 0.022 | 65605 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 2.145 | 2.595 | 2.595 | 589826 | 8193 | 0 / 0 / 0 |
| open | setup | 3 | 19.647 | 28.399 | 28.399 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.025 | 0.026 | 0.026 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.004 | 0.004 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 3.338 | 3.373 | 3.373 | 3405 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 3.046 | 3.348 | 3.348 | 3405 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.003 | 0.004 | 0.004 | 2 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.002 | 0.003 | 0.003 | 2 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.002 | 0.002 | 0.002 | 2 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.002 | 0.002 | 0.002 | 2 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.028 | 0.029 | 0.029 | 65538 | 1 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.008 | 0.027 | 0.027 | 65538 | 1 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.027 | 0.030 | 0.030 | 65538 | 1 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.010 | 0.032 | 0.032 | 65538 | 1 | 0 / 0 / 0 |
| undo | first | 3 | 0.077 | 0.083 | 0.083 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.007 | 0.012 | 0.012 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant2

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.015 | 0.018 | 0.018 | 65546 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.012 | 0.014 | 0.014 | 65546 | 1 | 0 / 0 / 0 |
| duplicate_view | setup | 3 | 0.001 | 0.001 | 0.001 | 0 | 0 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 5.213 | 5.275 | 5.275 | 3339 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.036 | 0.047 | 0.047 | 65605 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.019 | 0.026 | 0.026 | 65605 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 2.162 | 2.557 | 2.557 | 524290 | 8192 | 0 / 0 / 0 |
| open | setup | 3 | 5.702 | 6.630 | 6.630 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.023 | 0.023 | 0.023 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.003 | 0.003 | 0.003 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 3.181 | 3.257 | 3.257 | 3405 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 3.100 | 3.347 | 3.347 | 3405 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.003 | 0.003 | 0.003 | 2 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.002 | 0.003 | 0.003 | 2 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.002 | 0.003 | 0.003 | 2 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.002 | 0.002 | 0.002 | 2 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.009 | 0.009 | 0.009 | 65538 | 1 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.009 | 0.027 | 0.027 | 65538 | 1 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.012 | 0.017 | 0.017 | 65538 | 1 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.010 | 0.028 | 0.028 | 65538 | 1 | 0 / 0 / 0 |
| undo | first | 3 | 0.061 | 0.066 | 0.066 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.006 | 0.008 | 0.008 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant3

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 0.104 | 0.107 | 0.107 | 65546 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 0.102 | 0.122 | 0.122 | 65546 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.278 | 4.497 | 4.497 | 1703 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 0.116 | 0.121 | 0.121 | 65605 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 0.105 | 0.149 | 0.149 | 65605 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 2.736 | 2.942 | 2.942 | 589826 | 8193 | 0 / 0 / 0 |
| open | setup | 3 | 8.289 | 25.499 | 25.499 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 0.024 | 0.027 | 0.027 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 0.004 | 0.005 | 0.005 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.369 | 2.659 | 2.659 | 1769 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.306 | 2.658 | 2.658 | 1769 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 0.063 | 0.063 | 0.063 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 0.010 | 0.032 | 0.032 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant4

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 9.186 | 9.630 | 9.630 | 65546 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 8.293 | 12.924 | 12.924 | 65546 | 1 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 4.183 | 4.221 | 4.221 | 1701 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 44.302 | 57.929 | 57.929 | 65605 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 8.481 | 9.500 | 9.500 | 65605 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 2.179 | 2.313 | 2.313 | 589826 | 8193 | 0 / 0 / 0 |
| open | setup | 3 | 11.961 | 16.388 | 16.388 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 9.462 | 9.591 | 9.591 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 8.453 | 9.396 | 9.396 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 2.482 | 2.486 | 2.486 | 1767 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 2.443 | 2.586 | 2.586 | 1767 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.000 | 0.000 | 0.000 | 0 | 0 | 0 / 0 / 0 |
| undo | first | 3 | 8.589 | 9.001 | 9.001 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 8.721 | 8.960 | 8.960 | 2 | 0 | 0 / 0 / 0 |

## 1mib-short-middle-variant5

| Operation | Bucket | n | Median | p95 | p99 | Read bytes, median | Discoveries, median | Copied records (pieces / lines / history), median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| backspace | first | 3 | 8.440 | 8.673 | 8.673 | 65546 | 1 | 0 / 0 / 0 |
| backspace | later | 9 | 8.461 | 13.117 | 13.117 | 65546 | 1 | 0 / 0 / 0 |
| duplicate_view | setup | 3 | 0.001 | 0.001 | 0.001 | 0 | 0 | 0 / 0 / 0 |
| initial_draw | setup | 3 | 5.224 | 5.271 | 5.271 | 3342 | 0 | 0 / 0 / 0 |
| insert | first | 3 | 52.431 | 65.268 | 65.268 | 65605 | 1 | 0 / 0 / 0 |
| insert | later | 9 | 8.155 | 10.667 | 10.667 | 65605 | 1 | 0 / 0 / 0 |
| navigation | setup | 3 | 2.832 | 2.907 | 2.907 | 589826 | 8192 | 0 / 0 / 0 |
| open | setup | 3 | 15.713 | 15.966 | 15.966 | 0 | 0 | 0 / 0 / 0 |
| redo | first | 3 | 9.045 | 9.227 | 9.227 | 1 | 0 | 0 / 0 / 0 |
| redo | later | 9 | 9.137 | 10.101 | 10.101 | 1 | 0 | 0 / 0 / 0 |
| redraw | first | 3 | 3.311 | 3.369 | 3.369 | 3408 | 0 | 0 / 0 / 0 |
| redraw | later | 9 | 3.247 | 3.405 | 3.405 | 3408 | 0 | 0 / 0 / 0 |
| synchronize | first | 3 | 0.005 | 0.006 | 0.006 | 2 | 0 | 0 / 0 / 0 |
| synchronize | later | 9 | 0.006 | 0.007 | 0.007 | 2 | 0 | 0 / 0 / 0 |
| synchronize_backspace | first | 3 | 0.006 | 0.006 | 0.006 | 2 | 0 | 0 / 0 / 0 |
| synchronize_backspace | later | 9 | 0.005 | 0.012 | 0.012 | 2 | 0 | 0 / 0 / 0 |
| synchronize_redo | first | 3 | 0.121 | 0.155 | 0.155 | 65538 | 1 | 0 / 0 / 0 |
| synchronize_redo | later | 9 | 0.103 | 0.117 | 0.117 | 65538 | 1 | 0 / 0 / 0 |
| synchronize_undo | first | 3 | 0.104 | 0.123 | 0.123 | 65538 | 1 | 0 / 0 / 0 |
| synchronize_undo | later | 9 | 0.109 | 0.126 | 0.126 | 65538 | 1 | 0 / 0 / 0 |
| undo | first | 3 | 8.368 | 8.965 | 8.965 | 2 | 0 | 0 / 0 / 0 |
| undo | later | 9 | 8.931 | 12.273 | 12.273 | 2 | 0 | 0 / 0 / 0 |

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
- Interrupted full matrix: sandbox denied a process-group timeout signal during 1 GB fixture/open preparation. The five shared/history/fragmentation variants were subsequently completed against the same saved executable; remaining 1 GB cases were not collected for this intermediate stage.
- Interrupted full matrix: sandbox denied a process-group timeout signal during 1 GB fixture/open preparation. The five shared/history/fragmentation variants were subsequently completed against the same saved executable; remaining 1 GB cases were not collected for this intermediate stage.
