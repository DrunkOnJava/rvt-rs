//! RE-09 revised — concatenate all gzip chunks of a partition stream
//! into one logical buffer, then scan for schema class-tag occurrences.
//!
//! Rationale: gzip chunk boundaries are compression-artificial, not
//! semantic. Element records aren't aligned to chunks. Scanning the
//! full concatenated buffer should reveal where tagged classes appear.

use rvt::{RevitFile, compression, formats, streams};
use std::collections::BTreeMap;

fn main() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());

    for file in ["Revit_IFC5_Einhoven.rvt", "2024_Core_Interior.rvt"] {
        let path = format!("{project_dir}/{file}");
        let Ok(mut rf) = RevitFile::open(&path) else {
            continue;
        };

        // Parse schema → tag map.
        let formats_raw = rf.read_stream(streams::FORMATS_LATEST).unwrap();
        let formats_d = compression::inflate_at(&formats_raw, 0).unwrap();
        let schema = formats::parse_schema(&formats_d).unwrap();
        let tag_to_name: BTreeMap<u16, &str> = schema
            .classes
            .iter()
            .filter_map(|c| c.tag.map(|t| (t, c.name.as_str())))
            .collect();

        // Pick all partition streams.
        let partitions: Vec<String> = rf
            .stream_names()
            .into_iter()
            .filter(|s| s.starts_with("Partitions/"))
            .collect();

        println!("\n=== {file} — {} classes tagged ===", tag_to_name.len());

        for s in &partitions {
            let raw = rf.read_stream(s).unwrap();
            let chunks = compression::inflate_all_chunks(&raw);
            let concat: Vec<u8> = chunks.into_iter().flatten().collect();
            if concat.len() < 4 {
                continue;
            }

            // Count tag hits across concat.
            let mut hits: BTreeMap<&str, usize> = BTreeMap::new();
            let mut total_hits = 0usize;
            for i in 0..concat.len().saturating_sub(1) {
                let v = u16::from_le_bytes([concat[i], concat[i + 1]]);
                if let Some(&name) = tag_to_name.get(&v) {
                    *hits.entry(name).or_insert(0) += 1;
                    total_hits += 1;
                }
            }

            // Top-10 most frequent.
            let mut top: Vec<(&str, usize)> = hits.iter().map(|(k, v)| (*k, *v)).collect();
            top.sort_by_key(|(_, v)| std::cmp::Reverse(*v));

            println!(
                "\n  {s}: {} B decomp, {} distinct tagged classes, {} total tag hits",
                concat.len(),
                hits.len(),
                total_hits
            );
            for (name, count) in top.iter().take(10) {
                println!("    {count:>6} × {name}");
            }
        }
    }
}
