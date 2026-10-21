#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

# Explicit stable snapshots; update these together with remote invocation proof.
npm install --global --prefix "$HOME/.local" --allow-scripts=@anthropic-ai/claude-code \
  @openai/codex@0.163.0 @anthropic-ai/claude-code@2.1.306
manager="$(node -p 'require("./viewer/package.json").packageManager')"
[[ "$manager" =~ ^pnpm@[0-9]+\.[0-9]+\.[0-9]+$ ]] || {
  echo "Unsupported packageManager: $manager" >&2
  exit 1
}
npm install --global --prefix "$HOME/.local" --allow-scripts=pnpm "$manager"
git lfs install --local
codex --version
claude --version
pnpm --version
