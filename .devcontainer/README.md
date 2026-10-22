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
(`192.168.178.22:8888`) — on this host it
is `providers.unsloth.apiKey` in `~/.pi/agent/models.json`. The container gets
the provider definition from `.devcontainer/models.json` (same file, but with
`apiKey: "$UNSLOTH_API_KEY"`), so only the key itself lives in `.env`.

## Run pi interactively (from the repo root)

Credentials come from `.devcontainer/.env` (gitignored; create it once from
`.env.example`) via `--env-file`, and the provider definition from
`.devcontainer/models.json` — **do not mount the host's `~/.pi/agent`**:

```sh
podman run --rm -it \
  --network host \
  --env-file .devcontainer/.env \
  -v "$PWD:/workspace" \
  -v pi-agent-home:/root/.pi/agent \
  -v .devcontainer/models.json:/root/.pi/agent/models.json:ro \
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
- **Network:** the commands use `--network host` because on this host the
  rootless podman bridge cannot reach the Unsloth Studio server
  (`192.168.178.22:8888`) while the host network namespace can (verified).
  Trade-off: the container shares the host's network stack. The server accepts
  concurrent clients, so a container run does not block your host pi session.
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
