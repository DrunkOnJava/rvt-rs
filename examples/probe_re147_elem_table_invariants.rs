//! RE-147 (probe): `Global/ElemTable`'s own invariants on any file, read
//! from `0x06` by `elem_table::parse_records` (#152).
//!
//! For each file: the release, the stated record count (`u32` at `0x02`),
//! the records read, the record size and the bytes after the last record,
//! whether the ids rise strictly, the first record's id and owner, how many
//! records name an owner, how many of those owners are ids of the table,
//! self-owned records and owner chains that loop. #152's checklist asks for
//! these on a 2014 family and a 2026 project with set owners.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re147_elem_table_invariants -- FILE.rvt ...

use rvt::{RevitFile, compression, elem_table, streams::GLOBAL_ELEM_TABLE};
use std::collections::{BTreeMap, BTreeSet};

fn num(value: Option<impl ToString>) -> String {
    value.map_or_else(|| "null".to_string(), |v| v.to_string())
}

fn probe(path: &str) -> rvt::Result<String> {
    let file = std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut rf = RevitFile::open(path)?;
    let revit = rf.basic_file_info().ok().map(|info| info.version);
    let raw = rf.read_stream(GLOBAL_ELEM_TABLE)?;
    let d = compression::inflate_stream_at(GLOBAL_ELEM_TABLE, &raw, 8)?;
    let stated = d
        .get(2..6)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    let layout = elem_table::read_layout(&mut rf)?;
    let records = elem_table::parse_records(&mut rf)?;
    let tail = records
        .last()
        .map(|r| d.len().saturating_sub(r.offset + r.raw.len()));
    let rising = records
        .windows(2)
        .all(|w| w[1].id_primary > w[0].id_primary);
    let ids: BTreeSet<u32> = records.iter().map(|r| r.id_primary).collect();
    let owners: BTreeMap<u32, u32> = records
        .iter()
        .filter_map(|r| r.owner_id.map(|owner| (r.id_primary, owner)))
        .collect();
    let declared = owners.values().filter(|o| ids.contains(o)).count();
    let self_owned = owners.iter().filter(|(id, owner)| id == owner).count();
    let looping = owners
        .keys()
        .filter(|&&start| {
            let mut at = start;
            for _ in 0..=owners.len() {
                match owners.get(&at) {
                    Some(&next) if next == start => return true,
                    Some(&next) => at = next,
                    None => return false,
                }
            }
            true
        })
        .count();
    let first = records.first();
    Ok(format!(
        "{{\"file\":{file:?},\"revit\":{},\"stated\":{},\"records\":{},\"stride\":{},\"start\":{},\
         \"tail\":{},\"rising\":{rising},\"first_id\":{},\"first_owner\":{},\
         \"owners\":{},\"owners_declared\":{declared},\"self_owned\":{self_owned},\"in_loops\":{looping}}}",
        num(revit),
        num(stated),
        records.len(),
        layout.stride,
        layout.start,
        num(tail),
        num(first.map(|r| r.id_primary)),
        num(first.and_then(|r| r.owner_id)),
        owners.len(),
    ))
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: probe_re147_elem_table_invariants FILE.rvt ...");
        std::process::exit(2);
    }
    for path in &paths {
        match probe(path) {
            Ok(line) => println!("{line}"),
            Err(error) => println!("{{\"file\":{path:?},\"error\":{:?}}}", error.to_string()),
        }
    }
}
