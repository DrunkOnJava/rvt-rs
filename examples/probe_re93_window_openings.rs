//! RE-93: a family type's length parameters sit in its value block, and a
//! window's opening is its type's Width and Height, centred on its origin
//! and standing its type's Default Sill Height above it.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 architectural sample, a type's
//! value block (its ElementId, 56 bytes of 0xff and 3 zero bytes, RE-77)
//! holds length parameters as `f64 feet · ff × 8 · i64 parameter`: a
//! BuiltInParameter's negative id (Width -1001301, Height -1001300) or the
//! declared ElementId of a family parameter, whose definition names it
//! 0x56 bytes past its id (`u32 n · UTF-16`, then a `revit.local.family:`
//! id). Revit's IFC4 export cuts each window's opening the type's Width
//! wide, centred on the window's origin (RE-87) along its X axis, and the
//! type's Height high from 3.0 ft above the origin, which is the type's
//! Default Sill Height on every type that stores one.
//!
//! The probe prints each window type read and each window's opening. Score
//! the export against Revit's with `tools/re/opening_boxes_vs_ifc.py`.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re93_window_openings -- MODEL.rvt

use rvt::RevitFile;
use rvt::partition_schema_mvp::{
    TYPE_ID_FIELD, TYPE_NAME_FIELD, recover_partition_schema_mvp, window_opening_from_fields,
};
use rvt::partition_type_parameters::type_window_openings;
use rvt::walker::{InstanceField, WalkerLimits};
use std::collections::{BTreeMap, BTreeSet};

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    let mut types: BTreeMap<u32, String> = BTreeMap::new();
    for window in &mvp.windows {
        let (mut id, mut name) = (None, None);
        for (field, value) in &window.fields {
            match (field.as_str(), value) {
                (TYPE_ID_FIELD, InstanceField::ElementId { id: type_id, .. }) => {
                    id = Some(*type_id)
                }
                (TYPE_NAME_FIELD, InstanceField::String(text)) => name = Some(text.clone()),
                _ => {}
            }
        }
        if let Some(id) = id {
            types.insert(id, name.unwrap_or_default());
        }
    }
    let ids: BTreeSet<u32> = types.keys().copied().collect();
    let openings = type_window_openings(&mut rf, version, &ids);
    println!(
        "release {version}: {} windows of {} types, {} types with Width, Height and Default Sill Height",
        mvp.windows.len(),
        types.len(),
        openings.len()
    );
    for (id, name) in &types {
        match openings.get(id) {
            Some(o) => println!(
                "  type {id:>8} {name:<28} width {:>7.4}  height {:>7.4}  default sill {:>7.4} ft",
                o.width_feet, o.height_feet, o.default_sill_feet
            ),
            None => println!("  type {id:>8} {name:<28} not read"),
        }
    }
    let mut drawn = 0;
    for window in &mvp.windows {
        let Some(o) = window_opening_from_fields(&window.fields) else {
            continue;
        };
        drawn += 1;
        println!(
            "  window {:>8}  centre ({:.4}, {:.4})  axis ({:.4}, {:.4})  base {:.4}  {:.4} x {:.4} ft",
            window.id.unwrap_or(0),
            o.centre[0],
            o.centre[1],
            o.axis[0],
            o.axis[1],
            o.base_feet,
            o.width_feet,
            o.height_feet
        );
    }
    println!("{drawn} of {} windows carry an opening", mvp.windows.len());
    Ok(())
}
