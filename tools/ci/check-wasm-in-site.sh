#!/usr/bin/env bash
# The site vite built holds the WASM that the build artifact holds (#1457).
#
# Usage:
#   tools/ci/check-wasm-in-site.sh WASM_SHA256_FILE SITE_DIR
#
# The viewer's browser tests serve the site, not viewer/pkg, and Deploy viewer
# publishes it, so the bytes that matter are the ones in SITE_DIR. vite copies
# the WASM into the site unchanged (`assetsInlineLimit` is 0, so nothing
# inlines it); a bundler that re-encoded or renamed the asset, or dropped it,
# would otherwise pass the checks on viewer/pkg and fail on a copy nobody
# tested. WASM_SHA256_FILE is the `wasm.sha256` that the build writes with
# `sha256sum`. Fails unless some .wasm under SITE_DIR has that sha256.
set -euo pipefail

sums="$1"
site="$2"

expected=$(cut -d' ' -f1 "${sums}")
found=$(find "${site}" -name '*.wasm' -exec sha256sum {} + | cut -d' ' -f1)
echo "WASM sha256 in the site: ${found:-none}"
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
  echo "WASM sha256 in the site: ${found:-none}" >> "${GITHUB_STEP_SUMMARY}"
fi
if ! grep -qx "${expected}" <<<"${found}"; then
  echo "::error::the site holds no WASM with sha256 ${expected}"
  exit 1
fi
