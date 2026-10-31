# rvt-rs viewer

Zero-upload, client-side Revit viewer powered by
[`rvt-rs`](https://github.com/DrunkOnJava/rvt-rs) compiled to WebAssembly.

Hosted at <https://drunkonjava.github.io/rvt-rs/>.

Install and local build paths are documented in
[`docs/install.md`](../docs/install.md).

## Privacy posture (VW1-21)

- **Nothing leaves the browser tab.** The dropped `.rvt` / `.rfa` /
  `.rte` / `.rft` bytes are parsed in-process. There are no `fetch`,
  `XMLHttpRequest`, `WebSocket`, `EventSource`, or `sendBeacon`
  imports in the compiled WebAssembly — the CI job
  `.github/workflows/deploy-viewer.yml` fails the build if any show
  up.
- **No telemetry.** The page CSP blocks `connect-src` to everything
  except `self`, which is only needed for the static WASM bundle,
  and `blob:`, which is needed for the local GLB object URL. No
  analytics, no error reporting, no CDN font loads.
- **Browser-tested invariant.** CI runs
  `viewer/tests/no-network.spec.ts` (runtime traffic) and `viewer/tests/wasm-no-network-imports.spec.ts` (the import audit of `scripts/audit-wasm-imports.mjs`): it opens the built viewer,
  loads a public sample RFA, and fails with request URL plus initiator
  if any external or post-open network request occurs. See
  [`docs/viewer-privacy-posture.md`](../docs/viewer-privacy-posture.md).
- **Deterministic builds.** Every JS bundle is built from a pinned
  action SHA; the WASM comes from a pinned `wasm-pack` release.

## Local dev

```bash
# 1. Build the WASM into viewer/pkg/
(cd .. && wasm-pack build rvt-wasm --target web --out-dir ../viewer/pkg --out-name rvt)

# 2. Install node deps
cd viewer
pnpm install --frozen-lockfile

# 3. Serve with Vite (localhost:5173)
pnpm run dev
```

Drop a Revit file onto the page, or open a redistributable demo from the
gallery (staged from `docs/viewer-demos.json` via `pnpm run stage:demos`).
Nothing is sent anywhere.

## Supported MVP workflow

1. Open a file locally (drop / picker / demo gallery).
2. Read File status: Decode confidence, Export confidence, and IFC bar.
3. Inspect decoded entities in the tree or viewport.
4. Export IFC / glTF / plan only after checking the quality label.
5. Download diagnostics for scaffold or partial exports.

Tier1 redistributable demos (`architectural-2024`, `structural-2023`,
`mep-2024`) are staged from `corpus/tier1/` via `pnpm run stage:demos`.
They open cleanly but report scaffold ~25% confidence — that is expected.
Reliable typed levels/doors/windows from arbitrary projects still depend on
Rust decoder / partition-stream work outside this viewer shell.

The `synthetic-mvp` demo is not checked in — it is generated at staging time
by the `gen-fixture` binary, so build it from the repo root first:

```bash
cargo build --release --bin gen-fixture
```

`pnpm run stage:demos` (also run by `predev` / `prebuild`) looks for the
binary on `PATH`, in `target/release/`, and in `target/debug/`. If it finds
none it runs that cargo build itself; when cargo is unavailable it warns
`synthetic-mvp: missing (gen-fixture not found …)` and the gallery marks the
demo unavailable. Nothing is downloaded either way.

## Large models and resource limits

A file opens under the reader's default limits (256 MiB per stream and per
inflated member). When a stream is over them, File status reports "Part of the
file not read" and offers "Retry with large-model limits", which reads the same
file again with the limits `rvt-ifc --large-model` uses (streams up to 2 GiB).
A browser tab can use at most 4 GiB of memory, so a very large model can still
fail to open; `rvt-ifc --large-model` has no such ceiling. The button calls
`openRvtBytesWithDiagnosticsModeAndOpenLimits` with `{ largeModel: true }`.

`?maxStreamBytes=` and `?maxInflateBytes=` in the address open the first file
under those limits, so a small file reaches the same path. The browser test
`tests/large-model-retry.spec.ts` opens the tier-1 fixture at
`/?maxStreamBytes=200`, which stops its 236-byte BasicFileInfo, and retries.

## Doors and windows from saved meshes

The doors and windows of Revit 2023 to 2027 files are drawn from the meshes
Revit saved for them, where the whole mesh decodes, as `rvt-ifc` and
`rvt-gltf` draw them by default; the others keep their bounding box. File
status has a "Draw doors and windows from saved meshes" option, on by default.
Changing it reopens the open file with `savedMeshes` in the options of
`openRvtBytesWithDiagnosticsModeAndOpenLimits`, and `?savedMeshes=0` in the
address starts with it off (for every file, until the option is turned on).
The Shapes row says how many bodies came from saved meshes.

Reading the saved graphics is a second pass over the file, so it costs time
and memory. `tests/saved-meshes-measure.spec.ts` opens each reference model
with the option on and off in a fresh browser context and writes the load time,
the wasm memory after the load and the number of bodies drawn from saved meshes
to `measurements/saved-meshes.json`; Deploy viewer runs it and uploads the
file. Run it with `RVT_VIEWER_MEASURE=1 pnpm exec playwright test`; the
reference models are read from `_project_corpus/Revit`.

## Architecture

```text
User drops .rvt
     │
     ▼
main.ts ── postMessage ──▶ worker.ts
                                │
                                ├── init wasm (pkg/rvt.js)
                                ├── openRvtBytesWithDiagnosticsModeAndOpenLimits()
                                │                         → IfcModel + diagnostics JSON
                                ├── buildSceneGraph()     → SceneNode JSON
                                ├── modelToGlb()          → Uint8Array
                                └── buildSchedule()       → Schedule JSON
                                │
                          postMessage back
                                │
                                ▼
main.ts
  ├── Three.js scene + OrbitControls + GLTFLoader
  ├── scene tree panel
  ├── category toggles
  ├── export-quality label + IFC bar selector
  ├── File status (decode / export confidence)
  ├── element-info panel (raycast picking) + host / hosted jump rows
  └── status line
```

The worker is mandatory (VW1-19) — parsing a 100 MB RFA blocks the
main thread for seconds otherwise.

## Build outputs

- `viewer/pkg/` — generated by wasm-pack (git-ignored)
- `viewer/node_modules/` — npm (git-ignored)
- `viewer/dist/` — Vite build (git-ignored)

All three are produced by the deploy workflow on every push to
`main`.
