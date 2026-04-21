//! Probe the per-record payload bytes to find the id → offset binding.
//!
//! Project 2023 has 28-byte records: [4B marker][u32 id_primary]
//! [u32 id_secondary][16B payload]. Project 2024 has 40-byte records:
//! [8B marker][4B zero][u32 id_primary][12B][u32 id_secondary][8B].
//!
//! Hypothesis: the payload encodes a byte offset into Global/Latest.
//! If true, we should see:
//!   - payload u32 values that fit inside the decompressed Latest size
//!   - monotonic-ish growth as id increases
//!   - non-zero, non-trivial values

use rvt::{RevitFile, compression, elem_table, streams};

fn main() {
    let files = [
        (
            "project-2023",
            "/private/tmp/rvt-corpus-probe/magnetar/Revit/Revit_IFC5_Einhoven.rvt",
        ),
        (
            "project-2024",
            "/private/tmp/rvt-corpus-probe/magnetar/Revit/2024_Core_Interior.rvt",
        ),
    ];

    for (label, path) in files {
        let mut rf = match RevitFile::open(path) {
            Ok(rf) => rf,
            Err(e) => {
                println!("{label}: open error — {e}");
                continue;
            }
        };

        // Size of Global/Latest — payload offsets should fit inside this.
        let latest_raw = rf.read_stream(streams::GLOBAL_LATEST).unwrap();
        let latest_size = compression::inflate_at_auto(&latest_raw)
            .map(|(_, d)| d.len())
            .unwrap_or(0);

        let records = match elem_table::parse_records(&mut rf) {
            Ok(r) => r,
            Err(e) => {
                println!("{label}: records error — {e}");
                continue;
            }
        };

        println!(
            "{label}: Global/Latest decompressed = {latest_size} bytes, {} records",
            records.len()
        );

        // Dump the payload of the first 5 records + the last 3.
        println!("  first 5 record payloads:");
        for r in records.iter().take(5) {
            dump_record(r);
        }
        println!("  last 3 record payloads:");
        for r in records.iter().rev().take(3) {
            dump_record(r);
        }

        // Does any 4-byte aligned u32 inside the payload look like a
        // plausible offset into Global/Latest? Count how many records have
        // at least one such candidate, and the distribution.
        let mut hits = 0;
        let mut monotonic_u32_0 = true;
        let mut last_u32_0: u32 = 0;
        let mut latest_relative_u32s_at = vec![0usize; 10]; // count per position
        let marker_len = detect_marker_len(&records[0].raw);
        let body_len = records[0].raw.len() - marker_len;
        let max_positions = body_len / 4;
        latest_relative_u32s_at.resize(max_positions, 0);

        for r in records.iter() {
            let body = &r.raw[marker_len..];
            for (pos, chunk) in body.chunks_exact(4).enumerate() {
                let v = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                if pos < latest_relative_u32s_at.len() && v != 0 && (v as usize) < latest_size {
                    latest_relative_u32s_at[pos] += 1;
                }
            }
            // Just focus on the first u32 of the body after the ids.
            // For 28B layout: body[8..12] is the first u32 AFTER the two ids.
            // For 40B layout: body[0..4] is the leading zero/marker padding.
            let id_end_offset = if marker_len == 4 { 8 } else { 24 };
            if body.len() >= id_end_offset + 4 {
                let v = u32::from_le_bytes([
                    body[id_end_offset],
                    body[id_end_offset + 1],
                    body[id_end_offset + 2],
                    body[id_end_offset + 3],
                ]);
                if v != 0 && (v as usize) < latest_size {
                    hits += 1;
                }
                if v < last_u32_0 {
                    monotonic_u32_0 = false;
                }
                last_u32_0 = v;
            }
        }

        println!(
            "  payload u32 at each 4-byte position: how often does it fit inside Global/Latest ({latest_size} B)?"
        );
        for (pos, count) in latest_relative_u32s_at.iter().enumerate() {
            let marker_relative = marker_len + pos * 4;
            println!(
                "    body[{pos}] (record offset +{marker_relative}..+{}) — {count} / {}  ({:.1}%)",
                marker_relative + 4,
                records.len(),
                100.0 * *count as f64 / records.len() as f64
            );
        }
        println!(
            "  first post-id u32 candidate offset:     {hits} / {} records fit inside Global/Latest; monotonic={monotonic_u32_0}",
            records.len()
        );
    }
}

fn detect_marker_len(raw: &[u8]) -> usize {
    if raw.len() >= 8 && raw[..8] == [0xFF; 8] {
        8
    } else if raw.len() >= 4 && raw[..4] == [0xFF; 4] {
        4
    } else {
        0
    }
}

fn dump_record(r: &rvt::elem_table::ElemRecord) {
    print!(
        "    offset=0x{:x} id_primary={} id_secondary={} raw=[",
        r.offset, r.id_primary, r.id_secondary
    );
    for b in &r.raw {
        print!("{:02x} ", b);
    }
    println!("]");
}
