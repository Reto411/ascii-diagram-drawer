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

# Egress isolation tooling (see entrypoint.sh + README.md "Egress isolation").
# Own layer so the heavy layers above stay cached.
RUN apt-get update \
 && apt-get install -y --no-install-recommends iptables iproute2 \
 && rm -rf /var/lib/apt/lists/*

# Rust stable via rustup (container-local toolchain; host ~/.cargo is not exposed)
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable

# pi coding agent (rebuild the image to update pi)
RUN npm install -g --ignore-scripts @earendil-works/pi-coding-agent

# Bake pi's extension packages into the image. Isolated runs cannot reach
# github.com / the npm registry (egress whitelist — ADR 014), so install them
# at build time (full network available) and let entrypoint.sh copy them into
# a fresh agent-home volume on first run. Rebuild when settings.json's
# package list changes.
COPY .devcontainer/settings.json /tmp/pi-seed-settings.json
RUN export HOME=/opt/pi-build-home \
 && mkdir -p "$HOME/.pi/agent" \
 && cp /tmp/pi-seed-settings.json "$HOME/.pi/agent/settings.json" \
 && (pi --list-models || true) 2>/dev/null \
 && test -d "$HOME/.pi/agent/git/github.com/arcanemachine/pi-web-search" \
 && test -d "$HOME/.pi/agent/npm/node_modules/pi-blackhole" \
 && mkdir -p /opt/pi-seed \
 && mv "$HOME/.pi/agent/git" "$HOME/.pi/agent/npm" /opt/pi-seed/ \
 && rm -rf "$HOME" /tmp/pi-seed-settings.json

# Coverage tooling (quality gate: >=80% line coverage per AGENTS.md)
RUN cargo install cargo-tarpaulin

# WASM tooling for S-019 (web milestone) — isolated runs cannot install tools
# at runtime (egress allow-list, ADR 014), so bake them in:
#   wasm32 target        container rustup/cargo state is ephemeral (no volume),
#                        so the target must come from the image
#   trunk                builds the servable web bundle of gui-app (AC1)
#   wasm-bindgen-cli     pinned to the exact version Cargo.lock resolves for
#                        eframe 0.37 (wasm-bindgen 0.2.129) — a mismatched CLI
#                        fails when processing the built .wasm. This package
#                        also ships wasm-bindgen-test-runner, which runs the
#                        headless crates' test suites on wasm32 (AC2).
#                        KEEP THE PIN IN SYNC with Cargo.lock's wasm-bindgen
#                        version when eframe/wasm deps move — rebuild after.
RUN rustup target add wasm32-unknown-unknown \
 && cargo install trunk \
 && cargo install wasm-bindgen-cli --version 0.2.129

# Docs build check (mkdocs from the distro repo; PyPI is flaky under rootless podman)
RUN apt-get update \
 && apt-get install -y --no-install-recommends mkdocs \
 && rm -rf /var/lib/apt/lists/*

# Headless GUI smoke testing: virtual X display + screenshot capture/compare.
# GUI stories verify launch-alive checks and golden-screenshot pixel diffs
# under xvfb-run (the in-container model has no vision — see README).
# winit's X11 backend dlopens these client libraries at runtime (x11-dl);
# libgl1/libxrandr2 come in as xvfb dependencies, the rest must be listed here.
RUN apt-get update \
 && apt-get install -y --no-install-recommends xvfb xauth imagemagick \
    libxcursor1 libxi6 libxkbcommon0 \
 && rm -rf /var/lib/apt/lists/*

# libcap2-bin (capsh): the entrypoint drops CAP_NET_ADMIN after applying the
# egress firewall, so the agent cannot remove its own isolation. Own final
# layer to keep the cached layers above intact.
RUN apt-get update \
 && apt-get install -y --no-install-recommends libcap2-bin \
 && rm -rf /var/lib/apt/lists/*

COPY .devcontainer/entrypoint.sh /usr/local/bin/isolated-entrypoint.sh
RUN chmod +x /usr/local/bin/isolated-entrypoint.sh

WORKDIR /workspace
# ENTRYPOINT (not CMD) so args after the image name are passed to pi,
# e.g. `podman run ... pi-ascii --model ... -p "..."` (see README).
# The entrypoint applies the egress firewall when ISOLATED_EGRESS=1.
ENTRYPOINT ["/usr/local/bin/isolated-entrypoint.sh"]
