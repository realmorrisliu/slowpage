#!/bin/bash
# Print the spikes workspace target directory reported by cargo metadata.
# This does not set CARGO_TARGET_DIR. A cargo wrapper's own directory is used.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cargo metadata --manifest-path "$root/spikes/Cargo.toml" --format-version 1 --no-deps \
  | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p' \
  | head -n 1
