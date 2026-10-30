#!/bin/sh
# next-id.sh — print the next bug ID (B-NNN) based on existing doc/bugs/B-*.md files.
#
#   sh .agents/skills/bug-report/scripts/next-id.sh   →   B-001
set -eu
cd "$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

n=0
for f in doc/bugs/B-*.md; do
  [ -e "$f" ] || continue
  num=$(basename "$f")
  num=${num#B-}
  num=${num%.md}
  num=$(printf '%s' "$num" | sed 's/^0*//')
  [ -n "$num" ] || num=0
  if [ "$num" -gt "$n" ]; then n=$num; fi
done
printf 'B-%03d\n' $((n + 1))
