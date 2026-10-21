#!/usr/bin/env bash
# No source file over 300 lines. Lives in a script, not inline in hook YAML:
# an unquoted YAML scalar ends at ": ", which silently disables every hook.
set -euo pipefail

MAX=300
status=0

if [ "$#" -eq 0 ]; then
  set -- $(git ls-files '*.rs')
fi

for f in "$@"; do
  [ -f "$f" ] || continue
  lines=$(wc -l < "$f" | tr -d ' ')
  if [ "$lines" -gt "$MAX" ]; then
    echo "ERROR: $f has $lines lines (max $MAX)"
    status=1
  fi
done

exit $status
