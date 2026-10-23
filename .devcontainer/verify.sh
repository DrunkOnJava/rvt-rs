#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
evidence="${1:-${RVT_EVIDENCE_ROOT:-/workspaces/.rvt-rs-evidence}/$(basename "$root")/verify-$(date -u +%Y%m%dT%H%M%SZ)-$$}"
[[ "$evidence" = /* ]] || { echo 'Evidence directory must be absolute' >&2; exit 1; }
test ! -e "$evidence/verify.log" || { echo "Evidence already exists: $evidence; choose a new directory" >&2; exit 1; }
mkdir -p "$evidence"
printf 'Verification evidence: %s\n' "$evidence"
exec 3>&1
exec >"$evidence/verify.log" 2>&1
trap 'printf "Verification failed; read %s/verify.log\n" "$evidence" >&3' ERR
git rev-parse HEAD >"$evidence/source-sha.txt"
# What was verified is the working tree: keep its changes beside the commit.
git status --porcelain=v1 >"$evidence/source-status.txt"
git diff HEAD >"$evidence/source.patch"
[ ! -s "$evidence/source-status.txt" ] || printf 'dirty: see source-status.txt and source.patch\n' >>"$evidence/source-sha.txt"
{
  rustc --version; cargo --version; python3 --version; node --version
  npm --version; pnpm --version; wasm-pack --version; gh --version
  codex --version; claude --version
} >"$evidence/versions.txt"
bash .devcontainer/resume.sh
cargo fmt --all -- --check
bash tools/ci/verify-real-files.sh --check-only
profile="${RVT_BUILD_PROFILE:-ci}"
[ "$profile" != dev ] || profile=debug
bin="${CARGO_TARGET_DIR:-$root/target}/$profile"
# This script exercises what is already built and rebuilds nothing, so refuse
# to vouch for an artifact older than a source it is built from.
binding=$(.venv/bin/python -c 'import rvt._rvt as m; print(m.__file__)')
for artifact in "$bin/rvt-inspect" "$binding" viewer/pkg/rvt_bg.wasm; do
  newer=$(find src rvt-py/src rvt-wasm/src Cargo.toml Cargo.lock -newer "$artifact" -print -quit)
  if [ -n "$newer" ]; then
    printf '%s is older than %s; rebuild first (the rvt: smoke task does)\n' "$artifact" "$newer" >&3
    exit 1
  fi
done
newer=$(find viewer/src viewer/index.html viewer/vite.config.ts viewer/package.json -newer viewer/dist/index.html -print -quit)
if [ -n "$newer" ]; then
  printf 'viewer/dist is older than %s; rebuild first (the rvt: smoke task does)\n' "$newer" >&3
  exit 1
fi
"$bin/gen-fixture" codespace --classes Wall,Level,Project,Column,Door \
  --element-count 25 --year 2024 --output "$evidence/smoke.rvt"
.venv/bin/python - "$evidence/smoke.rvt" >"$evidence/python.json" <<'PY'
import json, rvt, sys
model = rvt.RevitFile(sys.argv[1])
assert model.version == 2024, model.version
assert not model.missing_required_streams(), model.missing_required_streams()
print(json.dumps({"version": model.version, "streams": model.stream_names(), "schema": model.schema_summary()}))
PY
"$bin/rvt-inspect" "$evidence/smoke.rvt" >"$evidence/inspect.txt"
"$bin/rvt-info" "$evidence/smoke.rvt" >"$evidence/info.txt"
"$bin/rvt-ifc" "$evidence/smoke.rvt" -o "$evidence/smoke.ifc" \
  --diagnostics "$evidence/diagnostics.json"
node viewer/scripts/audit-wasm-imports.mjs
node .devcontainer/browser-smoke.mjs "$evidence"
printf 'CLI, Python binding, WASM privacy audit and browser smoke passed.\n'
printf 'Verification passed. Evidence: %s\n' "$evidence" >&3
