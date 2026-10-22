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
cp .devcontainer/.env.example .devcontainer/.env && $EDITOR .devcontainer/.env
```

## Run pi interactively (from the repo root)

Credentials come from `.devcontainer/.env` (gitignored; create it once from
`.env.example`) via `--env-file` — **do not mount the host's `~/.pi/agent`**:

```sh
podman run --rm -it \
  --env-file .devcontainer/.env \
  -v "$PWD:/workspace" \
  -v pi-agent-home:/root/.pi/agent \
  pi-ascii
```

First run: approve project trust (or add `--approve` for non-interactive runs;
the decision persists in the volume's `trust.json`).

## Run the story-implementation agent headless

```sh
podman run --rm \
  --env-file .devcontainer/.env \
  -v "$PWD:/workspace" \
  -v pi-agent-home:/root/.pi/agent \
  pi-ascii -p "Use the story-implementation skill and work the story board until you run low on context."
```

Re-run as needed — `doc/stories/BOARD.md` makes every run resumable.

## Notes & troubleshooting

- **Credentials:** podman reads `.env` on the host (`--env-file`) and injects
  the values as env vars — the file itself never enters the container, and
  `podman run` fails fast if it is missing. `-e KEY=value` still works for
  one-off runs and overrides the file. If you want to reuse your host
  `auth.json`, copy it into the volume once (e.g. via a temporary container)
  instead of bind-mounting `~/.pi/agent`.
- **SELinux systems:** add `:Z` to the repo mount (`-v "$PWD:/workspace:Z"`) or
  run with `--security opt label:disable` if mounts are denied.
- **Updating pi:** rebuild the image (`podman build ...`).
- **New native deps** (e.g. a crate needing `libx-dev`): add to the apt list in
  the Dockerfile and rebuild — don't install into throwaway containers.
