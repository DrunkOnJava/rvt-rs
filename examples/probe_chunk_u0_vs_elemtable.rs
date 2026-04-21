//! RE-19 — test H6: 2024 partition chunks use one-chunk-per-element
//! framing with u32[0] = ElementId.
//!
//! Method:
//!   1. Parse Global/ElemTable → collect the set of declared
//!      ElementIds.
//!   2. For every Partitions/N stream, inflate_all_chunks → extract
//!      u32[0] of each chunk → accumulate into a set.
//!   3. Report: |elem_ids|, |chunk_u0s|, |intersection|, coverage%.
//!
//! Decision rules:
//!   - intersection / elem_ids > 0.5 → H6 strongly supported.
//!   - intersection / chunk_u0s close to 1.0 → every chunk's u0 IS
//!     an ElementId (false positive rate low).
//!   - Big gap → H6 refuted, chunk u0 is something else.

use rvt::{RevitFile, compression, elem_table};
use std::collections::BTreeSet;

fn main() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    for file in ["Revit_IFC5_Einhoven.rvt", "2024_Core_Interior.rvt"] {
        let path = format!("{project_dir}/{file}");
        let Ok(mut rf) = RevitFile::open(&path) else {
            continue;
        };

        // ElemTable IDs.
        let records = elem_table::parse_records(&mut rf).unwrap();
        let elem_ids: BTreeSet<u32> = records
            .iter()
            .flat_map(|r| [r.id_primary, r.id_secondary])
            .filter(|id| *id != 0 && *id != u32::MAX)
            .collect();

        // All partition-chunk u32[0] values.
        let mut chunk_u0s: BTreeSet<u32> = BTreeSet::new();
        let mut total_chunks = 0;
        let partitions: Vec<String> = rf
            .stream_names()
            .into_iter()
            .filter(|s| s.starts_with("Partitions/"))
            .collect();
        for p in &partitions {
            let raw = rf.read_stream(p).unwrap();
            for chunk in compression::inflate_all_chunks(&raw) {
                total_chunks += 1;
                if chunk.len() >= 4 {
                    let v = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    chunk_u0s.insert(v);
                }
            }
        }

        let intersection: BTreeSet<u32> = elem_ids.intersection(&chunk_u0s).copied().collect();

        println!("\n=== {file} ===");
        println!("  ElemTable distinct non-zero IDs: {}", elem_ids.len());
        println!(
            "  Total partition chunks: {total_chunks}, distinct chunk u32[0] values: {}",
            chunk_u0s.len()
        );
        println!(
            "  Intersection |chunk_u0 ∩ elem_ids|: {}",
            intersection.len()
        );
        if !elem_ids.is_empty() {
            println!(
                "  Coverage: {} / {} = {:.1}% of ElemIds appear as chunk u0",
                intersection.len(),
                elem_ids.len(),
                100.0 * intersection.len() as f64 / elem_ids.len() as f64
            );
        }
        if !chunk_u0s.is_empty() {
            println!(
                "  Precision: {} / {} = {:.1}% of chunk u0 values are ElemIds",
                intersection.len(),
                chunk_u0s.len(),
                100.0 * intersection.len() as f64 / chunk_u0s.len() as f64
            );
        }
        // Show a few sample u0 values that ARE elem_ids and a few that AREN'T.
        let in_both: Vec<u32> = intersection.iter().take(10).copied().collect();
        let u0_not_id: Vec<u32> = chunk_u0s.difference(&elem_ids).take(10).copied().collect();
        println!("  Sample matches: {:?}", in_both);
        println!("  Sample chunk u0s NOT in ElemTable: {:?}", u0_not_id);
    }
}
