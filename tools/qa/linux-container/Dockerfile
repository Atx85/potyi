FROM rust:bookworm

RUN apt-get update && apt-get install -y --no-install-recommends \
    cmake ninja-build pkg-config \
    libasound2-dev libpulse-dev \
    libx11-dev libxext-dev libxrandr-dev libxcursor-dev \
    libxfixes-dev libxi-dev libxss-dev libxtst-dev \
    libxkbcommon-dev libwayland-dev libdecor-0-dev \
    libgl1-mesa-dev libegl1-mesa-dev libdbus-1-dev libudev-dev \
    weston dbus-x11 \
    && rm -rf /var/lib/apt/lists/*

ENV CMAKE_POLICY_VERSION_MINIMUM=3.5
ENV CARGO_TARGET_DIR=/build
WORKDIR /workspace
CMD ["bash", "tools/qa/linux-container/run.sh"]
