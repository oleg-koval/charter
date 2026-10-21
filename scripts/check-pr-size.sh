#!/usr/bin/env bash
# C7: one concern per PR, under about 200 changed lines. Lockfiles do not count.
set -euo pipefail

BASE="${1:-${CHARTER_BASE:-origin/main}}"
MAX="${PR_MAX_LINES:-200}"

changed=$(git diff --numstat "$BASE"...HEAD -- . ':(exclude)Cargo.lock' \
  | awk '$1 != "-" { total += $1 + $2 } END { print total + 0 }')

if [ "$changed" -gt "$MAX" ]; then
  echo "ERROR: $changed changed lines against $BASE (max $MAX). Split the PR (C7, C33)."
  exit 1
fi
echo "ok: $changed changed lines against $BASE (max $MAX)"
