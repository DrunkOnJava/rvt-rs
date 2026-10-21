#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
evidence="${RVT_EVIDENCE_ROOT:-/workspaces/.rvt-rs-evidence}/$(basename "$root")/create-$(date -u +%Y%m%dT%H%M%SZ)-$$"
mkdir -p "$evidence"
printf 'Build and smoke evidence: %s\n' "$evidence"
if ! bash .cursor/install.sh >"$evidence/bootstrap.log" 2>&1; then
  printf 'Bootstrap failed. Read %s/bootstrap.log\n' "$evidence" >&2
  exit 1
fi
(
  cd viewer
  pnpm exec playwright install --with-deps chromium
) >"$evidence/browser-install.log" 2>&1
bash .devcontainer/verify.sh "$evidence"
printf 'First-create passed. Evidence: %s\n' "$evidence"
