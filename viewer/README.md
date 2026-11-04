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
  imports in the compiled WebAssembly — the CI jobs
  `.github/workflows/viewer-test.yml` and
  `.github/workflows/deploy-viewer.yml` fail the build if any show
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

## Progress while a model opens

The loading card shows the decoder's own progress, the stages `rvt-gltf
--progress` prints: the bar fills with a stage's count while it has one
(scanning for elements, decoding elements, reading partition streams, drawing
doors and windows from saved meshes) and sweeps for the rest, and the card
names the stage and counts it. `src/worker.ts` passes a function to
`openRvtBytesWithDiagnosticsModeAndOpenLimits` and posts each report from
inside the wasm call; the page keeps the list on `#status` as
`data-load-progress`. The "about N s" estimate from the file's size is what
the card says until the first report. `tests/load-progress.spec.ts` watches
the card fill on Core Interior. The stages and what the card cannot show are
in [`docs/viewer-api.md`](../docs/viewer-api.md#decode-progress).

## Large models and resource limits

A file opens under the reader's default limits (256 MiB per stream and per
inflated member). When a stream is over them, File status reports "Part of the
file not read" and offers "Retry with large-model limits", which reads the same
file again with the limits `rvt-ifc --large-model` uses (streams up to 2 GiB).
A browser tab can use at most 4 GiB of memory, so a very large model can still
fail to open; `rvt-ifc --large-model` is not held to a tab's 4 GiB (its caps are
a 2 GiB stream and, on a 64-bit machine with the memory, 8 GiB inflated, and a
file over 2 GiB is rejected whatever the flags). The button calls
`openRvtBytesWithDiagnosticsModeAndOpenLimits` with `{ largeModel: true }`.

A file whose size says it may need more than a tab's 4 GiB (an open has taken
at least about ten times the file's size, and the warning can fire on a file
that fits) is warned about in the loading card before the decode, and still
tried. When the wasm instance does run out of memory, the viewer says the model
is too large for the browser, how much memory the tab reached, and what reads a
larger file (`rvt-ifc` and `rvt-gltf` with `--large-model`, up to a 2 GiB file;
the Python `RevitFile` with a larger `max_stream_bytes`, `max_inflate_bytes` and,
over 2 GiB, `max_file_bytes`), instead of
`RuntimeError: unreachable`; the engine's own message is in the status panel.
A stop a long way short of the ceiling on a small file says "Ran out of memory"
and that the file may be damaged, since nothing measured says it needs more. How
it tells, the measured estimate and the options for larger models are in
[`docs/viewer-api.md`](../docs/viewer-api.md#models-too-large-for-the-browser).
`?memoryCeilingBytes=` in the address lowers the ceiling the warning compares
with. `tests/memory-ceiling.spec.ts` uses it for the warning, and
`tests/memory-ceiling-capped.spec.ts` for the stop, in a Chromium whose V8 caps
wasm memory (`--js-flags=--wasm-max-mem-pages=256` is 16 MiB), where Einhoven
really stops, because no model of several hundred MiB is in CI.
`tests/worker-failures.spec.ts` covers the other engines' wording of the trap, a
panic that is not memory, and a stop after the open call, by failing one wasm
export in a replacement worker; nothing in it ships.

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
to `measurements/saved-meshes.json`; `viewer-test.yml` (part of `CI passed`) runs
it in a job of its own and uploads the file. Run it with `RVT_VIEWER_MEASURE=1 pnpm exec playwright test`; the
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
