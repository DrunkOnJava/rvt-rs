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
{
  rustc --version; cargo --version; python3 --version; node --version
  npm --version; pnpm --version; wasm-pack --version; gh --version
  codex --version; claude --version
} >"$evidence/versions.txt"
bash .devcontainer/resume.sh
cargo fmt --all -- --check
bash tools/ci/verify-real-files.sh --check-only
bin="${CARGO_TARGET_DIR:-$root/target}/${RVT_BUILD_PROFILE:-ci}"
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
