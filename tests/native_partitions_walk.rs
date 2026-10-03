//! The native path walks every partition of every family, 2016 to 2026
//! (#421).
//!
//! `native_segments::walk` reads a partition's segment markers with the
//! file's own schema (`schema_registry::parse`). puzzbobb reported (#421)
//! what it meets before 2018:
//!
//! - the 2016 and 2017 schemas have no `SignatureMarker` class, and their
//!   partitions never write one;
//! - `SegmentMarker.m_continuationBits` is `(n << 2) | flags`, with `n` = 1
//!   from 2018 and 100 or more before, while the continuation flags (1 = from
//!   the previous segment, 2 = into the next) mean the same throughout.
//!
//! Every family's partitions must walk to their end, each yielding at least
//! one group. Corpus resolution follows `tests/schema_registry_catalogs.rs`.

mod common;

use common::{ALL_YEARS, sample_for_year};
use rvt::compression::prepare_stream_for_inflate;
use rvt::{RevitFile, native_document, native_segments, schema_registry};

fn require_family_corpus() -> bool {
    std::env::var("RVT_REQUIRE_CORPUS")
        .ok()
        .is_some_and(|v| v == "1" || v == "true")
}

#[test]
fn every_family_partition_walks_to_its_end() {
    let mut missing = Vec::new();
    let mut failures = Vec::new();
    for year in ALL_YEARS {
        let path = sample_for_year(year);
        if !path.exists() {
            missing.push(year);
            continue;
        }
        let mut rf = RevitFile::open(&path).expect("open");
        let registry = match native_document::read_single(&mut rf, "Formats/Latest")
            .and_then(|bytes| schema_registry::parse(&bytes))
        {
            Ok(registry) => registry,
            Err(error) => {
                failures.push(format!("{year}: schema: {error:#}"));
                continue;
            }
        };
        let names: Vec<String> = rf
            .stream_names()
            .iter()
            .filter(|n| n.starts_with("Partitions/"))
            .cloned()
            .collect();
        assert!(!names.is_empty(), "{year}: the family has partitions");
        for name in names {
            let stored = rf.read_stream(&name).expect("partition stream");
            let prepared = prepare_stream_for_inflate(&name, &stored);
            match native_segments::walk(&prepared, &registry, 256 * 1024 * 1024, |_, _| Ok(())) {
                Ok(stats) if stats.groups > 0 => {}
                Ok(_) => failures.push(format!("{year} {name}: no group")),
                Err(error) => failures.push(format!("{year} {name}: {error:#}")),
            }
        }
    }
    assert!(
        failures.is_empty(),
        "native_segments::walk reads every family partition to its end: {failures:?}"
    );
    assert!(
        missing.is_empty() || !require_family_corpus(),
        "family corpus incomplete, missing {missing:?}, and RVT_REQUIRE_CORPUS is set"
    );
    if !missing.is_empty() {
        eprintln!("skipped family releases (corpus absent): {missing:?}");
    }
}
