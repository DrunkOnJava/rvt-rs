# Install Guide

This guide covers the supported ways to install or run rvt-rs on a fresh
machine. Use [user-guide.md](user-guide.md) after installation to decide which
workflow fits your file.

## Browser Viewer

No install is required for the hosted viewer:

1. Open <https://drunkonjava.github.io/rvt-rs/>.
2. Drop a `.rvt`, `.rfa`, `.rte`, or `.rft` file.
3. Confirm the File status panel before exporting.

The viewer is client-side only. It does not upload model bytes.

## Rust CLI From crates.io

After a version is published to crates.io:

```bash
cargo install rvt
rvt-inspect --version
rvt-ifc --help
```

This installs the command-line binaries from the published crate. It requires a
Rust toolchain on the machine. Use [rustup.rs](https://rustup.rs/) if Rust is
not already installed.

Smoke test after installation:

```bash
rvt-inspect path/to/model.rvt
rvt-inspect path/to/model.rvt --json > model.inspect.json
```

## Python Package From PyPI

After a version is published to PyPI:

```bash
python -m venv .venv
. .venv/bin/activate
python -m pip install --upgrade pip
python -m pip install rvt
python -c "import rvt; print(rvt.__version__)"
```

Smoke test after installation:

```bash
python - <<'PY'
import json
import rvt

path = "path/to/model.rvt"
f = rvt.RevitFile(path)
print("version:", f.version)
print(json.loads(f.export_diagnostics_json())["confidence"]["level"])
PY
```

## Build From Source

Use the source path when testing unreleased commits.

```bash
git clone https://github.com/DrunkOnJava/rvt-rs
cd rvt-rs
cargo build --release
./target/release/rvt-inspect --version
```

Run a local source smoke test:

```bash
./target/release/rvt-inspect path/to/model.rvt
./target/release/rvt-ifc path/to/model.rvt -o model.ifc --mode strict \
  --diagnostics model.diagnostics.json
```

## Build Python From Source

Use `maturin` when testing Python bindings from a source checkout:

```bash
python -m venv .venv
. .venv/bin/activate
python -m pip install --upgrade pip maturin
maturin develop --manifest-path rvt-py/Cargo.toml
python -c "import rvt; print(rvt.__version__)"
```

Build a wheel instead of installing into the active environment:

```bash
maturin build --release --manifest-path rvt-py/Cargo.toml --out dist
python -m pip install dist/rvt-*.whl
```

## Build The Viewer Locally

The viewer needs a WASM package and Node dependencies:

```bash
wasm-pack build --target web -- --features wasm --no-default-features
rm -rf viewer/pkg
mv pkg viewer/pkg
cd viewer
npm install
npm run typecheck
npm run build
```

`npm run build` writes the static site to `viewer/dist`. Use
`npm run dev` for local development.

## Post-Publish Verification

Release managers should verify every published artifact from a clean shell. The
full release gate is in [release-checklist.md](release-checklist.md); the short
post-publish smoke is:

```bash
cargo install rvt --version X.Y.Z --locked
rvt-inspect --version

python -m venv /tmp/rvt-release-smoke
. /tmp/rvt-release-smoke/bin/activate
python -m pip install --upgrade pip
python -m pip install "rvt==X.Y.Z"
python -c "import rvt; print(rvt.__version__)"
```

Then open <https://drunkonjava.github.io/rvt-rs/> and confirm the viewer loads.
If a sample file is available, run `rvt-inspect` and the viewer diagnostics
download against the same file and compare the failure mode.
