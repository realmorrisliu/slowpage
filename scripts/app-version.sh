#!/bin/bash
# Read one release field from spikes/shell/nsh.json.
# version is semver, the same string CFBundleShortVersionString and the disk image use.
# build is the integer nsh.json stores. It becomes CFBundleVersion.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
field="${1:-version}"
file="${2:-$root/spikes/shell/nsh.json}"

if [ ! -f "$file" ]; then
  echo "missing $file" >&2
  exit 1
fi

case "$field" in
  version)
    value="$(sed -n 's/^[[:space:]]*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$file" | head -n 1)"
    if ! printf '%s' "$value" | grep -Eq '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?(\+[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$'; then
      echo "nsh.json version is not semver: ${value:-missing}" >&2
      exit 1
    fi
    ;;
  build)
    value="$(sed -n 's/^[[:space:]]*"build"[[:space:]]*:[[:space:]]*\([0-9][0-9]*\).*/\1/p' "$file" | head -n 1)"
    if [ -z "$value" ]; then
      echo "nsh.json build is missing" >&2
      exit 1
    fi
    ;;
  *)
    echo "unknown field $field" >&2
    exit 1
    ;;
esac

printf '%s\n' "$value"
