//! RE-01 (pivoted) + RE-02 — inspect Global/ElemTable records on real
//! project files. Each record is 28B (2023) or 40B (2024) with a
//! leading FF×4 or FF×8 marker. The parser exposes `id_primary` and
//! `id_secondary` but the remaining ~20-32 bytes are unstructured.
//! Those remaining bytes MUST contain the offset/reference that
//! points to the element's actual instance data — somewhere.
//!
//! This probe walks the first 20 records of each corpus file and
//! dumps:
//!   - The full raw record (hex + ASCII)
//!   - Re-interpretation of every 4-byte window as a u32 — so we can
//!     spot any plausible "offset into Global/Latest" or "stream index"
//!   - Context bytes in Global/Latest at each candidate-offset u32 to
//!     see if it lands on anything that looks like an element start

use rvt::{RevitFile, compression, elem_table, streams};

fn hexline(bytes: &[u8], prefix: &str) -> String {
    let hex: String = bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ");
    let ascii: String = bytes
        .iter()
        .map(|&b| {
            if (0x20..0x7f).contains(&b) {
                b as char
            } else {
                '.'
            }
        })
        .collect();
    format!("{prefix}{hex:<120}  |{ascii}|")
}

fn main() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    for name in ["Revit_IFC5_Einhoven.rvt", "2024_Core_Interior.rvt"] {
        let path = format!("{project_dir}/{name}");
        let Ok(mut rf) = RevitFile::open(&path) else {
            println!("{name}: open failed");
            continue;
        };

        // Parse ElemTable via the public RevitFile entry points.
        let header = elem_table::parse_header(&mut rf).unwrap();
        let records = elem_table::parse_records(&mut rf).unwrap();

        // Re-read once for layout detection (takes a byte slice).
        let et_raw = rf.read_stream(streams::GLOBAL_ELEM_TABLE).unwrap();
        let (_, et_d) = compression::inflate_at_auto(&et_raw).unwrap();
        let layout = elem_table::detect_layout(&et_d);
        drop(et_d);

        // Read Global/Latest for correlation probes.
        let gl_raw = rf.read_stream(streams::GLOBAL_LATEST).unwrap();
        let (_, gl) = compression::inflate_at_auto(&gl_raw).unwrap();

        println!("\n=== {name} ===");
        println!(
            "  ElemTable: elements={}, records={}, stride={}, first_rec_start=0x{:x}",
            header.element_count, header.record_count, layout.stride, layout.start
        );
        println!("  Global/Latest: {} B decompressed", gl.len());

        // Print stream inventory first so we know what else exists.
        print!("  Streams:");
        for s in rf.stream_names() {
            let sz = rf.read_stream(&s).map(|v| v.len()).unwrap_or(0);
            print!(" {s}={sz}B");
        }
        println!();

        // Sample records: 5 from the start, 5 from id~100, 5 from id~500, 5 from end.
        let total = records.len();
        let picks: Vec<usize> = {
            let mut v = vec![0usize, 1, 2, 3, 4];
            for center in [100, 500, 1000, total.saturating_sub(6)] {
                for off in 0..5 {
                    let idx = center + off;
                    if idx < total && !v.contains(&idx) {
                        v.push(idx);
                    }
                }
            }
            v
        };

        for &ri in &picks {
            let rec = &records[ri];
            println!(
                "\n  record[{ri:2}] @ 0x{:x}, id_primary=0x{:08x}/{}, id_secondary=0x{:08x}/{}",
                rec.offset, rec.id_primary, rec.id_primary, rec.id_secondary, rec.id_secondary
            );
            println!("{}", hexline(&rec.raw, "    raw: "));

            // Interpret every 4-byte-aligned window as a u32.
            let mut u32s = Vec::new();
            for off in (0..rec.raw.len().saturating_sub(3)).step_by(4) {
                let v = u32::from_le_bytes([
                    rec.raw[off],
                    rec.raw[off + 1],
                    rec.raw[off + 2],
                    rec.raw[off + 3],
                ]);
                u32s.push((off, v));
            }
            print!("    u32s: ");
            for (off, v) in &u32s {
                print!("[{off:02}]=0x{v:08x}/{v} ");
            }
            println!();

            // For each u32 that fits in Global/Latest as an offset, hex-dump 16 bytes there.
            for (off, v) in &u32s {
                let v_usize = *v as usize;
                if v_usize > 0x40 && v_usize < gl.len().saturating_sub(16) {
                    let end = (v_usize + 16).min(gl.len());
                    println!(
                        "      rec[{off:02}]=0x{v:08x} → GL[0x{v:08x}]: {}",
                        gl[v_usize..end]
                            .iter()
                            .map(|b| format!("{b:02x}"))
                            .collect::<Vec<_>>()
                            .join(" ")
                    );
                }
            }
        }
    }
}
