//! RE-200: a family instance's material parameters, where its type's
//! geometry-material map holds no material.
//!
//! FACT: a family type whose map (RE-82, `FamilySymbol.m_geomTag2MaterialId`)
//! holds only unset values (RE-149) draws geometry bound to material
//! parameters, or to none. Where the instance's own parameters
//! (`FamilyInstance.m_pInstParams`, a `FamilyParams` whose `m_params` entries
//! hold an ElementId value in `m_elemId`) name materials, Revit's export
//! gives the instance exactly those materials; where they name none, it gives
//! `<Unnamed>`. On `12_Rooms_Door_Windows.rvt` of the R2025 models the three
//! window trims and three muntin patterns are the first kind (9 elements,
//! "Wood - Stained", "Clad - White" or both, all Revit's); on
//! `2024_Core_Interior.rvt` the 256 columns and on the RE1 MEP models the
//! fittings, terminals and fixtures are the second.
//!
//! The probe prints one JSON line per column or product of the partition
//! decode whose type is one of those types: its ElementId, class, type and
//! the names of the materials its instance parameters hold, in their order.
//! A last line counts them. Verify against Revit's export with
//! `tools/re/element_materials_vs_ifc.py` on an `rvt-ifc` export, which
//! scores the same elements' material sets.
//!
//! Usage:
//!   cargo run --release --example probe_re200_instance_material_params -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::native_instance_materials::instance_parameter_materials;
use rvt::research::partition_schema_mvp::{TYPE_ID_FIELD, recover_partition_schema_mvp};
use rvt::research::partition_type_materials::unset_material_types;
use rvt::walker::{InstanceField, WalkerLimits};
use serde_json::json;
use std::collections::BTreeSet;

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: probe_re200_instance_material_params MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let decoded = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    let unset = unset_material_types(&mut rf, version);
    let records = rvt::elem_table::parse_records(&mut rf)?;
    let declared = rvt::elem_table::declared_ids(&records);
    let names =
        rvt::research::partition_materials::scan_material_names(&mut rf, version, &declared)?;
    let materials: BTreeSet<u32> = names.keys().copied().collect();

    let mut rows = Vec::new();
    for element in decoded.columns.iter().chain(&decoded.products) {
        let type_id = element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        });
        let (Some(id), Some(type_id)) = (element.id, type_id) else {
            continue;
        };
        if unset.contains(&type_id) {
            rows.push((id, element.class.clone(), type_id));
        }
    }
    let ids: BTreeSet<u64> = rows.iter().map(|row| u64::from(row.0)).collect();
    let found = instance_parameter_materials(&mut rf, &ids, &materials)?;
    let mut with_parameters = 0;
    for (id, class, type_id) in &rows {
        let materials: Vec<&str> = found
            .get(&u64::from(*id))
            .into_iter()
            .flatten()
            .map(|m| names.get(m).map_or("?", String::as_str))
            .collect();
        if !materials.is_empty() {
            with_parameters += 1;
        }
        println!(
            "{}",
            json!({"id": id, "class": class, "type": type_id, "instance_materials": materials})
        );
    }
    println!(
        "{}",
        json!({
            "unset_map_types": unset.len(),
            "instances": rows.len(),
            "with_instance_materials": with_parameters,
        })
    );
    Ok(())
}
