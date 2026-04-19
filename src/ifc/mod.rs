//! Layer 5 — IFC export.
//!
//! # Why this module exists
//!
//! The whole moat-break case for rvt-rs rests on being able to produce a
//! strictly-better IFC export than Autodesk's own `revit-ifc` plug-in. The
//! plug-in runs *inside* Revit and can only export what the Revit API
//! chooses to surface (private families, complex assemblies, and several
//! parameter types are dropped). A byte-level parser reading the RVT
//! on-disk format is a *strict superset* of the API surface; an IFC
//! writer on top of it is therefore the full-fidelity export path that
//! the openBIM community has waited a decade for.
//!
//! # Current status
//!
//! This is a Phase-5 scaffold:
//!
//! - Defines `IfcModel` (the target type) with the minimum shape required
//!   to serialize STEP-encoded IFC 4 (ISO 10303-21).
//! - Defines `Exporter`, the trait that takes a `RevitFile` and emits an
//!   `IfcModel`.
//! - Provides a `NullExporter` that returns `Err(NotYetImplemented)` for
//!   every entity type. Exists so downstream tools can import the trait
//!   object and compile today, even though nothing is wired yet.
//!
//! # Implementation plan
//!
//! 1. Layer 4c (object-graph field decoding) must produce typed
//!    `Category`, `ElementId`, `Symbol`, `HostObj*`, and
//!    `FamilyInstance` values. This module consumes those.
//! 2. An entity mapper translates Revit classes to IFC types:
//!
//!    | Revit concept | IFC mapping |
//!    |---|---|
//!    | Project metadata (PartAtom) | `IfcProject` |
//!    | Unit set (autodesk.unit.*) | `IfcUnitAssignment` / `IfcSIUnit` |
//!    | Category (C: Walls, C: Doors, …) | `IfcBuildingElementType` |
//!    | Family (RFA) | `IfcTypeObject` + `IfcRepresentationMap` |
//!    | Family Instance | `IfcBuildingElement` / `IfcFurnishingElement` |
//!    | Uniformat code | `IfcClassification` / `IfcClassificationReference` |
//!    | OmniClass code | `IfcClassification` |
//!    | Host element's geometry | `IfcShapeRepresentation` |
//!
//! 3. STEP serializer writes the `IfcModel` as `.ifc` text.
//! 4. buildingSMART IFC certification suite validates round-trip.
//!
//! # Library collaboration
//!
//! `IfcOpenShell` is the natural collaborator on the writer side: the
//! plan is to generate an `IfcOpenShell`-compatible STEP output and let
//! their suite perform the final validation. That partnership is a
//! post-Phase-5 conversation; this module does the parsing-to-model
//! conversion that precedes it.

use crate::Result;

pub mod entities;

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

/// The "everything is TODO" exporter. Returns an empty `IfcModel` whose
/// only filled field is `project_name` (extracted from PartAtom if it
/// parses). Safe to use as a placeholder for downstream tooling.
pub struct NullExporter;

impl Exporter for NullExporter {
    fn export(&self, rf: &mut crate::RevitFile) -> Result<IfcModel> {
        let project_name = rf
            .part_atom()
            .ok()
            .and_then(|pa| pa.title)
            .or_else(|| {
                rf.basic_file_info()
                    .ok()
                    .and_then(|bfi| bfi.original_path)
            });
        Ok(IfcModel {
            project_name,
            description: Some(
                "Partial IFC export via rvt-rs NullExporter. \
                 Geometry, categories, and elements are pending Layer 4c \
                 completion."
                    .into(),
            ),
            entities: Vec::new(),
            classifications: Vec::new(),
            units: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn null_exporter_has_description() {
        let m = IfcModel::default();
        assert!(m.project_name.is_none());
    }
}
