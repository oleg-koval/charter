#!/bin/sh
set -eu

fail() {
  printf 'charter installer: %s\n' "$1" >&2
  exit 1
}

command -v curl >/dev/null 2>&1 || fail "curl is required"
command -v tar >/dev/null 2>&1 || fail "tar is required"

os=$(uname -s)
arch=$(uname -m)
case "$os:$arch" in
  Linux:x86_64|Linux:amd64) target=x86_64-unknown-linux-musl ;;
  Linux:aarch64|Linux:arm64) target=aarch64-unknown-linux-musl ;;
  Darwin:x86_64) target=x86_64-apple-darwin ;;
  Darwin:arm64|Darwin:aarch64) target=aarch64-apple-darwin ;;
  *) fail "unsupported platform: $os $arch" ;;
esac

if command -v sha256sum >/dev/null 2>&1; then
  checksum_command=sha256sum
elif command -v shasum >/dev/null 2>&1; then
  checksum_command=shasum
else
  fail "sha256sum or shasum is required"
fi

version=${CHARTER_VERSION:-latest}
asset="charter-$target.tar.gz"
checksum_asset="charter-$target.sha256"
if [ "$version" = latest ]; then
  latest_url=$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/oleg-koval/charter/releases/latest") || fail "could not resolve the latest release"
  case "$latest_url" in
    */releases/tag/*) version=${latest_url##*/} ;;
    *) fail "no published release found" ;;
  esac
fi
case "$version" in
  v*) tag=$version ;;
  *) tag="v$version" ;;
esac
case "$tag" in
  *[!A-Za-z0-9._-]*) fail "invalid version: $version" ;;
esac
release_url="https://github.com/oleg-koval/charter/releases/download/$tag"

install_dir=${CHARTER_INSTALL_DIR:-"$HOME/.local/bin"}
tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/charter-install.XXXXXX") || fail "could not create temporary directory"
staged="$install_dir/.charter.new.$$"
cleanup() {
  rm -f "$tmp_dir/$asset" "$tmp_dir/$checksum_asset" "$staged"
  rmdir "$tmp_dir" 2>/dev/null || :
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM

curl -fsSL "$release_url/$asset" -o "$tmp_dir/$asset" || fail "could not download $asset"
curl -fsSL "$release_url/$checksum_asset" -o "$tmp_dir/$checksum_asset" || fail "could not download $checksum_asset"

expected=$(awk 'NR == 1 { print $1; exit }' "$tmp_dir/$checksum_asset")
case "$expected" in
  *[!0-9a-fA-F]*|'') fail "invalid SHA-256 checksum file" ;;
esac
[ "${#expected}" -eq 64 ] || fail "invalid SHA-256 checksum file"
if [ "$checksum_command" = shasum ]; then
  actual=$(shasum -a 256 "$tmp_dir/$asset" | awk '{ print $1 }')
else
  actual=$(sha256sum "$tmp_dir/$asset" | awk '{ print $1 }')
fi
[ "$actual" = "$expected" ] || fail "SHA-256 checksum mismatch for $asset"

mkdir -p "$install_dir" || fail "could not create install directory: $install_dir"
# Stage inside the install dir so the final mv is an atomic same-filesystem rename owned by this user.
tar -xzOf "$tmp_dir/$asset" charter > "$staged" || fail "could not extract charter binary"
[ -s "$staged" ] || fail "archive does not contain charter"
chmod 755 "$staged" || fail "could not set executable permissions"
mv -f "$staged" "$install_dir/charter" || fail "could not install charter to $install_dir"
printf 'Installed charter %s at %s/charter\n' "$tag" "$install_dir"
case ":$PATH:" in
  *":$install_dir:"*) ;;
  *) printf 'Add %s to your PATH to run charter.\n' "$install_dir" >&2 ;;
esac
