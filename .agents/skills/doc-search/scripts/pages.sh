#!/bin/sh
# pages.sh — search the project's documentation pages (MkDocs site under doc/).
#
#   sh .agents/skills/doc-search/scripts/pages.sh index [term]
#       One line per page from its YAML frontmatter:
#       path :: title / status / description. With a term, only pages whose
#       frontmatter matches (case-insensitive) are printed.
#   sh .agents/skills/doc-search/scripts/pages.sh find <ripgrep args...>
#       Full-text search across all pages (ripgrep, case-insensitive), first
#       80 matching lines. Extra flags pass through, e.g. -C 2 for context.
set -eu
cd "$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

mode=${1:-index}
case "$mode" in
  index)
    shift || true
    term=${1:-}
    find doc -name '*.md' -type f | sort | while IFS= read -r f; do
      head -n 20 "$f" | awk -v f="$f" '
        /^---$/ { c++; next }
        c==1 && /^(title|status|description):/ { line = (line == "" ? $0 : line " / " $0) }
        END { if (line != "") print f " :: " line }
      '
    done | { [ -n "$term" ] && rg -i --no-heading "$term" || cat; } | head -n 120
    ;;
  find)
    shift
    # $@ first: ripgrep takes the first non-option arg as the pattern, paths after.
    rg -in --no-heading -g '*.md' "$@" doc | head -n 80
    ;;
  *)
    echo "usage: pages.sh index [term] | find <ripgrep args>" >&2
    exit 2
    ;;
esac
