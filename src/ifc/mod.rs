//! Layer 5 — IFC export (document-level scaffold).
//!
//! # What this module currently produces
//!
//! A spec-valid but structurally minimal IFC4 STEP file containing:
//!
//! - `IfcProject` with name + description from PartAtom / BasicFileInfo
//! - `IfcSite` → `IfcBuilding` → `IfcBuildingStorey` spatial hierarchy
//!   (placeholder names today; real names from `Level` elements
//!   pending Layer 5b)
//! - `IfcClassification` + `IfcClassificationReference` for any
//!   OmniClass codes found in PartAtom
//! - Required framework entities (`IfcPerson`, `IfcOrganization`,
//!   `IfcApplication`, `IfcOwnerHistory`, `IfcSIUnit`×4,
//!   `IfcUnitAssignment`, `IfcGeometricRepresentationContext`)
//!
//! **Per-element entities now land as geometry-free IFC4 elements.**
//! When `IfcModel.entities` contains `BuildingElement { ifc_type, name,
//! type_guid }` values (populated by Layer 5b decoders: Wall, Floor,
//! Roof, Ceiling, Door, Window, Column, Beam), the writer emits each
//! as an `IFC<TYPE>` constructor with its own `IFCLOCALPLACEMENT`, and
//! bundles them via `IFCRELCONTAINEDINSPATIALSTRUCTURE` linked to the
//! storey. This means BlenderBIM / IfcOpenShell now see a real element
//! list — they can count walls, list rooms, and enumerate the spatial
//! tree. Geometry (`IfcShapeRepresentation`), materials, and property
//! sets still land in Phase 5 + 6 per `TODO-BLINDSIDE.md`.
//!
//! # Eventual implementation plan
//!
//! 1. Layer 5b (per-element walker) produces typed `Category`, `Level`,
//!    `Wall`, `Floor`, `Door`, `Window`, `Column`, `Beam`, etc.
//! 2. Phase 5 (geometry) extracts curves, faces, solids for each
//!    element.
//! 3. Entity mapper translates:
//!
//!    | Revit concept | IFC mapping |
//!    |---|---|
//!    | Project metadata (PartAtom) | `IfcProject` (done) |
//!    | Unit set (autodesk.unit.*) | `IfcUnitAssignment` / `IfcSIUnit` (pending real read) |
//!    | Level | `IfcBuildingStorey` (pending Layer 5b) |
//!    | Wall | `IfcWall` + geometry (pending Phase 5) |
//!    | Floor/Roof/Ceiling | `IfcSlab` / `IfcRoof` / `IfcCovering` (pending) |
//!    | Door/Window | `IfcDoor` / `IfcWindow` + `IfcRelVoidsElement` (pending) |
//!    | Column/Beam | `IfcColumn` / `IfcBeam` (pending) |
//!    | Family (RFA) | `IfcTypeObject` + `IfcRepresentationMap` (pending) |
//!    | Uniformat / OmniClass codes | `IfcClassificationReference` (done) |
//!    | Material | `IfcMaterial` / `IfcMaterialLayerSet` (pending) |
//!    | Parameters | `IfcPropertySet` + `IfcPropertySingleValue` (pending) |
//!    | Host geometry | `IfcShapeRepresentation` (pending Phase 5) |
//!
//! 4. STEP serializer writes the `IfcModel` as `.ifc` text (done at
//!    document level; extends to elements as Phase 6 lands).
//! 5. IfcOpenShell + buildingSMART validators verify output against
//!    the 11-release corpus (pending — IFC-41/43).
//!
//! # Library collaboration
//!
//! `IfcOpenShell` is the validation partner. Output is written in
//! IFC4 STEP (ISO 10303-21) so it interoperates directly with
//! IfcOpenShell, BlenderBIM, and the buildingSMART validator family.
//! No IfcOpenShell runtime dependency is needed — the STEP writer is
//! pure Rust.
//!
//! # Module index
//!
//! IFC4 exporter subsystem:
//!
//! | Module | What it does |
//! |---|---|
//! | [`category_map`] | Revit class → IFC4 type mapping (IFC-01) |
//! | [`entities`] | IFC4 entity taxonomy (walls, floors, doors, …) |
//! | [`from_decoded`] | Bridge: decoded Revit elements → IfcModel |
//! | [`step_writer`] | IfcModel → ISO-10303-21 STEP text |
//!
//! VW1 viewer data model — Rust-side primitives a browser /
//! desktop viewer binds to:
//!
//! | Module | What it does |
//! |---|---|
//! | [`scene_graph`] | Project → storey → element tree (VW1-05) + schedule (VW1-15) |
//! | [`pbr`] | Revit Material → glTF PBR mapping (VW1-06) |
//! | [`camera`] | Orbit-camera state + controls (VW1-07) |
//! | [`clipping`] | ClippingPlane + SectionBox + ViewMode (VW1-10/14) |
//! | [`measure`] | Distance / angle / polygon-area (VW1-13) |
//! | [`annotation`] | Note / leader / polyline / pin markups (VW1-12) |
//! | [`share`] | ViewerState URL-fragment serialization (VW1-24) |
//! | [`gltf`] | glTF 2.0 GLB binary exporter (VW1-04) |
//! | [`sheet`] | 2D SVG plan view emission (VW1-11) |
//!
//! Typical viewer pipeline:
//!
//! 1. `IfcModel` produced via [`RvtDocExporter`]
//! 2. [`scene_graph::build_scene_graph`] for the navigation tree
//! 3. [`scene_graph::CategoryFilter`] applied per user toggles
//! 4. [`gltf::model_to_glb`] for the 3D canvas, or
//!    [`sheet::render_plan_svg`] for the 2D drawing panel
//! 5. [`camera::CameraState`] + [`clipping::ViewMode`] drive the
//!    viewport's projection + spatial filter
//! 6. [`scene_graph::element_info_panel`] powers click-to-inspect
//! 7. [`share::encode_to_fragment`] serializes the whole state into
//!    a URL for collaboration

use crate::Result;

pub mod annotation;
pub mod camera;
pub mod category_map;
pub mod clipping;
pub mod entities;
pub mod from_decoded;
pub mod gltf;
pub mod measure;
pub mod pbr;
pub mod scene_graph;
pub mod share;
pub mod sheet;
pub mod step_writer;

pub use from_decoded::{BuilderOptions, ElementInput, build_ifc_model, entity_type_histogram};
pub use step_writer::write_step;

/// In-memory IFC model — what a successful export produces. Wire format
/// (STEP or IFC-JSON) is a separate concern handled by a serializer.
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct IfcModel {
    pub project_name: Option<String>,
    pub description: Option<String>,
    pub entities: Vec<entities::IfcEntity>,
    pub classifications: Vec<entities::Classification>,
    pub units: Vec<entities::UnitAssignment>,
    /// Real building storeys derived from Revit `Level` decoders. When
    /// empty, the STEP writer falls back to a single placeholder
    /// "Level 1" storey so the spatial hierarchy is still valid
    /// IFC4. When populated, each entry emits one `IfcBuildingStorey`
    /// with the Revit level's name + elevation in metres (converted
    /// from feet at emit time).
    pub building_storeys: Vec<Storey>,
    /// Materials available for association with BuildingElements.
    /// BuildingElement.material_index points into this list.
    pub materials: Vec<MaterialInfo>,
    /// Compound material assemblies (IFC-28). Referenced by
    /// `BuildingElement.material_layer_set_index`. Each layer
    /// inside a set references a material in `materials` above by
    /// index, so the two lists share a namespace — a layer can't
    /// reference a material that hasn't been registered there first.
    pub material_layer_sets: Vec<entities::MaterialLayerSet>,
    /// Compound structural profile assignments (IFC-30). Referenced
    /// by `BuildingElement.material_profile_set_index`. Used for
    /// columns and beams with named cross-sections (W12x26, HSS,
    /// circular columns). Profiles reference materials in
    /// `materials` above by index.
    pub material_profile_sets: Vec<entities::MaterialProfileSet>,
    /// Shared geometry maps for family / type instancing (IFC-21).
    /// Any `BuildingElement` whose `representation_map_index` is
    /// `Some(i)` routes through `representation_maps[i]` via an
    /// `IfcMappedItem` instead of emitting its own body chain. Each
    /// map's shape is serialised once; instances add a ~4-entity
    /// mapped-item wrap. Empty `Vec` leaves writer behaviour
    /// unchanged.
    pub representation_maps: Vec<entities::RepresentationMap>,
}

/// A single building storey derived from a Revit `Level` element.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Storey {
    pub name: String,
    /// Elevation in feet (Revit's native unit). The STEP writer
    /// converts to metres at emit time per IFC4 convention.
    pub elevation_feet: f64,
}

/// A single material entry ready for IFC emission. Derived from
/// a decoded Revit `Material` element via
/// [`from_decoded::materials_from_revit`].
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MaterialInfo {
    /// Display name ("Concrete", "Glass - Tinted", "Wood - Oak").
    pub name: String,
    /// Packed RGB `0x00BBGGRR` from the Revit material's color.
    /// `None` when the material didn't carry a color.
    pub color_packed: Option<u32>,
    /// Surface transparency in the 0..1 range. 0 = fully opaque.
    pub transparency: Option<f64>,
}

/// Stable schema version for [`ExportDiagnostics`] JSON.
pub const EXPORT_DIAGNOSTICS_SCHEMA_VERSION: u32 = 1;

/// IFC export mode represented in diagnostics sidecars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportDiagnosticsMode {
    Placeholder,
    Default,
    DiagnosticProxies,
}

/// Result type for callers that want both the IFC model and the
/// user/shareable export diagnostics sidecar in one pass.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExportResult {
    pub model: IfcModel,
    pub diagnostics: ExportDiagnostics,
}

/// JSON-serialisable export diagnostics sidecar.
///
/// The schema is intentionally flat and conservative so CLI, Python,
/// and WASM callers can attach it to issue reports without understanding
/// Revit internals.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExportDiagnostics {
    pub schema_version: u32,
    pub mode: ExportDiagnosticsMode,
    pub input: ExportInputDiagnostics,
    pub decoded: DecodedExportDiagnostics,
    pub exported: ExportedModelDiagnostics,
    pub skipped: Vec<SkippedExportItem>,
    pub unsupported_features: Vec<String>,
    pub warnings: Vec<String>,
    pub confidence: ExportConfidenceSummary,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExportInputDiagnostics {
    pub revit_version: Option<u32>,
    pub build: Option<String>,
    pub original_path: Option<String>,
    pub project_name: Option<String>,
    pub stream_count: usize,
    pub has_basic_file_info: bool,
    pub has_part_atom: bool,
    pub has_formats_latest: bool,
    pub has_global_latest: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DecodedExportDiagnostics {
    pub production_walker_elements: usize,
    pub diagnostic_proxy_candidates: usize,
    pub arcwall_records: usize,
    pub class_counts: std::collections::BTreeMap<String, usize>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExportedModelDiagnostics {
    pub total_entities: usize,
    pub building_elements: usize,
    pub building_elements_with_geometry: usize,
    pub by_ifc_type: std::collections::BTreeMap<String, usize>,
    pub classification_count: usize,
    pub unit_assignment_count: usize,
    pub material_count: usize,
    pub storey_count: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkippedExportItem {
    pub reason: String,
    pub count: usize,
    pub classes: std::collections::BTreeMap<String, usize>,
    pub sample_names: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExportConfidenceSummary {
    pub level: String,
    pub score: f32,
    pub has_project_metadata: bool,
    pub has_typed_elements: bool,
    pub has_geometry: bool,
    pub has_diagnostic_proxies: bool,
    pub warning_count: usize,
}

/// User-facing export quality requirement.
///
/// These modes do not change how bytes are decoded; they define how
/// much recovered model data a caller requires before accepting the
/// generated IFC. `Scaffold` is intentionally permissive and preserves
/// the historical `rvt-ifc` behavior. Stronger modes fail loudly instead
/// of writing an IFC that looks more complete than it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExportQualityMode {
    Scaffold,
    TypedNoGeometry,
    Geometry,
    Strict,
}

impl ExportQualityMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Scaffold => "scaffold",
            Self::TypedNoGeometry => "typed-no-geometry",
            Self::Geometry => "geometry",
            Self::Strict => "strict",
        }
    }

    pub fn parse(value: &str) -> std::result::Result<Self, ExportQualityModeParseError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "scaffold" => Ok(Self::Scaffold),
            "typed-no-geometry" | "typed_no_geometry" => Ok(Self::TypedNoGeometry),
            "geometry" => Ok(Self::Geometry),
            "strict" => Ok(Self::Strict),
            _ => Err(ExportQualityModeParseError {
                value: value.to_string(),
            }),
        }
    }

    pub fn validate(
        self,
        diagnostics: &ExportDiagnostics,
    ) -> std::result::Result<(), ExportQualityValidationError> {
        let mut failures = Vec::new();

        match self {
            Self::Scaffold => {}
            Self::TypedNoGeometry => {
                require_typed_elements(diagnostics, &mut failures);
            }
            Self::Geometry => {
                require_typed_elements(diagnostics, &mut failures);
                require_geometry(diagnostics, &mut failures);
            }
            Self::Strict => {
                require_typed_elements(diagnostics, &mut failures);
                require_geometry(diagnostics, &mut failures);
                if !diagnostics.confidence.has_project_metadata {
                    failures.push("no project metadata was recovered".to_string());
                }
                if diagnostics.exported.unit_assignment_count == 0 {
                    failures.push("no Revit unit assignment was recovered".to_string());
                }
                if diagnostics.exported.storey_count == 0 {
                    failures.push("no Revit level/storey data was recovered".to_string());
                }
                if !diagnostics.unsupported_features.is_empty() {
                    failures.push(format!(
                        "unsupported exporter features remain: {}",
                        diagnostics.unsupported_features.join(", ")
                    ));
                }
                if !diagnostics.warnings.is_empty() {
                    failures.push(format!(
                        "{} export warning(s) remain",
                        diagnostics.warnings.len()
                    ));
                }
            }
        }

        if failures.is_empty() {
            Ok(())
        } else {
            Err(ExportQualityValidationError {
                mode: self,
                reason: failures.join("; "),
                confidence_level: diagnostics.confidence.level.clone(),
            })
        }
    }
}

impl std::fmt::Display for ExportQualityMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ExportQualityMode {
    type Err = ExportQualityModeParseError;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        Self::parse(value)
    }
}

#[derive(Debug, Clone, thiserror::Error)]
#[error(
    "unknown IFC export mode `{value}`; expected scaffold, typed-no-geometry, geometry, or strict"
)]
pub struct ExportQualityModeParseError {
    pub value: String,
}

#[derive(Debug, Clone, thiserror::Error)]
#[error(
    "IFC export mode `{mode}` cannot be satisfied: {reason} (confidence level: {confidence_level})"
)]
pub struct ExportQualityValidationError {
    pub mode: ExportQualityMode,
    pub reason: String,
    pub confidence_level: String,
}

fn require_typed_elements(diagnostics: &ExportDiagnostics, failures: &mut Vec<String>) {
    if !diagnostics.confidence.has_typed_elements {
        failures.push(format!(
            "no validated typed IFC elements were exported (building_elements={}, confidence_level={})",
            diagnostics.exported.building_elements, diagnostics.confidence.level
        ));
    }
}

fn require_geometry(diagnostics: &ExportDiagnostics, failures: &mut Vec<String>) {
    if !diagnostics.confidence.has_geometry {
        failures.push(format!(
            "no exported building element has geometry (building_elements_with_geometry={})",
            diagnostics.exported.building_elements_with_geometry
        ));
    }
}

/// Trait every IFC exporter implements. Multiple implementations exist
/// as we phase this up: a null exporter that returns `NotYetImplemented`
/// for everything, a partial one that emits only project+units, and
/// eventually a full one.
pub trait Exporter {
    fn export(&self, rf: &mut crate::RevitFile) -> Result<IfcModel>;
}

/// Returned by exporters that cannot yet produce a given entity class.
#[derive(Debug, Clone, thiserror::Error)]
#[error("IFC export not yet implemented: {reason}")]
pub struct NotYetImplemented {
    pub reason: String,
}

/// Placeholder exporter — returns an `IfcModel` whose only filled
/// field is `project_name` (extracted from PartAtom if it parses).
/// Geometry, categories, and per-element entities are absent. Safe
/// to use as a stand-in for downstream tooling that wants to test
/// the `Exporter` plumbing without requiring real model data.
///
/// For the real document-level exporter with spatial hierarchy +
/// classifications, use [`RvtDocExporter`] instead.
///
/// (Renamed from `NullExporter` in v0.1.3 — the old name implied
/// it returns `NotYetImplemented`, which it does not.)
pub struct PlaceholderExporter;

impl Exporter for PlaceholderExporter {
    fn export(&self, rf: &mut crate::RevitFile) -> Result<IfcModel> {
        let project_name = rf
            .part_atom()
            .ok()
            .and_then(|pa| pa.title)
            .or_else(|| rf.basic_file_info().ok().and_then(|bfi| bfi.original_path));
        Ok(IfcModel {
            project_name,
            description: Some(
                "Partial IFC export via rvt-rs PlaceholderExporter. \
                 Geometry, categories, and elements are pending Layer 5b \
                 walker + Phase 5 geometry work."
                    .into(),
            ),
            entities: Vec::new(),
            classifications: Vec::new(),
            units: Vec::new(),
            building_storeys: Vec::new(),
            materials: Vec::new(),
            material_layer_sets: Vec::new(),
            material_profile_sets: Vec::new(),
            representation_maps: Vec::new(),
        })
    }
}

impl PlaceholderExporter {
    pub fn export_with_diagnostics(&self, rf: &mut crate::RevitFile) -> Result<ExportResult> {
        let model = self.export(rf)?;
        let diagnostics = build_export_diagnostics(rf, &model, ExportDiagnosticsMode::Placeholder);
        Ok(ExportResult { model, diagnostics })
    }
}

/// Document-level exporter — populates an `IfcModel` with project
/// metadata from PartAtom + BasicFileInfo + (when locatable) ADocument's
/// walker-read instance fields. Produces a spec-valid but structurally
/// minimal IFC4 file when paired with `step_writer::write_step`.
///
/// Current coverage: project name + document description + (soon)
/// OmniClass classification reference. Pending walker expansion:
/// units from `autodesk.unit.*` identifiers, categories from the
/// family-graph references, building-element geometry.
pub struct RvtDocExporter;

impl Exporter for RvtDocExporter {
    fn export(&self, rf: &mut crate::RevitFile) -> Result<IfcModel> {
        self.export_with_limits(rf, crate::walker::WalkerLimits::default())
    }
}

impl RvtDocExporter {
    pub fn export_with_limits(
        &self,
        rf: &mut crate::RevitFile,
        limits: crate::walker::WalkerLimits,
    ) -> Result<IfcModel> {
        export_rvt_doc(rf, RvtDocExportMode::Default, limits)
    }

    pub fn export_with_diagnostics(&self, rf: &mut crate::RevitFile) -> Result<ExportResult> {
        self.export_with_diagnostics_and_limits(rf, crate::walker::WalkerLimits::default())
    }

    pub fn export_with_diagnostics_and_limits(
        &self,
        rf: &mut crate::RevitFile,
        limits: crate::walker::WalkerLimits,
    ) -> Result<ExportResult> {
        let model = self.export_with_limits(rf, limits)?;
        let diagnostics = build_export_diagnostics_with_limits(
            rf,
            &model,
            ExportDiagnosticsMode::Default,
            limits,
        );
        Ok(ExportResult { model, diagnostics })
    }
}

/// Diagnostic document exporter.
///
/// This exporter starts from the same conservative model as
/// [`RvtDocExporter`], then appends low-confidence schema-scan hits as
/// `IFCBUILDINGELEMENTPROXY` elements with `Pset_RvtRsDiagnosticCandidate`
/// provenance. It is intended for reverse-engineering and issue
/// attachments, not for normal model exchange.
pub struct DiagnosticRvtDocExporter;

impl Exporter for DiagnosticRvtDocExporter {
    fn export(&self, rf: &mut crate::RevitFile) -> Result<IfcModel> {
        self.export_with_limits(rf, crate::walker::WalkerLimits::default())
    }
}

impl DiagnosticRvtDocExporter {
    pub fn export_with_limits(
        &self,
        rf: &mut crate::RevitFile,
        limits: crate::walker::WalkerLimits,
    ) -> Result<IfcModel> {
        export_rvt_doc(rf, RvtDocExportMode::DiagnosticProxies, limits)
    }

    pub fn export_with_diagnostics(&self, rf: &mut crate::RevitFile) -> Result<ExportResult> {
        self.export_with_diagnostics_and_limits(rf, crate::walker::WalkerLimits::default())
    }

    pub fn export_with_diagnostics_and_limits(
        &self,
        rf: &mut crate::RevitFile,
        limits: crate::walker::WalkerLimits,
    ) -> Result<ExportResult> {
        let model = self.export_with_limits(rf, limits)?;
        let diagnostics = build_export_diagnostics_with_limits(
            rf,
            &model,
            ExportDiagnosticsMode::DiagnosticProxies,
            limits,
        );
        Ok(ExportResult { model, diagnostics })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RvtDocExportMode {
    Default,
    DiagnosticProxies,
}

fn export_rvt_doc(
    rf: &mut crate::RevitFile,
    mode: RvtDocExportMode,
    walker_limits: crate::walker::WalkerLimits,
) -> Result<IfcModel> {
    // Identity from PartAtom if present; fall back to
    // BasicFileInfo's original path.
    let part = rf.part_atom().ok();
    let bfi = rf.basic_file_info().ok();
    let project_name = part
        .as_ref()
        .and_then(|pa| pa.title.clone())
        .or_else(|| bfi.as_ref().and_then(|b| b.original_path.clone()));

    let description = {
        let mut d = Vec::new();
        if let Some(b) = &bfi {
            d.push(format!("Revit {} export", b.version));
            if let Some(message) =
                crate::arc_wall_record::ArcWallRecord::standard_decoder_status(b.version)
                    .diagnostic_message()
            {
                d.push(message);
            }
        }
        if let Some(p) = &part {
            if let Some(id) = &p.id {
                d.push(format!("id={id}"));
            }
        }
        if d.is_empty() {
            None
        } else {
            Some(d.join("; "))
        }
    };

    // OmniClass / Uniformat classification references, if present
    // in PartAtom.
    let mut classifications = Vec::new();
    if let Some(p) = &part {
        let omni_items: Vec<_> = p
            .categories
            .iter()
            .filter(|c| c.term.starts_with(char::is_numeric) && c.term.contains('.'))
            .map(|c| entities::ClassificationItem {
                code: c.term.clone(),
                name: None,
            })
            .collect();
        if !omni_items.is_empty() {
            classifications.push(entities::Classification {
                source: entities::ClassificationSource::OmniClass,
                edition: None,
                items: omni_items,
            });
        }
    }

    // A single IfcProject entity at the model level (step_writer
    // emits its STEP form; other entity types are wired in below
    // from the walker's element stream).
    let mut entities = vec![entities::IfcEntity::Project {
        name: project_name.clone(),
        description: description.clone(),
        long_name: part.as_ref().and_then(|p| p.title.clone()),
    }];

    // L5B-11.7 — pull every walker-recoverable element out of
    // Global/Latest and emit one `BuildingElement` entity per
    // hit. Unknown classes route to IFCBUILDINGELEMENTPROXY via
    // `category_map::lookup`. Walker failure (stream missing,
    // schema unparseable, inflate error) falls through with no
    // element entities — we never regress the metadata-only
    // output. The order — `Project` first, then elements — is
    // load-bearing for `step_writer`, which walks `entities`
    // in order and assumes index 0 is the project.
    append_production_walker_elements(rf, &mut entities, walker_limits);
    if mode == RvtDocExportMode::DiagnosticProxies {
        append_diagnostic_walker_proxy_candidates(rf, &mut entities, walker_limits);
    }

    // RE-14.3 — record-level ArcWall decode path. The walker's
    // generic schema-driven iter_elements() does not recognise
    // Partitions/* record framing, so ArcWall instances (tag
    // 0x0191) are invisible to that path. We scan the partition
    // streams directly here and emit one IFCWALL per standard
    // ArcWall record, alongside the walker's generic output. The
    // scanner is version-gated because the 2024 ArcWall envelope
    // has a different tag/variant distribution.
    //
    // See `reports/element-framing/RE-14.3-synthesis.md` for the
    // wire-format evidence this is based on, and
    // `tests/arc_wall_corpus.rs` for the real-file coverage.
    let partition_streams: Vec<String> = rf
        .stream_names()
        .into_iter()
        .filter(|s| s.starts_with("Partitions/"))
        .collect();
    if let Some(revit_version) = bfi.as_ref().map(|b| b.version) {
        for partition in &partition_streams {
            let Ok(raw) = rf.read_stream(partition) else {
                continue;
            };
            let chunks = crate::compression::inflate_all_chunks(&raw);
            let concat: Vec<u8> = chunks.into_iter().flatten().collect();
            if concat.len() < crate::arc_wall_record::STANDARD_RECORD_MIN_SIZE {
                continue;
            }
            let scan = crate::arc_wall_record::ArcWallRecord::scan_standard_for_revit_version(
                revit_version,
                &concat,
            );
            for off in scan.offsets {
                if let Ok(rec) =
                    crate::arc_wall_record::ArcWallRecord::decode_standard(&concat, off)
                {
                    let (sx, sy, sz) = rec.start_point();
                    let _end = rec.end_point();
                    entities.push(entities::IfcEntity::BuildingElement {
                        ifc_type: "IFCWALL".to_string(),
                        name: format!("ArcWall-{partition}-{off}"),
                        type_guid: None,
                        storey_index: None,
                        material_index: None,
                        property_set: None,
                        location_feet: Some([sx, sy, sz]),
                        rotation_radians: None,
                        extrusion: None,
                        host_element_index: None,
                        material_layer_set_index: None,
                        material_profile_set_index: None,
                        solid_shape: None,
                        representation_map_index: None,
                    });
                }
            }
        }
    }

    Ok(IfcModel {
        project_name,
        description,
        entities,
        classifications,
        units: Vec::new(),
        building_storeys: Vec::new(),
        materials: Vec::new(),
        material_layer_sets: Vec::new(),
        material_profile_sets: Vec::new(),
        representation_maps: Vec::new(),
    })
}

fn append_production_walker_elements(
    rf: &mut crate::RevitFile,
    entities: &mut Vec<entities::IfcEntity>,
    walker_limits: crate::walker::WalkerLimits,
) {
    if let Ok(decoded_iter) = crate::walker::iter_elements_with_limits(
        rf,
        crate::walker::PRODUCTION_ELEMENT_MIN_SCORE,
        walker_limits,
    ) {
        for decoded in decoded_iter {
            let mapping = category_map::lookup(&decoded.class);
            let ifc_type = mapping
                .map(|m| m.ifc_type.to_string())
                .unwrap_or_else(|| "IFCBUILDINGELEMENTPROXY".to_string());
            let name = match decoded.id {
                Some(id) => format!("{}-{}", decoded.class, id),
                None => format!("{}-unnamed", decoded.class),
            };
            let type_guid = decoded.id.map(|id| id.to_string());
            entities.push(entities::IfcEntity::BuildingElement {
                ifc_type,
                name,
                type_guid,
                storey_index: None,
                material_index: None,
                property_set: None,
                location_feet: None,
                rotation_radians: None,
                extrusion: None,
                host_element_index: None,
                material_layer_set_index: None,
                material_profile_set_index: None,
                solid_shape: None,
                representation_map_index: None,
            });
        }
    }
}

fn append_diagnostic_walker_proxy_candidates(
    rf: &mut crate::RevitFile,
    entities: &mut Vec<entities::IfcEntity>,
    walker_limits: crate::walker::WalkerLimits,
) {
    for candidate in collect_diagnostic_walker_proxy_candidates(rf, walker_limits).candidates {
        let name = match candidate.decoded.id {
            Some(id) => format!("{}-{}", candidate.decoded.class, id),
            None => format!(
                "{}-offset-{:x}",
                candidate.decoded.class, candidate.decoded.byte_range.start
            ),
        };
        let property_set = diagnostic_candidate_property_set(&candidate.decoded, candidate.score);

        entities.push(entities::IfcEntity::BuildingElement {
            ifc_type: "IFCBUILDINGELEMENTPROXY".to_string(),
            name,
            type_guid: candidate.decoded.id.map(|id| id.to_string()),
            storey_index: None,
            material_index: None,
            property_set: Some(property_set),
            location_feet: None,
            rotation_radians: None,
            extrusion: None,
            host_element_index: None,
            material_layer_set_index: None,
            material_profile_set_index: None,
            solid_shape: None,
            representation_map_index: None,
        });
    }
}

#[derive(Debug, Clone)]
struct DiagnosticProxyCandidate {
    decoded: crate::walker::DecodedElement,
    score: i64,
}

#[derive(Debug, Default, Clone)]
struct DiagnosticProxyCandidateCollection {
    candidates: Vec<DiagnosticProxyCandidate>,
    warnings: Vec<String>,
}

fn collect_diagnostic_walker_proxy_candidates(
    rf: &mut crate::RevitFile,
    walker_limits: crate::walker::WalkerLimits,
) -> DiagnosticProxyCandidateCollection {
    let mut collection = DiagnosticProxyCandidateCollection::default();

    let formats_raw = match rf.read_stream(crate::streams::FORMATS_LATEST) {
        Ok(raw) => raw,
        Err(err) => {
            collection
                .warnings
                .push(format!("Unable to read Formats/Latest: {err}"));
            return collection;
        }
    };
    let formats_d = match crate::compression::inflate_at(&formats_raw, 0) {
        Ok(bytes) => bytes,
        Err(err) => {
            collection
                .warnings
                .push(format!("Unable to inflate Formats/Latest: {err}"));
            return collection;
        }
    };
    let schema = match crate::formats::parse_schema(&formats_d) {
        Ok(schema) => schema,
        Err(err) => {
            collection
                .warnings
                .push(format!("Unable to parse Formats/Latest schema: {err}"));
            return collection;
        }
    };
    let raw = match rf.read_stream(crate::streams::GLOBAL_LATEST) {
        Ok(raw) => raw,
        Err(err) => {
            collection
                .warnings
                .push(format!("Unable to read Global/Latest: {err}"));
            return collection;
        }
    };
    let (_, latest) = match crate::compression::inflate_at_auto(&raw) {
        Ok(inflated) => inflated,
        Err(err) => {
            collection
                .warnings
                .push(format!("Unable to inflate Global/Latest: {err}"));
            return collection;
        }
    };

    let class_by_name: std::collections::HashMap<&str, &crate::formats::ClassEntry> = schema
        .classes
        .iter()
        .map(|class| (class.name.as_str(), class))
        .collect();
    let scan = crate::walker::scan_candidates_with_limits(
        &schema,
        &latest,
        crate::walker::DIAGNOSTIC_ELEMENT_MIN_SCORE,
        walker_limits,
    );
    if let Some(hit) = scan.limit_hit {
        collection
            .warnings
            .push(format!("{}: {}", hit.code(), hit.message()));
    }
    let mut seen_ids = std::collections::BTreeSet::<u32>::new();
    let mut seen_offsets = std::collections::BTreeSet::<usize>::new();

    for candidate in scan.candidates {
        if candidate.score >= crate::walker::PRODUCTION_ELEMENT_MIN_SCORE {
            continue;
        }
        let Some(class) = class_by_name.get(candidate.class_name.as_str()).copied() else {
            continue;
        };
        let mut decoded = crate::walker::decode_instance_with_limits(
            &latest,
            candidate.offset,
            class,
            walker_limits,
        );
        let self_id = crate::walker::find_self_id_field(class)
            .and_then(|index| decoded.fields.get(index))
            .and_then(|(_, field)| match field {
                crate::walker::InstanceField::ElementId { id, .. } if *id != 0 => Some(*id),
                _ => None,
            });
        if let Some(id) = self_id {
            if !seen_ids.insert(id) {
                continue;
            }
            decoded.id = Some(id);
        } else if !seen_offsets.insert(candidate.offset) {
            continue;
        }

        collection.candidates.push(DiagnosticProxyCandidate {
            decoded,
            score: candidate.score,
        });
    }

    collection
}

fn diagnostic_candidate_property_set(
    decoded: &crate::walker::DecodedElement,
    score: i64,
) -> entities::PropertySet {
    let mut properties = vec![
        entities::Property {
            name: "DiagnosticReason".into(),
            value: entities::PropertyValue::Text(
                "low-confidence schema scan candidate; omitted from default export".into(),
            ),
        },
        entities::Property {
            name: "DecodedClass".into(),
            value: entities::PropertyValue::Text(decoded.class.clone()),
        },
        entities::Property {
            name: "SourceStream".into(),
            value: entities::PropertyValue::Text(crate::streams::GLOBAL_LATEST.into()),
        },
        entities::Property {
            name: "ByteStart".into(),
            value: entities::PropertyValue::Integer(usize_to_i64_saturating(
                decoded.byte_range.start,
            )),
        },
        entities::Property {
            name: "ByteEnd".into(),
            value: entities::PropertyValue::Integer(usize_to_i64_saturating(
                decoded.byte_range.end,
            )),
        },
        entities::Property {
            name: "CandidateScore".into(),
            value: entities::PropertyValue::Integer(score),
        },
    ];
    if let Some(id) = decoded.id {
        properties.push(entities::Property {
            name: "ElementId".into(),
            value: entities::PropertyValue::Integer(i64::from(id)),
        });
    }

    entities::PropertySet {
        name: "Pset_RvtRsDiagnosticCandidate".into(),
        properties,
    }
}

fn usize_to_i64_saturating(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

/// Build the JSON diagnostics sidecar for an exported IFC model.
///
/// This function is intentionally best-effort: stream/schema failures are
/// reported in `warnings` instead of making the already-created IFC model
/// unusable for bug reports.
pub fn build_export_diagnostics(
    rf: &mut crate::RevitFile,
    model: &IfcModel,
    mode: ExportDiagnosticsMode,
) -> ExportDiagnostics {
    build_export_diagnostics_with_limits(rf, model, mode, crate::walker::WalkerLimits::default())
}

pub fn build_export_diagnostics_with_limits(
    rf: &mut crate::RevitFile,
    model: &IfcModel,
    mode: ExportDiagnosticsMode,
    walker_limits: crate::walker::WalkerLimits,
) -> ExportDiagnostics {
    let bfi = rf.basic_file_info().ok();
    let part = rf.part_atom().ok();
    let stream_names = rf.stream_names();
    let diagnostic_candidates = collect_diagnostic_walker_proxy_candidates(rf, walker_limits);
    let exported = exported_model_diagnostics(model);
    let diagnostic_proxy_elements = count_diagnostic_proxy_elements(model);
    let arcwall_records = count_arcwall_records(model);
    let production_walker_elements = exported
        .building_elements
        .saturating_sub(arcwall_records)
        .saturating_sub(diagnostic_proxy_elements);
    let candidate_class_counts = diagnostic_candidate_class_counts(&diagnostic_candidates);

    let mut skipped = Vec::new();
    if mode == ExportDiagnosticsMode::Default && !diagnostic_candidates.candidates.is_empty() {
        skipped.push(SkippedExportItem {
            reason: "low_confidence_schema_scan_candidate".into(),
            count: diagnostic_candidates.candidates.len(),
            classes: candidate_class_counts.clone(),
            sample_names: diagnostic_candidates
                .candidates
                .iter()
                .take(12)
                .map(diagnostic_candidate_display_name)
                .collect(),
        });
    }

    let mut warnings = diagnostic_candidates.warnings;
    if exported.building_elements == 0 {
        warnings.push("No building elements were exported; output is scaffold-only.".into());
    }
    if model.units.is_empty() {
        warnings
            .push("No Revit unit assignment was recovered; STEP output uses default units.".into());
    }
    if model.building_storeys.is_empty() {
        warnings.push(
            "No Revit levels were recovered; STEP output uses the fallback spatial storey.".into(),
        );
    }
    if mode == ExportDiagnosticsMode::Default && !skipped.is_empty() {
        warnings.push(format!(
            "Suppressed {} low-confidence schema scan candidates from default export.",
            skipped[0].count
        ));
    }
    if mode == ExportDiagnosticsMode::DiagnosticProxies && diagnostic_proxy_elements > 0 {
        warnings.push(
            "Diagnostic proxy elements are low-confidence scan candidates, not validated model elements."
                .into(),
        );
    }
    if mode == ExportDiagnosticsMode::Placeholder {
        warnings.push("Placeholder export mode omits decoded elements by design.".into());
    }

    let has_project_metadata = model.project_name.is_some()
        || bfi.is_some()
        || part.as_ref().and_then(|p| p.title.as_ref()).is_some();
    let confidence = export_confidence_summary(
        mode,
        &exported,
        has_project_metadata,
        diagnostic_proxy_elements,
        warnings.len(),
    );

    ExportDiagnostics {
        schema_version: EXPORT_DIAGNOSTICS_SCHEMA_VERSION,
        mode,
        input: ExportInputDiagnostics {
            revit_version: bfi.as_ref().map(|b| b.version),
            build: bfi.as_ref().and_then(|b| b.build.clone()),
            original_path: bfi
                .as_ref()
                .and_then(|b| b.original_path.as_ref())
                .map(|path| crate::redact::redact_sensitive(path)),
            project_name: model
                .project_name
                .clone()
                .or_else(|| part.as_ref().and_then(|p| p.title.clone())),
            stream_count: stream_names.len(),
            has_basic_file_info: bfi.is_some(),
            has_part_atom: part.is_some(),
            has_formats_latest: stream_names
                .iter()
                .any(|name| name == crate::streams::FORMATS_LATEST),
            has_global_latest: stream_names
                .iter()
                .any(|name| name == crate::streams::GLOBAL_LATEST),
        },
        decoded: DecodedExportDiagnostics {
            production_walker_elements,
            diagnostic_proxy_candidates: diagnostic_candidates.candidates.len(),
            arcwall_records,
            class_counts: candidate_class_counts,
        },
        exported,
        skipped,
        unsupported_features: unsupported_export_features(model),
        warnings,
        confidence,
    }
}

fn exported_model_diagnostics(model: &IfcModel) -> ExportedModelDiagnostics {
    let mut by_ifc_type = std::collections::BTreeMap::<String, usize>::new();
    let mut building_elements = 0usize;
    let mut building_elements_with_geometry = 0usize;
    for entity in &model.entities {
        if let entities::IfcEntity::BuildingElement {
            ifc_type,
            location_feet,
            extrusion,
            solid_shape,
            representation_map_index,
            ..
        } = entity
        {
            building_elements += 1;
            *by_ifc_type.entry(ifc_type.clone()).or_insert(0) += 1;
            if location_feet.is_some()
                && (extrusion.is_some()
                    || solid_shape.is_some()
                    || representation_map_index.is_some())
            {
                building_elements_with_geometry += 1;
            }
        }
    }

    ExportedModelDiagnostics {
        total_entities: model.entities.len(),
        building_elements,
        building_elements_with_geometry,
        by_ifc_type,
        classification_count: model.classifications.len(),
        unit_assignment_count: model.units.len(),
        material_count: model.materials.len(),
        storey_count: model.building_storeys.len(),
    }
}

fn count_arcwall_records(model: &IfcModel) -> usize {
    model
        .entities
        .iter()
        .filter(|entity| {
            matches!(
                entity,
                entities::IfcEntity::BuildingElement { name, .. } if name.starts_with("ArcWall-")
            )
        })
        .count()
}

fn count_diagnostic_proxy_elements(model: &IfcModel) -> usize {
    model
        .entities
        .iter()
        .filter(|entity| {
            matches!(
                entity,
                entities::IfcEntity::BuildingElement {
                    property_set: Some(property_set),
                    ..
                } if property_set.name == "Pset_RvtRsDiagnosticCandidate"
            )
        })
        .count()
}

fn diagnostic_candidate_class_counts(
    collection: &DiagnosticProxyCandidateCollection,
) -> std::collections::BTreeMap<String, usize> {
    let mut out = std::collections::BTreeMap::new();
    for candidate in &collection.candidates {
        *out.entry(candidate.decoded.class.clone()).or_insert(0) += 1;
    }
    out
}

fn diagnostic_candidate_display_name(candidate: &DiagnosticProxyCandidate) -> String {
    match candidate.decoded.id {
        Some(id) => format!("{}-{id}", candidate.decoded.class),
        None => format!(
            "{}-offset-{:x}",
            candidate.decoded.class, candidate.decoded.byte_range.start
        ),
    }
}

fn unsupported_export_features(model: &IfcModel) -> Vec<String> {
    let exported = exported_model_diagnostics(model);
    let mut features = Vec::new();
    if model.units.is_empty() {
        features.push("project_units_from_revit_bytes".into());
    }
    if model.building_storeys.is_empty() {
        features.push("revit_levels_to_ifc_storeys".into());
    }
    if exported.building_elements_with_geometry == 0 {
        features.push("real_file_element_geometry".into());
    }
    if model.materials.is_empty() {
        features.push("revit_materials_and_compound_assemblies".into());
    }
    features.push("partition_decoders_for_doors_windows_floors_and_mvp_classes".into());
    features
}

fn export_confidence_summary(
    mode: ExportDiagnosticsMode,
    exported: &ExportedModelDiagnostics,
    has_project_metadata: bool,
    diagnostic_proxy_elements: usize,
    warning_count: usize,
) -> ExportConfidenceSummary {
    let has_typed_elements = exported
        .by_ifc_type
        .iter()
        .any(|(ifc_type, count)| *count > 0 && ifc_type != "IFCBUILDINGELEMENTPROXY");
    let has_geometry = exported.building_elements_with_geometry > 0;
    let has_diagnostic_proxies = diagnostic_proxy_elements > 0;
    let level = if mode == ExportDiagnosticsMode::Placeholder || exported.building_elements == 0 {
        "scaffold"
    } else if has_geometry {
        "geometry"
    } else if has_diagnostic_proxies {
        "diagnostic_partial"
    } else if has_typed_elements {
        "typed_no_geometry"
    } else {
        "proxy_only"
    };

    let mut score = 0.10f32;
    if has_project_metadata {
        score += 0.15;
    }
    if exported.unit_assignment_count > 0 {
        score += 0.10;
    }
    if exported.building_elements > 0 {
        score += 0.20;
    }
    if has_typed_elements {
        score += 0.20;
    }
    if has_geometry {
        score += 0.25;
    }
    if has_diagnostic_proxies {
        score -= 0.05;
    }

    ExportConfidenceSummary {
        level: level.into(),
        score: score.clamp(0.0, 1.0),
        has_project_metadata,
        has_typed_elements,
        has_geometry,
        has_diagnostic_proxies,
        warning_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn placeholder_exporter_default_model_has_no_name() {
        let m = IfcModel::default();
        assert!(m.project_name.is_none());
    }
}
