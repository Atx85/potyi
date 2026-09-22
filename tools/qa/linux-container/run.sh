#!/usr/bin/env bash
set -euo pipefail

cargo test --locked --no-run
cargo test --locked startup:: -- --test-threads=1
SDL_VIDEODRIVER=dummy cargo test --locked folder_drop_ -- --include-ignored --test-threads=1

export XDG_RUNTIME_DIR
XDG_RUNTIME_DIR="$(mktemp -d /tmp/potyi-wayland.XXXXXX)"
export WAYLAND_DISPLAY=potyi-test
chmod 700 "$XDG_RUNTIME_DIR"
weston --backend=headless-backend.so --use-pixman --no-config \
    --socket="$WAYLAND_DISPLAY" --idle-time=0 \
    --log="$XDG_RUNTIME_DIR/weston.log" &
weston_pid=$!
cleanup() {
    cat "$XDG_RUNTIME_DIR/weston.log"
    kill "$weston_pid" 2>/dev/null || true
    wait "$weston_pid" 2>/dev/null || true
    rm -rf "$XDG_RUNTIME_DIR"
}
trap cleanup EXIT
for attempt in {1..100}; do
    if [[ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]]; then break; fi
    if ! kill -0 "$weston_pid" 2>/dev/null; then exit 1; fi
    sleep 0.1
done
test -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY"
# Older EGL libraries can retain thread-local cleanup callbacks after SDL
# unloads them. Keep EGL mapped until the Rust test process exits.
export LD_PRELOAD="libEGL.so.1${LD_PRELOAD:+:$LD_PRELOAD}"
SDL_VIDEODRIVER=wayland SDL_RENDER_DRIVER=software \
    timeout 120s dbus-run-session -- cargo test --locked wayland_window_startup_smoke \
    -- --include-ignored --test-threads=1 --nocapture
SDL_VIDEODRIVER=wayland SDL_RENDER_DRIVER=software \
    timeout 120s dbus-run-session -- cargo test --locked folder_drop_ \
    -- --include-ignored --test-threads=1 --nocapture
