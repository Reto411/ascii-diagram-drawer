#!/bin/sh
# One-shot start for the isolated pi environment.
#   1. builds/starts the egress proxy (idempotent — see setup-egress.sh)
#   2. launches an interactive pi agent in a fully isolated container
# No -p prompt → you get the TUI session (/exit to leave).
#
# Usage:  sh .devcontainer/start.sh
# Env overrides: PI_MODEL, PI_THINKING, PI_AGENT_HOME (volume), PI_IMAGE
set -eu
cd "$(dirname "$0")/.."

IMAGE="${PI_IMAGE:-pi-ascii}"
VOLUME="${PI_AGENT_HOME:-pi-agent-home}"
MODEL="${PI_MODEL:-unsloth/Qwen3.8-27B-GGUF:UD-Q4_K_XL}"
THINKING="${PI_THINKING:-medium}"

# --- preconditions -----------------------------------------------------------
if ! podman image exists "$IMAGE" 2>/dev/null; then
  echo "error: image '$IMAGE' not found — build it first:" >&2
  echo "  podman build -t $IMAGE -f .devcontainer/Dockerfile ." >&2
  exit 1
fi
if [ ! -f .devcontainer/.env ]; then
  echo "error: .devcontainer/.env missing — create it from .env.example (fill in UNSLOTH_API_KEY)" >&2
  exit 1
fi
podman volume exists "$VOLUME" 2>/dev/null || podman volume create "$VOLUME" >/dev/null

# --- 1. egress proxy ----------------------------------------------------------
sh .devcontainer/setup-egress.sh

# --- 2. pi agent (isolated, interactive) --------------------------------------
IT=
[ -t 0 ] && [ -t 1 ] && IT="-it"

exec podman run --rm $IT \
  --network slirp4netns:allow_host_loopback=true \
  --cap-add NET_ADMIN \
  -e ISOLATED_EGRESS=1 \
  -e http_proxy=http://10.0.2.2:3128 \
  -e https_proxy=http://10.0.2.2:3128 \
  -e HTTP_PROXY=http://10.0.2.2:3128 \
  -e HTTPS_PROXY=http://10.0.2.2:3128 \
  -e no_proxy=localhost,127.0.0.1,::1,10.0.2.2 \
  -e NO_PROXY=localhost,127.0.0.1,::1,10.0.2.2 \
  --env-file .devcontainer/.env \
  -v "$PWD:/workspace" \
  -v "$VOLUME":/root/.pi/agent \
  -v .devcontainer/models.json:/root/.pi/agent/models.json:ro \
  -v .devcontainer/settings.json:/root/.pi/agent/settings.json:ro \
  -v .devcontainer/pi-blackhole-config.json:/root/.pi/agent/pi-blackhole/pi-blackhole-config.json:ro \
  "$IMAGE" --model "$MODEL" --thinking "$THINKING"
