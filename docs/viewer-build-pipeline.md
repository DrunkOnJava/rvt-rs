# Viewer build pipeline (VW1-01/02/03/18/19/20/23)

The Rust-side viewer data model is complete (see
[`docs/viewer-api.md`](viewer-api.md)). This document spells out
the remaining WASM + JavaScript + deployment work so any
contributor picking up the frontend work has a concrete recipe.

## VW1-01 — WASM build of `rvt-core`

### Cargo.toml changes

Add an optional `wasm` feature alongside the existing `python` one:

```toml
[features]
python = ["dep:pyo3"]
wasm = ["dep:wasm-bindgen", "dep:serde-wasm-bindgen", "dep:js-sys"]

[dependencies]
# existing deps above
wasm-bindgen = { version = "0.2", optional = true }
serde-wasm-bindgen = { version = "0.6", optional = true }
js-sys = { version = "0.3", optional = true }
```

Crate-type already includes `cdylib` (for the pyo3 wheel) so no
change there — the wasm build reuses it.

### `src/wasm.rs` (new)

Thin wrapper around the viewer API. Every function takes JSON in
/ JSON out so the binding layer is mechanical. Example stubs —
actual implementations call through to `crate::ifc::*`:

```rust
//! WASM bindings (VW1-01/02).

#![cfg(feature = "wasm")]

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn open_rvt_bytes(bytes: &[u8]) -> Result<JsValue, JsValue> {
    let mut rf = crate::RevitFile::open_bytes(bytes.to_vec())
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let model = crate::ifc::RvtDocExporter
        .export(&mut rf)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_wasm_bindgen::to_value(&model).map_err(Into::into)
}

#[wasm_bindgen]
pub fn build_scene_graph(model: JsValue) -> Result<JsValue, JsValue> {
    let model: crate::ifc::IfcModel = serde_wasm_bindgen::from_value(model)?;
    let scene = crate::ifc::scene_graph::build_scene_graph(&model);
    Ok(serde_wasm_bindgen::to_value(&scene)?)
}

// Repeat for every function in viewer-api.md:
//   model_to_glb, render_plan_svg, element_info_panel, build_schedule,
//   encode_to_fragment, decode_from_fragment, CategoryFilter::apply,
//   Measurement builders, camera controls.
```

### `src/lib.rs` changes

```rust
#[cfg(feature = "wasm")]
pub mod wasm;
```

### Build commands

```bash
# Install once
cargo install wasm-pack

# Build
wasm-pack build --target web --features wasm --no-default-features
# → pkg/rvt_bg.wasm + pkg/rvt.js + pkg/rvt.d.ts
```

### Makefile target

```make
.PHONY: wasm
wasm:
	wasm-pack build --target web --features wasm --no-default-features
```

### CI check (VW1-01)

Add to `.github/workflows/ci.yml`:

```yaml
wasm-build:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: actions-rust-lang/setup-rust-toolchain@v1
    - run: cargo install wasm-pack
    - run: wasm-pack build --target web --features wasm --no-default-features
    - name: Verify no network imports
      run: |
        # VW1-21 invariant check — no fetch / XMLHttpRequest / WebSocket
        # imports in the compiled .wasm
        wasm-objdump -x pkg/rvt_bg.wasm | \
          grep -E '"(fetch|XMLHttpRequest|WebSocket)"' && exit 1 || true
```

## VW1-02 — JS bindings via wasm-bindgen

The build above produces `pkg/rvt.js` + `pkg/rvt.d.ts`. The JS
surface mirrors `src/wasm.rs` function-for-function with camelCase
names (wasm-bindgen's default).

TypeScript usage:

```ts
import init, { openRvtBytes, buildSceneGraph, modelToGlb } from 'rvt';

await init();  // load the .wasm

const bytes = await file.arrayBuffer();
const model = openRvtBytes(new Uint8Array(bytes));
const scene = buildSceneGraph(model);
const glb = modelToGlb(model);  // Uint8Array
```

## VW1-03 — Three.js integration

```ts
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';

const glb = modelToGlb(model);
const blob = new Blob([glb], { type: 'model/gltf-binary' });
const url = URL.createObjectURL(blob);

const loader = new GLTFLoader();
loader.load(url, (gltf) => {
    scene.add(gltf.scene);
    URL.revokeObjectURL(url);
});
```

Camera binding: the Rust-side `CameraState::eye()` gives the
world-space camera position; apply it to Three.js's `PerspectiveCamera`
or `OrthographicCamera` per `ViewMode::is_orthographic()`.

Click-picking: Three.js `Raycaster` returns the intersected object;
use the `userData.entityIndex` (set when building the scene) to
call `elementInfoPanel(model, entityIndex)` for the info panel.

## VW1-18 — Static site on GitHub Pages

Create a `viewer/` subdirectory with:

```
viewer/
├── index.html
├── main.ts
├── demos/              # content from docs/viewer-demos.json
│   ├── rac_basic_sample_family_2024.rfa
│   └── ...
├── pkg/                # wasm-pack output (git-ignored, built in CI)
└── vite.config.ts      # or any JS build tool
```

GitHub Actions workflow (`.github/workflows/deploy-viewer.yml`):

```yaml
on:
  push:
    branches: [main]
    paths:
      - 'src/**'
      - 'viewer/**'
  workflow_dispatch:

jobs:
  build-and-deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - run: cargo install wasm-pack
      - run: wasm-pack build --target web --features wasm --no-default-features
             --out-dir viewer/pkg
      - uses: actions/setup-node@v4
        with: { node-version: 20 }
      - working-directory: viewer
        run: npm ci && npm run build
      - uses: actions/upload-pages-artifact@v3
        with: { path: viewer/dist }
      - uses: actions/deploy-pages@v4
```

Custom domain + HTTPS handled by GitHub Pages defaults.

## VW1-19 — WebWorker offloading

Large RFAs (multi-megabyte) block the main thread when decoded
synchronously. Solution: run `openRvtBytes` + `buildSceneGraph`
in a dedicated worker.

```ts
// worker.ts
import init, { openRvtBytes, buildSceneGraph } from 'rvt';
self.onmessage = async (e) => {
    await init();
    const model = openRvtBytes(e.data.bytes);
    const scene = buildSceneGraph(model);
    self.postMessage({ model, scene });
};

// main.ts
const worker = new Worker(new URL('./worker.ts', import.meta.url), {
    type: 'module',
});
worker.postMessage({ bytes });
worker.onmessage = (e) => {
    renderScene(e.data.scene, e.data.model);
};
```

## VW1-20 — Progressive streaming

For multi-hundred-megabyte RFAs, even worker execution is slow.
Two approaches:

1. **Partition loading** — `Formats/Latest` + `BasicFileInfo` +
   `PartAtom` arrive first (sub-second), then the scene graph
   streams in element-chunks. Requires Rust-side chunking of
   `build_scene_graph` — not shipped today.
2. **Server-side pre-extraction** — out of scope per VW1-21's
   client-side-only posture. Skip.

First pass: show a progress bar backed by `read_stream`'s
per-chunk inflate; second pass needs a new
`scene_graph::build_scene_graph_chunked` that yields
`SceneNode`s incrementally.

## VW1-23 — Drag-and-drop user RVT support

Straightforward browser pattern:

```ts
document.body.addEventListener('dragover', (e) => e.preventDefault());
document.body.addEventListener('drop', async (e) => {
    e.preventDefault();
    const file = e.dataTransfer?.files[0];
    if (!file) return;
    if (!/\.(rvt|rfa|rte|rft)$/i.test(file.name)) {
        alert('Please drop a Revit file.');
        return;
    }
    const bytes = new Uint8Array(await file.arrayBuffer());
    // Send to worker (VW1-19).
    worker.postMessage({ bytes });
});
```

Combined with VW1-21's client-side-only posture: the dropped file
never leaves the browser.

## Assembly order

A contributor tackling the frontend work ships in this order:

1. VW1-01 (WASM build) — unblocks everything else.
2. VW1-02 (JS bindings) — trivial once VW1-01 lands.
3. VW1-03 (Three.js integration) — core 3D rendering.
4. VW1-23 (drag-and-drop) — users can load their own RVTs.
5. VW1-18 (static site deploy) — share a public URL.
6. VW1-19 (WebWorker) — unblock the main thread on large files.
7. VW1-20 (progressive streaming) — optional, only needed at
   hundred-MB scale.

Steps 1-5 are a 1-2 day sprint; 6-7 are polish.
