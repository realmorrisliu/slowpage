#!/bin/sh
# Downloads the public fixtures and the pdfium dylib the spike measures.
set -eu
root="$(cd "$(dirname "$0")/.." && pwd)"
dest="${SPIKE_FIXTURES:-/tmp/superreader-fixtures}"
pdfium="${SPIKE_PDFIUM:-/tmp/pdfium}"
mkdir -p "$dest" "$pdfium"
curl -fsSL -o "$dest/gpt3.pdf" "https://arxiv.org/pdf/2005.14165"
curl -fsSL -A "Mozilla/5.0" -o "$dest/pride-se.epub" \
  "https://standardebooks.org/ebooks/jane-austen/pride-and-prejudice/downloads/jane-austen_pride-and-prejudice.epub?source=download"
curl -fsSL -A "Mozilla/5.0" -o "$dest/zhaohua.epub" \
  "https://gutenberg.pglaf.org/cache/epub/25271/pg25271.epub"
echo "91e59f1acb65d7bc77fa2d52fca8b223b893ad837f55c62de3e85fa848e0f293  $dest/zhaohua.epub" | shasum -a 256 -c -
curl -fsSL -L -o "$pdfium/pdfium-mac-arm64.tgz" \
  "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F8125/pdfium-mac-arm64.tgz"
tar -xzf "$pdfium/pdfium-mac-arm64.tgz" -C "$pdfium"
echo "fixtures $dest"
echo "pdfium $pdfium/lib/libpdfium.dylib"
