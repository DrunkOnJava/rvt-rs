//! RE-01/03 pivot — inspect Global/PartitionTable + head of each
//! Partitions/N stream. Hypothesis: real element instance data
//! lives in the Partitions/* streams, not Global/Latest. ElemTable
//! records carry only ElementId pairs (no offsets). The mapping
//! from ElementId → partition+offset must be elsewhere — either in
//! Global/PartitionTable (130B, probably class→partition), or
//! embedded in each partition's own header.

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
    for name in ["Revit_IFC5_Einhoven.rvt", "2024_Core_Interior.rvt"] {
        let path = format!("{project_dir}/{name}");
        let Ok(mut rf) = RevitFile::open(&path) else {
            continue;
        };
        println!("\n=== {name} ===");
        let streams = rf.stream_names();

        // Dump Global/PartitionTable first — it's the suspect for the
        // class→partition map.
        if streams.iter().any(|s| s == "Global/PartitionTable") {
            let raw = rf.read_stream("Global/PartitionTable").unwrap();
            let decomp = compression::inflate_at_auto(&raw)
                .map(|(_, d)| d)
                .unwrap_or_else(|_| raw.clone());
            println!(
                "\n--- Global/PartitionTable ({} B raw, {} B decomp) ---",
                raw.len(),
                decomp.len()
            );
            hexdump(&decomp, 16);
        }

        // For each Partitions/N, decompress and dump the head.
        let mut part_streams: Vec<String> = streams
            .iter()
            .filter(|s| s.starts_with("Partitions/"))
            .cloned()
            .collect();
        part_streams.sort();

        for s in &part_streams {
            let raw = rf.read_stream(s).unwrap();
            let (prefix, decomp) = match compression::inflate_at_auto(&raw) {
                Ok((p, d)) => (Some(p), d),
                Err(_) => (None, raw.clone()),
            };
            println!(
                "\n--- {s} ({} B raw, {} B decomp, inflate_prefix={:?}) ---",
                raw.len(),
                decomp.len(),
                prefix
            );
            hexdump(&decomp, 6);
        }
    }
}
