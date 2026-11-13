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
  bridge as JSON, no binary FFI needed), except `ElementParameter`, which
  is `Serialize` alone, a read result the viewer never sends back.
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
byte-order mark. The element schedule lists the elements the IFC export
writes, each on the storey it puts it on, so a piece the export folds into
another element's body has no row. The viewer's Schedule panel downloads both
with `excel = true`, since most people open them in Excel.

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
  wall where it is on the same storey. An element is under the storey
  the IFC export puts it in (`step_writer::storey_of` over
  `step_writer::placements`, the writer's own decision, which the MCP
  server and the elements CSV read as well): the storey it is contained
  in, that of its room, that of its whole (a stair's run, landing or
  stringer, or a curtain wall's member) or that of the host an opening
  cuts. An element the
  export contains in the building only, with no storey, is under the
  synthetic `Unassigned` storey. A piece the export folds into the body of
  another element (a further piece of an element of separate loops) is not
  an element there and has no node.
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
    is the storey the IFC export puts the element in (`storey_of`: its own,
    or that of its room, its whole or, for an opening, its host), the one
    its row has in the schedule and the tree; `index` addresses the same
    `IFCBUILDINGSTOREY` scene node, so the viewer can jump to it. It is
    `None` where the export contains the element in the building only.
    `storey_name` and `storey_elevation_feet` are the same storey. A piece
    the export folds into the body of another element is still drawn, so a
    click can pick it, and its panel is that of the element whose body it is
    part of.
  - `entity_index` is the index of the element the panel describes: the one
    asked for or, for a folded piece, the element that owns it. The viewer
    selects that element (its tree row, its GlobalId, and a highlight and a
    zoom over the element and all its pieces, `IfcModel::body_pieces`), so a
    click on any loop of a ceiling of separate loops selects the ceiling.
    Serialised models from before it carry none: it reads as 0.
  - `extent_rows` is empty for an element with folded pieces: the export
    writes its body as several items, which one width, depth and height
    cannot state, and the elements CSV leaves the same size cells empty. Its
    `missing` does not list `extents`.
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
  (the CSV writer is `schedule_csv`, above): one row for each element the
  IFC export writes, each on the storey it puts it in (`ScheduleRow::storey`,
  empty where the export contains it in the building only), so a piece the
  export folds into another element's body has no row. `Schedule::groups` is the
  per-IFC-type breakdown (`ScheduleTypeGroup { ifc_type, count, entity_indices,
  storeys }`, biggest first) that drives the viewer's schedule
  panel and its per-type scene highlight. Rebuild it with
  `Schedule::from_rows` after resorting or filtering rows.

## Saved parameters

The info panel lists an element's saved parameters (#35, RE-207) under
"Saved parameters", beside the property sets the IFC export writes. They are
not part of `element_info_panel`, which stays what the export holds: they are
read when an element is selected, by `savedParameters`, in a worker of its
own (`viewer/src/parameters-worker.ts`).

`savedParameters(bytes, ids, limits)` returns JSON text (not a JS object, so
no `Map` or large number is made of it). `ids` is a `Float64Array` of
ElementIds. The keys are the binding's contract under `RELEASE.md`:

```json
{
  "release": 2023,
  "release_read": true,
  "release_experimental": false,
  "releases_read": [2015, 2027],
  "releases_measured": [2023, 2027],
  "elements": { "123456": [ { "name": "Mark", "parameter_id": 4711, "kind": "family", "...": "..." } ] },
  "unresolved_family_slots": 0,
  "instance_parameters_without_value": 0
}
```

- `elements[id]` holds the entries `rvt-elements` prints as that element's
  `parameters` (`docs/schemas/decoded-elements.schema.json`; the same Rust
  value, `rvt::research::native_element_parameters::ElementParameter`):
  `name` (the definition's caption, or `null` for a built-in parameter, which
  keeps its negative `parameter_id` and no name, #255), `kind`
  (`built_in`, `family`, `shared`, `global`, `project`, `undefined`),
  `storage_type`, `value` in Revit's internal units, `si_value` and
  `si_unit` for a length, area, volume or angle, `target_name` and
  `target_family` for an ElementId that names a material or a family type,
  `level` (`instance`, or `type` for the values an instance takes from its
  type), and the record the value came from. An element with no record is not
  in `elements`; one whose record holds none has an empty list.
- `release_read` is false, and `elements` empty, for a file whose release is
  outside `releases_read` (Revit 2015 to 2027). `release_experimental` is true
  for a release in `releases_read` that `releases_measured` (Revit 2023 to
  2027) leaves out: Revit 2015 to 2022 files are read experimentally, since no
  Revit export of those releases saves a parameter to check them against
  (RE-281, `docs/support-matrix.json`).
- `unresolved_family_slots` and `instance_parameters_without_value` count what
  the read could not list for these elements, so the panel can say so.

The panel shows the instance parameters and then the type parameters, named
ones first, with the nameless ones behind a fold that groups them by why they
have no name (a built-in parameter's name is not in the file, #255; an id the
file holds no definition for). Each row says in words where its parameter is
defined, and a value reads as saved, with its unit where the entry has one:
`3 ft · 0.9144 m`, `Yes`, `Wall type · ElementId 123`. It says so when the file
is of a release that is not read, when the release is read only
experimentally, when the element has no saved parameters, when the read failed
(with "Try again"), and while it runs; a button that is pressed is gone, so
focus moves to the group's message and then to its result, and a live region
announces the read and its result.

**Nothing is requested once the first file is open.** Both workers, the open's
and this one, are bundled into the page's own script (Vite's `?worker&inline`)
and start from blob URLs, which the page's `Content-Security-Policy` allows
with `blob:` in `worker-src` (`docs/viewer-privacy-posture.md`); the page's other
directives are unchanged. The decoder's glue is a static import of the page. The
first open's worker fetches the wasm once (the page tells it where it is, as a
blob worker has no base to resolve against), compiles it
(`WebAssembly.compileStreaming`) and hands the module to the page, which keeps
it: the next open, the workers that read, and the main thread's IFC, plan and
CSV exports start from it (`initSync({ module })` in a worker, the module given
to `default` on the main thread, whose synchronous instantiation of a large
module is not allowed), so there is no chunk or wasm to fetch. A page taken
offline after it has loaded still reads, exports and opens the file again
(`viewer/tests/parameters-row.spec.ts`, `no-network.spec.ts`). Every lazy load in
`viewer/src` that could run after the open is therefore gone: the main thread's
`import('../pkg/rvt.js')` is static, the workers' scripts are inlined, and the
wasm's `new URL(..., import.meta.url)` is resolved once by the page. What still
fetches is opening a gallery demo, which is an open, and the catalog and
thumbnails the page loads to show the gallery.

The read is a pass over the file, so it does not run in the worker that opened
it: that worker would be held up for as long as it takes, and the wasm linear
memory of an instance only grows. A worker is started for one element, reads
the file again from the `File` the page already holds, and ends with its
answer (or when another element is selected, or a storey or category, or the
panel is replaced by one with no parameters), so the page holds no more of the
file than before and a read costs memory only while it runs. A worker the
browser ends without a word (it raises no event) is found by a Web Lock it
holds, so the group cannot say "Reading" for ever.

**When a selection is read on its own.** A read has taken 0.74 to 1.04 times
the wasm memory of the worker that opened the file, a ratio that rises with the
file, and nothing above the BIM4LCA office's 910 MiB has been measured. The
page therefore reads on selection only while the wasm memory it holds (the
open's, and the main thread's instance if an export made one) is at most 1 GiB
(or a quarter of `?memoryCeilingBytes=` where that lowers the 4 GiB ceiling),
and shows a button above that, saying that clicking may close the tab: a
renderer that runs out of memory takes the tab and the open model with it, with
no error to show. After a read has run out of memory, or its worker has gone
without a word, that file is read only when asked.

The page keeps each element's answer, up to 64 elements and 8 Mi characters of
JSON between them (the answers measured below are about 2,300 characters each),
the least recently selected going first and the answer just read always staying,
until a file is opened again, so an element selected a second time reads
nothing. The page's address (`?parametersRemembered=`,
`?parametersRememberedCharacters=`) can lower either limit, for the browser
tests, and never raises it.

Measured in the viewer tests, run 42125048710 on head 3146f14e (CI runner, WASM without LTO): a read's wasm
instance took 17 MiB on Einhoven (the open's worker 23 MiB), 275 MiB on Core
Interior (326 MiB) and 943 MiB on the BIM4LCA office as built (910 MiB), 0.74
to 1.04 times the open's, in a median of 153 ms, 1.5 to 1.9 s and 3.9 s (the
module is handed over, so a read starts without a fetch or a compile; the
times are a CI runner's and vary with it). On the office the page kept 2
answers, 4,654 characters of JSON. Under a V8 cap of 927 MiB of wasm memory,
between the office's open and its read, the open succeeds and the read runs out
of memory for real (`viewer/tests/parameters-row.spec.ts`).

**What the first page load carries.** The page's one script is 999.56 kB (271.47
kB gzipped) where the build of d1de8adf (run 42109657349) had 959.29 kB (262.82
kB): 40.27 kB, 8.65 kB gzipped, more. Most of it is the decoder's glue (12.91
kB) and the two workers (12.43 and 9.82 kB), which were fetched as files the
first time they were needed and are now inside it (a worker's base64 is a third
larger than its script); the rest is this round's panel code. The built site
holds the script, the stylesheet, the page and the wasm, whose one sha256
`check-wasm-in-site.sh` finds in the site (run 42125048710, build of this
head). The wasm is fetched once, by the first open.

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

Every material carries `extras: { colourSource, alphaSource }`
(`gltf::MaterialExtras`, #1812), which loaders surface as the material's
`userData`. Each says where its value comes from:

- `"file"`: the Revit material's own colour, or one minus its own
  transparency (RE-53).
- `"rvt-rs"`: rvt-rs chose it. That covers the category colours above,
  translucent windows and faint rooms included, the mid-grey of a material
  whose colour was not read, and the opaque alpha of one whose transparency
  was not read.

The two are marked apart because a material decoded from `Global/Latest`
reads its colour and its transparency from two fields, so either can be read
without the other. A consumer that stores colour or opacity as model data
should keep only what is marked `"file"`. Metallic and roughness are always
rvt-rs's, guessed from the material's name.

A view's visibility and graphics overrides (a category or element set to
50% transparency in one view, say) are not read, and nothing in the GLB or
the IFC reflects them. Revit's own IFC export ignores them too. In Autodesk's
`revit-ifc`, `BodyExporter.GetBestColourAndOpacity` takes an element's colour
and opacity from its material, or else from its category's material, and
`CategoryUtil` writes `IfcSurfaceStyleRendering.Transparency` from the
material's own transparency.

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
since #271, up from 1.15 MB/s) for the "about N s" hint the loading card
opens with. It is a laptop's figure: Deploy viewer's runner takes about
24 s to open the same file (the measurement in
[Models too large for the browser](#models-too-large-for-the-browser)), so
since #1195 the card shows the decoder's own progress as soon as it reports,
and the hint is only the first thing it says.

## Export phase

A Revit project can hold several phases, and an export shows one: the last, as
Revit's own does. An element that does not exist in the export phase, and a
room whose own phase is not it, are left out, and the diagnostics count them
(#1339, RE-326). `limits.phase` of
`openRvtBytesWithDiagnosticsModeAndOpenLimits` exports another, as `rvt-ifc
--phase` and the Python API's `phase` do: a phase's name (matched exactly
first) or its ElementId as a number. Left out, the export is the last phase,
so a call without it returns what it always did, except that a project whose
last phase holds no room has the rooms of the latest phase that holds any
(`diagnostics.room_phase_fallback` names the phase and counts them, #1359); a
phase passed is exact. A phase the file lacks
throws, and its message lists the file's phases.

The result carries `phases`, the project's phases in order, `[{ id, name? }]`,
the last being the default export's; it is empty for a family, for Revit 2014
and earlier, and for a file whose phases could not be read. The viewer lists
them in the File status panel ("Phase", the last marked as the default) and
reads the file again with the chosen phase, as it does for the saved-mesh
option; the model, the schedules and every export it offers are then that
phase's. A file with one phase shows it fixed, and a file with none hides the
field. The worker forwards `limits.phase` from its `parse` message and sends
`phases` with `ready`.

## Decode progress

`openRvtBytesWithDiagnosticsModeAndOpenLimits(bytes, mode, limits, progress)`
takes an optional `progress` function, called as `progress(stage, done, total)`
while the decoder works (#1195). They are the reports of the decoder's
`WalkerControl` (#1072), the ones `rvt-gltf --progress` prints, passed across
the wasm boundary:

| `stage` | `done` of `total` | The card shows |
|---|---|---|
| `schema-parse` | 0, then 1 of 1 | Reading the schema |
| `candidate-scan` | bytes of `Global/Latest` scanned, of its size | Scanning for elements |
| `element-decode` | scan candidates decoded, of the candidates | Decoding elements |
| `partition-scan` | 0 with no total where the walk reaches the partitions; then each `Partitions/*` stream inflated for the first time, of all of them (the first pass over the partitions inflates every one, and the handle keeps them, #266); 1 of 1 where the walk ends | Reading partition streams |
| `saved-graphics` | 0, then 1 of 1 | Reading saved graphics |
| `saved-meshes` | elements meshed, of the elements | Drawing family instances from saved meshes |

`total` is `null` where a stage has none yet. The decoder reports every
megabyte scanned and every 256th candidate; the binding calls `progress` at a
stage's start, at its end and each time a stage with a total passes another
whole percent, so a stage costs at most about a hundred calls and a decode a
few hundred. The function is held in a `thread_local` and the closure handed to
`WalkerControl::with_progress` captures nothing, because a `js_sys::Function`
is not `Send + Sync`. The worker posts each report from inside the wasm call
(`{ type: 'stage', stage, done, total, wasmMemoryBytes }`), and the browser
delivers it to the page while the call runs, because posting does not wait for
the worker's event loop.

The bar fills while a stage has a total above one and sweeps otherwise. The
decoder reports nothing between the last partition stream's inflate and the
saved graphics (the model is built there) or after the saved meshes (the
diagnostics and the copy out of wasm), so the card names those stretches
"Building the model" and "Finishing the model" and sweeps; the steps after the
open call (scene graph, glTF, schedule) are the worker's own and sweep too.
How much of an open that is, measured, is under
[Models too large for the browser](#models-too-large-for-the-browser). The
viewer does not cancel a decode: the call is synchronous in the worker, so a
`CancelToken` has no one to set it. The "about N s" hint from the file's size
is what the card says until the first report.

## Models too large for the browser

A wasm32 instance addresses at most 4 GiB, and a single `Vec` cannot pass
2 GiB (`isize::MAX`). A `Vec` growing past that panics with `capacity overflow`,
which the panic hook writes to `console.error`; an allocation that
`memory.grow` cannot satisfy does not panic: Rust aborts, no hook runs, and
JavaScript sees `RuntimeError: unreachable` and nothing else. During an open the
file is also held twice in wasm memory: the binding copies `bytes` in, and
`RevitFile::open_bytes_with_limits(bytes.to_vec())` copies it again.

What the viewer does (#1195):

- **Before the decode**, from the file's size alone: it expects the open to need
  at least about ten times the file (`PEAK_MEMORY_PER_FILE_BYTE` in
  `viewer/src/main.ts`), and when that passes 4 GiB (a file over about 410 MiB)
  the card says the file may be too large, with the estimate and the figure it
  rests on, and the open goes ahead. It is a warning, never a refusal, and it
  can fire on a file that fits (below).
- **When the instance stops**, the worker decides whether it ran out of memory:
  a `RuntimeError` of the trap with no `rvt-rs wasm panic:` line on
  `console.error` before it (`unreachable` in V8, `unreachable executed` in
  Firefox, `Unreachable code should not be executed` in Safari, matched as
  `/unreach/i`), or a panic or engine message that names an allocation
  (`capacity overflow`, `memory allocation of N bytes failed`, `Array buffer
  allocation failed`). Any other panic is a bug, and the page shows the panic's
  own message and nothing about memory. The worker reports the wasm memory the
  instance had reached, the decoder's last report (so a stop in the stretch it
  reports nothing in says "building the model") or, once the open call has
  returned, the worker step it was in, and the engine's own message, which the
  status panel shows as "Engine message". The worker is ended afterwards.
- **What the page says of a memory stop** depends on how far the instance got.
  When it reached half the ceiling, or the file's estimate passes the ceiling,
  the page says "Too large for the browser": how far it got, that a tab holds at
  most 4 GiB and an open has taken at least about ten times the file, and what
  reads a larger file as the code takes it: `rvt-ifc` and `rvt-gltf` with
  `--large-model` (streams up to 2 GiB and, on a 64-bit machine with the memory,
  inflating up to 8 GiB; they have no flag for `max_file_bytes`, the 2 GiB cap on
  the whole file, so a larger file is rejected whatever the flags), and the
  Python `RevitFile` with a larger `max_stream_bytes` and `max_inflate_bytes`,
  and `max_file_bytes` for a file over 2 GiB; it has no `--large-model` preset.
  The threshold is half the ceiling because one wasm32 allocation cannot pass
  2 GiB (`isize::MAX`): an allocation that failed with under 2 GiB already held
  was not stopped by the 4 GiB ceiling alone, while one that failed with 2 GiB
  or more held can have been. A browser that gives a tab less than 4 GiB (a
  phone, say) stops below half of the 4 GiB the page assumes, and gets the
  hedged wording for a model that is truly too large for it. When the instance
  stopped a long way short of the ceiling on a file whose estimate fits, such as
  a damaged file tripping `capacity overflow` at 60 MiB, a claim that the file
  needs more memory would be one nothing measured, so the page says "Ran out of
  memory", that the file may be damaged or may have hit a limit other than the
  browser's memory, and that running the CLI on it tells which. A tab the
  browser kills outright sends nothing; the page shows an error for a worker
  that stops.

The pre-check's figure is measured, not assumed. Deploy viewer's
`tests/saved-meshes-measure.spec.ts` opens each reference model in a fresh
Chromium 156 context on GitHub's ubuntu-24.04 runner (4 CPUs, 16 GB) and
records the wasm linear memory after the open, which only grows and so is the
peak, beside the estimate (ten times the file) and their ratio (Deploy viewer run
[41700152800](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41700152800),
saved meshes on; the same peaks with them off, and the same in the two runs
before it):

| Model | File MiB | Wasm peak MiB | Peak / file (about) | Estimate MiB | Estimate / peak |
|---|---|---|---|---|---|
| 2024_Core_Interior | 32 | 324 | 10 | 322 | 0.99 |
| RE1-Electrical | 13 | 170 | 13 | 128 | 0.76 |
| RE1-Architecture | 6 | 100 | 16 | 63 | 0.63 |
| RE1-Plumbing | 3 | 60 | 18 | 33 | 0.56 |
| RE1-Mechanical | 3 | 71 | 22 | 32 | 0.45 |
| Revit_IFC5_Einhoven | 0.9 | 23 | 27 | 9 | 0.37 |

The estimate is under the peak on every model (0.37 to 0.99), tightest on the
largest, because fixed costs weigh more on a small file; "at least about ten
times" is the lowest multiple measured, and it is the largest model's. The
multiple falls as files grow (27 times down to 10 across these six), no model
past 32 MiB is in CI, and the only larger evidence goes the other way: native
peaks of 3.7 GB for a 438 MB file (8.4 times, with the stream and inflate limits
raised) and 5.1 GB for a 1.13 GB file (4.5 times), both from #1075 and from
outside the browser. So the estimate under-counts every small model and may
over-count a large one: a 438 MB file is warned about (10 times is 4.1 GiB)
though 8.4 times is 3.4 GiB, and may fit once wasm's second copy of the file is
added (0.4 GiB more). The warning can therefore fire on a file that fits, which
is why it is a warning, and the stop is the verdict. The size of the file is the
basis because the sizes the reader knows before
inflating add nothing to it: a partition's stored size is nearly all of the
file (the largest stream is 96% of the 438 MB file and 98% of the 1.13 GB
one in #1075), and the inflated size is not stored (Revit's truncated gzip has
no `ISIZE` trailer).

The options for opening a larger model, as of 2026-11-03. Only the first three
rows are measured here; the rest are read from the code and from the release
notes cited, and none is built:

| Option | What it would do | Status |
|---|---|---|
| Say so, and point to the CLI | The viewer's answer today: warn from the file size, report the stop, name `rvt-ifc`, `rvt-gltf` and Python | Shipped (#1195) |
| Skip the saved-graphics pass | Free the memory of the pass that draws saved meshes | A gain on one of the six reference models: the wasm memory is the same with it on and off on Einhoven (17 MiB), RE1 Architecture (95), RE1 Mechanical (57), RE1 Plumbing (54) and RE1 Electrical (161), and 16 MiB more with it on for Core Interior (308 to 324), where the pass draws every boxed element (#1191, #1694, Deploy viewer run 43151719658) |
| Read fewer streams | Leave `Partitions/*` streams unread | The streams are the model: `stream_not_read_resource_limit` (#1075) already leaves a model empty of what they hold |
| One copy of the file | A binding that takes the file as an owned `Vec<u8>` saves one copy of the file (a tenth of the peak at ten times) | Not done: retyping `bytes` is a breaking change to the stable wasm API (`tools/ci/wasm-api.py`), so it needs a new binding or a `!` PR |
| Stream partitions | Inflate, use and drop one partition at a time | A library change: `RevitFile::inflated_partition` keeps every partition for the handle's lifetime on purpose (#266 took Core Interior from 26 s to 1.7 s natively by inflating each once), and a dozen passes read them |
| wasm64 (memory64) | Raise the ceiling past 4 GiB | Chrome 133 and Firefox 134 ship memory64; Safari has it in Technology Preview, and an iOS Safari 27 report says a 64-bit `Memory` is accepted but a memory64 module does not compile ([wllama#297](https://github.com/ngxson/wllama/issues/297)). wasm-bindgen supports the `wasm64-unknown-unknown` target since 0.2.120 ([release notes](https://github.com/wasm-bindgen/wasm-bindgen/releases), 2026-04-28; the lock has 0.2.118). The target is tier 3 (nightly and `build-std`). It would be a second wasm build chosen by feature detection, with the 32-bit build kept for Safari |

Where an open's time goes is in the second table of
`tests/saved-meshes-measure.spec.ts`, built from the reports the page receives,
and it limits what the progress bar can show. On Core Interior (Deploy viewer
[41700152800](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41700152800),
the open takes 17 s alone in this run, and 22 s and 24 s in earlier runs on other
runners): the schema, the candidate scan and the element
decode take 0.1 s (the file has no `Global/Latest` candidates; its elements
come from the partition records), and the `partition-scan` stage covers 15.9 s,
of which the first 2.5 s are real counts. `tests/load-progress.spec.ts` opens
the same file while it samples the card, on a runner shared with another test,
so it is slower (31.6 s) but has the same shape: the card fills 12% for each of
the 8 partition streams inflated, from 0.65 s to 3.1 s, and after the last
inflate the decoder reports nothing until the saved graphics start, 26.4 s later
(84% of that open): the
partition-schema recovery (levels, materials, rooms, walls, doors, windows,
slabs and products, each a pass over the inflated partitions) and the export's
own passes. The saved graphics take 0.3 s and the saved meshes 0.3 s, and
the three worker steps 1.4 s. The RE1 models have the same shape: 4.6 s of
Architecture's 5.1 s and 2.6 s of Plumbing's 3.0 s are under `partition-scan`.
So the bar is determinate for a tenth of a Core Interior open, and the card says
"Building the model" with a sweep for the rest. Reporting progress through the
partition-schema recovery is the work that would change that; it needs a
control passed into those passes, which is a library change this one does not
make.

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

`rvt-wasm/src/lib.rs` exports these functions through `wasm-bindgen`, and
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
| `openRvtBytesWithDiagnosticsMode` | `bytes`, `mode: string` | `{ model, diagnostics, phases }` | The same at an export quality mode: `scaffold`, `typed-no-geometry`, `geometry` or `strict`, the stronger ones failing when the file cannot meet them. Every element whose body would be its record's bounding box is drawn from its saved mesh, as in the next row with the default options |
| `openRvtBytesWithDiagnosticsModeAndOpenLimits` | `bytes`, `mode: string`, `limits: { largeModel?, maxStreamBytes?, maxInflateBytes?, savedMeshes?, phase? }` or `null`, `progress?: (stage: string, done: number, total: number \| null) => void` | `{ model, diagnostics, phases }` | `openRvtBytesWithDiagnosticsMode` under the reader's resource limits: `largeModel` is `rvt-ifc --large-model` (streams up to 2 GiB, inflating up to the 4 GiB wasm32 holds, the walker's scan cap lifted to match), `maxStreamBytes` and `maxInflateBytes` are whole numbers of bytes that win over it, and a field left out keeps its default. `savedMeshes` (default `true`) draws every element of a Revit 2023 to 2027 file that would be a bounding box from the mesh Revit saved for it, as `rvt-ifc` does by default; `false` keeps their bounding boxes and skips the pass over the saved graphics. `phase` (a phase's name, matched exactly first, or its ElementId as a number) exports that project phase, as `rvt-ifc --phase` does; see [Export phase](#export-phase). A stream a limit stops is listed in the diagnostics (`stream_not_read_resource_limit`, a `Not read whole:` warning each) and the viewer offers "Retry with large-model limits"; unknown fields and fractional or negative limits throw. When the saved graphics cannot be read the model keeps its boxes and the diagnostics carry a warning. `progress`, when given, is called as the decoder works with the stage name (`schema-parse`, `candidate-scan`, `element-decode`, `partition-scan`, `saved-graphics`, `saved-meshes`) and how many of how many it has done (`total` is `null` where the stage has no count yet), at a stage's start, its end and each whole percent between; see [Decode progress](#decode-progress). A function that throws is ignored |
| `quickSummary` | `bytes` | `Summary` | Release, build, PartAtom and the stream inventory, read without decoding the model |
| `fileMetadata` | `bytes` | `FileMetadata` | Document identity (release, worksharing, last save, GUID, save counter), from the two identity streams only |
| `scheduleCsv` | `model`, `kind: string`, `metric: boolean`, `excel: boolean` | `string` | The element (`kind` `"elements"`: the elements the IFC export writes, each on the storey it puts it on) or room (`"rooms"`) schedule as CSV, the columns of the [user guide](user-guide.md#get-a-schedule-into-excel) |
| `buildSceneGraph` | `model` | scene tree | The model's scene graph |
| `distinctIfcTypes` | `scene` | `string[]` | The IFC types in a scene, for the layer toggles |
| `elementInfoPanel` | `model`, `entity_index: number` | info panel | What the info panel shows for one element, on the storey the IFC export puts it in |
| `savedParameters` | `bytes`, `ids: Float64Array` (ElementIds), `limits: { largeModel?, maxStreamBytes?, maxInflateBytes? }` or `null` | JSON `string` | The saved parameters of the elements `ids`, the read `rvt-elements` and Python's `decoded_elements()` list as an element's `parameters` (RE-207), as `{ release, release_read, release_experimental, releases_read, releases_measured, elements, unresolved_family_slots, instance_parameters_without_value }`; see [Saved parameters](#saved-parameters). Opens `bytes` for the call alone, under the limits the file was opened with, and keeps nothing of them |
| `buildSchedule` | `model` | schedule table | A flat table of the building elements the IFC export writes, each on the storey it puts it in |
| `modelToGlb` | `model` | `Uint8Array` | The model as a glTF 2.0 binary |
| `modelToIfcStep` | `model` | `string` | The model as an IFC4 STEP document |
| `renderPlanSvg` | `model`, `options: SheetOptions` or `null` | `string` | A plan view as SVG |
