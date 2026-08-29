//! Lane Seven — IFC export quality modes on redistributable tier1 fixtures.
//!
//! Tier1 synthetics are scaffold-oriented: stronger modes must fail
//! closed, default/scaffold must not emit HostObjAttr proxies, and
//! the diagnostics sidecar must match the published schema shape.

use rvt::ifc::{
    ExportDiagnosticsMode, ExportQualityMode, Exporter, RvtDocExporter, entities::IfcEntity,
    write_step,
};
use rvt::{Result, RevitFile};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn tier1_arch() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("corpus/tier1/architectural-2024/architectural-2024.rvt")
}

fn open_tier1() -> Result<RevitFile> {
    let path = tier1_arch();
    assert!(
        path.exists(),
        "tier1 architectural fixture missing at {}",
        path.display()
    );
    RevitFile::open(&path)
}

fn host_obj_attr_names(model: &rvt::ifc::IfcModel) -> Vec<String> {
    model
        .entities
        .iter()
        .filter_map(|e| match e {
            IfcEntity::BuildingElement { name, .. } if name.starts_with("HostObjAttr-") => {
                Some(name.clone())
            }
            _ => None,
        })
        .collect()
}

fn temp_dir(prefix: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("{prefix}-{}-{nonce}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

#[test]
fn export_quality_mode_parse_accepts_aliases() {
    assert_eq!(
        ExportQualityMode::parse("scaffold").unwrap(),
        ExportQualityMode::Scaffold
    );
    assert_eq!(
        ExportQualityMode::parse("typed-no-geometry").unwrap(),
        ExportQualityMode::TypedNoGeometry
    );
    assert_eq!(
        ExportQualityMode::parse("typed_no_geometry").unwrap(),
        ExportQualityMode::TypedNoGeometry
    );
    assert_eq!(
        ExportQualityMode::parse("geometry").unwrap(),
        ExportQualityMode::Geometry
    );
    assert_eq!(
        ExportQualityMode::parse("strict").unwrap(),
        ExportQualityMode::Strict
    );
    assert!(ExportQualityMode::parse("nope").is_err());
}

#[test]
fn tier1_scaffold_export_has_no_hostobjattr_proxies() -> Result<()> {
    let mut rf = open_tier1()?;
    let result =
        RvtDocExporter.export_with_diagnostics_mode_and_limits(
            &mut rf,
            ExportQualityMode::Scaffold,
            Default::default(),
        )?;

    assert_eq!(result.diagnostics.mode, ExportDiagnosticsMode::Default);
    assert!(
        host_obj_attr_names(&result.model).is_empty(),
        "default/scaffold must not emit HostObjAttr-* proxies: {:?}",
        host_obj_attr_names(&result.model)
    );

    ExportQualityMode::Scaffold
        .validate(&result.diagnostics)
        .expect("scaffold accepts tier1 envelope");

    let step = write_step(&result.model);
    assert!(step.contains("IFCPROJECT("));
    assert!(!step.contains("HostObjAttr-"));
    Ok(())
}

#[test]
fn tier1_stronger_modes_fail_closed() -> Result<()> {
    let mut rf = open_tier1()?;
    let result =
        RvtDocExporter.export_with_diagnostics_mode_and_limits(
            &mut rf,
            ExportQualityMode::Scaffold,
            Default::default(),
        )?;

    for mode in [
        ExportQualityMode::TypedNoGeometry,
        ExportQualityMode::Geometry,
        ExportQualityMode::Strict,
    ] {
        let err = mode
            .validate(&result.diagnostics)
            .expect_err("tier1 synthetics are scaffold-only");
        assert_eq!(err.mode, mode);
    }
    Ok(())
}

#[test]
fn tier1_typed_no_geometry_mode_strips_geometry_claims() -> Result<()> {
    let mut rf = open_tier1()?;
    let result =
        RvtDocExporter.export_with_diagnostics_mode_and_limits(
            &mut rf,
            ExportQualityMode::TypedNoGeometry,
            Default::default(),
        )?;

    for entity in &result.model.entities {
        if let IfcEntity::BuildingElement {
            location_feet,
            extrusion,
            solid_shape,
            host_element_index,
            ..
        } = entity
        {
            assert!(location_feet.is_none());
            assert!(extrusion.is_none());
            assert!(solid_shape.is_none());
            assert!(host_element_index.is_none());
        }
    }
    assert_eq!(
        result.diagnostics.exported.building_elements_with_geometry,
        0
    );
    Ok(())
}

#[test]
fn tier1_diagnostics_sidecar_is_schema_shaped() -> Result<()> {
    let mut rf = open_tier1()?;
    let result = RvtDocExporter.export_with_diagnostics(&mut rf)?;
    let json = serde_json::to_value(&result.diagnostics).expect("serialize");

    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["mode"], "default");
    for key in [
        "input",
        "decoded",
        "exported",
        "skipped",
        "unsupported_features",
        "warnings",
        "confidence",
    ] {
        assert!(json.get(key).is_some(), "missing diagnostics key {key}");
    }
    assert!(json["confidence"]["level"].is_string());
    assert!(json["exported"]["building_elements"].is_number());
    assert!(json["decoded"]["production_walker_elements"].is_number());
    Ok(())
}

#[test]
fn rvt_ifc_cli_modes_and_diagnostics_on_tier1() {
    let fixture = tier1_arch();
    if !fixture.exists() {
        eprintln!("skipping CLI tier1 test: fixture missing");
        return;
    }

    let dir = temp_dir("rvt-ifc-modes");
    let ifc_path = dir.join("out.ifc");
    let diag_path = dir.join("out.diagnostics.json");

    let ok = Command::new(env!("CARGO_BIN_EXE_rvt-ifc"))
        .arg(&fixture)
        .arg("-o")
        .arg(&ifc_path)
        .arg("--mode")
        .arg("scaffold")
        .arg("--diagnostics")
        .arg(&diag_path)
        .output()
        .expect("run rvt-ifc scaffold");
    assert!(
        ok.status.success(),
        "scaffold should succeed\n{}",
        String::from_utf8_lossy(&ok.stderr)
    );
    assert!(ifc_path.exists());
    assert!(diag_path.exists());

    let json: Value =
        serde_json::from_slice(&std::fs::read(&diag_path).expect("read diag")).expect("parse");
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["confidence"]["level"], "scaffold");

    let ifc_text = std::fs::read_to_string(&ifc_path).expect("read ifc");
    assert!(!ifc_text.contains("HostObjAttr-"));

    let strict_ifc = dir.join("strict.ifc");
    let strict_diag = dir.join("strict.diagnostics.json");
    let strict = Command::new(env!("CARGO_BIN_EXE_rvt-ifc"))
        .arg(&fixture)
        .arg("-o")
        .arg(&strict_ifc)
        .arg("--mode")
        .arg("strict")
        .arg("--diagnostics")
        .arg(&strict_diag)
        .output()
        .expect("run rvt-ifc strict");
    assert!(
        !strict.status.success(),
        "strict must fail on tier1 scaffold-only fixtures"
    );
    assert!(
        !strict_ifc.exists(),
        "strict must not write IFC when validation fails"
    );
    assert!(
        strict_diag.exists(),
        "strict should still write diagnostics for triage"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn exporter_trait_default_matches_scaffold_content() -> Result<()> {
    let mut a = open_tier1()?;
    let mut b = open_tier1()?;
    let via_trait = RvtDocExporter.export(&mut a)?;
    let via_mode = RvtDocExporter.export_with_mode_and_limits(
        &mut b,
        ExportQualityMode::Scaffold,
        Default::default(),
    )?;
    assert_eq!(via_trait.entities.len(), via_mode.entities.len());
    assert!(host_obj_attr_names(&via_trait).is_empty());
    Ok(())
}

#[test]
fn load_export_diagnostics_schema_file_exists() {
    let schema = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("docs/schemas/export-diagnostics.schema.json");
    assert!(schema.exists(), "schema missing at {}", schema.display());
    let text = std::fs::read_to_string(&schema).expect("read schema");
    let json: Value = serde_json::from_str(&text).expect("schema JSON");
    assert_eq!(json["title"], "rvt IFC export diagnostics JSON");
}
