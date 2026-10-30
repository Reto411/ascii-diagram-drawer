#!/bin/sh
# next-id.sh — print the next change-request ID (CR-NNN) based on existing doc/crs/CR-*.md files.
#
#   sh .agents/skills/change-request/scripts/next-id.sh   →   CR-001
set -eu
cd "$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

n=0
for f in doc/crs/CR-*.md; do
  [ -e "$f" ] || continue
  # File names are CR-NNN-<slug>.md — extract the numeric prefix only.
  num=$(basename "$f" .md | sed -n 's/^CR-\([0-9][0-9]*\).*/\1/p')
  num=$(printf '%s' "$num" | sed 's/^0*//')
  [ -n "$num" ] || continue
  if [ "$num" -gt "$n" ]; then n=$num; fi
done
printf 'CR-%03d\n' $((n + 1))
