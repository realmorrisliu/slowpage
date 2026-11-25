#!/bin/bash
# The disk image version is the semver in nsh.json.
# Updates stay off. A config without publicKey is refused.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
version="$("$root/scripts/app-version.sh" version)"
build="$("$root/scripts/app-version.sh" build)"
if [ "$version" != "0.1.0-alpha.2" ]; then
  echo "candidate version is $version" >&2
  exit 1
fi
if [ "$build" != "1" ]; then
  echo "candidate build is $build" >&2
  exit 1
fi
gate="$("$root/scripts/updater-gate.sh")"
if [ "$gate" != "off" ]; then
  echo "product updater gate is $gate" >&2
  exit 1
fi
if grep -q '"updater"' "$root/spikes/shell/nsh.json"; then
  echo "nsh.json sets updater" >&2
  exit 1
fi
if ! grep -q 'default-features = false' "$root/spikes/shell/Cargo.toml"; then
  echo "nsh-platform-macos features are not closed" >&2
  exit 1
fi
if grep -q '0\.1\.0-alpha\.1' \
  "$root/scripts/package-macos.sh" \
  "$root/scripts/make-dmg.sh" \
  "$root/.github/workflows/release.yml"; then
  echo "a release file still hardcodes 0.1.0-alpha.1" >&2
  exit 1
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

cat > "$work/plain.json" << 'EOF'
{ "version": "0.1.0-alpha.2", "build": 1 }
EOF
if [ "$("$root/scripts/updater-gate.sh" "$work/plain.json")" != "off" ]; then
  echo "a file without updater did not stay off" >&2
  exit 1
fi

# Length check only. The bytes are random, discarded, and not a Slowpage key.
key="$(openssl rand -base64 32 | tr -d '\n')"

refuse() {
  name="$1"
  body="$2"
  printf '%s\n' "$body" > "$work/$name.json"
  if "$root/scripts/updater-gate.sh" "$work/$name.json" >"$work/$name.out" 2>"$work/$name.err"; then
    echo "$name was accepted" >&2
    exit 1
  fi
}

refuse github-open '{ "updater": { "github": "realmorrisliu/slowpage" } }'
refuse url-open '{ "updater": { "url": "https://example.com/latest.json" } }'
refuse empty-key '{ "updater": { "github": "realmorrisliu/slowpage", "publicKey": "" } }'
refuse short-key '{ "updater": { "github": "realmorrisliu/slowpage", "publicKey": "YQ==" } }'
refuse bad-b64 '{ "updater": { "github": "realmorrisliu/slowpage", "publicKey": "!!!!" } }'
refuse both "{ \"updater\": { \"github\": \"realmorrisliu/slowpage\", \"url\": \"https://example.com/latest.json\", \"publicKey\": \"$key\" } }"
refuse neither "{ \"updater\": { \"publicKey\": \"$key\" } }"
refuse bad-repo "{ \"updater\": { \"github\": \"not a repo\", \"publicKey\": \"$key\" } }"
refuse http "{ \"updater\": { \"url\": \"http://example.com/latest.json\", \"publicKey\": \"$key\" } }"

printf '%s\n' "{ \"updater\": { \"github\": \"realmorrisliu/slowpage\", \"publicKey\": \"$key\" } }" > "$work/keyed.json"
if [ "$("$root/scripts/updater-gate.sh" "$work/keyed.json")" != "keyed" ]; then
  echo "a 32-byte publicKey was refused" >&2
  exit 1
fi
cat > "$work/pretty.json" << EOF
{
  "version": "0.1.0-alpha.2",
  "updater": {
    "github": "realmorrisliu/slowpage",
    "publicKey": "$key"
  }
}
EOF
if [ "$("$root/scripts/updater-gate.sh" "$work/pretty.json")" != "keyed" ]; then
  echo "a pretty-printed publicKey was refused" >&2
  exit 1
fi
refuse array '{ "updater": [] }'

bad="$work/bad-version.json"
printf '%s\n' '{ "version": "alpha.2" }' > "$bad"
if "$root/scripts/app-version.sh" version "$bad" >/dev/null 2>"$work/bad.err"; then
  echo "a non-semver version was accepted" >&2
  exit 1
fi

echo "candidate-version ok"
