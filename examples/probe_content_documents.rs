//! RE-17 — inspect Global/ContentDocuments. Untouched stream, 30KB
//! on Einhoven 2023, 243KB on 2024. Hypothesis: may contain the
//! element-location index that ElemTable lacks.

use rvt::{RevitFile, compression};

fn hexdump(bytes: &[u8], lines: usize) {
    let total = lines.saturating_mul(16).min(bytes.len());
    for i in (0..total).step_by(16) {
        let end = (i + 16).min(total);
        let hex: String = bytes[i..end]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(" ");
        let ascii: String = bytes[i..end]
            .iter()
            .map(|&b| {
                if (0x20..0x7f).contains(&b) {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();
        println!("    {i:08x}  {hex:<48}  |{ascii}|");
    }
}

fn main() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    for file in ["Revit_IFC5_Einhoven.rvt", "2024_Core_Interior.rvt"] {
        let path = format!("{project_dir}/{file}");
        let Ok(mut rf) = RevitFile::open(&path) else {
            continue;
        };
        let Ok(raw) = rf.read_stream("Global/ContentDocuments") else {
            println!("{file}: no Global/ContentDocuments");
            continue;
        };

        // Try single-chunk inflate first, then multi-chunk.
        let (single_off, single) = compression::inflate_at_auto(&raw).unwrap_or((0, raw.clone()));
        let chunks = compression::inflate_all_chunks(&raw);
        let concat: Vec<u8> = chunks.iter().flatten().copied().collect();

        println!("\n=== {file} :: Global/ContentDocuments ===");
        println!(
            "  raw={} B, single_chunk={} B (prefix={}), all_chunks={} chunks / {} B concat",
            raw.len(),
            single.len(),
            single_off,
            chunks.len(),
            concat.len()
        );

        println!("\n  Head of concatenated buffer (first 256 B):");
        hexdump(&concat, 16);

        // Look for ASCII C++ type names, stream names, or record markers.
        let mut str_hits = Vec::new();
        let mut i = 0;
        while i + 4 <= concat.len() {
            // Scan for ASCII runs of length >= 6 (plausible identifier).
            let mut run = 0;
            while i + run < concat.len() && concat[i + run].is_ascii_graphic() {
                run += 1;
            }
            if run >= 6 {
                let s = std::str::from_utf8(&concat[i..i + run]).unwrap_or("?");
                str_hits.push((i, s.to_string()));
                i += run;
            } else {
                i += 1;
            }
        }
        println!(
            "\n  ASCII runs (>=6 chars): {} found, first 20:",
            str_hits.len()
        );
        for (off, s) in str_hits.iter().take(20) {
            println!("    @0x{off:06x}  {s:?}");
        }

        // Check if buffer could be a table: try interpreting as
        // [u32 count][count × record]
        if concat.len() >= 8 {
            let count = u32::from_le_bytes([concat[0], concat[1], concat[2], concat[3]]);
            println!(
                "\n  Leading u32 = {count} (as record count, implies {} B/record if full buffer)",
                if count > 0 {
                    (concat.len() - 4) / count as usize
                } else {
                    0
                }
            );
        }
    }
}
