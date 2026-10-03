//! Every family's `Formats/Latest` catalog (2016 to 2026) is read to its end
//! by `schema_registry::parse`, the parser the native record path uses, and
//! gives the same classes as `formats::schema_classes`, which reads every
//! one of them to its end (#154). puzzbobb reported that the first stops on
//! Revit 2018 (#421, #154).
//!
//! Corpus resolution follows `tests/elem_table_corpus.rs`: with
//! `RVT_REQUIRE_CORPUS` set a missing release fails, otherwise it skips and
//! says so.

mod common;

use common::{ALL_YEARS, sample_for_year};
use rvt::{RevitFile, native_document, schema_registry};
use std::collections::BTreeMap;

fn require_family_corpus() -> bool {
    std::env::var("RVT_REQUIRE_CORPUS")
        .ok()
        .is_some_and(|v| v == "1" || v == "true")
}

#[test]
fn every_family_catalog_is_read_to_its_end() {
    let mut missing = Vec::new();
    let mut failures = Vec::new();
    for year in ALL_YEARS {
        let path = sample_for_year(year);
        if !path.exists() {
            missing.push(year);
            continue;
        }
        let mut rf = RevitFile::open(&path).expect("open");
        let expected: BTreeMap<u16, String> = rf
            .schema_classes()
            .expect("schema_classes")
            .classes
            .into_iter()
            .map(|class| (class.tag, class.name))
            .collect();
        let bytes = native_document::read_single(&mut rf, "Formats/Latest").expect("catalog");
        match schema_registry::parse(&bytes) {
            Ok(registry) => {
                let got: BTreeMap<u16, String> = registry
                    .classes
                    .iter()
                    .map(|class| (class.tag, class.name.clone()))
                    .collect();
                if got != expected {
                    failures.push(format!(
                        "{year}: {} classes, schema_classes has {}",
                        got.len(),
                        expected.len()
                    ));
                }
            }
            Err(error) => failures.push(format!("{year}: {error:#}")),
        }
    }
    assert!(
        failures.is_empty(),
        "schema_registry::parse reads every family catalog to its end: {failures:?}"
    );
    assert!(
        missing.is_empty() || !require_family_corpus(),
        "family corpus incomplete, missing {missing:?}, and RVT_REQUIRE_CORPUS is set"
    );
    if !missing.is_empty() {
        eprintln!("skipped family releases (corpus absent): {missing:?}");
    }
}
