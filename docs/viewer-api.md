# Rust-side viewer API reference

The `rvt::ifc` module ships a complete Rust data model for a
browser/desktop 3D viewer. This document maps every public
function a WASM frontend (VW1-01/02/03) would expose, grouped by
the 11 submodules that make up the viewer surface.

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

## Scene graph

```rust
use rvt::ifc::scene_graph::{
    build_scene_graph, SceneNode,
    CategoryFilter, distinct_ifc_types,
    element_info_panel, ElementInfoPanel,
    build_schedule, Schedule, ScheduleRow,
};
```

- `build_scene_graph(&IfcModel) -> SceneNode` — project → storey
  → element tree with hosted doors/windows nested under their
  wall.
- `SceneNode::descendants_count()`, `find_by_name(&str)`,
  `flatten() -> Vec<(depth, &SceneNode)>`.
- `CategoryFilter { hidden }` with `hide()` / `show()` /
  `is_hidden()` / `apply(&SceneNode) -> SceneNode`.
- `distinct_ifc_types(&SceneNode) -> Vec<String>` — populate the
  layer-toggle UI.
- `element_info_panel(&IfcModel, entity_index) -> Option<ElementInfoPanel>` —
  click-to-inspect payload.
- `build_schedule(&IfcModel) -> Schedule` + `Schedule::to_csv()`
  — tabular element export.

## Camera

```rust
use rvt::ifc::camera::CameraState;
```

- `CameraState { target, distance, yaw, pitch, fov_radians, near, far }`.
- `orbit(delta_yaw, delta_pitch)` — clamps pitch to safe bounds.
- `pan([dx, dy, dz])`.
- `zoom(factor)` — clamps to [near*2, far/2].
- `focus_on(target)`, `frame_bbox(min, max)`.
- `eye() -> [f64; 3]` — world-space camera position.

## Clipping + view mode

```rust
use rvt::ifc::clipping::{ClippingPlane, SectionBox, ViewMode};
```

- `ViewMode { Plan, ThreeD, Section }` +
  `default_section_box(storey_elevation_feet, model_bbox)`.
- `ClippingPlane { origin, normal }` with
  `signed_distance(point)` + `contains(point)`.
- `SectionBox { min, max }` with `new(a, b)` (normalises),
  `infinite()`, `contains`, `expand_to`, `size`, `center`.

## Materials (PBR)

```rust
use rvt::ifc::pbr::PbrMaterial;
```

- `PbrMaterial::from_material_info(&MaterialInfo) -> PbrMaterial`
- Name-driven classifier: glass → double-sided + roughness 0.05,
  metal → metallic 1, wood → roughness 0.7, concrete → 0.9,
  paint/ceramic/tile → 0.5, default → 0.6.
- sRGB → linear color conversion on color unpack.

## Measurement

```rust
use rvt::ifc::measure::{
    distance, vector, dot, cross, magnitude, normalize,
    angle_abc, polygon_area_3d, polygon_perimeter,
    Measurement,
};
```

- `distance(a, b) -> f64`, `angle_abc(a, b, c) -> f64`,
  `polygon_area_3d(&[Point3]) -> f64`, `polygon_perimeter(&[Point3]) -> f64`.
- `Measurement { Distance, Angle, Area }` tagged enum with
  builder methods.

## Annotations

```rust
use rvt::ifc::annotation::{Annotation, AnnotationLayer};
```

- `Annotation { Note, Leader, Polyline, Pin }` tagged enum.
- `AnnotationLayer` with `push`, `remove_by_id`, `find`, `len`,
  `next_id(counter, kind)`.

## Rendering outputs

### 3D (glTF 2.0 binary)

```rust
use rvt::ifc::gltf::model_to_glb;
let glb_bytes = model_to_glb(&model);
// Write to disk or pass to Three.js's GLTFLoader via Blob.
```

### 2D (SVG plan view)

```rust
use rvt::ifc::sheet::{render_plan_svg, SheetOptions};
let svg_string = render_plan_svg(&model, &SheetOptions::default());
```

### IFC4 STEP

```rust
use rvt::ifc::step_writer::write_step;
let step_text = write_step(&model);
```

## URL sharing

```rust
use rvt::ifc::share::{ViewerState, encode_to_fragment, decode_from_fragment};
```

- `ViewerState { file_hash, camera, view_mode, section_box,
  category_filter, selected_name }`.
- `encode_to_fragment(&state) -> String` — base64-of-JSON ready
  for `window.location.hash = "#v=" + returned`.
- `decode_from_fragment(fragment) -> Option<ViewerState>` —
  strips `#`, `v=`, `#v=` prefixes.

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
//    - Measure tool  → Measurement::distance(a, b) / angle / area
//    - Layer toggle  → filter.hide(ifc_type); filter.apply(&scene)
//    - View mode     → mode.default_section_box(storey, bbox)
//    - Share URL     → encode_to_fragment(&state)
```

## What the WASM bindings need

For VW1-01/02/03, `wasm-bindgen` wraps the above with JavaScript-
callable surface:

```rust
#[wasm_bindgen(js_name = openRvtBytes)]
pub fn open_rvt_bytes(bytes: &[u8]) -> Result<JsValue, JsError> {
    let mut rf = RevitFile::open_bytes(bytes.to_vec())?;
    let model = RvtDocExporter.export(&mut rf)?;
    Ok(serde_wasm_bindgen::to_value(&model)?)
}

#[wasm_bindgen(js_name = openRvtBytesWithDiagnostics)]
pub fn open_rvt_bytes_with_diagnostics(bytes: &[u8]) -> Result<JsValue, JsError> {
    let mut rf = RevitFile::open_bytes(bytes.to_vec())?;
    let result = RvtDocExporter.export_with_diagnostics(&mut rf)?;
    Ok(serde_wasm_bindgen::to_value(&result)?)
}

#[wasm_bindgen(js_name = openRvtBytesWithDiagnosticsAndLimits)]
pub fn open_rvt_bytes_with_diagnostics_and_limits(
    bytes: &[u8],
    limits: JsValue,
) -> Result<JsValue, JsError> {
    let mut rf = RevitFile::open_bytes(bytes.to_vec())?;
    let limits = walker_limits_from_js(limits)?;
    let result = RvtDocExporter.export_with_diagnostics_and_limits(&mut rf, limits)?;
    Ok(serde_wasm_bindgen::to_value(&result)?)
}

#[wasm_bindgen]
pub fn scene_graph(model_json: JsValue) -> Result<JsValue, JsError> {
    let model: IfcModel = serde_wasm_bindgen::from_value(model_json)?;
    let scene = build_scene_graph(&model);
    Ok(serde_wasm_bindgen::to_value(&scene)?)
}

// ...repeat for every public function above.
```

Every function listed is JSON-round-trippable, so the binding
layer is mechanical — no custom FFI conversions needed.
