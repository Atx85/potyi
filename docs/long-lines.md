# Long-line rendering and navigation

The editor reads a horizontal window of text for each visible row. Off-screen
text is not rasterized. Cursor display, mouse hit testing, selections, and search
highlights use the same bounded reads and tab/Unicode column mapping.

Vertical movement and scrolling reuse cached line boundaries and lengths. A
bounded cache stores sparse byte, character, and visual-column checkpoints,
including the current window's start. It retains no line contents: up to 128
line/tab-width entries and 32 sparse checkpoints per entry (roughly 110 KiB of
position metadata at capacity on a 64-bit build). Reads use 4 KiB blocks. Entries
are invalidated when document text changes, and older entries are evicted.
There is no background scanning or idle polling added by this change.

Syntax matching retains its existing 16 KiB prefix limit. It is intersected with
the visible window, preserving syntax context near the beginning of a line
without allocating an entire very long line.

## Local before-and-after measurement

Fixture: three lines of 1,048,576 ASCII characters each, with LF endings.
Optimized build; 800×600 headless SDL window; 18-point font.

| Workload | Before (one sample) | After (median of seven) |
| --- | ---: | ---: |
| First text frame | 526.559 ms | 0.844 ms |
| Ten up/down movements with text redraws | 5,243.018 ms | 2.051 ms |
| Initial line indexing, measured separately | 10.949 ms | 11.928 ms |

The after measurements exclude one warmup. Fixture generation and font setup
are outside timers. Text rendering includes rasterization, but not the desktop
compositor, full application startup, selection painting, or title-bar drawing.
This is a targeted regression probe, not a CPU-percentage measurement or a
promise about other machines. [Raw samples](long-line-results.json).

First-time line discovery still scans text to find line endings and count
characters. Editing invalidates the line index; a jump into an uncached distant
part of a line may also require a scan. The bounded viewport does not eliminate
those costs. The earlier 1 GB opening benchmark uses short lines and does not
measure this workload.

## Reproduce

```sh
SDL_VIDEODRIVER=dummy cargo test --release --locked long_line_navigation_probe -- --ignored --nocapture --test-threads=1
```

Additional regression checks cover Unicode at 4 KiB/64 KiB block boundaries,
tabs, CRLF, edit invalidation, bounded cache storage and repeated read volume,
vertical movement, scrolled pixel output, and mouse-to-cursor mapping.
