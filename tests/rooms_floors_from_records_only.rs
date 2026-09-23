//! Rooms and floors come from partition element records or not at all.
//!
//! The partition MVP used to fill in name-only rooms (space-like display
//! strings) and plan-loop floors (closed runs of f64 pairs that miss the
//! ArcWall centrelines). Neither matched anything in Revit's own exports:
//! every family file on every release gained a room named "Office
//! Equipment", the Revit 2025 RE1 MEP models (`Drshelden/IFC-ECS`, MIT)
//! exported 38 slabs and 71 spaces where their exports hold none, and no
//! 2023 plan loop came within 10% of the plan area of a slab in the paired
//! exports.
//!
//! Families hold no rooms or floors, so every release must yield none.
//! Family corpus: `RVT_SAMPLES_DIR` via `tests/common`, with
//! `RVT_REQUIRE_CORPUS=1` turning a missing release into a failure.

mod common;

use rvt::RevitFile;
use rvt::walker;

fn require_family_corpus() -> bool {
    std::env::var("RVT_REQUIRE_CORPUS")
        .ok()
        .is_some_and(|v| v == "1" || v == "true")
}

#[test]
fn family_files_yield_no_rooms_or_floors_on_any_release() {
    let mut missing = Vec::new();
    let mut checked = 0;
    for year in common::ALL_YEARS {
        let path = common::sample_for_year(year);
        if !path.exists() {
            missing.push(year);
            continue;
        }
        let mut rf = RevitFile::open(&path).expect("open family");
        let invented: Vec<String> = walker::iter_elements(&mut rf)
            .expect("iter_elements")
            .filter(|e| matches!(e.class.as_str(), "Room" | "Floor"))
            .map(|e| format!("{} {:?}", e.class, e.provenance.decoder))
            .collect();
        assert!(
            invented.is_empty(),
            "{year}: rooms or floors on a family file: {invented:?}"
        );
        checked += 1;
    }
    assert!(
        missing.is_empty() || !require_family_corpus(),
        "family corpus incomplete — missing release(s): {missing:?}. RVT_REQUIRE_CORPUS \
         is set, so this is a regression, not a setup gap."
    );
    if checked == 0 {
        eprintln!("skipping: no family file under RVT_SAMPLES_DIR");
    }
}
