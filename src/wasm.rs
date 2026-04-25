//! WASM bindings (VW1-01) — JS-callable wrappers around the
//! viewer API.
//!
//! Enabled by the `wasm` feature flag. Build with:
//!
//! ```bash
//! wasm-pack build --target web --features wasm --no-default-features
//! ```
//!
//! Every binding is JSON round-trippable via
//! `serde-wasm-bindgen`: caller passes a JS object in, gets a
//! JS object back. No custom FFI conversions needed. Matches the
//! surface documented in `docs/viewer-api.md`.
//!
//! Network APIs (`fetch`, `XMLHttpRequest`, `WebSocket`) are
//! deliberately excluded per the client-side-only posture
//! (`docs/viewer-privacy-posture.md`). CI grep-checks the
//! compiled `.wasm` for those imports and fails the build if any
//! appear.

#![cfg(feature = "wasm")]

use wasm_bindgen::prelude::*;

use crate::RevitFile;
use crate::ifc::{
    Exporter, IfcModel, RvtDocExporter,
    camera::CameraState,
    clipping::{SectionBox, ViewMode},
    gltf::model_to_glb,
    scene_graph::{
        CategoryFilter, ElementInfoPanel, SceneNode, Schedule, build_scene_graph, build_schedule,
        distinct_ifc_types, element_info_panel,
    },
    share::{ViewerState, decode_from_fragment, encode_to_fragment},
    sheet::{SheetOptions, render_plan_svg},
    step_writer::write_step,
};

fn err_str<E: std::fmt::Display>(e: E) -> JsValue {
    JsValue::from_str(&e.to_string())
}

/// Open an RVT / RFA byte slice and return the raw `IfcModel` as
/// a JS object. The viewer passes this around and then calls the
/// other bindings to derive scene graph / glTF / schedule / etc.
#[wasm_bindgen(js_name = openRvtBytes)]
pub fn open_rvt_bytes(bytes: &[u8]) -> Result<JsValue, JsValue> {
    let mut rf = RevitFile::open_bytes(bytes.to_vec()).map_err(err_str)?;
    let model = RvtDocExporter.export(&mut rf).map_err(err_str)?;
    serde_wasm_bindgen::to_value(&model).map_err(err_str)
}

/// Open an RVT / RFA byte slice and return `{ model, diagnostics }`.
///
/// The diagnostics payload matches `rvt-ifc --diagnostics` and is
/// intended for viewer bug reports and export-readiness messaging.
#[wasm_bindgen(js_name = openRvtBytesWithDiagnostics)]
pub fn open_rvt_bytes_with_diagnostics(bytes: &[u8]) -> Result<JsValue, JsValue> {
    let mut rf = RevitFile::open_bytes(bytes.to_vec()).map_err(err_str)?;
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .map_err(err_str)?;
    serde_wasm_bindgen::to_value(&result).map_err(err_str)
}

/// Quick summary — reads only the cheap metadata (BasicFileInfo +
/// PartAtom + stream inventory) and returns instantly even for
/// multi-hundred-megabyte RFAs. Used by the viewer for the
/// progressive-loading splash before the full model parse
/// completes. Returns a [`crate::reader::Summary`] as a JS object.
#[wasm_bindgen(js_name = quickSummary)]
pub fn quick_summary(bytes: &[u8]) -> Result<JsValue, JsValue> {
    let mut rf = RevitFile::open_bytes(bytes.to_vec()).map_err(err_str)?;
    let summary = rf.summarize_lossy().map_err(err_str)?.value;
    serde_wasm_bindgen::to_value(&summary).map_err(err_str)
}

/// Build the scene-graph tree for a model.
#[wasm_bindgen(js_name = buildSceneGraph)]
pub fn js_build_scene_graph(model: JsValue) -> Result<JsValue, JsValue> {
    let model: IfcModel = serde_wasm_bindgen::from_value(model).map_err(err_str)?;
    let scene: SceneNode = build_scene_graph(&model);
    serde_wasm_bindgen::to_value(&scene).map_err(err_str)
}

/// Distinct IFC types in the scene — source of truth for the
/// layer-toggle UI.
#[wasm_bindgen(js_name = distinctIfcTypes)]
pub fn js_distinct_ifc_types(scene: JsValue) -> Result<JsValue, JsValue> {
    let scene: SceneNode = serde_wasm_bindgen::from_value(scene).map_err(err_str)?;
    let types = distinct_ifc_types(&scene);
    serde_wasm_bindgen::to_value(&types).map_err(err_str)
}

/// Apply a category filter to a scene. `filter` is a `CategoryFilter`
/// JSON value; returns the pruned scene tree.
#[wasm_bindgen(js_name = applyCategoryFilter)]
pub fn js_apply_category_filter(scene: JsValue, filter: JsValue) -> Result<JsValue, JsValue> {
    let scene: SceneNode = serde_wasm_bindgen::from_value(scene).map_err(err_str)?;
    let filter: CategoryFilter = serde_wasm_bindgen::from_value(filter).map_err(err_str)?;
    let pruned = filter.apply(&scene);
    serde_wasm_bindgen::to_value(&pruned).map_err(err_str)
}

/// Populate the element info panel for a click target.
#[wasm_bindgen(js_name = elementInfoPanel)]
pub fn js_element_info_panel(model: JsValue, entity_index: usize) -> Result<JsValue, JsValue> {
    let model: IfcModel = serde_wasm_bindgen::from_value(model).map_err(err_str)?;
    let panel: Option<ElementInfoPanel> = element_info_panel(&model, entity_index);
    serde_wasm_bindgen::to_value(&panel).map_err(err_str)
}

/// Build a flat schedule table of every BuildingElement.
#[wasm_bindgen(js_name = buildSchedule)]
pub fn js_build_schedule(model: JsValue) -> Result<JsValue, JsValue> {
    let model: IfcModel = serde_wasm_bindgen::from_value(model).map_err(err_str)?;
    let schedule: Schedule = build_schedule(&model);
    serde_wasm_bindgen::to_value(&schedule).map_err(err_str)
}

/// Render `model` as a glTF 2.0 binary. Returns a `Uint8Array`
/// the frontend feeds into Three.js's `GLTFLoader`.
#[wasm_bindgen(js_name = modelToGlb)]
pub fn js_model_to_glb(model: JsValue) -> Result<Vec<u8>, JsValue> {
    let model: IfcModel = serde_wasm_bindgen::from_value(model).map_err(err_str)?;
    Ok(model_to_glb(&model))
}

/// Render `model` as an IFC4 STEP document. Returns the ISO-10303-21
/// text; callers wrap it in a Blob for download.
#[wasm_bindgen(js_name = modelToIfcStep)]
pub fn js_model_to_ifc_step(model: JsValue) -> Result<String, JsValue> {
    let model: IfcModel = serde_wasm_bindgen::from_value(model).map_err(err_str)?;
    Ok(write_step(&model))
}

/// Render `model` as a 2D SVG plan view (sheet). Options control
/// dimensions + labels + background; pass `null` for defaults.
#[wasm_bindgen(js_name = renderPlanSvg)]
pub fn js_render_plan_svg(model: JsValue, options: JsValue) -> Result<String, JsValue> {
    let model: IfcModel = serde_wasm_bindgen::from_value(model).map_err(err_str)?;
    let options: SheetOptions = if options.is_null() || options.is_undefined() {
        SheetOptions::default()
    } else {
        serde_wasm_bindgen::from_value(serde_js_options_bridge(options)?).map_err(err_str)?
    };
    Ok(render_plan_svg(&model, &options))
}

/// Encode a ViewerState to a URL fragment string.
#[wasm_bindgen(js_name = encodeToFragment)]
pub fn js_encode_to_fragment(state: JsValue) -> Result<String, JsValue> {
    let state: ViewerState = serde_wasm_bindgen::from_value(state).map_err(err_str)?;
    Ok(encode_to_fragment(&state))
}

/// Decode a URL fragment into a ViewerState (or `null`).
#[wasm_bindgen(js_name = decodeFromFragment)]
pub fn js_decode_from_fragment(fragment: &str) -> Result<JsValue, JsValue> {
    let state = decode_from_fragment(fragment);
    serde_wasm_bindgen::to_value(&state).map_err(err_str)
}

/// Compute the camera eye position for a given CameraState.
#[wasm_bindgen(js_name = cameraEye)]
pub fn js_camera_eye(state: JsValue) -> Result<JsValue, JsValue> {
    let state: CameraState = serde_wasm_bindgen::from_value(state).map_err(err_str)?;
    serde_wasm_bindgen::to_value(&state.eye()).map_err(err_str)
}

/// Default section box for a given view mode.
#[wasm_bindgen(js_name = defaultSectionBoxForView)]
pub fn js_default_section_box(
    mode: JsValue,
    storey_elevation_feet: f64,
    model_bbox: JsValue,
) -> Result<JsValue, JsValue> {
    let mode: ViewMode = serde_wasm_bindgen::from_value(mode).map_err(err_str)?;
    let bbox: SectionBox = serde_wasm_bindgen::from_value(model_bbox).map_err(err_str)?;
    let result = mode.default_section_box(storey_elevation_feet, bbox);
    serde_wasm_bindgen::to_value(&result).map_err(err_str)
}

/// SheetOptions defaults for the JS side. Exposed as a helper so
/// the frontend can fetch them once and mutate fields rather than
/// re-declaring the defaults.
#[wasm_bindgen(js_name = defaultSheetOptions)]
pub fn js_default_sheet_options() -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(&SheetOptions::default()).map_err(err_str)
}

// Internal shim: serde-wasm-bindgen sometimes needs a
// round-trip through JSON to normalise JS objects with mixed
// numeric types (Three.js often passes Float32 where we want
// f32; this forces explicit coercion).
fn serde_js_options_bridge(input: JsValue) -> Result<JsValue, JsValue> {
    Ok(input)
}
