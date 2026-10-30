//! RE-203: a family type's materials are its `m_geomTag2MaterialId` read by
//! name; a material's name is its `Material.m_name`.
//!
//! FACT: RE-82's value-block scan also takes the materials of a component
//! the type nests (Snowdon Towers' 21 "Counter Top w Sink Hole" take their
//! sink's three appliance materials). The type's own map, read from its
//! native record, names Revit's set; geometry it maps to −1 is exported as
//! the instance's material parameters' materials or `<Unnamed>`. The
//! material-name scan (RE-58) reads one of Snowdon Towers' 220 materials
//! wrong ('1/4" Head' for "Bronze, Architectural"), where the material's own
//! record holds the right name.
//!
//! The probe prints, for every family type a column, door, window or product
//! of the partition decode names, the scanned set and the native map's
//! named set and unset flag where they differ or the map holds −1 values.
//! A last line counts them. The material names are `scan_material_names`',
//! which now prefer `Material.m_name`; RE-203's report lists the one the
//! scan read differently.
//!
//! Usage:
//!   cargo run --release -p rvt-probes --example probe_re203_type_maps -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::partition_schema_mvp::{TYPE_ID_FIELD, recover_partition_schema_mvp};
use rvt::walker::{InstanceField, WalkerLimits};
use serde_json::json;
use std::collections::BTreeSet;

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: probe_re203_type_maps MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let decoded = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    let scanned = rvt::research::partition_type_materials::type_material_names(&mut rf, version);
    let records = rvt::elem_table::parse_records(&mut rf)?;
    let declared = rvt::elem_table::declared_ids(&records);
    let names =
        rvt::research::partition_materials::scan_material_names(&mut rf, version, &declared)?;
    let materials: BTreeSet<u32> = names.keys().copied().collect();
    let types: BTreeSet<u64> = decoded
        .columns
        .iter()
        .chain(&decoded.doors)
        .chain(&decoded.windows)
        .chain(&decoded.products)
        .filter_map(|e| {
            e.fields.iter().find_map(|(name, value)| match value {
                InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => {
                    Some(u64::from(*id))
                }
                _ => None,
            })
        })
        .collect();
    let native =
        rvt::research::native_type_materials::geometry_materials(&mut rf, &types, &materials)?;
    let mut differing = 0usize;
    for (type_id, found) in &native {
        let named: BTreeSet<&str> = found
            .materials
            .iter()
            .filter_map(|m| names.get(m).map(String::as_str))
            .collect();
        let scan: BTreeSet<&str> = u32::try_from(*type_id)
            .ok()
            .and_then(|t| scanned.get(&t))
            .into_iter()
            .flatten()
            .map(String::as_str)
            .collect();
        if named != scan || found.unset {
            differing += usize::from(named != scan);
            println!(
                "{}",
                json!({"type": type_id, "family": found.family, "scanned": scan, "native": named, "unset": found.unset})
            );
        }
    }
    println!(
        "{}",
        json!({"types": native.len(), "differing_sets": differing})
    );
    Ok(())
}
