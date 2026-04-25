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
//!   Einhoven sample *must* produce at least one `BuildingElement`
//!   entity — the walker pinned this at 9 HostObjAttr hits in the
//!   L5B-11.7 bring-up. This is the production assertion that the
//!   walker→IFC plumbing actually threads data through rather than
//!   silently emitting metadata-only output.

mod common;

use common::{ALL_YEARS, sample_for_year, samples_dir};
use rvt::ifc::{Exporter, RvtDocExporter, entities::IfcEntity, write_step};
use rvt::{Result, RevitFile};

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

        // Element count in the entity list must match the element
        // count emitted by step_writer. Use a loose grep — any of
        // the concrete element IFC types produced by the walker
        // fallback path counts.
        let step_be_count = step.matches("IFCBUILDINGELEMENTPROXY(").count();
        // step_writer adds framework-level proxies for openings etc,
        // so `step_be_count >= be_count` is the invariant we can
        // safely assert — not strict equality.
        assert!(
            step_be_count >= be_count,
            "{year}: model claims {be_count} BuildingElement entities but STEP \
             emitted only {step_be_count} IFCBUILDINGELEMENTPROXY lines"
        );
    }
    Ok(())
}

#[test]
fn walker_to_ifc_einhoven_project_has_nonzero_elements() -> Result<()> {
    // This is the production-coverage assertion: a real project
    // `.rvt` from Revit 2023 must yield >= 1 walker-recovered
    // element. If it doesn't, the walker → IFC plumbing regressed
    // (or scan_candidates' score threshold was tightened without
    // also tightening the scan_candidates -> iter_elements dedup).
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

    // Also check STEP output — the walker-emitted entities must
    // actually appear in the serialised IFC.
    let step = write_step(&model);
    let proxy_count = step.matches("IFCBUILDINGELEMENTPROXY(").count();
    assert!(
        proxy_count >= be_count,
        "STEP writer emitted {proxy_count} IFCBUILDINGELEMENTPROXY lines \
         but the model claims {be_count} BuildingElement entities — \
         step_writer is dropping walker output"
    );

    eprintln!(
        "Einhoven 2023: {be_count} walker BuildingElements, \
         {proxy_count} IFCBUILDINGELEMENTPROXY in STEP"
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
    let model = RvtDocExporter.export(&mut rf)?;

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

    let step = write_step(&model);
    let step_wall_count = step.matches("IFCWALL(").count();
    assert!(
        step_wall_count >= 1,
        "STEP writer emitted {step_wall_count} IFCWALL lines — expected ≥1"
    );

    eprintln!(
        "DEC-05: Einhoven yields {wall_count} IFCWALL model entities, \
         {step_wall_count} IFCWALL lines in STEP output"
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
