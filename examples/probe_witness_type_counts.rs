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

/// How many elements of each entity an `IfcRelDefinesByType` relates, by
/// the type entity they are related to.
fn typed(step: &str) -> BTreeMap<(String, String), usize> {
    let mut entities: BTreeMap<u64, (String, String)> = BTreeMap::new();
    for line in step.lines() {
        let Some(rest) = line.strip_prefix('#') else {
            continue;
        };
        let Some((id, body)) = rest.split_once('=') else {
            continue;
        };
        let Some((entity, args)) = body.split_once('(') else {
            continue;
        };
        if let Ok(id) = id.trim().parse() {
            entities.insert(id, (entity.trim().to_string(), args.to_string()));
        }
    }
    let refs = |args: &str| -> Vec<u64> {
        args.split('#')
            .skip(1)
            .filter_map(|s| {
                s.chars()
                    .take_while(char::is_ascii_digit)
                    .collect::<String>()
                    .parse()
                    .ok()
            })
            .collect()
    };
    let mut out = BTreeMap::new();
    for (entity, args) in entities.values() {
        if entity != "IFCRELDEFINESBYTYPE" {
            continue;
        }
        let all = refs(args);
        let Some((&ty, related)) = all.split_last() else {
            continue;
        };
        let ty_entity = entities
            .get(&ty)
            .map(|(e, _)| e.clone())
            .unwrap_or_default();
        for element in related.iter().skip(1) {
            let element_entity = entities
                .get(element)
                .map(|(e, _)| e.clone())
                .unwrap_or_default();
            *out.entry((element_entity, ty_entity.clone())).or_default() += 1;
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
        Some(step) => {
            println!("Revit's export: {:?}", counts(&step));
            println!("Revit's typed elements (element, type): {:?}", typed(&step));
            let space_types: Vec<&str> = step
                .lines()
                .filter(|line| line.contains("=IFCSPACETYPE("))
                .collect();
            println!("Revit's IfcSpaceType: {}", space_types.len());
            for line in space_types.iter().take(6) {
                println!("  {}", line.chars().take(240).collect::<String>());
            }
            for line in step.lines().filter(|l| l.contains("=IFCSPACE(")).take(3) {
                println!("  space: {}", line.chars().take(240).collect::<String>());
            }
        }
        None => println!("no reference export found"),
    }
    let mut rf = RevitFile::open(&path)?;
    let result = RvtDocExporter.export_with_diagnostics(&mut rf)?;
    let ours = write_step(&result.model);
    println!("rvt-rs's export: {:?}", counts(&ours));
    println!(
        "rvt-rs's typed elements (element, type): {:?}",
        typed(&ours)
    );
    Ok(())
}
