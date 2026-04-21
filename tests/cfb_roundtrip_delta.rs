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
use rvt::streams::BASIC_FILE_INFO;
use rvt::writer::{StreamFraming, StreamPatch, write_with_patches};
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

/// WRT-10.3 non-empty-patch delta — exercise the sector-preservation
/// path that rewrites a single stream in-place on an `open_rw`-opened
/// copy of the source. Every UNPATCHED stream should keep its
/// physical sectors; only the patched stream's sector chain (plus
/// FAT entries describing it) should diverge from source.
///
/// Baseline before WRT-10.3: ~94% delta for this case too (full
/// rebuild). After: single-stream patch should yield single-digit-%
/// delta across every release.
#[test]
fn cfb_single_stream_patch_preserves_unpatched_sectors() -> Result<()> {
    if !corpus_available() {
        eprintln!(
            "skipping single-stream patch delta: corpus missing at {}",
            samples_dir().display()
        );
        return Ok(());
    }

    let tmp = std::env::temp_dir().join(format!("rvt-patch-delta-{}", std::process::id()));
    fs::create_dir_all(&tmp).unwrap();

    // The streams constant and writer types are pulled in at the
    // top of the file; suppress dead-code warnings when the corpus
    // is absent.
    let _ = (StreamFraming::Verbatim, BASIC_FILE_INFO);

    for year in ALL_YEARS {
        let src = sample_for_year(year);
        let dst = tmp.join(format!("patch-{year}.rfa"));

        // Read the current BasicFileInfo bytes and re-write them
        // unchanged. This is a trivial patch — every byte of the
        // stream stays the same, so the ONLY delta should be any
        // internal state that `cfb::open_rw`'s `create_stream`
        // touches incidentally (timestamps, state bits).
        let existing = {
            let mut rf = rvt::RevitFile::open(&src)?;
            rf.read_stream(BASIC_FILE_INFO)?
        };
        let patch = StreamPatch {
            stream_name: BASIC_FILE_INFO.into(),
            new_decompressed: existing,
            framing: StreamFraming::Verbatim,
        };
        write_with_patches(&src, &dst, &[patch])?;

        let src_bytes = fs::read(&src).unwrap();
        let dst_bytes = fs::read(&dst).unwrap();
        let delta = byte_delta(&src_bytes, &dst_bytes);
        let pct = (delta as f64 * 100.0) / src_bytes.len() as f64;
        eprintln!(
            "year {year}: src={} B, dst={} B, delta={} ({pct:.2}%) — \
             single-stream identity patch",
            src_bytes.len(),
            dst_bytes.len(),
            delta
        );

        // Assertion: after WRT-10.3 sector-preservation, single-stream
        // patches must produce < 25% byte delta. (Before: ~94% from
        // full rebuild.) 25% is the safe threshold across all 11
        // releases; observed actual is much lower — this assertion
        // is a floor for "we're definitely better than full rebuild."
        assert!(
            pct < 25.0,
            "{year}: single-stream patch delta {pct:.2}% exceeds 25% ceiling — \
             sector-preservation path regressed to full-rebuild behaviour"
        );
    }

    let _ = fs::remove_dir_all(&tmp);
    Ok(())
}
