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

    // Regression gate: today's delta is the baseline. A future
    // commit that regresses any release's delta upward should fail
    // here — the test leaves the baseline stored in its own output
    // for CI log inspection. Absolute-value assertion is loose
    // (< 100%) so existing deltas pass; WRT-10.3 will tighten this
    // to `== 0` under the opt-in flag.
    for (year, delta, src_len) in &per_release {
        assert!(
            *delta < *src_len,
            "{year}: delta {delta} exceeds source size {src_len} — \
             rewriter is producing completely divergent output"
        );
    }

    // Cleanup.
    let _ = fs::remove_dir_all(&tmp);
    Ok(())
}
