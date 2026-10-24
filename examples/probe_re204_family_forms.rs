//! RE-204, RE-205 and RE-206: what a loaded family's own document shows.
//!
//! FACT: a type whose map holds −1 values has `<Unnamed>` in Revit's export
//! only where its family document shows a visible, non-void form with no
//! material (`m_materialId` −1, `m_cutting` false, `IS_VISIBLE_PARAM` not 0);
//! a material only hidden forms have is left out; a nested family's forms
//! count, with a `Visible` bound to a family parameter evaluated on the
//! nested instance, else its type, by the parameter's caption. On Snowdon
//! Towers: Washer_Dryer_Combo (21) and TableWithUmbrella (1) have only hidden
//! unset forms, Refrig_Base_wDrawers's "Clearance" extrusion is hidden, and
//! AC_Bandstand Canopy's nested dome is hidden by "Include Dome"; RE1
//! Electrical's 284621-FirePanel has no unset form and nests nothing.
//!
//! The probe prints one JSON line per family a column, door, window or
//! product's type belongs to whose document is read: whether it shows
//! geometry with no material and the materials only its hidden forms have.
//!
//! Usage:
//!   cargo run --release --example probe_re204_family_forms -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::partition_schema_mvp::{TYPE_ID_FIELD, recover_partition_schema_mvp};
use rvt::walker::{InstanceField, WalkerLimits};
use serde_json::json;
use std::collections::BTreeSet;

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: probe_re204_family_forms MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let decoded = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
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
    let maps =
        rvt::research::native_type_materials::geometry_materials(&mut rf, &types, &materials)?;
    let families: BTreeSet<u64> = maps.values().filter_map(|m| m.family).collect();
    let forms = rvt::research::native_family_forms::family_forms(&mut rf, &families)?;
    let (mut hiding, mut no_unset) = (0usize, 0usize);
    for (family, found) in &forms {
        let hidden: Vec<&str> = found
            .hidden_only
            .iter()
            .filter_map(|m| names.get(m).map(String::as_str))
            .collect();
        hiding += usize::from(!hidden.is_empty());
        no_unset += usize::from(found.shows_unset == Some(false));
        println!(
            "{}",
            json!({"family": family, "shows_unset": found.shows_unset, "hidden_only": hidden})
        );
    }
    println!(
        "{}",
        json!({"families": families.len(), "read": forms.len(), "showing_no_unset": no_unset, "with_hidden_only": hiding})
    );
    Ok(())
}
