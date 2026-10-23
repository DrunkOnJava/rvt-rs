#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
printf 'rvt-rs remote checkout: %s\n' "$PWD"
git --no-pager log -1 --format='%H %s'
for tool in cargo rustc python3 node pnpm gh codex claude; do
  command -v "$tool"
done
profile="${RVT_BUILD_PROFILE:-ci}"
[ "$profile" != dev ] || profile=debug
test -x "${CARGO_TARGET_DIR:-target}/$profile/rvt-inspect"
test -s viewer/pkg/rvt_bg.wasm
.venv/bin/python -c 'import rvt; print("Python binding:", rvt.__version__)'
test -x "$HOME/.local/bin/jupyter-lab"
printf 'Resume check passed; no install, rebuild or server was started.\n'
