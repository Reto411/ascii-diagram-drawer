---
title: 014 - egress-isolation-squid-proxy
description: agent devcontainer has no direct internet — netns firewall restricts it to a loopback-bound Squid allow-list proxy (crates.io, docs.rs, rust-lang.org, rustup) plus the local model server
scope: tooling
status: proposed
version: 4
updated: 2026-10-28
---

# ADR 014: Egress isolation via netns firewall + Squid allow-list proxy

## Context

The pi agent runs in the `.devcontainer/` image. Previously it ran with
`--network host`, i.e. **full internet egress plus full access to the host's
loopback services**. A 2026-10-27 attempt to move off host networking
(slirp4netns with `allow_host_loopback`) was reverted by the user: it fixed
model-server reachability but provided no isolation.

Goal: the agent may reach only what its workflow needs — crates.io (sparse
index, downloads, API), rustup endpoints, and the local Unsloth model server
(`127.0.0.1:8888`, loopback-only binding by user policy). Everything else is
denied at the network level, not by convention.

Constraints found empirically on this host (podman 5.4.2, **netavark** backend):

- netavark has no `slirp4netns` driver for *named* networks
  (`podman network create -d slirp4netns` → "failed to find driver or plugin").
- Mode networks (`--network host`, `--network slirp4netns`) **cannot be
  combined** with named networks ("cannot set multiple networks without bridge
  network mode").
- Containers on CNI bridges cannot reach the host's loopback-bound services.

These rule out the textbook design (agent on an internal podman network, all
traffic — including model — through a Squid container that has its own egress
interface): no single container can hold both a shared-network interface and
host-loopback/slirp reachability.

## Decision

Two enforcement layers:

1. **Netns firewall in the agent container.** `entrypoint.sh` (new image
   entrypoint) applies, when `ISOLATED_EGRESS=1`, `iptables`/`ip6tables`
   OUTPUT rules that REJECT everything except `10.0.2.2:3128` and
   `10.0.2.2:8888` (plus local loopback). Fails closed: if the rules cannot be
   applied, the container exits instead of starting unisolated. Requires
   `--cap-add NET_ADMIN` — granted only to install the rules; the entrypoint
   then drops it again via `capsh` before exec-ing pi, so the agent (root in
   the container) cannot modify netfilter and cannot remove its own isolation
   (verified: `iptables` ops from the reduced process tree fail with EPERM,
   while the rules persist in the kernel's netns).
2. **Squid allow-list proxy** (`egress-proxy` container, built from
   `.devcontainer/proxy/`, runs with `--network host`, bound to
   `127.0.0.1:3128` only). Allowed destinations: `crates.io` + subdomains,
   `docs.rs` (generated crate API docs — rustdoc HTML is not hosted on
   crates.io itself), `rust-lang.org` + subdomains (incl. `doc.rust-lang.org`:
   std/core/alloc API docs, The Rust Book, Rust Reference), `rustup.rs`,
   `sh.rustup.rs`; all else 403.

The agent container runs with `--network slirp4netns:allow_host_loopback=true`,
so `10.0.2.2` aliases the **host's loopback**: it reaches Squid at
`10.0.2.2:3128` and the model server directly at `10.0.2.2:8888`. Model
traffic bypasses the proxy on purpose (`models.json` baseUrl is
`http://10.0.2.2:8888/v1`, `10.0.2.2` is in `no_proxy`; pi's undici
`EnvHttpProxyAgent` honors both). All other HTTP(S) clients (cargo, rustup,
curl, git) are pointed at the proxy via standard `http(s)_proxy` env vars.

Host-side setup is idempotent: `.devcontainer/setup-egress.sh`.

## Alternatives considered

| Alternative | Why rejected |
|-------------|--------------|
| Pure Squid design (agent on internal podman net, all traffic incl. model proxied) | Impossible here: no named slirp network under netavark; mode+named networks rejected → proxy container could not reach both internet and host-loopback model server |
| Keep `--network host`, rely on Squid env vars only | No enforcement — any tool ignoring proxy env vars (or a future one) gets full egress; loopback access stays unfiltered |
| iptables-only, no proxy | Would need to whitelist IP addresses for crates.io/rustup (CDN IPs change); domain allow-listing is the right granularity and Squid logs every decision |
| Bind Unsloth beyond `127.0.0.1` so a bridge network could reach it | Rejected by user policy (no LAN/0.0.0.0 binding) |
| tinyproxy / 3proxy instead of Squid | No CONNECT support (tinyproxy) → no HTTPS; Squid is the standard, logs well, config is auditable in-repo |

## Consequences

- **Positive:** hard isolation with a single audited choke point; every
  external request is logged by Squid (allow/deny + host); model server stays
  loopback-only and is additionally filtered to port 8888 at the firewall;
  fails closed on firewall errors; setup is one idempotent script.
- **Negative / costs:** `--cap-add NET_ADMIN` on the agent container (scoped to
  its own netns, rootless; dropped again before pi starts — see Decision);
  extra container + image to maintain; github.com and
  the npm registry are not whitelisted — pi's own extension packages are
  therefore baked into the image at build time (`/opt/pi-seed`) and seeded
  into a fresh agent-home volume on first run (package-list changes need an
  image rebuild); in-container web tools get 403s.
- **Follow-ups:** flip `status` to `accepted` after user review; if new pi
  packages or git-based crate deps appear, extend the whitelist deliberately;
  consider a host systemd unit for `egress-proxy` so it survives reboots
  (currently started via `setup-egress.sh`).

## Version history

- **v4 (2026-10-28):** whitelist extended with `docs.rs` — the canonical host
  for generated third-party crate API docs (rustdoc HTML), which crates.io
  itself does not serve. Motivation: the isolated agent's reference material
  should include dependency API docs, not just sources. Verified through the
  live proxy after re-running `setup-egress.sh`; all previously allowed hosts
  still allow, non-whitelisted hosts (e.g. google.com) still get Squid 403.
- **v3 (2026-10-28):** closes the self-removal hole — after applying the
  firewall rules, the entrypoint drops `CAP_NET_ADMIN` from pi's process tree
  (`capsh --drop=cap_net_admin`, new `libcap2-bin` image dependency), so a
  prompt-injected agent cannot flush its own netns firewall. Verified in a
  live isolated run (CapEff without cap_net_admin; model call unaffected).
- **v2 (2026-10-28):** pi extension packages are now baked into the agent
  image at build time and seeded into a fresh agent-home volume by
  `entrypoint.sh` — runtime github/npm fetches are no longer needed or
  possible. Adds `.devcontainer/start.sh` (one-shot: egress proxy +
  interactive isolated pi run). Decision unchanged.
- **v1 (2026-10-28):** initial decision. Implemented and verified same day
  (allow/deny matrix, direct-egress rejection, model chat under full
  isolation, fresh-crate cargo build through the proxy). Supersedes in
  practice the reverted 2026-10-27 slirp-only networking change (no ADR had
  been recorded for it).
