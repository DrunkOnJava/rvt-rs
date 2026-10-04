//! Probe: the entity counts behind Core Interior's witness changes of
//! 2026-10-04 (B60, B72, B73, B55), in Revit's own export and in rvt-rs's.
//!
//! Counts IfcColumnType, IfcDoorType (and IfcDoorStyle), IfcSlabType,
//! IfcOpeningElement and IfcRelDefinesByType in the reference export beside
//! the model (`<model>_slim.ifc`, else `<model>.ifc`, else under
//! `../IFC Exports/`) and in rvt-rs's export of the model, and how many
//! columns each export relates to a type.
//!
//! Usage: probe_witness_type_counts <2024_Core_Interior.rvt>

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const ENTITIES: [&str; 7] = [
    "IFCCOLUMNTYPE",
    "IFCDOORTYPE",
    "IFCDOORSTYLE",
    "IFCSLABTYPE",
    "IFCOPENINGELEMENT",
    "IFCRELDEFINESBYTYPE",
    "IFCCOLUMN",
];

fn counts(step: &str) -> BTreeMap<&'static str, usize> {
    let mut out: BTreeMap<&'static str, usize> = ENTITIES.iter().map(|e| (*e, 0)).collect();
    for line in step.lines() {
        let Some((_, body)) = line.split_once('=') else {
            continue;
        };
        let Some((entity, _)) = body.split_once('(') else {
            continue;
        };
        if let Some(count) = out.get_mut(entity.trim()) {
            *count += 1;
        }
    }
    out
}

fn reference(model: &Path) -> Option<String> {
    let stem = model.file_stem()?.to_string_lossy().to_string();
    let dir = model.parent()?;
    [
        dir.join(format!("{stem}_slim.ifc")),
        dir.join(format!("{stem}.ifc")),
        dir.join(format!("../IFC Exports/{stem}_slim.ifc")),
    ]
    .iter()
    .find_map(|path| std::fs::read_to_string(path).ok())
}

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("model path"));
    if !path.to_string_lossy().contains("Core_Interior") {
        println!("not Core Interior");
        return Ok(());
    }
    match reference(&path) {
        Some(step) => println!("Revit's export: {:?}", counts(&step)),
        None => println!("no reference export found"),
    }
    let mut rf = RevitFile::open(&path)?;
    let result = RvtDocExporter.export_with_diagnostics(&mut rf)?;
    println!("rvt-rs's export: {:?}", counts(&write_step(&result.model)));
    Ok(())
}
