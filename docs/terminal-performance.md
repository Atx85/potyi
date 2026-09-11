# Terminal performance

The terminal favors bounded memory and low CPU use. Output history remains file-backed; rendering does not keep another full copy in RAM.

- Incoming process output uses a bounded 512 KiB raw-byte queue. Readers block when it is full, and combine wake notifications instead of posting one UI event for every chunk.
- Each UI turn consumes at most 16 chunks (256 KiB of raw output), stopping earlier after approximately 2 ms of decoding. It appends the combined text once. Input events are handled between batches.
- Streaming redraws are limited to approximately 60 per second. Keyboard and mouse actions can redraw immediately. A frame deadline is active only when output needs displaying.
- Quiet terminals use the editor's one-second event wait, including while a command is running silently. Output and directory workers wake that wait when results arrive. There is no continuous idle redraw timer. The one-second fallback also checks child completion after its output streams have closed.
- Rendered text is reused with an 8 MiB budget for estimated RGBA pixel storage, text keys, and entry structures, plus a 512-entry limit. SDL/driver allocation overhead is additional. The cache is released on return to the editor and invalidated when font size or display scale changes.
- Appending rewraps the final visual row. Whole-line history trimming rebases retained row offsets instead of rewrapping history. Resizing, font changes, clearing, or trimming inside an unfinished logical line require a full reflow.

Selection, Unicode, wrapped links, grep locations, and file-opening safeguards are preserved.

## Reproducible probe

From the repository root:

```sh
SHELL=/bin/sh SDL_VIDEODRIVER=dummy cargo test --locked terminal_performance_probe -- --ignored --nocapture --test-threads=1
```

The probe uses an 800×600 headless SDL window and the development profile (`opt-level = 1`). It measures elapsed time, checks texture reuse and eviction, and verifies cache invalidation/release. It has no timing pass/fail threshold.

On the same local machine, before and after these changes:

| Workload | Before | After |
| --- | ---: | ---: |
| 120 unchanged terminal redraws | 244.3 ms | 61.5 ms |
| Append/layout 2,060,800 bytes without newlines in 128 chunks | 4,371.5 ms | 71.7 ms |

These are targeted headless measurements, not a comparison against a standalone terminal, an idle CPU measurement, or a guarantee for every command or display driver. Repeated redraws in the probe deliberately exercise drawing; an idle application does not perform them.

The full regression suite includes scheduling, bounded batch consumption, worker wakeups, incremental wrapping, history trimming, cache eviction, selection, and clickable search results:

```sh
SHELL=/bin/sh SDL_VIDEODRIVER=dummy cargo test --locked -- --include-ignored --test-threads=1
```

## Command startup latency

On macOS/Linux, commands now use the selected shell with `-c` rather than `-lc`. The application environment and PATH are inherited, and shell syntax is preserved. Login profiles are no longer rerun for every submission. A user can explicitly run a login shell for a command that needs it, such as `zsh -lc 'your-command'`.

The following probe measures submission through process startup, streamed output, and its first rendered frame, using the real selected shell:

```sh
SHELL=/bin/zsh SDL_VIDEODRIVER=dummy cargo test --locked terminal_command_latency_probe -- --ignored --nocapture --test-threads=1
```

On the same local macOS setup, with zsh login-profile configuration in place:

| Command | Before: first drawn output | After: first drawn output |
| --- | ---: | ---: |
| `printf terminal-ready` | 4,454.8 ms | 17.8 ms |
| `grep -n 'fn main' src/main.rs` | 3,694.5 ms | 29.8 ms |
| `printf 'ready\n' \| grep -n ready` | 4,000.9 ms | 40.5 ms |

These are local headless measurements; shell configuration, command workload, and machine affect results. The regression test separately verifies that login profiles are skipped while ordinary shell initialization, environment variables, PATH lookup, quoting, and pipes still work. This change introduces no resident shell, additional buffers, background polling, or idle redraw activity. Windows already uses `cmd /D /S /C` to avoid its AutoRun startup commands.
