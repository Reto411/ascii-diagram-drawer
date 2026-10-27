# Isolated pi development environment for ascii-diagram-drawer.
# Containers built from this image should only ever be given this repo as a
# mount (see README.md) — that is the filesystem boundary for agent runs.
FROM debian:trixie-slim

ENV DEBIAN_FRONTEND=noninteractive

# Base tools + Node 24 (NodeSource) for the pi coding agent.
# gdb is required at runtime by cargo-tarpaulin.
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      bash ca-certificates curl gnupg git ripgrep \
      build-essential pkg-config gdb libssl-dev \
      python3 python3-pip python3-venv \
 && curl -fsSL https://deb.nodesource.com/setup_24.x | bash - \
 && apt-get install -y --no-install-recommends nodejs \
 && rm -rf /var/lib/apt/lists/*

# Rust stable via rustup (container-local toolchain; host ~/.cargo is not exposed)
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable

# pi coding agent (rebuild the image to update pi)
RUN npm install -g --ignore-scripts @earendil-works/pi-coding-agent

# Coverage tooling (quality gate: >=80% line coverage per AGENTS.md)
RUN cargo install cargo-tarpaulin

# Docs build check (mkdocs from the distro repo; PyPI is flaky under rootless podman)
RUN apt-get update \
 && apt-get install -y --no-install-recommends mkdocs \
 && rm -rf /var/lib/apt/lists/*

# Headless GUI smoke testing: virtual X display + screenshot capture/compare.
# GUI stories verify launch-alive checks and golden-screenshot pixel diffs
# under xvfb-run (the in-container model has no vision — see README).
RUN apt-get update \
 && apt-get install -y --no-install-recommends xvfb xauth imagemagick \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace
# ENTRYPOINT (not CMD) so args after the image name are passed to pi,
# e.g. `podman run ... pi-ascii --model ... -p "..."` (see README).
ENTRYPOINT ["pi"]
