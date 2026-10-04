//! B56 (probe, #228): what internal element ElementId 3 is. RE-161 found it
//! declared by every file's ElemTable with no name entry, and the first slot
//! of most element records' reference lists. Prints each data object (RE-153)
//! of ElementId 3 with its schema class, size and stream, and the printable
//! UTF-16 strings in it.
//!
//! Usage: probe_b56_element_3 <model.rvt>

use rvt::RevitFile;
use rvt::partition_room_parameters::data_objects;
use std::path::PathBuf;

fn strings(bytes: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut q = 0;
    while q + 4 <= bytes.len() {
        let n = u32::from_le_bytes(bytes[q..q + 4].try_into().expect("4 bytes")) as usize;
        if (2..=64).contains(&n) && q + 4 + 2 * n <= bytes.len() {
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
    let mut rf = RevitFile::open(&path)?;
    let classes = rf.schema_classes()?;
    let class_name = |class: u32| {
        classes
            .classes
            .iter()
            .find(|c| u32::from(c.tag) == class & 0xffff)
            .map_or_else(|| format!("{class:#x}"), |c| c.name.clone())
    };
    let declared = rvt::elem_table::declared_element_ids(&mut rf)?;
    println!("ElementId 3 declared: {}", declared.contains(&3));
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for (p, object) in data_objects(buf) {
            if object.element_id != 3 {
                continue;
            }
            let bytes = &buf[p..object.end];
            println!(
                "{stream} at {p}: class {} ({:#x}), {} bytes, strings {:?}",
                class_name(object.class),
                object.class,
                bytes.len(),
                strings(bytes).into_iter().take(12).collect::<Vec<_>>()
            );
        }
    }
    Ok(())
}
