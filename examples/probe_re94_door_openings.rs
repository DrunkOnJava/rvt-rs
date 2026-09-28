//! RE-94: Revit cuts a door at its type's rough opening exactly where the
//! door's body is its type's Rough Height tall.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 architectural sample, a door
//! type's value block (RE-77, RE-93) holds Rough Width (-1001305) and Rough
//! Height (-1001304) as `f64 feet · ff × 8 · i64 parameter`. Revit's IFC4
//! export cuts 57 of the 126 doors at Rough Width × Rough Height, centred on
//! the door's origin (RE-87) along its X axis and from the origin up, and
//! those are exactly the doors whose record box is Rough Height tall. The
//! other doors' bodies are their frame's height, and Revit cuts them at
//! Width + 0.208 ft by Height + 0.104 ft, which no value read gives.
//!
//! The probe prints each door type read and, per door, its record box
//! height against its type's Rough Height and whether it carries the rough
//! opening. Score the export against Revit's with
//! `tools/re/opening_boxes_vs_ifc.py`.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re94_door_openings -- MODEL.rvt

use rvt::RevitFile;
use rvt::partition_schema_mvp::{
    TYPE_ID_FIELD, TYPE_NAME_FIELD, filler_opening_from_fields, recover_partition_schema_mvp,
};
use rvt::partition_type_parameters::type_door_openings;
use rvt::walker::{InstanceField, WalkerLimits};
use std::collections::{BTreeMap, BTreeSet};

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    let field = |fields: &[(String, InstanceField)], wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let type_of = |fields: &[(String, InstanceField)]| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        })
    };
    let mut names: BTreeMap<u32, String> = BTreeMap::new();
    for door in &mvp.doors {
        if let Some(id) = type_of(&door.fields) {
            let name = door.fields.iter().find_map(|(field, value)| match value {
                InstanceField::String(text) if field == TYPE_NAME_FIELD => Some(text.clone()),
                _ => None,
            });
            names.insert(id, name.unwrap_or_default());
        }
    }
    let ids: BTreeSet<u32> = names.keys().copied().collect();
    let openings = type_door_openings(&mut rf, version, &ids);
    println!(
        "release {version}: {} doors of {} types, {} types with Rough Width and Rough Height",
        mvp.doors.len(),
        names.len(),
        openings.len()
    );
    for (id, name) in &names {
        match openings.get(id) {
            Some(o) => println!(
                "  type {id:>8} {name:<32} rough {:.4} x {:.4} ft",
                o.rough_width_feet, o.rough_height_feet
            ),
            None => println!("  type {id:>8} {name:<32} not read"),
        }
    }
    let mut census: BTreeMap<&str, usize> = BTreeMap::new();
    for door in &mvp.doors {
        let rough = type_of(&door.fields).and_then(|id| openings.get(&id));
        let height = field(&door.fields, "m_bboxHeight");
        let carries = filler_opening_from_fields(&door.fields).is_some();
        let outcome = match (rough, height) {
            (None, _) | (_, None) => "type or box not read",
            (Some(o), Some(h)) if (h - o.rough_height_feet).abs() <= 1e-3 => {
                if carries {
                    "box is Rough Height tall: rough opening"
                } else {
                    "box is Rough Height tall, but no opening (opening-only type or no transform)"
                }
            }
            _ => "box is not Rough Height tall: keeps its box opening",
        };
        *census.entry(outcome).or_default() += 1;
    }
    for (outcome, count) in &census {
        println!("  {count:>4}  {outcome}");
    }
    Ok(())
}
