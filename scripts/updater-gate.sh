#!/bin/bash
# nsh.json without "updater" keeps updates off.
# An updater object without a 32-byte publicKey is refused.
# GitHub would otherwise accept an unsigned latest.json.
# A present key is not a running update check and not a signature.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
file="${1:-$root/spikes/shell/nsh.json}"

if [ ! -f "$file" ]; then
  echo "missing $file" >&2
  exit 1
fi

object="$(awk '
  BEGIN { found = 0; depth = 0; out = "" }
  {
    line = $0
    if (!found) {
      if (match(line, /"updater"[[:space:]]*:/)) {
        found = 1
        line = substr(line, RSTART + RLENGTH)
      } else {
        next
      }
    }
    n = length(line)
    for (i = 1; i <= n; i++) {
      c = substr(line, i, 1)
      if (!started) {
        if (c == "{") {
          started = 1
          depth = 1
          out = "{"
        } else if (c !~ /[[:space:]]/) {
          printf "updater is not an object\n" > "/dev/stderr"
          bad = 1
          exit 2
        }
        continue
      }
      out = out c
      if (c == "{") depth++
      if (c == "}") {
        depth--
        if (depth == 0) {
          print out
          exit 0
        }
      }
    }
  }
  END {
    if (bad) exit 2
    if (!found) exit 0
    if (!started || depth != 0) {
      printf "updater object did not close\n" > "/dev/stderr"
      exit 2
    }
  }
' "$file")" || {
  echo "updater-gate: $file" >&2
  exit 1
}

if [ -z "$object" ]; then
  printf 'off\n'
  exit 0
fi

field() {
  key="$1"
  printf '%s' "$object" | sed -n "s/.*\"${key}\"[[:space:]]*:[[:space:]]*\"\\([^\"]*\\)\".*/\\1/p" | head -n 1
}

public="$(field publicKey)"
github="$(field github)"
url="$(field url)"

if [ -z "$public" ]; then
  echo "updater without publicKey stays off: an unsigned manifest would be accepted" >&2
  exit 1
fi

trimmed="$(printf '%s' "$public" | tr -d '[:space:]')"
decoded="$(mktemp)"
if ! printf '%s' "$trimmed" | openssl enc -base64 -d -A >"$decoded" 2>/dev/null; then
  rm -f "$decoded"
  echo "updater.publicKey is not 32 bytes of base64" >&2
  exit 1
fi
bytes="$(wc -c < "$decoded" | tr -d '[:space:]')"
rm -f "$decoded"
if [ "$bytes" != "32" ]; then
  echo "updater.publicKey is not 32 bytes of base64" >&2
  exit 1
fi

if [ -n "$github" ] && [ -n "$url" ]; then
  echo "updater sets both github and url" >&2
  exit 1
fi
if [ -z "$github" ] && [ -z "$url" ]; then
  echo "updater needs github or url" >&2
  exit 1
fi
if [ -n "$github" ] && ! printf '%s' "$github" | grep -Eq '^[A-Za-z0-9._-]+/[A-Za-z0-9._-]+$'; then
  echo "updater.github is not owner/repo" >&2
  exit 1
fi
if [ -n "$url" ] && ! printf '%s' "$url" | grep -Eq '^https://'; then
  echo "updater.url is not https" >&2
  exit 1
fi
if printf '%s' "$object" | grep -q 'http://'; then
  echo "updater uses http" >&2
  exit 1
fi

printf 'keyed\n'
