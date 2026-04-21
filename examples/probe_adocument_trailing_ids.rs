//! RE-01 — hex-dump the byte regions around ADocument's pointers and
//! trailing ElementIds on real project files.
//!
//! Hypothesis driving this probe: my current `walker::scan_candidates`
//! returns only HostObjAttr because it scans `Global/Latest` blindly for
//! u16-tag windows. The real element table is likely reached by
//! following pointers out of ADocument — specifically the `m_elem_table`
//! pointer + the three trailing ElementIds the walker already extracts.
//!
//! This probe:
//!   1. Loads Einhoven 2023 + 2024_Core_Interior
//!   2. Runs the existing `read_adocument_lossy` walker to get the
//!      13-field ADocument instance
//!   3. Dumps every Pointer and ElementId field with its raw payload
//!   4. For each non-null pointer, attempts to interpret the two u32s
//!      as offsets into `Global/Latest` and hex-dumps 64 bytes at
//!      each candidate offset

use rvt::walker::InstanceField;
use rvt::{RevitFile, compression, streams, walker};

fn hexdump(bytes: &[u8], base: usize, lines: usize) {
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
        println!("    {:08x}  {:<48}  |{ascii}|", base + i, hex);
    }
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
        let adoc = match walker::read_adocument_lossy(&mut rf) {
            Ok(d) => d.value,
            Err(e) => {
                println!("{name}: read_adocument_lossy err {e}");
                continue;
            }
        };

        // Decompress Global/Latest once so we can hex-dump at arbitrary offsets.
        let raw = rf.read_stream(streams::GLOBAL_LATEST).unwrap();
        let (_, d) = compression::inflate_at_auto(&raw).unwrap();

        println!("\n=== {name} ===");
        println!(
            "  ADocument entry @ 0x{:x}, {} fields, {} B decompressed",
            adoc.entry_offset,
            adoc.fields.len(),
            d.len()
        );

        // Also print the bytes that immediately follow ADocument — the
        // trailing-ElementId region commonly points back into things
        // near the end of the record.
        let adoc_end = adoc
            .fields
            .iter()
            .rev()
            .find_map(|(_, v)| match v {
                InstanceField::Pointer { .. } => Some(8),
                InstanceField::ElementId { .. } => Some(8),
                _ => None,
            })
            .unwrap_or(0);
        let _ = adoc_end;

        for (idx, (name, value)) in adoc.fields.iter().enumerate() {
            match value {
                InstanceField::Pointer { raw } => {
                    println!(
                        "  [field {idx:2}] {name}: Pointer({:10}, {:10}) = 0x{:08x}, 0x{:08x}",
                        raw[0], raw[1], raw[0], raw[1]
                    );
                    for (label, off) in [("lo", raw[0] as usize), ("hi", raw[1] as usize)] {
                        if off != 0 && off != u32::MAX as usize && off < d.len() {
                            println!(
                                "    → {label}=0x{off:08x} (valid offset into Global/Latest):"
                            );
                            let end = (off + 64).min(d.len());
                            hexdump(&d[off..end], off, 4);
                        }
                    }
                }
                InstanceField::ElementId { tag, id } => {
                    println!(
                        "  [field {idx:2}] {name}: ElementId(tag=0x{tag:04x}, id=0x{id:08x} / {id})"
                    );
                }
                _ => {}
            }
        }
    }
}
