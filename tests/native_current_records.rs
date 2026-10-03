//! The native record path emits a current record for every element the
//! index declares (#548).
//!
//! On Einhoven (2023) and Core Interior (2024) every element
//! `Global/ElemTable` declares has a channel-101 record in some partition
//! (RE-145: 2,615 of 2,615 and 26,425 of 26,425), so `native_document::extract`
//! over channel 101 must emit one record for each of them, and report no
//! selected element without one. An element whose stored revision routes to
//! a partition that holds none of its records is still a current element.
//!
//! Corpus resolution follows `tests/elem_table_frame.rs`: a configured but
//! incomplete project corpus fails, an absent one skips and says so.

use rvt::{RevitFile, native_document};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn project_file(name: &str) -> Option<PathBuf> {
    let dir = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from)?;
    let path = dir.join(name);
    assert!(
        path.exists(),
        "RVT_PROJECT_CORPUS_DIR is set to {} but {name} is not there",
        dir.display()
    );
    Some(path)
}

#[test]
fn every_declared_element_has_its_current_record_emitted() {
    let mut read = 0;
    for name in ["Revit_IFC5_Einhoven.rvt", "2024_Core_Interior.rvt"] {
        let Some(path) = project_file(name) else {
            continue;
        };
        read += 1;
        let mut rf = RevitFile::open(&path).expect("open");
        let options = native_document::Options {
            channels: BTreeSet::from([101]),
            ..native_document::Options::default()
        };
        let mut emitted = BTreeSet::new();
        let summary = native_document::extract(&mut rf, &options, |record| {
            emitted.insert(record.identity.element_id);
            Ok(())
        })
        .unwrap_or_else(|error| panic!("{name}: extract: {error:#}"));
        assert!(
            summary.selected_ids_without_records.is_empty(),
            "{name}: {} declared elements have no emitted record: {:?}",
            summary.selected_ids_without_records.len(),
            &summary.selected_ids_without_records
                [..summary.selected_ids_without_records.len().min(20)]
        );
        assert_eq!(
            emitted.len(),
            summary.indexed_elements,
            "{name}: one emitted record per declared element"
        );
    }
    if read == 0 {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
    }
}
