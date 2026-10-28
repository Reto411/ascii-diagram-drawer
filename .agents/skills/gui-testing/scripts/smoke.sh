#!/bin/sh
# Headless GUI smoke helper for the gui-testing skill (see ../SKILL.md).
# Runs the *native* gui-app under xvfb — use inside the devcontainer image
# (ships xvfb + ImageMagick); not meant for host runs.
set -u

die() { printf 'smoke.sh: %s\n' "$*" >&2; exit 2; }

usage() {
  cat >&2 <<'EOF'
usage: smoke.sh <command>
  launch                          app must stay up for 10 s (timeout kill = PASS)
  capture <out.png> [idle_s]      build, run under xvfb, screenshot after idle_s (default 2)
  diff <shot.png> <golden.png>    pixel-exact compare (AE metric)
EOF
  exit 2
}

cmd="${1:-}"
[ $# -gt 0 ] && shift || true
case "$cmd" in
  launch | capture | diff) : ;;
  *) usage ;;
esac

BIN=./target/debug/gui-app

build() { cargo build -q -p gui-app || die "cargo build -p gui-app failed"; }

case "$cmd" in
  launch)
    build
    set +e
    timeout 10 xvfb-run -a "$BIN"
    rc=$?
    set -e
    if [ "$rc" -eq 124 ]; then
      echo "PASS launch-alive: app stayed up the full 10 s (killed by timeout)"
    else
      printf 'FAIL launch-alive: app exited early with code %s\n' "$rc" >&2
      exit 1
    fi
    ;;

  capture)
    out="${1:-}"
    [ -n "$out" ] || die "capture needs <out.png>"
    idle="${2:-2}"
    case "$idle" in '' | *[!0-9]*) die "idle_s must be a non-negative integer, got '$idle'";; esac
    build
    set +e
    # App must still be alive at capture time — a crashed app fails loudly
    # instead of producing a black-frame "pass".
    xvfb-run -a sh -c "$BIN & pid=\$!; sleep $idle; if kill -0 \$pid 2>/dev/null; then import -window root '$out'; else printf 'FAIL capture: app died before screenshot\n' >&2; exit 1; fi"
    rc=$?
    set -e
    [ "$rc" -eq 0 ] && [ -s "$out" ] || die "capture failed (xvfb/import exit $rc, or $out missing/empty)"
    echo "PASS capture: wrote $out (idle ${idle}s)"
    ;;

  diff)
    shot="${1:-}"
    golden="${2:-}"
    [ -n "$shot" ] && [ -n "$golden" ] || die "diff needs <shot.png> <golden.png>"
    [ -s "$shot" ] || die "shot not found or empty: $shot"
    [ -s "$golden" ] || die "golden not found or empty: $golden (generate it first — see SKILL.md)"
    set +e
    ae=$(compare -metric AE "$shot" "$golden" null: 2>&1)
    rc=$?
    set -e
    if [ "$rc" -eq 0 ]; then
      echo "PASS golden: pixel-identical ($ae)"
    elif [ "$rc" -eq 1 ]; then
      printf 'FAIL golden: %s pixels differ\n' "$ae" >&2
      exit 1
    else
      die "compare failed (exit $rc): $ae"
    fi
    ;;
esac
