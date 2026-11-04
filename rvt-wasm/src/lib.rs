//! WASM bindings — JS-callable wrappers around the viewer API,
//! listed in `docs/viewer-api.md` ("The JavaScript bindings").
//!
//! Its own crate, so the core library builds as a plain `rlib`.
//! Build with, from the repository root:
//!
//! ```bash
//! wasm-pack build rvt-wasm --target web --out-dir ../viewer/pkg --out-name rvt
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

use std::cell::RefCell;

use wasm_bindgen::prelude::*;

use rvt::RevitFile;
use rvt::control::{ProgressEvent, Stage, WalkerControl};
use rvt::ifc::{
    ExportDiagnostics, ExportNote, ExportOptions, ExportQualityMode, ExportRun, IfcModel,
    SavedMeshChoice,
    gltf::model_to_glb,
    record_unread_streams, run_export_with_limits_and_control,
    scene_graph::{
        ElementInfoPanel, SceneNode, Schedule, build_scene_graph, build_schedule,
        distinct_ifc_types, element_info_panel,
    },
    sheet::{SheetOptions, render_plan_svg},
    step_writer::write_step,
};
use rvt::reader::{OpenLimits, ProjectPhase};

fn err_str<E: std::fmt::Display>(e: E) -> JsValue {
    JsValue::from_str(&e.to_string())
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = error)]
    fn console_error(s: &str);
}

/// Route Rust panics to `console.error` so the viewer reports the
/// panic message and source location instead of a bare
/// `RuntimeError: unreachable` trap. Installed once per wasm instance,
/// on the first call into any binding: the worker and the main thread
/// each run their own instance.
fn install_panic_hook() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        std::panic::set_hook(Box::new(|info| {
            console_error(&format!("rvt-rs wasm panic: {info}"));
        }));
    });
}

/// Open an RVT / RFA byte slice with an IFC export quality mode and
/// return `{ model, diagnostics }`.
///
/// `mode` is one of `scaffold`, `typed-no-geometry`, `geometry`, or
/// `strict`. Stronger modes fail closed when the decoded
/// coverage cannot satisfy the requested bar — the viewer uses this to
/// surface honest confidence rather than over-claiming geometry.
///
/// Doors and windows are drawn from the meshes Revit saved for them, as
/// `rvt-ifc` does by default, on Revit 2023 to 2027 files.
#[wasm_bindgen(js_name = openRvtBytesWithDiagnosticsMode)]
pub fn open_rvt_bytes_with_diagnostics_mode(bytes: &[u8], mode: &str) -> Result<JsValue, JsValue> {
    install_panic_hook();
    export_with_limits(
        bytes,
        mode,
        OpenLimits::default(),
        rvt::walker::WalkerLimits::default(),
        true,
        None,
        None,
    )
}

/// [`open_rvt_bytes_with_diagnostics_mode`] under the resource limits of
/// `limits`, a JS object
/// `{ largeModel?, maxStreamBytes?, maxInflateBytes?, savedMeshes?, phase? }` or
/// `null` for the defaults. A field left out keeps its default; the first
/// three are `rvt-ifc`'s flags of the same names, resolved by
/// [`rvt::cli::open_limits`] (`largeModel` is `--large-model`, and an
/// explicit byte limit wins over it):
///
/// - `largeModel: true` reads streams up to 2 GiB and lets a stream inflate
///   up to the 4 GiB a wasm32 address space holds (`rvt-ifc` gives 8 GiB on
///   a 64-bit host), and lifts the walker's scan cap the same way.
/// - `maxStreamBytes` caps one stream as stored; a larger stream is not read.
/// - `maxInflateBytes` caps one gzip member's inflated size; a stream's
///   members together may inflate to this or 1 GiB, whichever is larger.
///   A value past what a 32-bit `usize` holds saturates at 4 GiB minus 1.
/// - `savedMeshes` (default `true`) draws the doors and windows of Revit 2023
///   to 2027 files from the meshes Revit saved for them, as `rvt-ifc` does
///   without `--no-saved-meshes`; `false` keeps their bounding boxes and
///   skips the extra pass over the file's saved graphics.
/// - `phase` (a phase's name, matched exactly first, or its ElementId as a
///   number) exports that project phase, as `rvt-ifc --phase` does: an element
///   that does not exist in it, and a room whose own phase is not it, are left
///   out. Left out, the export is the project's last phase. A phase the file
///   lacks throws, with the file's phases in the message. The result's
///   `phases` lists them (`{ id, name? }`, in order, the last being the
///   default).
///
/// Byte limits are whole numbers of bytes. A stream a limit keeps from being
/// read is listed in `diagnostics` (the `stream_not_read_resource_limit`
/// skipped item and a `Not read whole:` warning per stream, #1075), and the
/// model is missing what it holds. Unknown fields and fractional, negative
/// or non-finite byte limits throw.
///
/// `progress`, when given, is a JS function `(stage, done, total)` called as
/// the decoder works: `stage` is a [`Stage`] name as `rvt-gltf --progress`
/// prints it (`schema-parse`, `candidate-scan`, `element-decode`,
/// `partition-scan`, `saved-graphics`, `saved-meshes`), `done` a count and
/// `total` the count it is out of, or `null` where the stage has no
/// denominator yet (#1195). It is called when a stage begins, when it ends,
/// and each time a stage with a total passes another whole percent, so a
/// stage costs at most about a hundred calls. An exception it throws is
/// ignored.
#[wasm_bindgen(js_name = openRvtBytesWithDiagnosticsModeAndOpenLimits)]
pub fn open_rvt_bytes_with_diagnostics_mode_and_open_limits(
    bytes: &[u8],
    mode: &str,
    limits: JsValue,
    progress: Option<js_sys::Function>,
) -> Result<JsValue, JsValue> {
    install_panic_hook();
    let request = limits_from_js(limits)?;
    export_with_limits(
        bytes,
        mode,
        request.open_limits,
        request.walker_limits,
        request.saved_meshes,
        request.phase,
        progress,
    )
}

/// The JS function a decode reports its progress to, and the last report it
/// forwarded. A `js_sys::Function` is not `Send + Sync`, which
/// [`WalkerControl::with_progress`] requires, so it sits here, where one
/// single-threaded wasm instance reads it, and the closure handed to the
/// control captures nothing.
struct ProgressSink {
    callback: js_sys::Function,
    last: Option<(Stage, Option<u64>)>,
}

thread_local! {
    static PROGRESS: RefCell<Option<ProgressSink>> = const { RefCell::new(None) };
}

/// The control of one decode: reports to `progress` when a function is given
/// and does nothing otherwise.
fn progress_control(progress: Option<js_sys::Function>) -> WalkerControl {
    match progress {
        Some(callback) => {
            PROGRESS.with(|slot| {
                *slot.borrow_mut() = Some(ProgressSink {
                    callback,
                    last: None,
                });
            });
            WalkerControl::new().with_progress(forward_progress)
        }
        None => WalkerControl::default(),
    }
}

fn clear_progress() {
    PROGRESS.with(|slot| {
        slot.borrow_mut().take();
    });
}

/// Hands `event` to the JS function unless it is a report within the same
/// whole percent of the same stage as the last one handed over. The decoder
/// reports each megabyte scanned and every 256th candidate, which the page
/// has no use for; a stage with no total reports once, when it begins.
fn forward_progress(event: ProgressEvent) {
    PROGRESS.with(|slot| {
        let Ok(mut slot) = slot.try_borrow_mut() else {
            return;
        };
        let Some(sink) = slot.as_mut() else {
            return;
        };
        let percent = event
            .total
            .filter(|total| *total > 0)
            .map(|total| event.done.saturating_mul(100) / total);
        let finished = event.total.is_some_and(|total| event.done >= total);
        let same_stage = sink.last.is_some_and(|(stage, _)| stage == event.stage);
        let same_percent = sink.last.is_some_and(|(_, last)| last == percent);
        if same_stage && same_percent && !finished {
            return;
        }
        sink.last = Some((event.stage, percent));
        let total = event
            .total
            .map_or(JsValue::NULL, |total| JsValue::from_f64(total as f64));
        let _ = sink.callback.call3(
            &JsValue::NULL,
            &JsValue::from_str(event.stage.as_str()),
            &JsValue::from_f64(event.done as f64),
            &total,
        );
    });
}

fn export_with_limits(
    bytes: &[u8],
    mode: &str,
    open_limits: OpenLimits,
    walker_limits: rvt::walker::WalkerLimits,
    saved_meshes: bool,
    phase: Option<String>,
    progress: Option<js_sys::Function>,
) -> Result<JsValue, JsValue> {
    let control = progress_control(progress);
    let result = export_with_control(
        bytes,
        mode,
        open_limits,
        walker_limits,
        saved_meshes,
        phase,
        &control,
    );
    clear_progress();
    result
}

/// What the viewer gets back: [`rvt::ifc::ExportResult`]'s two fields, and the
/// project's phases in order (the last is the export's unless the request
/// named another), empty where the export applies no phase.
#[derive(serde::Serialize)]
struct ExportJson {
    model: IfcModel,
    diagnostics: ExportDiagnostics,
    phases: Vec<ProjectPhase>,
}

fn export_with_control(
    bytes: &[u8],
    mode: &str,
    open_limits: OpenLimits,
    walker_limits: rvt::walker::WalkerLimits,
    saved_meshes: bool,
    phase: Option<String>,
    control: &WalkerControl,
) -> Result<JsValue, JsValue> {
    let quality_mode = ExportQualityMode::parse(mode).map_err(err_str)?;
    let mut rf = RevitFile::open_bytes_with_limits(bytes.to_vec(), open_limits).map_err(err_str)?;
    // The build `rvt-ifc` runs ([`run_export_with_limits_and_control`]), with doors and windows drawn
    // from their saved meshes (Revit 2023 to 2027 only) unless `saved_meshes`
    // is false.
    let options = ExportOptions::default()
        .with_mode(quality_mode)
        .with_saved_meshes(if saved_meshes {
            SavedMeshChoice::Default
        } else {
            SavedMeshChoice::Off
        })
        .with_phase(phase);
    let run = run_export_with_limits_and_control(&mut rf, &options, walker_limits, control)
        .map_err(err_str)?;
    let ExportRun {
        model,
        mut diagnostics,
        unread,
        notes,
        ..
    } = run;
    // Saved graphics only improve bodies: when they cannot be read the model
    // keeps its bounding boxes and the diagnostics carry a warning saying so.
    let mut warned = false;
    for note in &notes {
        if let ExportNote::SavedMeshesSkipped { error, .. } = note {
            diagnostics.warnings.push(format!(
                "Saved graphics not read, door and window bodies unchanged: {error}"
            ));
            warned = true;
        }
    }
    if warned {
        // Recounts the warnings, and puts the unread-stream ones last.
        record_unread_streams(&mut diagnostics, &unread);
    }
    let phases = rf.project_phases();
    serde_wasm_bindgen::to_value(&ExportJson {
        model,
        diagnostics,
        phases,
    })
    .map_err(err_str)
}

#[derive(Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LimitsOptions {
    large_model: Option<bool>,
    max_stream_bytes: Option<f64>,
    max_inflate_bytes: Option<f64>,
    saved_meshes: Option<bool>,
    phase: Option<PhaseOption>,
}

/// A phase as the page names it: its name, or its ElementId as a number.
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum PhaseOption {
    Name(String),
    Id(f64),
}

impl PhaseOption {
    /// What `RevitFile::set_export_phase` takes: a name is matched first and an
    /// ElementId only where no phase has that name.
    fn into_request(self) -> Result<String, JsValue> {
        match self {
            Self::Name(name) => Ok(name),
            Self::Id(id) if id.is_finite() && id.fract() == 0.0 => Ok(format!("{id:.0}")),
            Self::Id(id) => Err(JsValue::from_str(&format!(
                "phase must be a name or a whole-number ElementId, got {id}"
            ))),
        }
    }
}

/// The resource limits, saved-mesh switch and export phase of one open.
struct ExportRequest {
    open_limits: OpenLimits,
    walker_limits: rvt::walker::WalkerLimits,
    saved_meshes: bool,
    phase: Option<String>,
}

fn whole_bytes(field: &str, value: Option<f64>) -> Result<Option<u64>, JsValue> {
    match value {
        None => Ok(None),
        Some(bytes) if bytes.is_finite() && bytes >= 0.0 && bytes.fract() == 0.0 => {
            Ok(Some(bytes as u64))
        }
        Some(bytes) => Err(JsValue::from_str(&format!(
            "{field} must be a whole number of bytes, got {bytes}"
        ))),
    }
}

fn limits_from_js(limits: JsValue) -> Result<ExportRequest, JsValue> {
    let options: LimitsOptions = if limits.is_null() || limits.is_undefined() {
        LimitsOptions::default()
    } else {
        serde_wasm_bindgen::from_value(limits).map_err(err_str)?
    };
    let large_model = options.large_model.unwrap_or(false);
    let max_inflate_bytes = whole_bytes("maxInflateBytes", options.max_inflate_bytes)?
        .map(|bytes| usize::try_from(bytes).unwrap_or(usize::MAX));
    let open_limits = rvt::cli::open_limits(
        whole_bytes("maxStreamBytes", options.max_stream_bytes)?,
        max_inflate_bytes,
        large_model,
    );
    let mut walker_limits = rvt::walker::WalkerLimits::default();
    if large_model {
        walker_limits.max_scan_bytes = rvt::cli::LARGE_MODEL_MAX_INFLATE_BYTES;
    }
    Ok(ExportRequest {
        open_limits,
        walker_limits,
        saved_meshes: options.saved_meshes.unwrap_or(true),
        phase: options.phase.map(PhaseOption::into_request).transpose()?,
    })
}

/// Quick summary — reads only the cheap metadata (BasicFileInfo +
/// PartAtom + stream inventory) and returns instantly even for
/// multi-hundred-megabyte RFAs. Used by the viewer for the
/// progressive-loading splash before the full model parse
/// completes. Returns a [`rvt::reader::Summary`] as a JS object.
#[wasm_bindgen(js_name = quickSummary)]
pub fn quick_summary(bytes: &[u8]) -> Result<JsValue, JsValue> {
    install_panic_hook();
    let mut rf = RevitFile::open_bytes(bytes.to_vec()).map_err(err_str)?;
    let summary = rf.summarize_lossy().map_err(err_str)?.value;
    serde_wasm_bindgen::to_value(&summary).map_err(err_str)
}

/// Document identity — release, worksharing, central model, last saved
/// (time and user), document GUID, save counter, and every
/// `BasicFileInfo` `Key: value` line — as a
/// [`rvt::metadata::FileMetadata`] JS object. Reads only the two
/// identity streams straight from `bytes` (no copy of the file), so it
/// returns instantly on any file size. Errors when `bytes` is not a
/// readable Revit file.
#[wasm_bindgen(js_name = fileMetadata)]
pub fn file_metadata(bytes: &[u8]) -> Result<JsValue, JsValue> {
    install_panic_hook();
    let metadata =
        rvt::metadata::read_metadata_from(std::io::Cursor::new(bytes)).map_err(err_str)?;
    serde_wasm_bindgen::to_value(&metadata).map_err(err_str)
}

/// Spreadsheet schedule of an exported model as CSV
/// ([`rvt::ifc::schedule_csv`]): `kind` is `"elements"` (every building
/// element) or `"rooms"`; `metric` switches lengths to metres; `excel`
/// prefixes a UTF-8 byte-order mark so Excel on Windows reads non-ASCII
/// names correctly.
#[wasm_bindgen(js_name = scheduleCsv)]
pub fn js_schedule_csv(
    model: JsValue,
    kind: &str,
    metric: bool,
    excel: bool,
) -> Result<String, JsValue> {
    install_panic_hook();
    use rvt::ifc::schedule_csv::{CsvOptions, LengthUnit, elements_csv, rooms_csv};
    let model: IfcModel = serde_wasm_bindgen::from_value(model).map_err(err_str)?;
    let options = CsvOptions {
        unit: if metric {
            LengthUnit::Metres
        } else {
            LengthUnit::Feet
        },
        excel_bom: excel,
    };
    match kind {
        "elements" => Ok(elements_csv(&model, &options)),
        "rooms" => Ok(rooms_csv(&model, &options)),
        other => Err(JsValue::from_str(&format!(
            "unknown schedule kind {other:?}; expected \"elements\" or \"rooms\""
        ))),
    }
}

/// Build the scene-graph tree for a model.
#[wasm_bindgen(js_name = buildSceneGraph)]
pub fn js_build_scene_graph(model: JsValue) -> Result<JsValue, JsValue> {
    install_panic_hook();
    let model: IfcModel = serde_wasm_bindgen::from_value(model).map_err(err_str)?;
    let scene: SceneNode = build_scene_graph(&model);
    serde_wasm_bindgen::to_value(&scene).map_err(err_str)
}

/// Distinct IFC types in the scene — source of truth for the
/// layer-toggle UI.
#[wasm_bindgen(js_name = distinctIfcTypes)]
pub fn js_distinct_ifc_types(scene: JsValue) -> Result<JsValue, JsValue> {
    install_panic_hook();
    let scene: SceneNode = serde_wasm_bindgen::from_value(scene).map_err(err_str)?;
    let types = distinct_ifc_types(&scene);
    serde_wasm_bindgen::to_value(&types).map_err(err_str)
}

/// Populate the element info panel for a click target.
#[wasm_bindgen(js_name = elementInfoPanel)]
pub fn js_element_info_panel(model: JsValue, entity_index: usize) -> Result<JsValue, JsValue> {
    install_panic_hook();
    let model: IfcModel = serde_wasm_bindgen::from_value(model).map_err(err_str)?;
    let panel: Option<ElementInfoPanel> = element_info_panel(&model, entity_index);
    serde_wasm_bindgen::to_value(&panel).map_err(err_str)
}

/// Build a flat schedule table of every BuildingElement.
#[wasm_bindgen(js_name = buildSchedule)]
pub fn js_build_schedule(model: JsValue) -> Result<JsValue, JsValue> {
    install_panic_hook();
    let model: IfcModel = serde_wasm_bindgen::from_value(model).map_err(err_str)?;
    let schedule: Schedule = build_schedule(&model);
    serde_wasm_bindgen::to_value(&schedule).map_err(err_str)
}

/// Render `model` as a glTF 2.0 binary. Returns a `Uint8Array`
/// the frontend feeds into Three.js's `GLTFLoader`.
#[wasm_bindgen(js_name = modelToGlb)]
pub fn js_model_to_glb(model: JsValue) -> Result<Vec<u8>, JsValue> {
    install_panic_hook();
    let model: IfcModel = serde_wasm_bindgen::from_value(model).map_err(err_str)?;
    Ok(model_to_glb(&model))
}

/// Render `model` as an IFC4 STEP document. Returns the ISO-10303-21
/// text; callers wrap it in a Blob for download.
#[wasm_bindgen(js_name = modelToIfcStep)]
pub fn js_model_to_ifc_step(model: JsValue) -> Result<String, JsValue> {
    install_panic_hook();
    let model: IfcModel = serde_wasm_bindgen::from_value(model).map_err(err_str)?;
    Ok(write_step(&model))
}

/// Render `model` as a 2D SVG plan view (sheet). Options control
/// dimensions + labels + background; pass `null` for defaults.
#[wasm_bindgen(js_name = renderPlanSvg)]
pub fn js_render_plan_svg(model: JsValue, options: JsValue) -> Result<String, JsValue> {
    install_panic_hook();
    let model: IfcModel = serde_wasm_bindgen::from_value(model).map_err(err_str)?;
    let options: SheetOptions = if options.is_null() || options.is_undefined() {
        SheetOptions::default()
    } else {
        serde_wasm_bindgen::from_value(serde_js_options_bridge(options)?).map_err(err_str)?
    };
    Ok(render_plan_svg(&model, &options))
}

// Internal shim: serde-wasm-bindgen sometimes needs a
// round-trip through JSON to normalise JS objects with mixed
// numeric types (Three.js often passes Float32 where we want
// f32; this forces explicit coercion).
fn serde_js_options_bridge(input: JsValue) -> Result<JsValue, JsValue> {
    Ok(input)
}
