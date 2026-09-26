#!/usr/bin/env bash
#
# Build + publish the macOS quick_stats binaries (the ones the LinkMesh
# desktop bundles for mac). Run on a dev Mac.
#
# For each apple target (arm64 + x64) this script:
#   1. builds quick_stats against the SAME libtorch (2.2.2, matching arch)
#      that LinkMesh's `yarn download:deps` bundles into libs/mac, so the
#      published artifact and the desktop's runtime libtorch cannot drift
#   2. stages it as builds/quick_stats-<version>-mac-<arch> (version = the
#      Cargo.toml version — that is the registry key)
#   3. uploads it to THIS project's generic package registry, the pinned
#      location LinkMesh's `yarn download:deps` fetches:
#        https://gitlab.byte.lu/api/v4/projects/14/packages/generic/quick_stats/<version>/
#   4. copies it into the LinkMesh repo's libs/mac/ so the DEV flow works
#      immediately (QuickStatsController's non-packaged path resolves the
#      pinned artifact from libs/mac without waiting on the registry)
#
# Usage:
#   LINKMESH_DIR=~/git_repos/linkmesh ./publish_mac_binaries.sh
#
#   GITLAB_TOKEN=glpat-...  required for the registry upload — any token
#                           with Developer+ on this project (project 14).
#                           Without it the script still builds and copies
#                           into linkmesh's libs/, and skips the upload
#                           with a loud warning: LinkMesh PACKAGING for
#                           mac would then fail at staging until a
#                           token-carrying run has published the pair.
#
#   LIBTORCH_ROOT  override where the mac libtorch trees live (default:
#                 $LINKMESH_DIR/libs/mac — run `yarn download:deps mac`
#                 there first).
set -euo pipefail

cd "$(dirname -- "$0")/.."

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
[ -n "$VERSION" ] || { echo "FATAL: could not read version from Cargo.toml"; exit 1; }

LINKMESH_DIR="${LINKMESH_DIR:-$HOME/git_repos/linkmesh}"
LIBTORCH_ROOT="${LIBTORCH_ROOT:-$LINKMESH_DIR/libs/mac}"
LIBTORCH_VERSION=2.2.2
GITLAB_URL="${GITLAB_URL:-https://gitlab.byte.lu}"
PROJECT_ID="${PROJECT_ID:-14}"   # olivier/quick_stats

rustup target add aarch64-apple-darwin x86_64-apple-darwin

# Respect a custom CARGO_TARGET_DIR (dev machines may share one rust target
# dir); default is cargo's own `target/`.
TARGET_DIR="${CARGO_TARGET_DIR:-target}"

rm -rf builds && mkdir -p builds

for arch in arm64 x64; do
  target=aarch64-apple-darwin
  [ "$arch" = "x64" ] && target=x86_64-apple-darwin
  libtorch="$LIBTORCH_ROOT/libtorch-${LIBTORCH_VERSION}-mac-cpu-${arch}/libtorch"
  if [ ! -d "$libtorch" ]; then
    echo "FATAL: mac libtorch not found at $libtorch"
    echo "       run \`yarn download:deps mac\` in $LINKMESH_DIR first"
    exit 1
  fi
  echo "==> cargo build --release --target=$target (LIBTORCH=$libtorch)"
  LIBTORCH="$libtorch" LIBTORCH_INCLUDE="$libtorch" LIBTORCH_LIB="$libtorch" \
    cargo build --release --target="$target"
  cp "$TARGET_DIR/$target/release/quick_stats" "builds/quick_stats-${VERSION}-mac-${arch}"
  chmod +x "builds/quick_stats-${VERSION}-mac-${arch}"
done

# --- 1) registry upload (skippable, loud if skipped without a token) -----
if [ -n "${GITLAB_TOKEN:-}" ]; then
  for f in builds/quick_stats-${VERSION}-mac-*; do
    curl -sfS --header "PRIVATE-TOKEN: $GITLAB_TOKEN" --upload-file "$f" \
      "$GITLAB_URL/api/v4/projects/$PROJECT_ID/packages/generic/quick_stats/$VERSION/$(basename "$f")"
    echo "published $f"
  done
else
  echo "WARNING: GITLAB_TOKEN not set — the mac binaries were NOT uploaded to the registry."
  echo "         The LinkMesh dev flow (libs/mac) IS updated, but mac PACKAGING"
  echo "         will fail at staging until this script runs with a token."
fi

# --- 2) dev-flow copy into the LinkMesh libs/ cache ----------------------
mkdir -p "$LINKMESH_DIR/libs/mac"
for f in builds/quick_stats-${VERSION}-mac-*; do
  cp "$f" "$LINKMESH_DIR/libs/mac/$(basename "$f")"
  echo "copied $(basename "$f") -> $LINKMESH_DIR/libs/mac/"
done

echo "done — quick_stats $VERSION mac artifacts built."
