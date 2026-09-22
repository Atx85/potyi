#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" != --wayland ]]; then
    cat /etc/fedora-release
    kwin_wayland --version
    rustc --version
    cargo test --locked --no-run
    cargo test --locked startup:: -- --test-threads=1
    SDL_VIDEODRIVER=dummy cargo test --locked folder_drop_ -- --include-ignored --test-threads=1
    exec dbus-run-session -- bash "$0" --wayland
fi

export XDG_RUNTIME_DIR
XDG_RUNTIME_DIR="$(mktemp -d /tmp/potyi-kwin.XXXXXX)"
chmod 700 "$XDG_RUNTIME_DIR"
export WAYLAND_DISPLAY=potyi-test
export XDG_SESSION_TYPE=wayland
export XDG_CURRENT_DESKTOP=KDE
export LIBGL_ALWAYS_SOFTWARE=1
kwin_wayland --virtual --width 1024 --height 768 --no-lockscreen \
    --socket "$WAYLAND_DISPLAY" > "$XDG_RUNTIME_DIR/kwin.log" 2>&1 &
kwin_pid=$!
cleanup() {
    cat "$XDG_RUNTIME_DIR/kwin.log"
    kill "$kwin_pid" 2>/dev/null || true
    wait "$kwin_pid" 2>/dev/null || true
    rm -rf "$XDG_RUNTIME_DIR"
}
trap cleanup EXIT
for attempt in {1..200}; do
    if [[ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]]; then break; fi
    if ! kill -0 "$kwin_pid" 2>/dev/null; then exit 1; fi
    sleep 0.1
done
test -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY"
export SDL_VIDEODRIVER=wayland
export SDL_RENDER_DRIVER=software
timeout 120s cargo test --locked wayland_window_startup_smoke \
    -- --include-ignored --test-threads=1 --nocapture
timeout 120s cargo test --locked folder_drop_ \
    -- --include-ignored --test-threads=1 --nocapture
