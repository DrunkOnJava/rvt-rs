//! RE-31 addendum probe (#152): which element a `Global/ElemTable` owner
//! field belongs to.
//!
//! FACT: the table's records start at `0x06` and each ends with its owner
//! field, so the field that opens the frame rvt-rs parses (record `+0` on
//! the 28-byte layout, `+4` on the 40-byte one) closes the record before
//! it. `ElemRecord::owner_id` now reports it for that record. Read that
//! way no record owns itself and no owner chain loops; read as the frame's
//! own field, some do.
//!
//! The probe checks both pairings against an independent oracle: the
//! element's own partition element record (RE-21), whose counted reference
//! list at `+0x88` names the elements it refers to. For each record whose
//! owner is set, it counts how often the owner appears in the element's
//! own list, under the shipped pairing (`owner_id`) and under the frame's
//! own field (the next record's `owner_id`, read back one).
//!
//! ```bash
//! cargo run --profile ci --example probe_re31_owner_frame -- 2024_Core_Interior.rvt
//! ```

use rvt::partition_element_records as per;
use rvt::{RevitFile, compression};
use std::collections::{BTreeMap, BTreeSet};

/// A category and whether the record is placed.
type Kind = (i64, bool);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: probe_re31_owner_frame <file.rvt>")?;
    let mut rf = RevitFile::open(&path)?;
    let records = rvt::elem_table::parse_records(&mut rf)?;
    let declared: BTreeSet<u32> = records.iter().map(|r| r.id_primary).collect();

    // Every element record in the partitions, by ElementId: its references.
    let mut references: BTreeMap<u32, BTreeSet<u64>> = BTreeMap::new();
    let mut category: BTreeMap<u32, Kind> = BTreeMap::new();
    for stream in rf
        .stream_names()
        .into_iter()
        .filter(|s| s.starts_with("Partitions/"))
    {
        let Ok(raw) = rf.read_stream(&stream) else {
            continue;
        };
        let concat: Vec<u8> = compression::inflate_all_chunks_for_stream(&stream, &raw)
            .into_iter()
            .flatten()
            .collect();
        if concat.len() < per::RECORD_MIN_LEN {
            continue;
        }
        for offset in 0..=(concat.len() - per::RECORD_MIN_LEN) {
            if let Some(record) = per::decode_at(&stream, &concat, offset, &declared) {
                category
                    .entry(record.element_id)
                    .or_insert((record.builtin_category, record.placement_kind != 0));
                references
                    .entry(record.element_id)
                    .or_default()
                    .extend(record.references);
            }
        }
    }

    let shipped: Vec<(u32, Option<u32>)> =
        records.iter().map(|r| (r.id_primary, r.owner_id)).collect();
    // The frame's own field is the previous record's owner as shipped.
    let own_field: Vec<(u32, Option<u32>)> = records
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let owner = i.checked_sub(1).and_then(|p| records[p].owner_id);
            (r.id_primary, owner)
        })
        .collect();
    for (label, pairs) in [
        ("owner_id (shipped)", &shipped),
        ("frame's own field", &own_field),
    ] {
        let owners: BTreeMap<u32, u32> = pairs
            .iter()
            .filter_map(|(id, owner)| owner.map(|o| (*id, o)))
            .collect();
        let own = owners.iter().filter(|(id, o)| id == o).count();
        let with_record: Vec<(&u32, &u32)> = owners
            .iter()
            .filter(|(id, o)| id != o && references.contains_key(id))
            .collect();
        let hits = with_record
            .iter()
            .filter(|(id, o)| references[id].contains(&u64::from(**o)))
            .count();
        println!(
            "{label}: {} owners set, {own} name their own record; of {} that name another element and have an element record, {hits} ({:.1} %) are in its reference list",
            owners.len(),
            with_record.len(),
            100.0 * hits as f64 / with_record.len().max(1) as f64
        );
    }
    // What the shipped owners point at: holder and named element by
    // category and placement (counts of at least 50).
    let mut pairs_by_category: BTreeMap<(Kind, Kind), usize> = BTreeMap::new();
    for (id, owner) in shipped.iter().filter_map(|(id, o)| o.map(|o| (*id, o))) {
        if let (Some(holder), Some(named)) = (category.get(&id), category.get(&owner)) {
            *pairs_by_category.entry((*holder, *named)).or_default() += 1;
        }
    }
    let mut sorted: Vec<_> = pairs_by_category
        .into_iter()
        .filter(|(_, n)| *n >= 50)
        .collect();
    sorted.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    println!("shipped owners by category (holder -> named, placed):");
    for (((hc, hp), (nc, np)), n) in sorted {
        println!("  {hc} {hp} -> {nc} {np}: {n}");
    }
    Ok(())
}
