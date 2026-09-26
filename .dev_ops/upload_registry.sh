#!/usr/bin/env bash
#
# Upload prebuilt quick_stats artifacts to THIS project's generic package
# registry — the manual bridge between the GitHub builds (release v<ver>)
# and the registry LinkMesh pulls from.
#
# Usage:
#   GITLAB_TOKEN=glpat-... .dev_ops/upload_registry.sh quick_stats-0.2.1-*
#
#   Filenames must follow the registry convention (that is what the
#   GitHub release assets are named):
#     quick_stats-<version>-linux-x64
#     quick_stats-<version>-linux-aarch64
#     quick_stats-<version>-mac-arm64
#     quick_stats-<version>-mac-x64
#     quick_stats-<version>-win-x64.exe
#
#   GITLAB_TOKEN: a Personal Access Token with the `api` scope and
#   Developer+ on this project (project 14). The registry itself is read
#   by LinkMesh WITHOUT any token.
#
# Download the release assets in one go:
#   gh release download v0.2.1 -R Luxbit/quick_stats -D /tmp/qs && .dev_ops/upload_registry.sh /tmp/qs/quick_stats-*
set -euo pipefail

GITLAB_URL="${GITLAB_URL:-https://gitlab.byte.lu}"
PROJECT_ID="${PROJECT_ID:-14}" # olivier/quick_stats

[ -n "${GITLAB_TOKEN:-}" ] || {
  echo "FATAL: GITLAB_TOKEN not set (Personal Access Token, api scope, Developer+ on project $PROJECT_ID)"
  exit 1
}
[ $# -gt 0 ] || {
  echo "usage: $0 quick_stats-<version>-<os>-<arch>[.exe] ..."
  exit 1
}

for f in "$@"; do
  [ -f "$f" ] || { echo "FATAL: not a file: $f"; exit 1; }
  base=$(basename "$f")
  version=$(printf '%s' "$base" | sed -n 's/^quick_stats-\([^-]*\)-.*/\1/p')
  [ -n "$version" ] || { echo "FATAL: cannot parse version from $base (expected quick_stats-<version>-<os>-<arch>)"; exit 1; }
  echo "Uploading $base -> quick_stats/$version/"
  curl -fsS --header "PRIVATE-TOKEN: $GITLAB_TOKEN" --upload-file "$f" \
    "$GITLAB_URL/api/v4/projects/$PROJECT_ID/packages/generic/quick_stats/$version/$base"
  echo "  ok"
done
echo "done — $# file(s) in the registry."
