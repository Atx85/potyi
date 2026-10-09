# Pötyi adapter changes

Vendored from vt100 0.16.2 (MIT; see LICENSE). The adapter adds an optional
normal-screen row sink and row encoding/wrap restoration so the experimental
terminal can keep scrollback in files. Upstream behaviour is unchanged when no
sink is installed. Session parsers use zero in-memory scrollback; the alternate
screen is never written to history. The adapter also bounds oversized insert/scroll counts to screen dimensions,
keeps line insertion/deletion within the scrolling region,
preserves the history sink across resets, and uses vte's fixed 1 KiB OSC buffer.
Other parser behaviour remains upstream.
