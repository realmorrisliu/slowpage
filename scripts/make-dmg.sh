#!/bin/bash
# dist/<name>-<version>.dmg with a drag-to-Applications layout.
# Signs and notarizes only when SR_SIGN_IDENTITY is a real identity and a notary
# profile or API key is set. Ad hoc builds stay local and unsigned by Apple.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
manifest="$root/spikes/shell/nsh.json"
name="$(sed -n 's/^[[:space:]]*"name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$manifest" | head -n 1)"
if [ -z "$name" ]; then
  echo "nsh.json needs name" >&2
  exit 1
fi
app="$root/dist/${name}.app"
version="$("$root/scripts/app-version.sh" version "$manifest")"
"$root/scripts/updater-gate.sh" "$manifest" >/dev/null
dmg="$root/dist/${name}-${version}.dmg"
identity="${SR_SIGN_IDENTITY:--}"

# SetFile -a C and codesign --deep leave diskimages-helper holding the volume.
# One detach then exits 16 (Resource busy). A later detach can say the mount
# is already gone. That is success. A volume hdiutil info still lists, or any
# other detach error, is a failure.
detach_pause() {
  if [ "${SR_DETACH_FAST:-}" = "1" ]; then
    return 0
  fi
  sleep "$1"
}

mount_listed() {
  info="$(mktemp)"
  if ! hdiutil info >"$info" 2>/dev/null; then
    rm -f "$info"
    return 2
  fi
  if grep -F -q -- "$1" "$info"; then
    rm -f "$info"
    return 0
  fi
  rm -f "$info"
  return 1
}

mount_already_gone() {
  err_file="$1"
  point="$2"
  listed=0
  mount_listed "$point" || listed=$?
  if [ "$listed" -ne 1 ]; then
    return 1
  fi
  grep -E -q 'No such file or directory|not currently attached|not currently mounted|No such disk|is not attached' "$err_file"
}

detach_mount() {
  point="$1"
  if [ -z "$point" ]; then
    echo "detach needs a mount point" >&2
    return 1
  fi
  attempt=1
  err="$(mktemp)"
  while [ "$attempt" -le 5 ]; do
    if hdiutil detach "$point" >/dev/null 2>"$err"; then
      rm -f "$err"
      return 0
    fi
    if mount_already_gone "$err" "$point"; then
      rm -f "$err"
      return 0
    fi
    echo "hdiutil detach busy at ${point}, retry ${attempt}" >&2
    cat "$err" >&2
    detach_pause "$attempt"
    attempt=$((attempt + 1))
  done
  echo "hdiutil detach still busy at ${point}; forcing" >&2
  if hdiutil detach -force "$point" >/dev/null 2>"$err"; then
    rm -f "$err"
  elif mount_already_gone "$err" "$point"; then
    rm -f "$err"
    return 0
  else
    cat "$err" >&2
    rm -f "$err"
    return 1
  fi
  listed=0
  mount_listed "$point" || listed=$?
  if [ "$listed" -eq 0 ]; then
    echo "volume still mounted at ${point}" >&2
    return 1
  fi
  if [ "$listed" -eq 2 ]; then
    echo "hdiutil info failed after detach of ${point}" >&2
    return 1
  fi
}

if [ "${1:-}" = "--detach" ]; then
  detach_mount "${2:-}"
  exit $?
fi

if [ ! -d "$app" ]; then
  echo "missing $app (run scripts/package-macos.sh)" >&2
  exit 1
fi

icns="$root/dist/AppIcon.icns"
if [ ! -f "$icns" ]; then
  icns="$("$root/scripts/make-icns.sh")"
fi

stage="$(mktemp -d)"
mount="$(mktemp -d)"
cleanup() {
  status=$?
  detach_mount "$mount" || status=1
  rm -rf "$stage" "$mount"
  exit "$status"
}
trap cleanup EXIT
ditto "$app" "$stage/${name}.app"
ln -s /Applications "$stage/Applications"
rm -f "$dmg"
writable="$root/dist/${name}-rw.dmg"
rm -f "$writable"
hdiutil create -volname "$name" -srcfolder "$stage" -ov -format UDRW "$writable" >/dev/null
hdiutil attach "$writable" -mountpoint "$mount" -nobrowse >/dev/null
cp "$icns" "$mount/.VolumeIcon.icns"
SetFile -a C "$mount"
detach_mount "$mount"
hdiutil convert "$writable" -format UDZO -ov -o "$dmg" >/dev/null
rm -f "$writable"

if [ "$identity" != "-" ]; then
  codesign --force --sign "$identity" --timestamp "$dmg"
fi

verify_mounted_app() {
  point="$(mktemp -d)"
  hdiutil attach "$dmg" -mountpoint "$point" -nobrowse -readonly >/dev/null
  set +e
  echo "codesign verify dmg app"
  codesign --verify --deep --strict --verbose=2 "$point/${name}.app"
  status=$?
  set -e
  detach_mount "$point"
  rmdir "$point" || true
  if [ "$status" -ne 0 ]; then
    echo "codesign verify failed for the app inside $dmg" >&2
    exit "$status"
  fi
}
verify_mounted_app

staple() {
  local target="$1"
  local attempt
  for attempt in 1 2 3 4 5 6; do
    if xcrun stapler staple "$target"; then
      return 0
    fi
    echo "stapler attempt ${attempt} did not find a ticket for ${target}" >&2
    sleep 15
  done
  return 1
}

notarize() {
  if [ -n "${SR_NOTARY_PROFILE:-}" ]; then
    xcrun notarytool submit "$dmg" --keychain-profile "$SR_NOTARY_PROFILE" --wait --output-format json
  elif [ -n "${SR_NOTARY_KEY:-}" ]; then
    xcrun notarytool submit "$dmg" \
      --key "$SR_NOTARY_KEY" \
      --key-id "$SR_NOTARY_KEY_ID" \
      --issuer "$SR_NOTARY_ISSUER" \
      --wait \
      --output-format json
  else
    return 0
  fi
}

if [ "$identity" != "-" ] && { [ -n "${SR_NOTARY_PROFILE:-}" ] || [ -n "${SR_NOTARY_KEY:-}" ]; }; then
  mkdir -p "$root/dist"
  set +e
  notarize >"$root/dist/notary.json" 2>"$root/dist/notary.err"
  notary_status=$?
  set -e
  if [ "$notary_status" -ne 0 ]; then
    cat "$root/dist/notary.err" >&2
    if grep -q "agreement" "$root/dist/notary.err"; then
      echo "Apple notarization refused: the Developer Program agreement is missing or expired." >&2
    fi
    echo "$dmg"
    exit 0
  fi
  staple "$dmg"
  staple "$app"
  verify_mounted_app
fi

echo "$dmg"
