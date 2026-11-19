#!/bin/bash
# Holds /tmp/superreader-verify.lock, then runs the S2.1 driver.
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
    rmdir "$LOCK" 2>/dev/null || rm -rf "$LOCK"
    echo "verify lock released"
  fi
}
trap release EXIT

while true; do
  if mkdir "$LOCK" 2>/dev/null; then
    break
  fi
  echo "waiting for verify lock"
  sleep 30
done
printf '%s\n' "$$" >"$LOCK/pid"
git branch --show-current >"$LOCK/branch"
date -u +%Y-%m-%dT%H:%M:%SZ >"$LOCK/started"
echo "verify lock acquired $(cat "$LOCK/branch") $(cat "$LOCK/started")"
sysctl -n vm.loadavg
python3 -u docs/verify/runs/shell-s2-1/drive.py
echo "EXIT:$?"
