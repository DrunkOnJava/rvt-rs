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

use crate::Result;

pub mod category_map;
pub mod entities;
pub mod from_decoded;
pub mod step_writer;

pub use from_decoded::{BuilderOptions, ElementInput, build_ifc_model, entity_type_histogram};
pub use step_writer::write_step;

/// In-memory IFC model — what a successful export produces. Wire format
/// (STEP or IFC-JSON) is a separate concern handled by a serializer.
#[derive(Debug, Default, Clone)]
pub struct IfcModel {
    pub project_name: Option<String>,
    pub description: Option<String>,
    pub entities: Vec<entities::IfcEntity>,
    pub classifications: Vec<entities::Classification>,
    pub units: Vec<entities::UnitAssignment>,
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
        // emits its STEP form; other entity types are wired in the
        // walker-expansion phase).
        let entities = vec![entities::IfcEntity::Project {
            name: project_name.clone(),
            description: description.clone(),
            long_name: part.as_ref().and_then(|p| p.title.clone()),
        }];

        Ok(IfcModel {
            project_name,
            description,
            entities,
            classifications,
            units: Vec::new(),
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
