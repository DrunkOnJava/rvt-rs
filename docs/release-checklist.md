# Release Checklist

Use this checklist for every tagged release. A failed pre-publish check
blocks publication. A failed post-publish check requires a hotfix,
yank, or release-note correction before announcing the release.

Set these variables first:

```bash
export VERSION=0.1.2
export SAMPLE=/absolute/path/to/racbasicsamplefamily-2024.rfa
```

## Pre-Publish Gates

Run the normal source checks on the release commit:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo test --doc
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features -p rvt
```

Verify the crate can install and the CLIs run on a sample:

```bash
rm -rf /tmp/rvt-release-install
cargo install --path . --locked --root /tmp/rvt-release-install
/tmp/rvt-release-install/bin/rvt-inspect --version
/tmp/rvt-release-install/bin/rvt-info "$SAMPLE"
/tmp/rvt-release-install/bin/rvt-inspect "$SAMPLE" --json > /tmp/rvt.inspect.json
/tmp/rvt-release-install/bin/rvt-ifc "$SAMPLE" \
  -o /tmp/rvt.ifc \
  --diagnostics /tmp/rvt.diagnostics.json
test -s /tmp/rvt.inspect.json
test -s /tmp/rvt.ifc
test -s /tmp/rvt.diagnostics.json
```

Build and import the Python wheel locally:

```bash
rm -rf dist /tmp/rvt-wheel-smoke
maturin build --release --manifest-path rvt-py/Cargo.toml --out dist
python -m venv /tmp/rvt-wheel-smoke
. /tmp/rvt-wheel-smoke/bin/activate
python -m pip install --upgrade pip
python -m pip install dist/rvt-*.whl
python - <<'PY'
import json
import os
import rvt

sample = os.environ["SAMPLE"]
f = rvt.RevitFile(sample)
diag = json.loads(f.export_diagnostics_json())
print("rvt", rvt.__version__)
print("version", f.version)
print("confidence", diag["confidence"]["level"])
PY
deactivate
```

Build and load the viewer artifact:

```bash
wasm-pack build --target web -- --features wasm --no-default-features
rm -rf viewer/pkg
mv pkg viewer/pkg
cd viewer
npm ci
npx playwright install --with-deps chromium
npm run build
RVT_VIEWER_SAMPLE="$SAMPLE" npm run test:network
cd ..
```

The tag publish workflow repeats the release-critical checks:

- source crate install + CLI sample smoke;
- wheel install/import on Linux, macOS, and Windows;
- docs build with warnings denied;
- viewer build + browser sample load;
- crates.io publish;
- PyPI/TestPyPI publish.

## Tag And Publish

Confirm `Cargo.toml`, `rvt-py/Cargo.toml`, and Python metadata agree on
the release version, then tag:

```bash
git status --short
git tag "v${VERSION}"
git push origin "v${VERSION}"
```

For TestPyPI dry runs, dispatch `Publish (crates.io + PyPI)` manually
with `test-pypi=true`. The crates.io publish job is skipped for manual
TestPyPI runs.

## Post-Publish Verification

Verify crates.io from a clean shell:

```bash
rm -rf /tmp/rvt-crates-smoke
cargo install rvt --version "$VERSION" --locked --root /tmp/rvt-crates-smoke
/tmp/rvt-crates-smoke/bin/rvt-inspect --version
/tmp/rvt-crates-smoke/bin/rvt-info "$SAMPLE"
```

Verify PyPI on every supported OS family. On each machine:

```bash
python -m venv /tmp/rvt-pypi-smoke
. /tmp/rvt-pypi-smoke/bin/activate
python -m pip install --upgrade pip
python -m pip install "rvt==${VERSION}"
python - <<'PY'
import json
import os
import rvt

sample = os.environ["SAMPLE"]
f = rvt.RevitFile(sample)
diag = json.loads(f.export_diagnostics_json())
print("rvt", rvt.__version__)
print("version", f.version)
print("confidence", diag["confidence"]["level"])
PY
deactivate
```

Verify docs.rs in a browser:

```bash
python -m webbrowser "https://docs.rs/rvt/${VERSION}/rvt/"
```

Verify the hosted viewer:

1. Open <https://drunkonjava.github.io/rvt-rs/>.
2. Drop the same `$SAMPLE`.
3. Confirm the status panel reaches `loaded`.
4. Download diagnostics and confirm the confidence level matches the
   CLI/Python smoke output.

## Release Notes Evidence

Paste a short evidence block into the release notes:

```text
Release verification:
- cargo install rvt --version VERSION --locked: PASS
- Python wheel install/import: PASS on Linux/macOS/Windows
- CLI sample smoke: PASS on SAMPLE_NAME
- Viewer sample load: PASS
- docs.rs page: PASS
- Publish workflow run: URL
```

If any line is not `PASS`, do not announce the release. Open a hotfix
issue, link the failing workflow/log, and decide whether to yank the
artifact or publish a corrected patch release.
