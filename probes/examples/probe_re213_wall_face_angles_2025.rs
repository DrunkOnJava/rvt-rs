//! RE-213: Revit 2025 frames a wall type's face angles with twelve zero
//! bytes after its `u32 2`, where 2024 has eight.
//!
//! FACT: on Autodesk's Snowdon Towers architectural sample saved by Revit
//! 2025, RE-86's frame (`u32 2`, eight zero bytes, `f64` 0.7, a count of 3)
//! occurs nowhere in a wall type's data. The same frame with twelve zero
//! bytes (`WALL_FACE_ANGLES_FRAME_2025`) occurs 58 times, once in each of
//! 58 elements' data, 516 to 997 bytes past the element's data header. The
//! three "Solar Wall" types and 703553 store [0, 10 degrees, 0] after it,
//! the other 54 [0, 0, 0], as on 2024.
//!
//! The probe prints, for every element data header followed by either
//! frame within the element's data, one JSON object: `element`, `frame`
//! (`2024` or `2025`), `offset` past the header and the three `angles`.
//! The last line, on stderr, counts each frame.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re213_wall_face_angles_2025 -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::partition_compound_structure as pcs;
use rvt::research::partition_names;

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let Some(header) = partition_names::element_data_header(version) else {
        eprintln!("Revit {version}: no element data header measured");
        return Ok(());
    };
    let frames: [(&str, &[u8]); 2] = [
        ("2024", &pcs::WALL_FACE_ANGLES_FRAME),
        ("2025", &pcs::WALL_FACE_ANGLES_FRAME_2025),
    ];
    let mut counts = [0usize; 2];
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let hits: Vec<usize> = memchr::memmem::find_iter(buf, &header).collect();
        for (index, &hit) in hits.iter().enumerate() {
            let id_at = hit + header.len();
            let Some(id) = buf
                .get(id_at..id_at + 8)
                .and_then(|b| b.first_chunk::<8>())
                .map(|b| u64::from_le_bytes(*b))
            else {
                continue;
            };
            let end = hits.get(index + 1).copied().unwrap_or(buf.len());
            let data = &buf[id_at + 8..end];
            for (k, (name, frame)) in frames.iter().enumerate() {
                let Some(at) = memchr::memmem::find(data, frame) else {
                    continue;
                };
                let Some(angles) = pcs::wall_type_face_angles_after(data, frame) else {
                    continue;
                };
                counts[k] += 1;
                println!(
                    "{{\"element\":{id},\"frame\":{name},\"offset\":{},\"angles\":[{},{},{}]}}",
                    at + header.len() + 8,
                    angles[0],
                    angles[1],
                    angles[2]
                );
            }
        }
    }
    eprintln!(
        "Revit {version}: 2024 frame in {} elements' data, 2025 frame in {}",
        counts[0], counts[1]
    );
    Ok(())
}
