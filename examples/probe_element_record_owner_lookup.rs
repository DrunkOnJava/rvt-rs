//! Research probe (#211 / #216): resolve the `+0x32` owner ids observed on
//! non-exported category records back to their own element records, by
//! scanning every `BuiltInCategory` in the published id band.
//!
//! Not part of the shipped decode path.

use rvt::partition_element_records as per;
use rvt::{RevitFile, compression};
use std::collections::{BTreeMap, BTreeSet};

type OwnerHit = (String, i64, u32, [f64; 6]);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: probe <file.rvt> <id,...>");
    let wanted: BTreeSet<u32> = std::env::args()
        .nth(2)
        .unwrap_or_default()
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    let mut rf = RevitFile::open(&path)?;

    let declared: BTreeSet<u32> = rvt::elem_table::parse_records(&mut rf)?
        .into_iter()
        .map(|r| r.id_primary)
        .collect();
    println!("wanted declared in ElemTable:");
    for id in &wanted {
        println!("  {id}: declared={}", declared.contains(id));
    }

    let streams: Vec<String> = rf
        .stream_names()
        .into_iter()
        .filter(|s| s.starts_with("Partitions/"))
        .collect();

    // Brute scan: every offset whose leading u64 is a declared id and which
    // validates as a record header.
    let mut hits: BTreeMap<u32, Vec<OwnerHit>> = BTreeMap::new();
    let mut total = 0usize;
    for stream in &streams {
        let Ok(raw) = rf.read_stream(stream) else {
            continue;
        };
        let chunks = compression::inflate_all_chunks_for_stream(stream, &raw);
        let concat: Vec<u8> = chunks.into_iter().flatten().collect();
        if concat.len() < per::RECORD_MIN_LEN {
            continue;
        }
        for offset in 0..=(concat.len() - per::RECORD_MIN_LEN) {
            if let Some(record) = per::decode_at(stream, &concat, offset, &declared) {
                total += 1;
                if wanted.contains(&record.element_id) {
                    hits.entry(record.element_id).or_default().push((
                        stream.clone(),
                        record.builtin_category,
                        record.flags,
                        record.bbox_feet,
                    ));
                }
            }
        }
    }
    println!("total decodable element records: {total}");
    for (id, rows) in &hits {
        for (stream, category, flags, bbox) in rows {
            println!("  owner {id}: {stream} category={category} flags=0x{flags:x} bbox={bbox:?}");
        }
    }
    for id in &wanted {
        if !hits.contains_key(id) {
            println!("  owner {id}: NO element record of this shape");
        }
    }
    Ok(())
}
