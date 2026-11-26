#!/bin/bash
# Rasterize or downsample the Slowpage icon PNG/SVG into AppIcon.icns.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
src="${1:-$root/assets/icon/Slowpage.png}"
out="${2:-$root/dist/AppIcon.icns}"

if [ ! -f "$src" ]; then
  echo "missing app icon at $src" >&2
  exit 1
fi
case "$src" in
  *.png|*.svg) ;;
  *)
    echo "app icon must be a PNG or SVG: $src" >&2
    exit 1
    ;;
esac
if ! command -v iconutil >/dev/null 2>&1; then
  echo "iconutil is required to build the icns" >&2
  exit 1
fi

target_dir="$("$root/scripts/cargo-target-dir.sh")"
renderer="$target_dir/release/spike-icon"
if [ ! -x "$renderer" ]; then
  cargo build --manifest-path "$root/spikes/Cargo.toml" -p spike-icon --release
  target_dir="$("$root/scripts/cargo-target-dir.sh")"
  renderer="$target_dir/release/spike-icon"
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
iconset="$work/AppIcon.iconset"
mkdir -p "$iconset" "$(dirname "$out")" "$root/dist"
"$renderer" "$src" "$iconset" "$root/dist"
iconutil -c icns "$iconset" -o "$out"
echo "$out"
