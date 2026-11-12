//! RE-444 (probe): what each frame of a wall framed in several partitions
//! names as its type.
//!
//! FACT under test: the newest frame of a wall names its type the way the older
//! frames do, with exactly one type-definition record of the Walls category in
//! its `+0x88` reference list (`partition_type_records::unique_type_reference`,
//! RE-28). On the BIM4LCA timber office at building permit the walls 1436343 and
//! 2114950 name no type with their newest frame (RE-444, section 6), and the
//! type is a read of that list, so this prints the list.
//!
//! Output:
//!
//!   file <path> release <release>
//!   walltypes definitions <n> records <n> multi_framed_walls <n>
//!   wall <id> frames <n> newest <p>/<offset> <frame> ...
//!   named <id> <stream>/<offset>/<definition|record> ...
//!   summary newest_one <n> newest_zero <n> newest_many <n> newest_no_list <n> pick_differs <n> lost_with_newest <n> gained_with_newest <n>
//!
//! A `wall` line is printed for each multi-framed wall whose frames do not all
//! read the same: `<frame>` is `<p>/<offset>:<kind>:<slots>`, with `<kind>` one of
//! `no_list` (the list does not decode), `zero` (no slot is a type definition),
//! `one=<id>` and `many=<id>,<id>`; `<slots>` are the list's slots in order,
//! each as the id (`T` after it for a type-definition record of the category,
//! `t` for any other type record of the category). A `named` line follows for
//! each type record those lines name, with the frames of that record.
//!
//! `lost_with_newest` counts the walls whose frame the old rule kept names one
//! type and whose newest frame names none or several, and `gained_with_newest`
//! the reverse.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re444_frame_type_slots -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::partition_element_records as per;
use rvt::research::partition_type_records as ptr;
use std::collections::{BTreeMap, BTreeSet};

/// The vertical extent of a frame as the old selector compared it: quantised to 1e-4 ft.
fn z_key(record: &per::PartitionElementRecord) -> i64 {
    let (_, _, dz) = record.extents_feet();
    if dz.is_finite() {
        (dz * 10_000.0).round() as i64
    } else {
        i64::MIN
    }
}

/// How a frame's reference list reads: the kind and the slots as printed.
struct Reading {
    kind: String,
    slots: String,
    named: BTreeSet<u32>,
    one: Option<u32>,
}

fn read_frame(
    frame: &per::PartitionElementRecord,
    definitions: &BTreeSet<u32>,
    records: &BTreeSet<u32>,
) -> Reading {
    let mut named = BTreeSet::new();
    let slots: Vec<String> = frame
        .references
        .iter()
        .map(|slot| match u32::try_from(*slot) {
            Ok(id) if definitions.contains(&id) => {
                named.insert(id);
                format!("{id}T")
            }
            Ok(id) if records.contains(&id) => {
                named.insert(id);
                format!("{id}t")
            }
            Ok(id) => id.to_string(),
            Err(_) => format!("{slot:#x}"),
        })
        .collect();
    let one = ptr::unique_type_reference(&frame.references, definitions);
    let defined: BTreeSet<u32> = frame
        .references
        .iter()
        .filter_map(|slot| u32::try_from(*slot).ok())
        .filter(|id| definitions.contains(id))
        .collect();
    let kind = if frame.references.is_empty() {
        "no_list".to_string()
    } else if defined.is_empty() {
        "zero".to_string()
    } else if let Some(id) = one {
        format!("one={id}")
    } else {
        let ids: Vec<String> = defined.iter().map(u32::to_string).collect();
        format!("many={}", ids.join(","))
    };
    Reading {
        kind,
        slots: slots.join(","),
        named,
        one,
    }
}

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    println!("file {path} release {version}");
    if !per::supports_revit_version(version) || !ptr::supports_revit_version(version) {
        println!("not read: release {version} is outside the element records' range");
        return Ok(());
    }
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let type_records = ptr::scan_type_records(&mut rf, version, per::OST_WALLS, &declared)?;
    let definitions = ptr::type_definition_ids(&type_records);
    let records: BTreeSet<u32> = type_records
        .iter()
        .map(|record| record.element_id)
        .collect();
    let walls = per::scan_category_records_multi(&mut rf, version, &[per::OST_WALLS], &declared)?;
    let mut by_id: BTreeMap<u32, Vec<per::PartitionElementRecord>> = BTreeMap::new();
    for record in walls {
        if record.is_exported_instance() && record.has_volume() {
            by_id.entry(record.element_id).or_default().push(record);
        }
    }
    let multi = by_id.values().filter(|frames| frames.len() > 1).count();
    println!(
        "walltypes definitions {} records {} multi_framed_walls {multi}",
        definitions.len(),
        records.len()
    );
    let (mut newest_one, mut newest_zero, mut newest_many, mut newest_no_list) = (0, 0, 0, 0);
    let (mut pick_differs, mut lost, mut gained) = (0, 0, 0);
    let mut named: BTreeSet<u32> = BTreeSet::new();
    for (id, frames) in by_id.iter().filter(|(_, frames)| frames.len() > 1) {
        let mut ordered: Vec<&per::PartitionElementRecord> = frames.iter().collect();
        ordered.sort_by(|a, b| a.frame_order().cmp(&b.frame_order()));
        let readings: Vec<Reading> = ordered
            .iter()
            .map(|frame| read_frame(frame, &definitions, &records))
            .collect();
        let newest = ordered.len() - 1;
        match readings[newest].kind.as_str() {
            "no_list" => newest_no_list += 1,
            "zero" => newest_zero += 1,
            kind if kind.starts_with("one=") => newest_one += 1,
            _ => newest_many += 1,
        }
        // The frame the old rule kept: the greatest vertical extent, then the newest.
        let kept = (0..ordered.len())
            .max_by(|a, b| {
                z_key(ordered[*a])
                    .cmp(&z_key(ordered[*b]))
                    .then_with(|| ordered[*a].frame_order().cmp(&ordered[*b].frame_order()))
            })
            .expect("a multi-framed wall has frames");
        let same = readings
            .iter()
            .all(|reading| reading.kind == readings[0].kind);
        if !same {
            pick_differs += 1;
        }
        lost += usize::from(readings[kept].one.is_some() && readings[newest].one.is_none());
        gained += usize::from(readings[kept].one.is_none() && readings[newest].one.is_some());
        if same {
            continue;
        }
        let printed: Vec<String> = ordered
            .iter()
            .zip(&readings)
            .map(|(frame, reading)| {
                format!(
                    "{}/{}:{}:{}",
                    frame.frame_order().0,
                    frame.offset,
                    reading.kind,
                    reading.slots
                )
            })
            .collect();
        for reading in &readings {
            named.extend(reading.named.iter().copied());
        }
        println!(
            "wall {id} frames {} newest {}/{} {}",
            ordered.len(),
            ordered[newest].frame_order().0,
            ordered[newest].offset,
            printed.join(" ")
        );
    }
    for id in named {
        let frames: Vec<String> = type_records
            .iter()
            .filter(|record| record.element_id == id)
            .map(|record| {
                format!(
                    "{}/{}/{}",
                    record.stream,
                    record.offset,
                    if record.is_type_definition() {
                        "definition"
                    } else {
                        "record"
                    }
                )
            })
            .collect();
        println!("named {id} {}", frames.join(" "));
    }
    println!(
        "summary newest_one {newest_one} newest_zero {newest_zero} newest_many {newest_many} newest_no_list {newest_no_list} pick_differs {pick_differs} lost_with_newest {lost} gained_with_newest {gained}"
    );
    Ok(())
}
