//! RE-40: design options on the project corpus.
//!
//! Revit's own IFC export leaves out elements in a design option set's
//! non-primary options, and rvt-rs does the same once it reads the set's
//! primary from its name entry. The licensed project files have no design
//! options, so this pins that nothing on them is read as one: no option set
//! resolves, and no placed instance carries a design option at `+0x2a`.
//! The positive evidence (Snowdon Towers, local only) is in
//! `reports/element-framing/RE-40-design-option-primary.md`.
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR`. Skips what is absent.

use rvt::RevitFile;
use rvt::partition_element_records::{RECOVERED_CATEGORIES, scan_category_records_multi};
use std::collections::BTreeSet;
use std::path::PathBuf;

const MODELS: [&str; 5] = [
    "2024_Core_Interior.rvt",
    "RE1-Architecture.rvt",
    "RE1-Mechanical.rvt",
    "RE1-Plumbing.rvt",
    "RE1-Electrical.rvt",
];

#[test]
fn files_without_design_options_read_none() {
    let Some(corpus) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let mut checked = 0;
    for name in MODELS {
        let path = corpus.join(name);
        if !path.is_file() {
            eprintln!("skipping {name}: not under RVT_PROJECT_CORPUS_DIR");
            continue;
        }
        let mut rf = RevitFile::open(&path).expect("open");
        let version = rf.basic_file_info().expect("BasicFileInfo").version;
        let options = rf.design_options();
        assert!(options.sets.is_empty(), "{name}: {:?}", options.sets);
        assert!(
            options.unresolved_sets.is_empty(),
            "{name}: {:?}",
            options.unresolved_sets
        );
        let declared: BTreeSet<u32> = rvt::elem_table::parse_records(&mut rf)
            .expect("ElemTable")
            .into_iter()
            .map(|r| r.id_primary)
            .collect();
        let categories: Vec<i64> = RECOVERED_CATEGORIES.iter().map(|(c, _)| *c).collect();
        let records =
            scan_category_records_multi(&mut rf, version, &categories, &declared).expect("scan");
        let placed: Vec<_> = records
            .iter()
            .filter(|r| r.is_exported_instance())
            .collect();
        assert!(!placed.is_empty(), "{name}: no placed instances");
        let in_option: Vec<(u32, u32)> = placed
            .iter()
            .filter_map(|r| r.design_option.map(|o| (r.element_id, o)))
            .collect();
        assert!(in_option.is_empty(), "{name}: {in_option:?}");
        checked += 1;
    }
    eprintln!("checked {checked} model(s)");
}
