#!/usr/bin/env bash
# Optional audit, outside check.sh: prove the privacy checks can fail.
#
# For each leak in scripts/plant_leak.py, copy the tool, plant the leak in
# web/app.js, build, and run the core boundary tests and the browser check
# (Chromium). Every leak must be caught by at least one of them. A check
# that never fails proves nothing.
#
#   ./scripts/prove-privacy.sh
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PY="${BUILDER_PYTHON:-$ROOT/.venv/bin/python}"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/status-trends-prove.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT
missed=0
leaks=$(python3 "$ROOT/scripts/plant_leak.py" --list) || { echo "could not list the leaks"; exit 1; }
[ -n "$leaks" ] || { echo "no leaks to plant: the proof would prove nothing"; exit 1; }
for leak in $leaks; do
  rm -rf "$WORK/copy"
  rsync -a --exclude .git --exclude .venv "$ROOT/" "$WORK/copy/"
  python3 "$ROOT/scripts/plant_leak.py" "$WORK/copy" "$leak"
  caught=""
  if ! (cd "$WORK/copy" && cargo test -q --test boundary --test no_storage >/dev/null 2>&1); then
    caught="core boundary tests"
  fi
  if (cd "$WORK/copy" && ./scripts/build.sh >/dev/null 2>&1); then
    out=$(cd "$WORK/copy" && BUILDER_ENGINES=chromium BUILDER_IDLE=1 "$PY" tests/browser/check.py 2>&1)
    if ! grep -q "BROWSER CHECK PASSED" <<<"$out"; then
      caught="${caught:+$caught; }browser check: $(grep -m1 '  FAIL ' <<<"$out" | cut -c8-90)"
    fi
  fi
  if [ -n "$caught" ]; then
    echo "  caught $leak by $caught"
  else
    echo "  MISSED $leak"
    missed=1
  fi
done
if [ "$missed" = 0 ]; then echo "PRIVACY PROOF PASSED: every planted leak was caught"; else echo "PRIVACY PROOF FAILED"; exit 1; fi
