//! RE-15-06 (#86) — locate real Material / Level / Space names in partition strings.
use rvt::RevitFile;
use rvt::object_graph;
use rvt::partition_name_candidates::{NameBucket, collect_name_candidates, is_display_name};
use std::collections::BTreeMap;

const FILES: &[&str] = &["Revit_IFC5_Einhoven.rvt", "2024_Core_Interior.rvt"];
const EXPECTED_FRAGMENTS: &[&str] = &[
    "Concrete", "Level", "Floor", "Glass", "Gypsum", "Door", "Wall", "Space", "Room", "Corridor",
    "Office",
];

fn main() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    println!("RE-15-06 name probe — corpus dir: {project_dir}");
    for file in FILES {
        let path = format!("{project_dir}/{file}");
        let mut rf = match RevitFile::open(&path) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("{file}: open failed: {e}");
                continue;
            }
        };
        let records = match object_graph::string_records_from_partitions(&mut rf) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("{file}: string extract failed: {e}");
                continue;
            }
        };
        let mut buckets: BTreeMap<&str, usize> = BTreeMap::new();
        for rec in &records {
            let b = if !is_display_name(&rec.value) {
                if rec.value.starts_with("autodesk.") {
                    "forge_uri"
                } else {
                    "other"
                }
            } else {
                "display_name"
            };
            *buckets.entry(b).or_insert(0) += 1;
        }
        let values: Vec<&str> = records.iter().map(|r| r.value.as_str()).collect();
        let candidates = collect_name_candidates(values.iter().copied());
        println!(
            "\n=== {file} — partition string records: {} ===",
            records.len()
        );
        for (k, v) in &buckets {
            println!("    {k:<14} {v:>6}");
        }
        for frag in EXPECTED_FRAGMENTS {
            let n = records
                .iter()
                .filter(|r| {
                    r.value
                        .to_ascii_lowercase()
                        .contains(&frag.to_ascii_lowercase())
                })
                .count();
            println!("    fragment {frag:<12} {n}");
        }
        for bucket in [
            NameBucket::MaterialLike,
            NameBucket::LevelLike,
            NameBucket::SpaceLike,
        ] {
            let vals: Vec<_> = candidates
                .iter()
                .filter(|(b, _)| *b == bucket)
                .map(|(_, s)| s.as_str())
                .collect();
            println!("  {bucket:?}: {} unique", vals.len());
            for v in vals.iter().take(20) {
                println!("    - {v}");
            }
        }
    }
}
