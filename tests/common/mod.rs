//! Shared helpers for integration tests.
//!
//! The 11-release `rac_basic_sample_family` corpus lives in
//! `phi-ag/rvt` — rvt-rs does not redistribute these Autodesk-owned
//! files (see SECURITY.md). Corpus location is resolvable by either:
//!
//!   - `RVT_SAMPLES_DIR` env var (used by CI when it checks out
//!     `phi-ag/rvt` into `_corpus/examples/Autodesk`), or
//!   - the default `_corpus/examples/Autodesk` under the crate root, where
//!     `tools/check-local.sh` looks and `git clone https://github.com/phi-ag/rvt _corpus`
//!     puts the files.

#![allow(dead_code)]

use std::path::PathBuf;

pub fn samples_dir() -> PathBuf {
    if let Ok(env_dir) = std::env::var("RVT_SAMPLES_DIR") {
        return PathBuf::from(env_dir);
    }
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("_corpus/examples/Autodesk");
    p
}

pub fn sample_for_year(year: u32) -> PathBuf {
    let filename = match year {
        2016..=2019 => format!("rac_basic_sample_family-{year}.rfa"),
        2020..=2027 => format!("racbasicsamplefamily-{year}.rfa"),
        _ => panic!("unknown sample year {year}"),
    };
    samples_dir().join(filename)
}

pub const ALL_YEARS: [u32; 12] = [
    2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026, 2027,
];

/// The Unix timestamp the committed synthetic IFC fixtures are written with,
/// so their header and owner history do not change from run to run.
pub const SYNTHETIC_FIXTURE_TIMESTAMP: i64 = 1_700_000_000;

/// Write `step`, the writer's output for a synthetic model written at
/// [`SYNTHETIC_FIXTURE_TIMESTAMP`], to `$CARGO_TARGET_TMPDIR/synthetic-fixtures/<name>`.
/// CI's IfcOpenShell job validates it there (`ifcopenshell.validate` and
/// `tools/ci/ifc_schema_arity.py`, whose placement probes pin these models'
/// walls, slab and column), and the test job uploads it as the
/// `synthetic-fixtures` artifact, from which each release pull request
/// refreshes the viewer's download copies in `tests/fixtures/`. Nothing
/// compares it with those copies: a change to the writer's output is checked
/// for validity, not for being a change.
pub fn write_synthetic_output(name: &str, step: &str) {
    let out = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("synthetic-fixtures");
    std::fs::create_dir_all(&out).expect("synthetic-fixtures dir");
    std::fs::write(out.join(name), step).expect("write the synthetic output");
}
