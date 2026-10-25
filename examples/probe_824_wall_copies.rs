//! #824: the walls whose element data copies disagree on their location
//! line, each copy's line, and the copy `native_document` calls current.
//!
//! FACT under test: where a wall's copies in different partitions disagree,
//! the current partition's copy (RE-216) holds Revit's line, as it does an
//! instance's transform.
//!
//! Output, per wall with copies whose first bounded line disagrees:
//!   wall <id> current <partition|none>
//!   copy <id> <partition> <start x y z> <end x y z>
//!
//! `tools/re/wall_copies_vs_ifc.py` compares each copy with Revit's wall.
//!
//! Usage:
//!   cargo run --profile ci --example probe_824_wall_copies -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::partition_beam_axes::{BEAM_DATA_WINDOW, first_bounded_line};
use rvt::research::partition_element_records as per;
use std::collections::{BTreeMap, BTreeSet};

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let Some(layout) = rvt::research::partition_names::element_data_layout(version) else {
        return Ok(());
    };
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let walls: BTreeSet<u32> =
        per::scan_category_records_multi(&mut rf, version, &[per::OST_WALLS], &declared)?
            .iter()
            .map(|r| r.element_id)
            .collect();
    let mut copies: BTreeMap<u32, Vec<(u32, [f64; 3], [f64; 3])>> = BTreeMap::new();
    let mut holders: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Some(partition) = stream
            .strip_prefix("Partitions/")
            .and_then(|n| n.parse::<u32>().ok())
        else {
            continue;
        };
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let hits: Vec<usize> = memchr::memmem::find_iter(buf, &layout.header).collect();
        for (index, &hit) in hits.iter().enumerate() {
            let id_at = hit + layout.header.len();
            let Some(id) = layout.id_at(buf, id_at).filter(|id| walls.contains(id)) else {
                continue;
            };
            holders.entry(id).or_default().insert(partition);
            let end = hits
                .get(index + 1)
                .copied()
                .unwrap_or(buf.len())
                .min(hit.saturating_add(BEAM_DATA_WINDOW))
                .min(buf.len());
            if let Some(line) = buf.get(id_at + 8..end).and_then(first_bounded_line) {
                copies
                    .entry(id)
                    .or_default()
                    .push((partition, line.start(), line.end()));
            }
        }
    }
    let disagreeing: BTreeMap<u32, BTreeSet<u32>> = copies
        .iter()
        .filter(|(_, c)| c.iter().any(|copy| (copy.1, copy.2) != (c[0].1, c[0].2)))
        .filter_map(|(id, _)| Some((*id, holders.get(id)?.clone())))
        .collect();
    let current = rvt::research::native_document::current_partitions(&mut rf, &disagreeing);
    for id in disagreeing.keys() {
        println!(
            "wall {id} current {}",
            current.get(id).map_or("none".into(), |p| p.to_string())
        );
        for (partition, start, end) in &copies[id] {
            println!(
                "copy {id} {partition} {:.6} {:.6} {:.6} {:.6} {:.6} {:.6}",
                start[0], start[1], start[2], end[0], end[1], end[2]
            );
        }
    }
    Ok(())
}
