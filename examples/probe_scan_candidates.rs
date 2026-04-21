//! L5B-11.3 smoke — run `scan_candidates` against real Global/Latest.
//!
//! Reports counts by score tier so we can see the signal/noise
//! ratio before building build_handle_index on top.
//!
//! Path resolves via `RVT_PROJECT_CORPUS_DIR`.

use rvt::{RevitFile, compression, formats, streams, walker};

fn main() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    let files = [
        format!("{project_dir}/Revit_IFC5_Einhoven.rvt"),
        format!("{project_dir}/2024_Core_Interior.rvt"),
    ];
    for path in files {
        let Ok(mut rf) = RevitFile::open(&path) else {
            println!("{path}: open failed");
            continue;
        };
        let Ok(raw) = rf.read_stream(streams::FORMATS_LATEST) else {
            continue;
        };
        let Ok((_, decomp)) = compression::inflate_at_auto(&raw) else {
            continue;
        };
        let Ok(schema) = formats::parse_schema(&decomp) else {
            continue;
        };

        let Ok(latest_raw) = rf.read_stream(streams::GLOBAL_LATEST) else {
            continue;
        };
        let Ok((_, d)) = compression::inflate_at_auto(&latest_raw) else {
            continue;
        };

        let name = path.rsplit('/').next().unwrap();
        println!(
            "\n=== {name}: {} classes, {} B Global/Latest ===",
            schema.classes.len(),
            d.len()
        );

        for threshold in [i64::MIN + 1, -100, 0, 20, 50, 80] {
            let candidates = walker::scan_candidates(&schema, &d, threshold);
            let mut by_class: std::collections::BTreeMap<String, usize> =
                std::collections::BTreeMap::new();
            for c in &candidates {
                *by_class.entry(c.class_name.clone()).or_insert(0) += 1;
            }
            let top3: Vec<(String, usize)> = {
                let mut v: Vec<_> = by_class.iter().map(|(k, v)| (k.clone(), *v)).collect();
                v.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
                v.into_iter().take(3).collect()
            };
            println!(
                "  threshold ≥ {threshold:3}: {} candidates · {} distinct classes · top: {:?}",
                candidates.len(),
                by_class.len(),
                top3
            );
        }
    }
}
