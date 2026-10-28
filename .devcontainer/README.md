# .devcontainer — isolated pi environment

This folder defines a **filesystem boundary** for running pi on this project:
containers built from `Dockerfile` only ever receive **this repo**, mounted at
`/workspace`. Nothing else from the host is visible (no `~`, no other projects,
no host credentials). Per pi's security docs, the working folder alone is *not*
a boundary — this container is.

## What's in the image

- Node 24 + `@earendil-works/pi-coding-agent` (`pi`)
- Rust stable (rustup) + `cargo-tarpaulin` (+ `gdb`, which tarpaulin needs)
- `git`, `ripgrep`, build essentials
- `mkdocs` (default theme) for docs build checks
- `xvfb` + ImageMagick (`import`/`compare`) for headless GUI smoke tests
- `iptables`/`iproute2` + `entrypoint.sh` — egress isolation gate (see "Egress isolation")
- pi extension packages (`pi-web-search`, `pi-lsp-extension`, `pi-blackhole`
  + deps) baked into `/opt/pi-seed` at build time and auto-seeded into a
  fresh agent-home volume on first run — no github/npm access needed at
  runtime (rebuild the image when `settings.json`'s package list changes)

## Build the image

```sh
podman build -t pi-ascii -f .devcontainer/Dockerfile .
```

Run from the **repo root** — the `.` context is required (the Dockerfile
copies `.devcontainer/settings.json`). The build itself needs **unrestricted
internet access**: it downloads Node, Rust, and pi, and bakes pi's extension
packages in from github.com/npm. That is deliberate — the image is what makes
the *runs* isolated, not the build.

One-time setup:

```sh
podman volume create pi-agent-home          # persists pi sessions/settings/trust between runs
cp .devcontainer/.env.example .devcontainer/.env && $EDITOR .devcontainer/.env   # fill in UNSLOTH_API_KEY
```

`UNSLOTH_API_KEY` is the key of the local Unsloth Studio server
(`127.0.0.1:8888`) — on this host it
is `providers.unsloth.apiKey` in `~/.pi/agent/models.json`. The container gets
the provider definition from `.devcontainer/models.json` (same file, but with
`apiKey: "$UNSLOTH_API_KEY"`), so only the key itself lives in `.env`.
The pi settings come from `.devcontainer/settings.json` — a repo-local mirror
of the host's `~/.pi/agent/settings.json` (packages, defaults, compaction).
Keep it in sync when your host config changes; never put secrets in it
(keys belong in `.env`).

## Run pi interactively (from the repo root)

One-shot start — egress proxy + fully isolated interactive pi session
(TUI, no `-p` prompt; `/exit` to leave):

```sh
sh .devcontainer/start.sh
```

Env overrides: `PI_MODEL`, `PI_THINKING`, `PI_AGENT_HOME` (volume name),
`PI_IMAGE`. The script starts the egress proxy idempotently
(`setup-egress.sh`) and then runs exactly the command below — shown for
reference and one-off tweaks.

Credentials come from `.devcontainer/.env` (gitignored; create it once from
`.env.example`) via `--env-file`, the provider definition from
`.devcontainer/models.json`, and the pi settings from
`.devcontainer/settings.json` — **do not mount the host's `~/.pi/agent`**:

```sh
podman run --rm -it \
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
  -v pi-agent-home:/root/.pi/agent \
  -v .devcontainer/models.json:/root/.pi/agent/models.json:ro \
  -v .devcontainer/settings.json:/root/.pi/agent/settings.json:ro \
  -v .devcontainer/pi-blackhole-config.json:/root/.pi/agent/pi-blackhole/pi-blackhole-config.json:ro \
  pi-ascii --model "unsloth/Qwen3.8-27B-GGUF:UD-Q4_K_XL" --thinking medium
```

First run: approve project trust (or add `--approve` for non-interactive runs;
the decision persists in the volume's `trust.json`).

## Run the story-implementation agent headless

```sh
podman run --rm \
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
  -v pi-agent-home:/root/.pi/agent \
  -v .devcontainer/models.json:/root/.pi/agent/models.json:ro \
  -v .devcontainer/settings.json:/root/.pi/agent/settings.json:ro \
  -v .devcontainer/pi-blackhole-config.json:/root/.pi/agent/pi-blackhole/pi-blackhole-config.json:ro \
  pi-ascii --model "unsloth/Qwen3.8-27B-GGUF:UD-Q4_K_XL" --thinking medium \
    -p "Use the story-implementation skill and work the story board until you run low on context."
```

Re-run as needed — `doc/stories/BOARD.md` makes every run resumable.

The flag set above is **required**, not optional: it is what makes the run
isolated (see below). VS Code "Attach to running container" / devcontainer
attach is not a supported way to get an isolated run — start containers with
the commands above.

## Egress isolation (Squid allow-list + netns firewall)

The agent container has **no direct internet access**. Two layers enforce it:

1. **Netns firewall** (`entrypoint.sh`, active when `ISOLATED_EGRESS=1`):
   inside the container, `iptables`/`ip6tables` REJECT every outgoing packet
   except to `10.0.2.2:3128` (the egress proxy) and `10.0.2.2:8888` (the
   model server). Fails closed — if the rules cannot be applied, the container
   does not start. Requires `--cap-add NET_ADMIN` — but only to install the
   rules: the entrypoint then **drops `CAP_NET_ADMIN` again** (via `capsh`)
   before starting pi, so no process inside the container can modify the
   firewall (netfilter ops fail with EPERM) and the agent cannot remove its
   own isolation. Only an operator `podman exec`-ing from the host keeps that
   ability.
2. **Squid allow-list** (container `egress-proxy`, runs with `--network host`,
   bound to `127.0.0.1:3128` only): all HTTP/HTTPS traffic from the agent must
   pass through it; it serves only `crates.io` + subdomains, `rust-lang.org`
   + subdomains, `rustup.rs`, `sh.rustup.rs`. Everything else gets a 403.

How the pieces connect — the container runs with
`--network slirp4netns:allow_host_loopback=true`, so `10.0.2.2` is an alias
for the **host's loopback**:

```
agent container ──(iptables: only these)──> 10.0.2.2:3128 = host 127.0.0.1:3128 (Squid)
                                              └──> crates.io / rustup hosts only
                              └────────────────> 10.0.2.2:8888 = host 127.0.0.1:8888 (Unsloth, direct)
```

- **Model traffic is not proxied.** `models.json` uses
  `baseUrl: http://10.0.2.2:8888/v1` and `10.0.2.2` is in `no_proxy`, so pi's
  HTTP client (undici `EnvHttpProxyAgent`) talks to Unsloth directly; all
  other HTTP(S) traffic (cargo, rustup, curl, git) goes through the proxy via
  the `http(s)_proxy` env vars above.
- **Why not a pure proxy design** (agent on an internal podman network,
  everything incl. model through Squid): podman here uses the netavark backend,
  which has no slirp4netns driver for named networks, and mode networks
  (`host`/`slirp4netns`) cannot be combined with named ones — so a proxy
  container could never reach both the internet *and* the host-loopback model
  server. See [ADR 014](../doc/adr/014-egress-isolation-squid-proxy.md).
- **Setup** (host side, idempotent): `sh .devcontainer/setup-egress.sh`
  builds the proxy image and (re)starts `egress-proxy`. Re-run after any
  `squid.conf` change or host reboot.
- **Extending the whitelist:** edit `.devcontainer/proxy/squid.conf`, then
  re-run `setup-egress.sh`. Squid 6 quirk: a bare `dstdomain` name matches the
  apex only, a leading dot matches subdomains only — both forms are needed and
  must live in separate ACLs (see the comment in the file).
- **Not covered by design:** github.com and the npm registry are *not* on
  the whitelist. That does not affect pi itself: its extension packages are
  baked into the image at build time and seeded into a fresh agent-home
  volume on first run (see "What's in the image") — resetting the volume is
  safe, and package-list changes need an image rebuild, not a runtime fetch.
  In-container web tools get 403s — that is the point.
- **Troubleshooting:** `podman logs egress-proxy` shows Squid's access log
  (every allow/deny with target host); inside the container,
  `iptables -S OUTPUT` shows the lockdown rules. If the model endpoint is
  unreachable, check Unsloth on the host first:
  `curl http://127.0.0.1:8888/v1/models`.
- **Verified 2026-10-28:** allow/deny matrix (crates.io API + index + static,
  rust-lang.org, rustup.rs allowed; google/pypi/github denied), direct egress
  rejected, other host-loopback ports rejected, real model chat completion
  under full isolation, fresh-crate `cargo build` through the proxy.

## Notes & troubleshooting

- **Credentials:** podman reads `.env` on the host (`--env-file`) and injects
  the values as env vars — the file itself never enters the container, and
  `podman run` fails fast if it is missing. `-e KEY=value` still works for
  one-off runs and overrides the file.
- **Provider config:** `.devcontainer/models.json` is bind-mounted read-only
  over the volume's path (the later mount wins) and references
  `$UNSLOTH_API_KEY`, which pi interpolates from the environment. Update both
  files together when the model or endpoint changes; if the key rotates, only
  `.env` needs updating.
- **Pi settings:** `.devcontainer/settings.json` is bind-mounted read-only
  over the volume's path and mirrors the host's `~/.pi/agent/settings.json`.
  Re-copy it from the host when your config changes (keep `defaultProvider`/
  `defaultModel` consistent with `models.json`). The listed `packages` are
  installed by pi into the volume on first container start; later runs reuse
  the cached installs. Note: github/npm are *not* on the egress whitelist, so
  package (re)fetches only work if the packages are already in the volume —
  see "Egress isolation" above.
- **pi-blackhole:** one of the packages above. It replaces Pi's LLM-based
  `/compact` with zero-cost algorithmic compaction. The config is bind-mounted
  read-only from `.devcontainer/pi-blackhole-config.json` (same pattern as
  `models.json`/`settings.json`) and currently pins
  `compaction: "auto"`, `compactionEngine: "blackhole"`, and
  `memory: false` — i.e. algorithmic compaction only, with the three
  background observational-memory workers (Observer/Reflector/Dropper)
  disabled, so no extra model calls happen. To change it, edit the repo file
  (a read-only mount also means `/blackhole settings` cannot save from inside
  the container). Full key reference: `docs/CONFIG.md` in the pi-blackhole
  repo.
- **Headless GUI smokes:** the image ships `xvfb` + ImageMagick so GUI stories
  can verify without a display, in three tiers:
  - *launch-alive:* `timeout 10 xvfb-run -a cargo run -p gui-app` — exit 124
    (killed by timeout) means the app started and stayed up; any other early
    exit is a crash.
  - *golden-screenshot diff:* capture with `xvfb-run -a sh -c 'cargo run -p
    gui-app & sleep N; import -window root shot.png'`, then
    `compare -metric AE shot.png golden.png null` (exit 0 = pixel-identical).
    Generate the golden once inside the same image version — pixel-exact
    matches are only stable within one image build.
  - *perceptual check (needs a vision model — CPU-bound, use sparingly):*
    only when launch-alive and pixel diff cannot answer the AC. When the
    session model supports images (`"images": true` in `models.json`),
    downscale first — `magick shot.png -resize 320x shot-low.png` — then
    `read` the low-res PNG: pi attaches it to the context and the model judges
    semantic properties ("box drawn at clicked position", "label legible").
    Keep images small and batch assertions into one screenshot; every vision
    call is slow on CPU. Save the screenshot as an artifact cited from the
    story's Verification section. Status 2026-10-27:
    the Unsloth endpoint accepts `image_url` content parts, but the loaded
    GGUF is not vision-capable (HTTP 400 "does not support vision") — load a
    Qwen VL variant in Unsloth Studio and flip `images: true` to activate.
- **Network / model endpoint:** see the [Egress isolation](#egress-isolation-squid-allow-list--netns-firewall)
  section. Short version: `models.json` uses `http://10.0.2.2:8888/v1`
  because under `slirp4netns:allow_host_loopback=true`, `10.0.2.2` is the
  host's loopback — the only way a container can reach the Unsloth server,
  which binds to `127.0.0.1` only (no LAN/0.0.0.0 binding, by policy). The
  server accepts concurrent clients, so a container run does not block your
  host pi session. If models ever fail to appear, check with
  `podman run --rm --network slirp4netns:allow_host_loopback=true \
   -e no_proxy=10.0.2.2 -v .devcontainer/models.json:/root/.pi/agent/models.json:ro pi-ascii --list-models`
- **Host auth.json:** if you ever want to reuse your host `auth.json` instead,
  copy it into the volume once (e.g. via a temporary container) rather than
  bind-mounting `~/.pi/agent`.
- **SELinux systems:** add `:Z` to the repo mount (`-v "$PWD:/workspace:Z"`) or
  run with `--security opt label:disable` if mounts are denied.
- **Updating pi:** rebuild the image (`podman build ...`).
- **New native deps** (e.g. a crate needing `libx-dev`): add to the apt list in
  the Dockerfile and rebuild — don't install into throwaway containers.
