#!/usr/bin/env bash
set -euo pipefail

script=$(cd "$(dirname "$0")" && pwd)/preflight-pr-size.sh
repo=$(mktemp -d)
trap 'rm -rf "$repo"' EXIT
git -C "$repo" init -q
git -C "$repo" config user.name Test
git -C "$repo" config user.email test@example.com
cd "$repo"
git commit -qm base --allow-empty
base=$(git rev-parse HEAD)
seq 200 > feature
git add feature
git commit -qm first
first=$(git rev-parse HEAD)
output=$(bash "$script" "$base" "$first")
[[ "$output" == *"changed-lines=200"* ]]
echo extra >> feature
git add feature
git commit -qm second
head=$(git rev-parse HEAD)
if bash "$script" "$base" "$head"; then exit 1; fi
output=$(bash "$script" "$first" "$head")
[[ "$output" == *"changed-lines=1"* ]]
if PR_MAX_LINES=1000 bash "$script" "$base" "$head"; then exit 1; fi
if bash "$script" missing "$head"; then exit 1; fi
seq 300 > Cargo.lock
git add Cargo.lock
git commit -qm lockfile
output=$(bash "$script" "$head" HEAD)
[[ "$output" == *"changed-lines=0"* ]]
head=$(git rev-parse HEAD)
printf '\0binary' > binary
git add binary
git commit -qm binary
if bash "$script" "$head" HEAD; then exit 1; fi
echo 'preflight PR size tests passed'
