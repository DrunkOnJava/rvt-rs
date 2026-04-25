//! L5B-11.8 integration test — walker → IFC pipeline end-to-end.
//!
//! Asserts that [`rvt::ifc::RvtDocExporter`] successfully threads
//! walker-recovered elements into the IFC4 STEP output without
//! regressing the metadata-only baseline (the 11-release family
//! corpus still needs to produce a spec-valid IfcProject + framework
//! entities even when the walker finds zero elements).
//!
//! Two tiers of coverage:
//!
//! - **Family corpus** (always available via `RVT_SAMPLES_DIR` or the
//!   default `../../samples/` path): for every release 2016–2026,
//!   the export must succeed and the STEP envelope must be valid.
//!   Zero walker elements is acceptable — families carry little
//!   instance data, so walker coverage varies by release.
//!
//! - **Project corpus** (gated on `RVT_PROJECT_CORPUS_DIR`, typically
//!   `/private/tmp/rvt-corpus-probe/magnetar/Revit`): the 2023
//!   Einhoven sample must produce non-empty element output through
//!   the version-gated ArcWall partition decoder, while production
//!   walker iteration must suppress low-confidence `HostObjAttr`
//!   candidates that remain available to diagnostic probes.

mod common;

use common::{ALL_YEARS, sample_for_year, samples_dir};
use rvt::ifc::{
    DiagnosticRvtDocExporter, ExportQualityMode, Exporter, RvtDocExporter, entities::IfcEntity,
    write_step,
};
use rvt::{Result, RevitFile, walker};

fn corpus_available() -> bool {
    ALL_YEARS.iter().all(|y| sample_for_year(*y).exists())
}

/// Count the `BuildingElement` entities produced by the exporter.
/// Metadata-only output is zero; walker-wired output is > 0.
fn count_building_elements(model: &rvt::ifc::IfcModel) -> usize {
    model
        .entities
        .iter()
        .filter(|e| matches!(e, IfcEntity::BuildingElement { .. }))
        .count()
}

fn count_step_building_elements_for_model(model: &rvt::ifc::IfcModel, step: &str) -> usize {
    let mut ifc_types = std::collections::BTreeSet::new();
    for entity in &model.entities {
        if let IfcEntity::BuildingElement { ifc_type, .. } = entity {
            ifc_types.insert(ifc_type.as_str());
        }
    }
    ifc_types
        .into_iter()
        .map(|ifc_type| step.matches(&format!("{ifc_type}(")).count())
        .sum()
}

#[test]
fn walker_to_ifc_every_family_release_produces_valid_step() -> Result<()> {
    if !corpus_available() {
        eprintln!(
            "skipping walker-to-ifc integration: family corpus missing at {}",
            samples_dir().display()
        );
        return Ok(());
    }

    for year in ALL_YEARS {
        let path = sample_for_year(year);
        let mut rf = RevitFile::open(&path)?;
        let model = RvtDocExporter.export(&mut rf)?;

        // Must contain at least the IfcProject entity — metadata-only
        // baseline. The walker integration MUST NOT accidentally
        // evict the Project entity or reorder the entity list.
        let project_count = model
            .entities
            .iter()
            .filter(|e| matches!(e, IfcEntity::Project { .. }))
            .count();
        assert_eq!(
            project_count, 1,
            "{year}: expected exactly one IfcProject, got {project_count}"
        );
        assert!(
            matches!(model.entities.first(), Some(IfcEntity::Project { .. })),
            "{year}: entities[0] must be the IfcProject (load-bearing for \
             step_writer — BuildingElement entities append after)"
        );

        // Walker element count is release-dependent on family corpus —
        // some releases have 0 walker-recovered instances (families
        // carry mostly type definitions, not instances). Just log the
        // count so CI output shows coverage; don't assert.
        let be_count = count_building_elements(&model);
        eprintln!("year {year}: {be_count} BuildingElement entities from walker");

        let step = write_step(&model);
        assert!(
            step.starts_with("ISO-10303-21;\n"),
            "{year}: walker wiring broke STEP header"
        );
        assert!(
            step.ends_with("END-ISO-10303-21;\n"),
            "{year}: walker wiring broke STEP terminator"
        );
        assert!(
            step.contains("IFCPROJECT("),
            "{year}: walker wiring lost the IfcProject entity"
        );

        // Element count in the entity list must be reflected in the
        // serialised STEP output. Count only the IFC element types the
        // model actually contains so this stays valid as production
        // output moves away from generic proxies.
        let step_be_count = count_step_building_elements_for_model(&model, &step);
        assert!(
            step_be_count >= be_count,
            "{year}: model claims {be_count} BuildingElement entities but STEP \
             emitted only {step_be_count} matching element constructor lines"
        );
    }
    Ok(())
}

#[test]
fn export_diagnostics_sidecar_reports_default_ifc_readiness() -> Result<()> {
    if !corpus_available() {
        eprintln!(
            "skipping export diagnostics assertion: family corpus missing at {}",
            samples_dir().display()
        );
        return Ok(());
    }

    let mut rf = RevitFile::open(sample_for_year(2024))?;
    let result = RvtDocExporter.export_with_diagnostics(&mut rf)?;
    let diagnostics = result.diagnostics;

    assert_eq!(diagnostics.schema_version, 1);
    assert_eq!(diagnostics.mode, rvt::ifc::ExportDiagnosticsMode::Default);
    assert!(diagnostics.input.has_formats_latest);
    assert!(diagnostics.input.has_global_latest);
    assert!(diagnostics.exported.total_entities >= 1);
    assert_eq!(
        diagnostics.exported.building_elements,
        count_building_elements(&result.model)
    );
    assert!(
        diagnostics.exported.unit_assignment_count >= 1,
        "2024 family corpus should recover at least one Revit unit assignment"
    );
    assert!(
        diagnostics
            .decoded
            .recovered_unit_identifiers
            .iter()
            .any(|unit| unit == "autodesk.unit.unit:meters-1.0.0"),
        "2024 family corpus should report its modal metric length unit"
    );
    assert!(
        !diagnostics
            .unsupported_features
            .iter()
            .any(|feature| feature == "project_units_from_revit_bytes"),
        "unit recovery should remove project_units_from_revit_bytes from unsupported features"
    );
    let step = write_step(&result.model);
    assert!(
        step.contains("IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.)"),
        "real-file unit recovery should emit metre length units instead of defaulting to millimetres"
    );
    assert!(
        !step.contains("IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.)"),
        "real-file unit recovery must not blindly use the legacy millimetre length default"
    );
    assert_eq!(
        diagnostics.confidence.warning_count,
        diagnostics.warnings.len()
    );

    let json = serde_json::to_value(&diagnostics).expect("diagnostics should serialize");
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["mode"], "default");
    assert!(json["input"].is_object());
    assert!(json["decoded"].is_object());
    assert!(json["exported"].is_object());
    assert!(json["warnings"].is_array());
    assert!(json["confidence"].is_object());

    Ok(())
}

#[test]
fn export_quality_modes_reject_incomplete_outputs() -> Result<()> {
    if !corpus_available() {
        eprintln!(
            "skipping export quality mode assertion: family corpus missing at {}",
            samples_dir().display()
        );
        return Ok(());
    }

    let mut rf = RevitFile::open(sample_for_year(2024))?;
    let result = RvtDocExporter.export_with_diagnostics(&mut rf)?;

    ExportQualityMode::Scaffold
        .validate(&result.diagnostics)
        .expect("scaffold mode should preserve historical IFC output");
    let strict_err = ExportQualityMode::Strict
        .validate(&result.diagnostics)
        .expect_err("strict mode must reject scaffold-only output");
    assert_eq!(strict_err.mode, ExportQualityMode::Strict);
    assert!(
        strict_err
            .reason
            .contains("no validated typed IFC elements")
            || strict_err
                .reason
                .contains("no exported building element has geometry"),
        "strict error should identify missing model data: {strict_err}"
    );

    Ok(())
}

#[test]
fn walker_to_ifc_einhoven_project_has_nonzero_elements() -> Result<()> {
    // This is the production-coverage assertion: a real project
    // `.rvt` from Revit 2023 must yield >= 1 element without falling
    // back to the old low-confidence HostObjAttr proxy output.
    let project_dir = match std::env::var("RVT_PROJECT_CORPUS_DIR") {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping project-corpus assertion: RVT_PROJECT_CORPUS_DIR unset");
            return Ok(());
        }
    };
    let path = format!("{project_dir}/Revit_IFC5_Einhoven.rvt");
    if !std::path::Path::new(&path).exists() {
        eprintln!("skipping: corpus file missing at {path}");
        return Ok(());
    }

    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export(&mut rf)?;
    let be_count = count_building_elements(&model);
    assert!(
        be_count >= 1,
        "walker should recover at least 1 element from Einhoven 2023 — \
         got {be_count}. Either iter_elements' min_score default was \
         tightened or scan_candidates regressed."
    );

    let host_obj_attr_count = model
        .entities
        .iter()
        .filter(|e| match e {
            IfcEntity::BuildingElement { name, .. } => name.starts_with("HostObjAttr-"),
            _ => false,
        })
        .count();
    assert_eq!(
        host_obj_attr_count, 0,
        "production IFC export must not surface HostObjAttr-* proxy elements"
    );

    // Also check STEP output — model element entities must
    // actually appear in the serialised IFC.
    let step = write_step(&model);
    let step_element_count = count_step_building_elements_for_model(&model, &step);
    assert!(
        step_element_count >= be_count,
        "STEP writer emitted {step_element_count} matching element lines \
         but the model claims {be_count} BuildingElement entities — \
         step_writer is dropping walker output"
    );

    eprintln!(
        "Einhoven 2023: {be_count} walker BuildingElements, \
         {step_element_count} matching element constructor lines in STEP"
    );
    Ok(())
}

#[test]
fn production_iter_elements_filters_hostobjattr_diagnostic_candidates() -> Result<()> {
    let project_dir = match std::env::var("RVT_PROJECT_CORPUS_DIR") {
        Ok(d) => d,
        Err(_) => {
            eprintln!(
                "skipping iter_elements false-positive assertion: RVT_PROJECT_CORPUS_DIR unset"
            );
            return Ok(());
        }
    };
    let path = format!("{project_dir}/Revit_IFC5_Einhoven.rvt");
    if !std::path::Path::new(&path).exists() {
        eprintln!("skipping iter_elements false-positive assertion: corpus file missing at {path}");
        return Ok(());
    }

    let mut production_rf = RevitFile::open(&path)?;
    let production: Vec<_> = walker::iter_elements(&mut production_rf)?.collect();
    assert!(
        production
            .iter()
            .all(|element| element.class != "HostObjAttr"),
        "production iter_elements must not return HostObjAttr parent-only candidates: {production:?}"
    );

    let mut diagnostic_rf = RevitFile::open(&path)?;
    let diagnostic: Vec<_> = walker::iter_elements_with_options(
        &mut diagnostic_rf,
        walker::DIAGNOSTIC_ELEMENT_MIN_SCORE,
    )?
    .collect();
    assert!(
        diagnostic
            .iter()
            .any(|element| element.class == "HostObjAttr"),
        "diagnostic element iteration should still expose HostObjAttr candidates for research"
    );

    Ok(())
}

#[test]
fn diagnostic_ifc_export_includes_hostobjattr_proxies_with_provenance() -> Result<()> {
    let project_dir = match std::env::var("RVT_PROJECT_CORPUS_DIR") {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping diagnostic IFC proxy assertion: RVT_PROJECT_CORPUS_DIR unset");
            return Ok(());
        }
    };
    let path = format!("{project_dir}/Revit_IFC5_Einhoven.rvt");
    if !std::path::Path::new(&path).exists() {
        eprintln!("skipping diagnostic IFC proxy assertion: corpus file missing at {path}");
        return Ok(());
    }

    let mut rf = RevitFile::open(&path)?;
    let model = DiagnosticRvtDocExporter.export(&mut rf)?;
    let diagnostic_proxy = model.entities.iter().find_map(|entity| match entity {
        IfcEntity::BuildingElement {
            ifc_type,
            name,
            property_set: Some(property_set),
            ..
        } if ifc_type == "IFCBUILDINGELEMENTPROXY"
            && name.starts_with("HostObjAttr-")
            && property_set.name == "Pset_RvtRsDiagnosticCandidate" =>
        {
            Some(property_set)
        }
        _ => None,
    });
    let Some(property_set) = diagnostic_proxy else {
        panic!("diagnostic export should include HostObjAttr proxy candidates with provenance");
    };

    let property_names: std::collections::BTreeSet<&str> = property_set
        .properties
        .iter()
        .map(|property| property.name.as_str())
        .collect();
    for required in [
        "DiagnosticReason",
        "DecodedClass",
        "SourceStream",
        "ByteStart",
        "ByteEnd",
        "CandidateScore",
        "ElementId",
    ] {
        assert!(
            property_names.contains(required),
            "diagnostic property set should contain {required}; got {property_names:?}"
        );
    }

    let step = write_step(&model);
    assert!(
        step.contains("IFCBUILDINGELEMENTPROXY("),
        "diagnostic STEP output should emit proxy constructors"
    );
    assert!(
        step.contains("Pset_RvtRsDiagnosticCandidate"),
        "diagnostic STEP output should preserve provenance property set"
    );

    Ok(())
}

#[test]
fn arcwall_decoder_yields_ifcwall_on_einhoven() -> Result<()> {
    // DEC-05 per RE-14.3: verify that the record-level ArcWall decoder
    // path in RvtDocExporter produces IFCWALL entities > 0 on a real
    // corpus file. The empirical RE-14.3 count is 28 standard ArcWall
    // records on Einhoven Partitions/5 (2 compound + 4 metadata/index
    // records are correctly not decoded by the standard path).
    let project_dir = match std::env::var("RVT_PROJECT_CORPUS_DIR") {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping DEC-05 assertion: RVT_PROJECT_CORPUS_DIR unset");
            return Ok(());
        }
    };
    let path = format!("{project_dir}/Revit_IFC5_Einhoven.rvt");
    if !std::path::Path::new(&path).exists() {
        eprintln!("skipping DEC-05: corpus file missing at {path}");
        return Ok(());
    }

    let mut rf = RevitFile::open(&path)?;
    let result = RvtDocExporter.export_with_diagnostics(&mut rf)?;
    let model = result.model;

    // Count IFCWALL entities in the model (by scanning entity variants
    // that carry an `ifc_type` field equal to "IFCWALL").
    let wall_count = model
        .entities
        .iter()
        .filter_map(|e| match e {
            rvt::ifc::entities::IfcEntity::BuildingElement { ifc_type, .. } => {
                Some(ifc_type.as_str())
            }
            _ => None,
        })
        .filter(|t| *t == "IFCWALL")
        .count();
    assert!(
        wall_count >= 10,
        "expected ≥10 IFCWALL entities on Einhoven (RE-14.3 observed 26 \
         standard ArcWall records); got {wall_count}"
    );
    let wall_geometry_count = model
        .entities
        .iter()
        .filter(|e| {
            matches!(
                e,
                rvt::ifc::entities::IfcEntity::BuildingElement {
                    ifc_type,
                    location_feet: Some(_),
                    extrusion: Some(_),
                    ..
                } if ifc_type == "IFCWALL"
            )
        })
        .count();
    assert!(
        wall_geometry_count >= 10,
        "expected ≥10 IFCWALL entities with rough ArcWall extrusion geometry; got {wall_geometry_count}"
    );
    assert!(
        result.diagnostics.exported.building_elements_with_geometry >= wall_geometry_count,
        "diagnostics should report the geometry-bearing wall count"
    );
    assert!(
        !result
            .diagnostics
            .unsupported_features
            .iter()
            .any(|feature| feature == "real_file_element_geometry"),
        "real_file_element_geometry should not remain unsupported once ArcWall geometry is emitted"
    );

    let step = write_step(&model);
    let step_wall_count = step.matches("IFCWALL(").count();
    assert!(
        step_wall_count >= 1,
        "STEP writer emitted {step_wall_count} IFCWALL lines — expected ≥1"
    );
    assert!(
        step.matches("IFCEXTRUDEDAREASOLID(").count() >= wall_geometry_count,
        "STEP should emit one swept solid per geometry-bearing ArcWall"
    );
    assert!(
        step.matches("IFCSHAPEREPRESENTATION(").count() >= wall_geometry_count,
        "STEP should attach shape representations to geometry-bearing ArcWalls"
    );

    eprintln!(
        "DEC-05: Einhoven yields {wall_count} IFCWALL model entities, \
         {wall_geometry_count} with geometry, {step_wall_count} IFCWALL lines in STEP output"
    );
    Ok(())
}

#[test]
fn arcwall_2024_project_does_not_run_2023_decoder() -> Result<()> {
    let project_dir = match std::env::var("RVT_PROJECT_CORPUS_DIR") {
        Ok(d) => d,
        Err(_) => {
            eprintln!("skipping 2024 ArcWall guard assertion: RVT_PROJECT_CORPUS_DIR unset");
            return Ok(());
        }
    };
    let path = format!("{project_dir}/2024_Core_Interior.rvt");
    if !std::path::Path::new(&path).exists() {
        eprintln!("skipping 2024 ArcWall guard assertion: corpus file missing at {path}");
        return Ok(());
    }

    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export(&mut rf)?;

    let arcwall_names: Vec<&str> = model
        .entities
        .iter()
        .filter_map(|e| match e {
            rvt::ifc::entities::IfcEntity::BuildingElement { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .filter(|name| name.starts_with("ArcWall-"))
        .collect();
    assert!(
        arcwall_names.is_empty(),
        "2024 export must not emit ArcWall-* entities from the 2023 decoder: {arcwall_names:?}"
    );

    let description = model.description.as_deref().unwrap_or_default();
    assert!(
        description.contains("ArcWall standard decoder skipped: Revit 2024"),
        "2024 export should surface the ArcWall version gate diagnostic, got: {description}"
    );

    Ok(())
}
