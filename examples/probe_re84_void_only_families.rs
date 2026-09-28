//! RE-84: doors and windows whose type draws no geometry (#309, #227).
//!
//! FACT: a door or window whose family type has a parameter value block
//! (RE-77) with no geometry-material map (RE-82) is exported by Revit as an
//! opening only: an `IfcOpeningElement` voiding its host, Tagged with the
//! door's or window's ElementId, and no `IfcDoor` or `IfcWindow`. Every door
//! and window whose type has a map is exported as the element.
//!
//! Prints each 2024/2025 door and window record with its type and how many
//! map materials that type has (`-` when the type has no value block, `?`
//! when the element has no type):
//!
//! ```text
//! cargo run --profile ci --example probe_re84_void_only_families -- model.rvt
//! ```

use rvt::partition_schema_mvp::{FAMILY_NAME_FIELD, TYPE_ID_FIELD};
use rvt::walker::InstanceField;

fn main() -> anyhow::Result<()> {
    let Some(path) = std::env::args().nth(1) else {
        anyhow::bail!("usage: probe_re84_void_only_families FILE.rvt");
    };
    let mut rf = rvt::RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let maps = rvt::partition_type_materials::type_material_names(&mut rf, version);
    let mvp = rvt::partition_schema_mvp::recover_partition_schema_mvp(
        &mut rf,
        version,
        rvt::walker::WalkerLimits::default(),
    )?;
    for element in mvp.doors.iter().chain(&mvp.windows) {
        let type_id = element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        });
        let family = element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::String(text) if name == FAMILY_NAME_FIELD => Some(text.as_str()),
            _ => None,
        });
        let materials = match type_id {
            None => "?".to_string(),
            Some(id) => maps.get(&id).map_or("-".into(), |m| m.len().to_string()),
        };
        println!(
            "{}\t{}\t{}\t{}\t{}",
            element.id.map_or(String::new(), |id| id.to_string()),
            element.class,
            type_id.map_or(String::new(), |id| id.to_string()),
            materials,
            family.unwrap_or("")
        );
    }
    Ok(())
}
