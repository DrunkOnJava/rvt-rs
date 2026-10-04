//! B47 (probe): what the low ElementIds that hold "Generic" are (RE-164).
//!
//! RE-164 found the text `Generic`, Revit's `ConnectionType` on RE1's pipes,
//! in thousands of objects, and ElementIds 23, 24, 25 and 27 named by most of
//! them. This prints, for every ElementId from 1 to 60, the class of each of
//! its data objects (RE-153), its size and its printable UTF-16 strings, on
//! RE1 Plumbing and the MEP samples.
//!
//! Usage: probe_b47_low_ids <model.rvt>

use rvt::RevitFile;
use rvt::partition_room_parameters::data_objects;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn strings(bytes: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut q = 0;
    while q + 4 <= bytes.len() {
        let n = u32::from_le_bytes(bytes[q..q + 4].try_into().expect("4 bytes")) as usize;
        if (1..=60).contains(&n) && q + 4 + 2 * n <= bytes.len() {
            let units: Vec<u16> = bytes[q + 4..q + 4 + 2 * n]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            if units.iter().all(|u| (0x20..0x7f).contains(u)) {
                out.push(String::from_utf16_lossy(&units));
                q += 4 + 2 * n;
                continue;
            }
        }
        q += 1;
    }
    out
}

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("model path"));
    let name = path.to_string_lossy().to_string();
    if !(name.contains("Plumbing") || name.contains("rme")) {
        println!("not an MEP model");
        return Ok(());
    }
    let mut rf = RevitFile::open(&path)?;
    let classes = rf.schema_classes()?;
    let mut seen: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for (p, object) in data_objects(buf) {
            if !(1..=60).contains(&object.element_id) {
                continue;
            }
            let class = classes
                .by_tag((object.class & 0xffff) as u16)
                .map_or_else(|| format!("{:#x}", object.class), |c| c.name.clone());
            let line = format!(
                "{class}, {} bytes, strings {:?}",
                object.end - p,
                strings(&buf[p..object.end])
                    .into_iter()
                    .take(8)
                    .collect::<Vec<_>>()
            );
            let lines = seen.entry(object.element_id).or_default();
            if !lines.contains(&line) {
                lines.push(line);
            }
        }
    }
    for (id, lines) in seen {
        for line in lines {
            println!("{id}: {line}");
        }
    }
    Ok(())
}
