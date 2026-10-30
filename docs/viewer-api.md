# Rust-side viewer API reference

The `rvt::ifc` module ships a complete Rust data model for a
browser/desktop 3D viewer, and the `rvt-wasm` crate exposes it to
JavaScript: [The JavaScript bindings](#the-javascript-bindings) below lists
every function the browser viewer can call. The sections before it map the
Rust functions behind them, grouped by the submodules that make up the
viewer surface.

Everything listed here is:

- `Sync + Send` where it's a struct (safe to pass across a
  `wasm-bindgen` boundary via JSON).
- `serde::Serialize + Deserialize` (pass across the JS ↔ WASM
  bridge as JSON, no binary FFI needed).
- Zero I/O (pure data → data transforms; the only I/O is
  `RevitFile::open` at the entry point).

## Input side

Takes bytes or a path, produces an `IfcModel`:

```rust
use rvt::{RevitFile, ifc::{Exporter, RvtDocExporter}};

let mut rf = RevitFile::open_bytes(user_uploaded_bytes)?;
let model = RvtDocExporter.export(&mut rf)?;
// `model: IfcModel` is the viewer's single source of truth.
```

## File identity

`fileMetadata(bytes)` returns the document identity
(`rvt::metadata::FileMetadata`, schema
[`file-metadata.schema.json`](schemas/file-metadata.schema.json)): release,
build, title, last saved time, worksharing state, central model path,
last-saved-by user, document GUID, save counter, and every `BasicFileInfo`
`Key: value` line. It reads only the two identity streams straight from the
byte slice, so the viewer worker sends it with the fast `summary` message
and the File status panel shows the Saved and Worksharing rows before the
model finishes parsing.

## Schedules

`scheduleCsv(model, kind, metric, excel)` returns a CSV schedule of an
exported model (`rvt::ifc::schedule_csv`): `kind` is `"elements"` or
`"rooms"`, `metric` switches lengths to metres, `excel` prefixes a UTF-8
byte-order mark. The viewer's Schedule panel downloads both with
`excel = true`, since most people open them in Excel.

## Scene graph

```rust
use rvt::ifc::scene_graph::{
    build_scene_graph, SceneNode,
    distinct_ifc_types,
    element_info_panel, ElementInfoPanel,
    build_schedule, Schedule, ScheduleRow,
};
```

- `build_scene_graph(&IfcModel) -> SceneNode` — project → storey
  → element tree with hosted doors/windows nested under their
  wall.
- `distinct_ifc_types(&SceneNode) -> Vec<String>` — populate the
  layer-toggle UI.
- `element_info_panel(&IfcModel, entity_index) -> Option<ElementInfoPanel>` —
  click-to-inspect payload. Every field arrives display-ready, so
  the frontend never re-derives a unit, a rounding rule, or an
  index → name lookup:
  - `host` / `hosted` carry the wall a door or window sits in and
    the openings a wall carries, each as a
    `RelatedElement { entity_index, name, ifc_type }` the viewer
    uses to re-select the other end of the relationship.
  - `joins: Vec<PanelJoin { end, runs_through, wall }>` carries the
    walls a wall butt-joins at its `"Start"` and `"End"`, where its join
    lists decide which one runs through (RE-70), or stops against part
    way along the other (a T joint, RE-73): `runs_through` is true
    where this wall runs on to the other's far face and false where it
    stops at its near face, and `wall` is the other wall as a
    `RelatedElement`. The viewer lists them under the host rows.
  - `storey: Option<PanelStorey { index, name, elevation_feet, elevation_label }>`
    resolves `storey_index`; `index` addresses the same
    `IFCBUILDINGSTOREY` scene node, so the viewer can jump to it.
  - `material: Option<PanelMaterial { index, name, element_count }>`
    resolves `material_index` and counts the elements using it, directly
    or as one of their layers.
  - `layers: Option<PanelLayers { name, order, rows, total }>`
    is the element's material layer set (RE-58): `rows` are
    `PanelLayer { label, value, material }`, each a material name (or
    `Material not read`), its thickness in feet and, where the material
    is known, `PanelMaterial { index, name, element_count }` to highlight
    it by, in the set's order;
    `order` says which side comes first (`exterior first` for a wall,
    `top first` for a floor, roof or ceiling); `total` adds them up.
  - `property_group: Option<PanelPropertyGroup { name, properties }>`
    is the element's property set as one titled group of
    `PanelProperty { name, value, kind, numeric }` rows — values
    carry their unit, booleans read as `Yes` / `No`.
  - `further_property_groups: Vec<PanelPropertyGroup>`: the element's
    standard property sets beside its own, such as `Pset_StairCommon`
    (RE-47); empty when there are none.
  - `placement_rows` / `extent_rows` are `PanelRow { label, value }`
    lists in feet (location X/Y/Z, rotation in degrees, extrusion
    width/depth/height plus the profile shape).
  - `missing` names the absent optionals from the fixed set
    `storey`, `material`, `properties`, `placement`, `extents`.
    The viewer reports one as "not recovered" only where the
    export diagnostics agree it is a known decode gap.
- `build_schedule(&IfcModel) -> Schedule` — tabular element export
  (the CSV writer is `schedule_csv`, above). `Schedule::groups` is the
  per-IFC-type breakdown (`ScheduleTypeGroup { ifc_type, count, entity_indices,
  storeys }`, biggest first) that drives the viewer's schedule
  panel and its per-type scene highlight. Rebuild it with
  `Schedule::from_rows` after resorting or filtering rows.

## Viewer state

```rust
use rvt::ifc::camera::CameraState;
use rvt::ifc::clipping::{SectionBox, ViewMode};
use rvt::ifc::scene_graph::CategoryFilter;
use rvt::ifc::share::{ViewerState, decode_from_fragment};
```

The plain-data pieces of a shared viewer state, which a URL fragment
carries as base64-encoded JSON. The viewer does not call them: it drives
its own camera and layer toggles in `viewer/src/main.ts`, and nothing in the
Rust crate moves these values.

- `CameraState { target, distance, yaw, pitch, fov_radians, near, far }`.
- `ViewMode { Plan, ThreeD, Section }`.
- `SectionBox { min, max }`.
- `CategoryFilter { hidden }` — the upper-case IFC types the layer
  toggles hide.
- `ViewerState { file_hash, camera, view_mode, section_box,
  category_filter, selected_name }`.
- `decode_from_fragment(fragment) -> Option<ViewerState>` — base64-of-JSON;
  strips `#`, `v=`, `#v=` prefixes and returns `None` for anything that
  is not a current `ViewerState`.

## Materials (PBR)

```rust
use rvt::ifc::pbr::PbrMaterial;
```

- `PbrMaterial::from_material_info(&MaterialInfo) -> PbrMaterial`
- Name-driven classifier: glass → double-sided + roughness 0.05,
  metal → metallic 1, wood → roughness 0.7, concrete → 0.9,
  paint/ceramic/tile → 0.5, default → 0.6.
- sRGB → linear color conversion on color unpack.

## Rendering outputs

### 3D (glTF 2.0 binary)

```rust
use rvt::ifc::gltf::model_to_glb;
let glb_bytes = model_to_glb(&model);
// Write to disk or pass to Three.js's GLTFLoader via Blob.
```

`build_gltf` writes `extras: { entityIndex, ifcType }` on every
element node, which glTF loaders surface as `userData`. That is what
raycast picking, the selection highlight, the category toggles and
the schedule's per-type highlight all match on; before #272 no
`extras` were written at all, so those behaviours were inert against
the 3-D scene.

Each element is drawn as the body the IFC export gives it
(`ifc::body_geometry`): a sketched slab as its
profile with its holes open, a steel section as its section, a swept,
revolved or brep solid as that solid, all turned by the element's
`rotation_radians`. A plain rectangular extrusion shares one unit cube
scaled by its node's matrix. An element with no body keeps its node,
with no mesh. A family instance whose type draws in exactly one material
(RE-82) is drawn in that material's colour when the colour was read
(RE-53); one drawn in several materials keeps its category's, since a
single body cannot show which part is which. An element with no material
of its own is drawn in its category's colour (`gltf::category_colour`, the plan's hues; rooms faint,
windows and curtain panels translucent). Element nodes are in the model's
frame (feet, +Z up) and
hang under one root node whose matrix, `gltf::Z_UP_FEET_TO_Y_UP_METRES`,
turns them into glTF's metres with +Y up.

### 2D (SVG plan view)

```rust
use rvt::ifc::sheet::{render_plan_svg, SheetOptions};
let svg_string = render_plan_svg(&model, &SheetOptions::default());
```

Each element is drawn as what its body covers in plan: an extrusion's
profile turned by the element's rotation, holes left open, and the
convex hull of any other solid. Slabs, roofs, coverings, plates and
spaces are drawn first, beneath the walls, columns and doors.

### IFC4 STEP

```rust
use rvt::ifc::step_writer::write_step;
let step_text = write_step(&model);
```

## Panic reporting and wasm32 memory

Three behaviours matter only in the browser build and are easy to
mistake for viewer bugs.

**Each thread initialises its own instance.** A `--target web` module has
to be initialised (`await init()`) in every JavaScript realm that calls
it. The worker parses; the main thread calls `modelToIfcStep` and
`renderPlanSvg` for the export buttons and initialises its own instance
first (`mainThreadWasm()` in `viewer/src/main.ts`). Calling an export
without that fails with `Cannot read properties of undefined (reading
'__wbindgen_add_to_stack_pointer')`.

**Panics reach the console.** Every binding installs a
`std::panic::set_hook` on its first call, guarded by a
`std::sync::Once` per wasm instance, that writes
`rvt-rs wasm panic: <info>` through `console.error` (#256). The worker
and the main thread each run their own instance, so the main-thread
export bindings (`modelToIfcStep`, `renderPlanSvg`) install it too.
The hook reports the panic message and its source location; without
it a panic surfaced only as `RuntimeError: unreachable`. Ordinary
parse failures are unaffected: they still return `Err(JsValue)`, which
wasm-bindgen throws as a JS error with the `rvt::Error` text.

**Decompression reserves at most 1 MiB per gzip member.**
`inflate_at_with_limits` used to reserve
`min(4 x remaining input, 256 MiB)` per member, and
`inflate_all_chunks_with_limits` keeps every member's buffer alive
in its result list, so a stream with dozens of members reserved
O(members x stream) bytes. Native allocators back untouched capacity
lazily — `rvt-ifc` peaked at 427 MB RSS on `2024_Core_Interior.rvt`
(33.7 MB) — but wasm32 linear memory commits every grown page, so the
same call grew the module to 4057 MB in 139 ms and trapped at the
4 GiB ceiling. The reservation is now capped by
`INITIAL_INFLATE_CAPACITY_BYTES` (1 MiB) as well as by the caller's
output limit, and the retained buffer is `shrink_to_fit`. Measured
after that change (#256) and before #266: Core Interior decoded under
Node in 29 s at 419 MB of linear memory, and the viewer loaded it in
headless Chromium in 28 s with 889 entities and 854 elements carrying
geometry; `Revit_IFC5_Einhoven.rvt` dropped from 43 MB to 20 MB.
Decoded bytes are unchanged — only the allocation strategy moved.

**Since #266 the same file decodes in about 3 s.** Each
`Partitions/*` stream is inflated once per file and memoised on the
`RevitFile` handle, instead of once per partition consumer and again
for the diagnostics pass; string candidates are screened on the raw
UTF-16 code units before any decode, and the category sweep uses
`memchr::memmem`. Natively, `rvt-ifc --mode geometry` on Core
Interior went from 26.07 s / 2641.4 MiB peak RSS to **1.69 s /
490.1 MiB** (Apple Silicon, `/usr/bin/time -l`, best of three), and
the emitted IFC is byte-identical. In the browser the 33.7 MB demo
decodes in **about 3 s**, measured on the deployed site — 6.7 s
including the 32 MB download. That measurement is what
`DECODE_BYTES_PER_SECOND` in `viewer/src/main.ts` encodes (11 MB/s
since #271, up from 1.15 MB/s) to set the loading card's "about N s"
hint; the progress bar stays indeterminate because the decoder
cannot report real progress.

## Full frontend pipeline

```rust
// 1. Read user-uploaded bytes.
let mut rf = RevitFile::open_bytes(bytes)?;
let model = RvtDocExporter.export(&mut rf)?;

// 2. Build viewer data.
let scene = build_scene_graph(&model);
let types = distinct_ifc_types(&scene);   // for layer panel
let schedule = build_schedule(&model);    // for schedule panel

// 3. Render initial view.
let glb = model_to_glb(&model);           // Three.js loads this
let svg = render_plan_svg(&model, &SheetOptions::default());

// 4. Bind interactions.
//    - Click element → element_info_panel(&model, entity_index)
```

## The JavaScript bindings

`rvt-wasm/src/lib.rs` exports these 12 functions through `wasm-bindgen`, and
`wasm-pack` builds them into `viewer/pkg/`. They are the wasm API that 1.0
keeps stable (`RELEASE.md`, "What 1.0 keeps stable"): CI fails a pull request
that removes or renames one, removes, moves or retypes a parameter, adds one
that cannot be left out, or changes what one returns, unless its title marks
the breaking change with `!` (`tools/ci/wasm-api.py`). An argument or result
is what `serde-wasm-bindgen` makes of the Rust value: an object for a struct,
an array for a list, and a `Map` for a map, such as the diagnostics'
`exported.by_ifc_type` or the model's `global_ids.elements`. `JSON.stringify`
writes a `Map` as `{}`, so the viewer turns the diagnostics' maps into objects
before it shows or saves them. A function that fails throws a string
describing the error.

<!-- wasm-bindings -->
| Binding | Arguments | Returns | What it does |
|---|---|---|---|
| `openRvtBytesWithDiagnosticsMode` | `bytes`, `mode: string` | `{ model, diagnostics }` | The same at an export quality mode: `scaffold`, `typed-no-geometry`, `geometry` or `strict`, the stronger ones failing when the file cannot meet them. Doors and windows are drawn from their saved meshes, as in the next row with the default options |
| `openRvtBytesWithDiagnosticsModeAndOpenLimits` | `bytes`, `mode: string`, `limits: { largeModel?, maxStreamBytes?, maxInflateBytes?, savedMeshes? }` or `null` | `{ model, diagnostics }` | `openRvtBytesWithDiagnosticsMode` under the reader's resource limits: `largeModel` is `rvt-ifc --large-model` (streams up to 2 GiB, inflating up to the 4 GiB wasm32 holds, the walker's scan cap lifted to match), `maxStreamBytes` and `maxInflateBytes` are whole numbers of bytes that win over it, and a field left out keeps its default. `savedMeshes` (default `true`) draws the doors and windows of Revit 2023 to 2027 files from the meshes Revit saved for them, as `rvt-ifc` does by default; `false` keeps their bounding boxes and skips the pass over the saved graphics. A stream a limit stops is listed in the diagnostics (`stream_not_read_resource_limit`, a `Not read whole:` warning each) and the viewer offers "Retry with large-model limits"; unknown fields and fractional or negative limits throw. When the saved graphics cannot be read the model keeps its boxes and the diagnostics carry a warning |
| `quickSummary` | `bytes` | `Summary` | Release, build, PartAtom and the stream inventory, read without decoding the model |
| `fileMetadata` | `bytes` | `FileMetadata` | Document identity (release, worksharing, last save, GUID, save counter), from the two identity streams only |
| `scheduleCsv` | `model`, `kind: string`, `metric: boolean`, `excel: boolean` | `string` | The element (`kind` `"elements"`) or room (`"rooms"`) schedule as CSV, the columns of the [user guide](user-guide.md#get-a-schedule-into-excel) |
| `buildSceneGraph` | `model` | scene tree | The model's scene graph |
| `distinctIfcTypes` | `scene` | `string[]` | The IFC types in a scene, for the layer toggles |
| `elementInfoPanel` | `model`, `entity_index: number` | info panel | What the info panel shows for one element |
| `buildSchedule` | `model` | schedule table | A flat table of every building element |
| `modelToGlb` | `model` | `Uint8Array` | The model as a glTF 2.0 binary |
| `modelToIfcStep` | `model` | `string` | The model as an IFC4 STEP document |
| `renderPlanSvg` | `model`, `options: SheetOptions` or `null` | `string` | A plan view as SVG |
