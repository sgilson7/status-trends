#!/usr/bin/env bash
# Publish dist/ to the gh-pages branch, which GitHub Pages serves at
# https://<owner>.github.io/status-trends/. A person's action (stage 8): run
# it after ./scripts/check.sh passes, then run the browser check against the
# live origin:
#
#   BUILDER_ORIGIN=https://sgilson7.github.io/status-trends .venv/bin/python tests/browser/check.py
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REMOTE="$(git -C "$ROOT" remote get-url origin)"
COMMIT="$(git -C "$ROOT" rev-parse --short HEAD)"
[ -z "$(git -C "$ROOT" status --porcelain)" ] || { echo "commit first: the published build must match a commit" >&2; exit 1; }
WORK="$(mktemp -d "${TMPDIR:-/tmp}/status-trends-pages.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT
"$ROOT/scripts/build.sh" >/dev/null
cp -R "$ROOT/dist/." "$WORK/"
touch "$WORK/.nojekyll"
cd "$WORK"
git init -q -b gh-pages
git add -A
git -c user.name="$(git -C "$ROOT" config user.name)" -c user.email="$(git -C "$ROOT" config user.email)" \
  commit -q -m "Publish dist/ from $COMMIT"
git push -q -f "$REMOTE" gh-pages
echo "pushed dist/ from $COMMIT to gh-pages"
