#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
printf 'rvt-rs remote checkout: %s\n' "$PWD"
git log -1 --format='%H %s'
for tool in cargo rustc python3 node pnpm gh codex claude; do
  command -v "$tool"
done
test -x "${CARGO_TARGET_DIR:-target}/${RVT_BUILD_PROFILE:-ci}/rvt-inspect"
test -s viewer/pkg/rvt_bg.wasm
.venv/bin/python -c 'import rvt; print("Python binding:", rvt.__version__)'
printf 'Resume check passed; no install, rebuild or server was started.\n'
