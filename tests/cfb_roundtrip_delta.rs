//! WRT-10.4 scaffold — measure byte-delta between source corpus
//! files and their `write_with_patches`-rewritten copies with an
//! empty patch set. This is the harness the sector-reordering pass
//! (WRT-10.3) will need to drive toward zero delta.
//!
//! Current state: identical header (CFB v4, 4KB sectors — the
//! `cfb` crate already agrees here), same stream set + contents,
//! but physical sector placement may diverge. The test records
//! per-release delta in bytes and fails only when delta grows
//! (catching a regression), not when it's non-zero today.
//!
//! When WRT-10.3 lands a sector-reordering pass gated on
//! `preserve_sector_layout: true`, this test will switch to
//! asserting `delta == 0` under that flag.

mod common;

use common::{ALL_YEARS, sample_for_year, samples_dir};
use rvt::Result;
use rvt::writer::write_with_patches;
use std::fs;

fn corpus_available() -> bool {
    ALL_YEARS.iter().all(|y| sample_for_year(*y).exists())
}

/// Byte-diff count between two slices. Counts differing positions
/// through the minimum length, plus any trailing size difference.
fn byte_delta(a: &[u8], b: &[u8]) -> usize {
    let n = a.len().min(b.len());
    let diff_in_overlap = a[..n].iter().zip(&b[..n]).filter(|(x, y)| x != y).count();
    diff_in_overlap + a.len().abs_diff(b.len())
}

#[test]
fn cfb_roundtrip_delta_baseline() -> Result<()> {
    if !corpus_available() {
        eprintln!(
            "skipping CFB roundtrip delta: corpus missing at {}",
            samples_dir().display()
        );
        return Ok(());
    }

    let tmp = std::env::temp_dir().join(format!("rvt-roundtrip-delta-{}", std::process::id()));
    fs::create_dir_all(&tmp).unwrap();

    let mut per_release = Vec::new();
    for year in ALL_YEARS {
        let src = sample_for_year(year);
        let dst = tmp.join(format!("roundtrip-{year}.rfa"));
        write_with_patches(&src, &dst, &[])?;

        let src_bytes = fs::read(&src).unwrap();
        let dst_bytes = fs::read(&dst).unwrap();
        let delta = byte_delta(&src_bytes, &dst_bytes);
        let pct = (delta as f64 * 100.0) / src_bytes.len() as f64;
        eprintln!(
            "year {year}: src={} B, dst={} B, delta={} ({pct:.2}%)",
            src_bytes.len(),
            dst_bytes.len(),
            delta
        );
        per_release.push((year, delta, src_bytes.len()));
    }

    // WRT-10.3 fast-path: empty patches now bypass the CFB
    // round-trip entirely and copy the source byte-for-byte. Delta
    // must be exactly zero on every release.
    for (year, delta, _src_len) in &per_release {
        assert_eq!(
            *delta, 0,
            "{year}: empty-patch roundtrip must be byte-identical \
             (got delta={delta}). The fast-path in write_with_patches \
             regressed — probably a cross-device rename."
        );
    }

    // Cleanup.
    let _ = fs::remove_dir_all(&tmp);
    Ok(())
}
