//! Research probe (#218, RE-24): dump every `OST_Levels` partition record on a
//! Revit 2024 project file together with the name/elevation block the level
//! owns, so the recovery can be scored offline against Revit's own export.
//!
//! Not part of the shipped decode path.

use rvt::partition_level_records as levels;
use rvt::{RevitFile, compression};
use std::collections::BTreeSet;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).expect("usage: probe <file.rvt>");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info().map(|i| i.version).unwrap_or(0);
    let elem_records = rvt::elem_table::parse_records(&mut rf)?;
    let declared: BTreeSet<u32> = elem_records.iter().map(|r| r.id_primary).collect();

    let streams: Vec<String> = rf
        .stream_names()
        .into_iter()
        .filter(|s| s.starts_with("Partitions/"))
        .collect();
    let mut records = Vec::new();
    let mut blocks = Vec::new();
    for stream in &streams {
        let Ok(raw) = rf.read_stream(stream) else {
            continue;
        };
        let chunks = compression::inflate_all_chunks_for_stream(stream, &raw);
        let concat: Vec<u8> = chunks.into_iter().flatten().collect();
        records.extend(levels::find_level_records(stream, &concat, &declared));
        for block in levels::find_name_blocks(&concat, &declared) {
            blocks.push(serde_json::json!({
                "stream": stream,
                "element_id": block.element_id,
                "name": block.name,
                "elevation_feet": block.elevation_feet,
            }));
        }
    }
    let recovered = levels::scan_partition_levels(&mut rf, version, &declared)?;
    let out = serde_json::json!({
        "file": path,
        "revit_version": version,
        "declared_ids": declared.len(),
        "level_category_records": records.len(),
        "level_elements": records.iter().filter(|r| r.is_level_element()).count(),
        "records": records,
        "name_blocks": blocks,
        "levels": recovered,
    });
    println!("{}", serde_json::to_string(&out)?);
    Ok(())
}
