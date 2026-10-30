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

/// Fail when `step`, the writer's output for a synthetic model written at
/// [`SYNTHETIC_FIXTURE_TIMESTAMP`], differs from the committed
/// `tests/fixtures/<name>`. The header's `FILE_NAME` line and the
/// `IfcApplication` version change with every release, so both are left out
/// of the comparison. On a difference the output is written to
/// `$CARGO_TARGET_TMPDIR/synthetic-fixtures/<name>`, which CI uploads as the
/// `synthetic-fixtures` artifact, so the fixture can be refreshed from it.
pub fn assert_matches_fixture(name: &str, step: &str) {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let committed = std::fs::read_to_string(&fixture)
        .unwrap_or_default()
        .replace('\r', "");
    if release_neutral(&committed) == release_neutral(step) {
        return;
    }
    let out = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("synthetic-fixtures");
    std::fs::create_dir_all(&out).expect("synthetic-fixtures dir");
    std::fs::write(out.join(name), step).expect("write the current output");
    panic!(
        "tests/fixtures/{name} is not what the writer produces now; the current output is {}, \
         CI's synthetic-fixtures artifact: commit it if the change is intended",
        out.join(name).display()
    );
}

/// `step` without the parts that change with every release: the header's
/// `FILE_NAME` line and the version in `IfcApplication`.
fn release_neutral(step: &str) -> String {
    step.lines()
        .filter(|line| !line.starts_with("FILE_NAME("))
        .map(|line| {
            if !line.contains("=IFCAPPLICATION(") {
                return line.to_string();
            }
            match line.find('\'').and_then(|open| {
                line[open + 1..]
                    .find('\'')
                    .map(|len| (open, open + 1 + len))
            }) {
                Some((open, close)) => format!("{}'*'{}", &line[..open], &line[close + 1..]),
                None => line.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
