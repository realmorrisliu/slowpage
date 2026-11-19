#!/bin/bash
set -eu
export PATH="${HOME}/.local/bin:${PATH}"
unset CARGO_TARGET_DIR
export SR_VERIFY_MAX_LOAD=999
export SR_VERIFY_LOAD_START=999
export SR_VERIFY_LOCK_HELD=1
LOCK=/tmp/superreader-verify.lock
cd "$(dirname "$0")/../../../.."
release() {
  if [[ -d "$LOCK" && "$(cat "$LOCK/pid" 2>/dev/null || true)" == "$$" ]]; then
    rm -f "$LOCK/pid" "$LOCK/branch" "$LOCK/started"
    rmdir "$LOCK" 2>/dev/null || true
    echo "verify lock released"
  fi
}
trap release EXIT
mkdir "$LOCK"
printf '%s\n' "$$" >"$LOCK/pid"
git branch --show-current >"$LOCK/branch"
date -u +%Y-%m-%dT%H:%M:%SZ >"$LOCK/started"
echo "verify lock acquired $(cat "$LOCK/branch") $(cat "$LOCK/started")"
python3 -u docs/verify/runs/shell-s2-1/exit-fullscreen.py
echo "EXIT:$?"
