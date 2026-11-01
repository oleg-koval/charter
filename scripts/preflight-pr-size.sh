#!/usr/bin/env bash
# Check C7 against the exact proposed PR base and head before creating it.
set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "usage: $0 <base-commit> <head-commit>" >&2
  exit 2
fi

base=$(git rev-parse --verify --end-of-options "$1^{commit}")
head=$(git rev-parse --verify --end-of-options "$2^{commit}")
merge_base=$(git merge-base "$base" "$head")
stats=$(git diff --numstat "$merge_base" "$head" -- . ':(exclude)Cargo.lock')

if printf '%s\n' "$stats" | awk -F '\t' '$1 == "-" || $2 == "-" { found = 1 } END { exit !found }'; then
  echo "ERROR: binary changes cannot be measured; review them explicitly before opening a PR." >&2
  exit 1
fi

changed=$(printf '%s\n' "$stats" | awk -F '\t' '{ total += $1 + $2 } END { print total + 0 }')
echo "base=$base head=$head merge-base=$merge_base changed-lines=$changed excluded=Cargo.lock limit=200"
if [ "$changed" -gt 200 ]; then
  echo "ERROR: split the PR or raise the C7 conflict for an explicit decision." >&2
  exit 1
fi
