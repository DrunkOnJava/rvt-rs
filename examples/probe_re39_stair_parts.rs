//! RE-39: a stair's runs, landings and stringers name their stair.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 architectural sample, the
//! reference list of every placed stair run, landing and stringer that
//! Revit's IFC4 export aggregates under an `IfcStair` names that stair's
//! ElementId, except one stringer. Railings name their stair too, but Revit
//! aggregates only 65 of the 70 that do.
//!
//! The probe prints, per part category, how many placed instances name
//! exactly one placed stair, none, or more than one. Ids come from the
//! record chain (RE-35).
//!
//! Usage:
//!   cargo run --profile ci --example probe_re39_stair_parts -- FILE.rvt

use rvt::RevitFile;
use rvt::partition_element_records::{
    BBOX_MARKER_OFFSET, CATEGORY_OFFSET, CONTAINER_NONE, CONTAINER_OFFSET, OST_STAIRS,
    OST_STAIRS_LANDINGS, OST_STAIRS_RAILING, OST_STAIRS_RUNS, OST_STAIRS_STRINGER_CARRIAGE,
    PLACEMENT_KIND_INSTANCE, PLACEMENT_KIND_OFFSET, REFERENCE_LIST_OFFSET, bbox_marker,
    decode_reference_list,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

fn u64_at(buf: &[u8], at: usize) -> Option<u64> {
    buf.get(at..at.checked_add(8)?)
        .map(|b| u64::from_le_bytes(b.try_into().expect("8 bytes")))
}

fn main() -> rvt::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("usage: FILE.rvt"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let Some(marker) = bbox_marker(version) else {
        println!("release {version}: no known bbox marker");
        return Ok(());
    };
    let declared: BTreeSet<u32> = rvt::elem_table::parse_records(&mut rf)?
        .into_iter()
        .map(|r| r.id_primary)
        .collect();
    let ids = rf.second_prologue_ids();
    // (category, ElementId, reference list) for every placed instance.
    let mut placed: Vec<(i64, u32, Vec<u64>)> = Vec::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for hit in memchr::memmem::find_iter(buf, &marker) {
            let Some(offset) = hit.checked_sub(BBOX_MARKER_OFFSET) else {
                continue;
            };
            let Some(category) = u64_at(buf, offset + CATEGORY_OFFSET).map(|v| v as i64) else {
                continue;
            };
            let placed_instance = u64_at(buf, offset + CONTAINER_OFFSET) == Some(CONTAINER_NONE)
                && u64_at(buf, offset + PLACEMENT_KIND_OFFSET)
                    .is_some_and(|v| (v & 0xffff_ffff) as u32 == PLACEMENT_KIND_INSTANCE);
            if !placed_instance {
                continue;
            }
            let own = u64_at(buf, offset)
                .and_then(|v| u32::try_from(v).ok())
                .filter(|id| declared.contains(id))
                .or_else(|| ids.get(&stream).and_then(|m| m.get(&offset)).copied());
            let Some(own) = own else {
                continue;
            };
            let references =
                decode_reference_list(buf, offset + REFERENCE_LIST_OFFSET).unwrap_or_default();
            placed.push((category, own, references));
        }
    }
    let stairs: BTreeSet<u64> = placed
        .iter()
        .filter(|(category, _, _)| *category == OST_STAIRS)
        .map(|(_, id, _)| u64::from(*id))
        .collect();
    println!("release {version}: {} placed stairs", stairs.len());
    let parts = [
        (OST_STAIRS_RUNS, "runs"),
        (OST_STAIRS_LANDINGS, "landings"),
        (OST_STAIRS_STRINGER_CARRIAGE, "stringers"),
        (OST_STAIRS_RAILING, "railings"),
    ];
    let mut counts: BTreeMap<&str, [usize; 3]> = BTreeMap::new();
    for (category, own, references) in &placed {
        let Some((_, label)) = parts.iter().find(|(c, _)| c == category) else {
            continue;
        };
        let named: BTreeSet<u64> = references
            .iter()
            .copied()
            .filter(|id| stairs.contains(id) && *id != u64::from(*own))
            .collect();
        counts.entry(label).or_default()[named.len().min(2)] += 1;
    }
    for (label, [none, one, more]) in counts {
        println!("  {label:<10} name one stair {one:>5}, none {none:>5}, more than one {more:>3}");
    }
    Ok(())
}
