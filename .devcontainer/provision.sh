#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

# Codex, Claude Code and pnpm, pinned with every dependency by
# .devcontainer/tools/package-lock.json and installed from it with `npm ci`,
# outside the checkout. Dependabot updates the lock; its package.json allows
# the two install scripts they need (`allowScripts`, npm 12).
tools="$HOME/.local/share/rvt-devcontainer-tools"
mkdir -p "$tools" "$HOME/.local/bin"
cp .devcontainer/tools/package.json .devcontainer/tools/package-lock.json "$tools/"
npm ci --prefix "$tools" --no-audit --no-fund
for bin in codex claude pnpm; do
  ln -sf "$tools/node_modules/.bin/$bin" "$HOME/.local/bin/$bin"
done
# The viewer's pnpm and this one are the same release.
manager="$(node -p 'require("./viewer/package.json").packageManager')"
locked="pnpm@$(node -p 'require(process.argv[1]).packages["node_modules/pnpm"].version' "$tools/package-lock.json")"
[[ "$manager" == "$locked" ]] || {
  echo "viewer/package.json's packageManager is $manager, but .devcontainer/tools pins $locked" >&2
  exit 1
}
git lfs install --local
"$HOME/.local/bin/codex" --version
"$HOME/.local/bin/claude" --version
"$HOME/.local/bin/pnpm" --version
