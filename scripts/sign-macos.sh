#!/bin/bash
# Sign the app. Inner libraries first, then the bundle.
# SR_SIGN_IDENTITY defaults to ad hoc (-). A real identity gets the hardened runtime.
set -euo pipefail

app="${1:?app bundle}"
root="$(cd "$(dirname "$0")/.." && pwd)"
identity="${SR_SIGN_IDENTITY:--}"
entitlements="$root/scripts/entitlements.plist"

sign_item() {
  target="$1"
  if [ "$identity" = "-" ]; then
    codesign --force --sign - "$target"
    return
  fi
  # Apple's timestamp server sometimes answers without a timestamp. Retry that
  # case. Any other codesign failure still stops the build.
  attempt=1
  while true; do
    err="$(mktemp)"
    if codesign --force --sign "$identity" --options runtime --timestamp "$target" 2>"$err"; then
      rm -f "$err"
      return
    fi
    if [ "$attempt" -ge 3 ] || ! grep -q -i timestamp "$err"; then
      cat "$err" >&2
      rm -f "$err"
      return 1
    fi
    echo "codesign timestamp failed for $target, retry $attempt" >&2
    rm -f "$err"
    attempt=$((attempt + 1))
    sleep $((attempt * 2))
  done
}

if [ -d "$app/Contents/Frameworks" ]; then
  while IFS= read -r item; do
    [ -n "$item" ] || continue
    sign_item "$item"
  done < <(find "$app/Contents/Frameworks" -type f -name '*.dylib')
fi

if [ "$identity" = "-" ]; then
  codesign --force --sign - "$app"
else
  codesign --force --sign "$identity" --options runtime --timestamp --entitlements "$entitlements" "$app"
fi

echo "codesign verify app"
codesign --verify --deep --strict --verbose=2 "$app"
