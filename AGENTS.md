# AGENTS.md

Guidance for AI coding agents working in the `rvt-rs` repository.

`rvt-rs` is an Apache-2 clean-room toolkit for inspecting Autodesk Revit files
(`.rvt`, `.rfa`, `.rte`, `.rft`). It has three buildable components:

1. **Rust core library + CLI binaries** (`src/`, workspace root crate `rvt`).
2. **Python bindings** (`rvt-py/` + `python/`), built with `maturin` / `pyo3`.
3. **WebAssembly browser viewer** (`viewer/`), built with `wasm-pack` + Vite.

## Cursor Cloud specific instructions

The Cloud Agent environment is defined by `.cursor/environment.json`, which runs
`.cursor/install.sh` on setup and starts the viewer dev server in a terminal
named `viewer-dev-server`. If setup already ran, the steps below are already
done and you can skip straight to the build/run commands.

### Toolchain requirements (skip the discovery step)

- **Rust must be stable ≥ 1.85.** The crate is `edition = "2024"`
  (`rust-version = "1.85"` in `Cargo.toml`). The base image historically shipped
  Rust 1.83, which **cannot** build this crate. `install.sh` runs
  `rustup default stable` and adds the `wasm32-unknown-unknown` target,
  `rustfmt`, and `clippy`. If you hit an edition-2024 error, run
  `rustup default stable` first.
- **Python** uses a project-local virtualenv at `.venv/` (Debian/Ubuntu images
  need the `python3-venv` apt package, which `install.sh` installs).
- **WASM tooling**: `wasm-pack` (installed by `install.sh`) plus `wabt`
  (`wasm-objdump`, used for the viewer's network-import audit).

### One-shot setup

```bash
bash .cursor/install.sh
```

This is idempotent and safe to re-run. It builds all three components:
Rust (`cargo build --release`), Python (`maturin develop`), and the viewer
(WASM + `npm ci` + `npm run build`).

### Rust: build, test, run the CLIs

```bash
cargo build --release                 # 14 CLI binaries land in target/release/
cargo test --release --lib --bins     # ~755 unit tests
cargo test --release --doc            # doc tests
cargo fmt --all -- --check            # matches CI
cargo clippy --all-targets --all-features -- -D warnings
```

There are no `.rvt` sample files committed (CI pulls external LFS corpora).
Generate a synthetic, license-free fixture with the `gen-fixture` binary and use
it to exercise the CLIs:

```bash
./target/release/gen-fixture demo \
  --classes Wall,Level,Project,Column,Door --element-count 25 --year 2024 \
  --output /tmp/rvt-demo/demo.rvt

./target/release/rvt-inspect /tmp/rvt-demo/demo.rvt
./target/release/rvt-info    /tmp/rvt-demo/demo.rvt
./target/release/rvt-ifc     /tmp/rvt-demo/demo.rvt -o /tmp/rvt-demo/demo.ifc \
  --diagnostics /tmp/rvt-demo/demo.diagnostics.json
```

Tests tagged corpus-dependent (`samples`, `ifc_roundtrip`,
`field_type_coverage`, `cfb_roundtrip_delta`, `project_count_fixtures`, …)
require external datasets (`phi-ag/rvt`, `magnetar-io/revit-test-datasets`) via
`RVT_SAMPLES_DIR` / `RVT_PROJECT_CORPUS_DIR`. Without those env vars they skip
gracefully — this is expected in the Cloud environment, not a failure.

### Python bindings

```bash
. .venv/bin/activate
maturin develop --manifest-path rvt-py/Cargo.toml   # rebuild after Rust changes
python -c "import rvt; print(rvt.__version__)"
python -m pytest tests/python -v                    # corpus tests skip w/o RVT_SAMPLES_DIR
deactivate
```

The compiled extension lands at `python/rvt/_rvt.abi3.so` — it is a build
artifact and is git-ignored (`*.so`). Do not commit it.

### Viewer (WASM + Vite)

Rebuild the WASM package whenever the Rust `wasm` surface (`src/wasm.rs`)
changes, then use the standard npm scripts:

```bash
# From the repo root — build WASM into viewer/pkg/
wasm-pack build --target web -- --features wasm --no-default-features
rm -rf viewer/pkg && mv pkg viewer/pkg

cd viewer
npm ci
npm run typecheck
npm run build          # static site -> viewer/dist
npm run dev            # dev server at http://localhost:5173  (the viewer-dev-server terminal)
```

Privacy invariant (VW1-21): the compiled WASM must import no network
primitives. Verify with:

```bash
wasm-objdump -j Import -x viewer/pkg/rvt_bg.wasm \
  | grep -iE '"(fetch|XMLHttpRequest|WebSocket|EventSource|sendBeacon)"' \
  && echo "VIOLATION" || echo "PASS: no network imports"
```

### Driving the viewer for manual testing

The dev server runs in the `viewer-dev-server` terminal (or start it with
`cd viewer && npm run dev`). To demonstrate it end-to-end:

1. Generate a fixture (see the `gen-fixture` command above), e.g.
   `/tmp/rvt-demo/demo.rvt`.
2. Open `http://localhost:5173/` in Chrome (use the `computerUse` subagent for
   GUI testing).
3. Click the **“Choose file…”** button (or drag-and-drop onto the page) and
   select the fixture. In the GTK file chooser you can type the absolute path
   directly (`/tmp/rvt-demo/demo.rvt`) and press Enter.
4. The parse runs in a Web Worker (WASM). Confirm the right-hand **File Status**
   panel updates — for a `year 2024` synthetic fixture it reports
   `Opened Revit 2024 · 13 streams`, `Schema and model streams found`, and
   `IFC · Scaffold · 25%`, matching the `rvt-inspect` CLI output on the same
   file.

Synthetic fixtures parse cleanly through the whole pipeline but decode as
`scaffold-only` (no validated building elements) — that is the expected result,
not a bug. Real element geometry requires the external corpora.
