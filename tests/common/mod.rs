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
