#!/bin/bash
# detach_mount: a vanished volume is success. A volume that is still
# mounted, or any other detach error, is a failure.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
dmg="$root/scripts/make-dmg.sh"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

cat > "$work/hdiutil" << 'EOF'
#!/bin/bash
set -euo pipefail
dir="${HDIUTIL_DIR:?}"
log="$dir/log"
scenario="$(cat "$dir/scenario")"
if [ "${1:-}" = "info" ]; then
  if [ -f "$dir/listed" ]; then
    cat "$dir/listed"
  fi
  if [ -f "$dir/info-fail" ]; then
    exit 1
  fi
  exit 0
fi
if [ "${1:-}" != "detach" ]; then
  echo "unexpected hdiutil $*" >&2
  exit 1
fi
force=0
target="${2:-}"
if [ "$target" = "-force" ]; then
  force=1
  target="${3:-}"
fi
printf 'force=%s target=%s\n' "$force" "$target" >> "$log"
n="$(wc -l < "$log" | tr -d ' ')"
case "$scenario" in
  ok)
    rm -f "$dir/listed"
    exit 0
    ;;
  busy-then-gone)
    if [ "$n" -eq 1 ]; then
      printf '%s\n' "$target" > "$dir/listed"
      echo 'hdiutil: couldn'"'"'t eject "disk7" - Resource busy' >&2
      exit 1
    fi
    rm -f "$dir/listed"
    echo 'hdiutil: detach failed - No such file or directory' >&2
    exit 1
    ;;
  stays-busy)
    printf '%s\n' "$target" > "$dir/listed"
    echo 'hdiutil: couldn'"'"'t eject "disk7" - Resource busy' >&2
    exit 1
    ;;
  force-gone)
    if [ "$force" -eq 0 ]; then
      printf '%s\n' "$target" > "$dir/listed"
      echo 'hdiutil: couldn'"'"'t eject "disk7" - Resource busy' >&2
      exit 1
    fi
    rm -f "$dir/listed"
    echo 'hdiutil: detach failed - No such file or directory' >&2
    exit 1
    ;;
  other-error)
    rm -f "$dir/listed"
    echo 'hdiutil: detach failed - Permission denied' >&2
    exit 1
    ;;
  gone-but-listed)
    printf '%s\n' "$target" > "$dir/listed"
    echo 'hdiutil: detach failed - No such file or directory' >&2
    exit 1
    ;;
  info-fails)
    rm -f "$dir/listed"
    echo 'hdiutil: detach failed - No such file or directory' >&2
    touch "$dir/info-fail"
    exit 1
    ;;
  *)
    echo "unknown scenario $scenario" >&2
    exit 1
    ;;
esac
EOF
chmod +x "$work/hdiutil"
export PATH="$work:$PATH"
export SR_DETACH_FAST=1

run_case() {
  name="$1"
  want="$2"
  printf '%s\n' "$name" > "$work/scenario"
  : > "$work/log"
  rm -f "$work/listed" "$work/info-fail"
  export HDIUTIL_DIR="$work"
  set +e
  "$dmg" --detach "/tmp/slowpage-mount-$name" >"$work/out" 2>"$work/err"
  status=$?
  set -e
  if [ "$status" -ne "$want" ]; then
    echo "$name exited $status, want $want" >&2
    cat "$work/err" >&2
    exit 1
  fi
}

run_case ok 0
grep -F -q 'force=0' "$work/log"
if grep -F -q 'force=1' "$work/log"; then
  echo "a clean detach used force" >&2
  exit 1
fi

run_case busy-then-gone 0
grep -F -q 'retry 1' "$work/err"
if grep -F -q 'forcing' "$work/err"; then
  echo "a vanished mount was forced" >&2
  exit 1
fi
gone="$(grep -c 'force=0' "$work/log" | tr -d ' ')"
if [ "$gone" -ne 2 ]; then
  echo "busy-then-gone made $gone plain detaches" >&2
  exit 1
fi

run_case stays-busy 1
grep -F -q 'forcing' "$work/err"
grep -F -q 'force=1' "$work/log"
grep -F -q 'Resource busy' "$work/err"

run_case force-gone 0
grep -F -q 'forcing' "$work/err"
grep -F -q 'force=1' "$work/log"

run_case other-error 1
if grep -F -q 'No such file' "$work/err"; then
  echo "permission denied was treated as a missing mount" >&2
  exit 1
fi

run_case gone-but-listed 1
grep -F -q 'forcing' "$work/err"

run_case info-fails 1

release="$root/.github/workflows/release.yml"
grep -F -q 'scripts/make-dmg.sh --detach' "$release"
if grep -F -q 'hdiutil detach' "$release"; then
  echo "release.yml still detaches on its own" >&2
  exit 1
fi
grep -F -q 'stapler validate' "$release"
grep -F -q 'source=Notarized Developer ID' "$release"

echo "detach-mount ok"
