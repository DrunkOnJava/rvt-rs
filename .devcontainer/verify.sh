#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
evidence="${1:?Usage: bash .devcontainer/verify.sh ABSOLUTE_EVIDENCE_DIRECTORY}"
[[ "$evidence" = /* ]] || { echo 'Evidence directory must be absolute' >&2; exit 1; }
mkdir -p "$evidence"
exec >"$evidence/verify.log" 2>&1
git rev-parse HEAD >"$evidence/source-sha.txt"
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
