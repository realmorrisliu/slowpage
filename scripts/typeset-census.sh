#!/usr/bin/env bash
# Phase 1 census. Official runs are the macos job in .github/workflows/ci.yml
# on a GitHub-hosted macOS runner. This does not call `just install`.
set -euo pipefail

root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
cd "$root"

if [ "$(uname -s)" != "Darwin" ]; then
  python3 scripts/typeset-gate --ci
  exit $?
fi

echo "census host $(sw_vers -productVersion) $(uname -m) $(hostname)"
found=0
while IFS= read -r path; do
  [ -n "$path" ] || continue
  echo "font $path"
  found=1
done < <(find /System/Library/Fonts /Library/Fonts "$HOME/Library/Fonts" \
  \( -iname '*Iowan*' -o -iname '*Songti*' -o -iname '*STSong*' \) -print 2>/dev/null || true)
if [ "$found" -eq 0 ]; then
  echo "font search found no Iowan or Songti file"
fi

dest="${SPIKE_FIXTURES:-/tmp/superreader-fixtures}"
mkdir -p "$dest"

fetch() {
  out="$1"
  shift
  if [ -s "$out" ]; then
    return 0
  fi
  for url in "$@"; do
    try=1
    while [ "$try" -le 4 ]; do
      if curl -fsSL -A "Mozilla/5.0" -o "$out" "$url"; then
        return 0
      fi
      try=$((try + 1))
      sleep 3
    done
  done
  rm -f "$out"
  echo "network: could not download $out. This is not a product or chapter failure." >&2
  return 1
}

fetch "$dest/pride-se.epub" \
  "https://standardebooks.org/ebooks/jane-austen/pride-and-prejudice/downloads/jane-austen_pride-and-prejudice.epub?source=download"
fetch "$dest/zhaohua.epub" \
  "https://gutenberg.pglaf.org/cache/epub/25271/pg25271.epub"
if ! echo "91e59f1acb65d7bc77fa2d52fca8b223b893ad837f55c62de3e85fa848e0f293  $dest/zhaohua.epub" | shasum -a 256 -c -; then
  rm -f "$dest/zhaohua.epub"
  fetch "$dest/zhaohua.epub" \
    "https://gutenberg.pglaf.org/cache/epub/25271/pg25271.epub"
  if ! echo "91e59f1acb65d7bc77fa2d52fca8b223b893ad837f55c62de3e85fa848e0f293  $dest/zhaohua.epub" | shasum -a 256 -c -; then
    echo "fixture sha mismatch for $dest/zhaohua.epub. This is not a chapter assertion." >&2
    exit 1
  fi
fi

python3 spikes/epub/fixtures/check_zhaohua.py
python3 scripts/typeset-gate --ci
