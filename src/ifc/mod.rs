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
        if let Ok(decoded_iter) = crate::walker::iter_elements(rf) {
            for decoded in decoded_iter {
                let mapping = super::ifc::category_map::lookup(&decoded.class);
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
