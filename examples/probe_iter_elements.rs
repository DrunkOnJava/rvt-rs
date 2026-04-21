//! L5B-11.6 smoke — run `iter_elements` against real Global/Latest
//! buffers and report how many elements the walker recovered,
//! grouped by class name.
//!
//! Path resolves via `RVT_PROJECT_CORPUS_DIR` (default
//! `/private/tmp/rvt-corpus-probe/magnetar/Revit`).
//!
//! This is a coverage probe, not a correctness check — the reported
//! "elements" include every scan_candidates hit that decoded
//! successfully, which over-counts against `ElemTable`'s declared
//! set until `build_handle_index` + self-id extraction is cross-
//! checked (L5B-11.7 wires that into the IFC exporter).

use rvt::{RevitFile, walker};

fn main() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    let files = [
        format!("{project_dir}/Revit_IFC5_Einhoven.rvt"),
        format!("{project_dir}/2024_Core_Interior.rvt"),
    ];
    for path in files {
        let name = path.rsplit('/').next().unwrap();
        let Ok(mut rf) = RevitFile::open(&path) else {
            println!("{name}: open failed");
            continue;
        };
        match walker::iter_elements(&mut rf) {
            Ok(iter) => {
                let elements: Vec<_> = iter.collect();
                let mut by_class: std::collections::BTreeMap<String, usize> =
                    std::collections::BTreeMap::new();
                let mut with_id = 0usize;
                for el in &elements {
                    *by_class.entry(el.class.clone()).or_insert(0) += 1;
                    if el.id.is_some() {
                        with_id += 1;
                    }
                }
                let mut top: Vec<_> = by_class.iter().collect();
                top.sort_by_key(|(_, c)| std::cmp::Reverse(**c));
                println!(
                    "\n=== {name}: {} elements ({} with id), {} distinct classes ===",
                    elements.len(),
                    with_id,
                    by_class.len()
                );
                for (class, count) in top.iter().take(10) {
                    println!("  {count:>6}  {class}");
                }
                if by_class.len() > 10 {
                    println!("  …   {} more classes", by_class.len() - 10);
                }
            }
            Err(e) => println!("{name}: iter_elements error — {e}"),
        }
    }
}
