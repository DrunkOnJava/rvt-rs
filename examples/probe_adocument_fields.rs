//! Dump the 13 ADocument fields as decoded by the walker on a real
//! 2023 project file. Quick-look probe for the walker → IFC work:
//! is each field decoding cleanly, and what are the pointer /
//! ElementId values we'd follow next?
//!
//! Path resolves via `RVT_PROJECT_CORPUS_DIR` (default
//! `/private/tmp/rvt-corpus-probe/magnetar/Revit`).

use rvt::{RevitFile, walker};

fn main() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    let path = format!("{project_dir}/Revit_IFC5_Einhoven.rvt");
    let mut rf = RevitFile::open(&path).unwrap();
    let d = walker::read_adocument_lossy(&mut rf).unwrap();
    println!(
        "ADocument: {} fields, entry=0x{:x}",
        d.value.fields.len(),
        d.value.entry_offset
    );
    for (i, (name, value)) in d.value.fields.iter().enumerate() {
        println!("  {:2}. {} = {:?}", i, name, value);
    }
}
