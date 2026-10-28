#!/usr/bin/env sh
# Host-side setup for egress isolation of the pi devcontainer.
# Builds the Squid proxy image and (re)starts it on the host network stack,
# bound to 127.0.0.1:3128 only. Idempotent: safe to re-run at any time.
# See README.md ("Egress isolation") for the full architecture.
set -eu

cd "$(dirname "$0")"

PROXY_NAME=egress-proxy
PROXY_IMAGE=pi-egress-proxy

podman build -q -t "$PROXY_IMAGE" proxy/

podman rm -f "$PROXY_NAME" >/dev/null 2>&1 || true
podman run -d --name "$PROXY_NAME" --network host "$PROXY_IMAGE"

echo "egress proxy running at 127.0.0.1:3128 (container: $PROXY_NAME)"
echo "smoke test: curl -x http://127.0.0.1:3128 -sI https://crates.io"
