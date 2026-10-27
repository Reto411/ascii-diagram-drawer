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

## Build the image

```sh
podman build -t pi-ascii -f .devcontainer/Dockerfile .
```

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

Credentials come from `.devcontainer/.env` (gitignored; create it once from
`.env.example`) via `--env-file`, the provider definition from
`.devcontainer/models.json`, and the pi settings from
`.devcontainer/settings.json` — **do not mount the host's `~/.pi/agent`**:

```sh
podman run --rm -it \
  --network host \
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
  --network host \
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
  installed by pi into the volume on first container start — needs network
  access to git/npm, which `--network host` provides; later runs reuse the
  cached installs in the volume.
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
- **Network:** Unsloth Studio runs on this machine and listens on both
  `127.0.0.1:8888` and the LAN IP; `models.json` uses loopback so it keeps
  working if the LAN IP changes. Loopback only resolves to the host under
  `--network host` (with bridge networking it would be the container's own)
  — which is also required here because the rootless podman bridge cannot
  reach the server at all (verified). Trade-off of `--network host`: the
  container shares the host's network stack. The server accepts concurrent
  clients, so a container run does not block your host pi session.
  If models ever fail to appear, check with
  `podman run --rm --network host pi-ascii --list-models`.
- **Host auth.json:** if you ever want to reuse your host `auth.json` instead,
  copy it into the volume once (e.g. via a temporary container) rather than
  bind-mounting `~/.pi/agent`.
- **SELinux systems:** add `:Z` to the repo mount (`-v "$PWD:/workspace:Z"`) or
  run with `--security opt label:disable` if mounts are denied.
- **Updating pi:** rebuild the image (`podman build ...`).
- **New native deps** (e.g. a crate needing `libx-dev`): add to the apt list in
  the Dockerfile and rebuild — don't install into throwaway containers.
