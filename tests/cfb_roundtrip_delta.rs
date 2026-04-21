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
use rvt::streams::{BASIC_FILE_INFO, PART_ATOM};
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

/// CFB-01: patched stream GROWS. Append 4KB to BasicFileInfo (one
/// extra sector). Must: (a) file stays readable, (b) patched stream
/// round-trips the new content, (c) unpatched streams still open
/// cleanly and return their original bytes.
#[test]
fn cfb_patch_growing_stream_preserves_unpatched() -> Result<()> {
    if !corpus_available() {
        eprintln!(
            "skipping growing-stream patch test: corpus missing at {}",
            samples_dir().display()
        );
        return Ok(());
    }

    let tmp = std::env::temp_dir().join(format!("rvt-grow-{}", std::process::id()));
    fs::create_dir_all(&tmp).unwrap();

    for year in ALL_YEARS {
        let src = sample_for_year(year);
        let dst = tmp.join(format!("grow-{year}.rfa"));

        // Snapshot the original PartAtom bytes to compare after patch.
        let (original_bfi, original_partatom) = {
            let mut rf = rvt::RevitFile::open(&src)?;
            let bfi = rf.read_stream(BASIC_FILE_INFO)?;
            let pa = rf.read_stream(PART_ATOM)?;
            (bfi, pa)
        };

        // Grow BasicFileInfo by 4 KB of trailer padding.
        let mut grown = original_bfi.clone();
        grown.extend(std::iter::repeat_n(0u8, 4096));
        let patch = StreamPatch {
            stream_name: BASIC_FILE_INFO.into(),
            new_decompressed: grown.clone(),
            framing: StreamFraming::Verbatim,
        };
        write_with_patches(&src, &dst, &[patch])?;

        // (a) dst is openable as a Revit file.
        let mut rw = rvt::RevitFile::open(&dst)?;

        // (b) patched stream round-trips the grown content.
        let bfi_after = rw.read_stream(BASIC_FILE_INFO)?;
        assert_eq!(
            bfi_after, grown,
            "{year}: grown BasicFileInfo did not round-trip"
        );

        // (c) unpatched stream (PartAtom) still opens and matches source.
        let pa_after = rw.read_stream(PART_ATOM)?;
        assert_eq!(
            pa_after, original_partatom,
            "{year}: unpatched PartAtom bytes changed after growing patch"
        );
    }

    let _ = fs::remove_dir_all(&tmp);
    Ok(())
}

/// CFB-02: patched stream SHRINKS. Cut BasicFileInfo in half.
/// cfb::open_rw may or may not reclaim the freed sectors; what we
/// require is that (a) the file stays valid, (b) the stream reads
/// back at its new length, (c) unpatched streams are unaffected.
#[test]
fn cfb_patch_shrinking_stream_preserves_unpatched() -> Result<()> {
    if !corpus_available() {
        eprintln!(
            "skipping shrinking-stream patch test: corpus missing at {}",
            samples_dir().display()
        );
        return Ok(());
    }

    let tmp = std::env::temp_dir().join(format!("rvt-shrink-{}", std::process::id()));
    fs::create_dir_all(&tmp).unwrap();

    for year in ALL_YEARS {
        let src = sample_for_year(year);
        let dst = tmp.join(format!("shrink-{year}.rfa"));

        let (original_bfi, original_partatom) = {
            let mut rf = rvt::RevitFile::open(&src)?;
            let bfi = rf.read_stream(BASIC_FILE_INFO)?;
            let pa = rf.read_stream(PART_ATOM)?;
            (bfi, pa)
        };
        // Skip years where BFI is already tiny.
        if original_bfi.len() < 32 {
            continue;
        }

        let shrunk = original_bfi[..original_bfi.len() / 2].to_vec();
        let patch = StreamPatch {
            stream_name: BASIC_FILE_INFO.into(),
            new_decompressed: shrunk.clone(),
            framing: StreamFraming::Verbatim,
        };
        write_with_patches(&src, &dst, &[patch])?;

        let mut rw = rvt::RevitFile::open(&dst)?;
        let bfi_after = rw.read_stream(BASIC_FILE_INFO)?;
        assert_eq!(
            bfi_after, shrunk,
            "{year}: shrunk BasicFileInfo did not round-trip"
        );

        let pa_after = rw.read_stream(PART_ATOM)?;
        assert_eq!(
            pa_after, original_partatom,
            "{year}: unpatched PartAtom bytes changed after shrinking patch"
        );
    }

    let _ = fs::remove_dir_all(&tmp);
    Ok(())
}

/// CFB-03: two streams patched simultaneously. Verify both round-trip
/// correctly and all OTHER streams in the file still read cleanly.
#[test]
fn cfb_multi_stream_patch_preserves_rest() -> Result<()> {
    if !corpus_available() {
        eprintln!(
            "skipping multi-stream patch test: corpus missing at {}",
            samples_dir().display()
        );
        return Ok(());
    }

    let tmp = std::env::temp_dir().join(format!("rvt-multi-{}", std::process::id()));
    fs::create_dir_all(&tmp).unwrap();

    for year in ALL_YEARS {
        let src = sample_for_year(year);
        let dst = tmp.join(format!("multi-{year}.rfa"));

        // Snapshot every stream's bytes before patching so we can
        // compare unpatched streams after.
        let all_streams: Vec<(String, Vec<u8>)> = {
            let mut rf = rvt::RevitFile::open(&src)?;
            let names = rf.stream_names();
            let mut out = Vec::with_capacity(names.len());
            for name in names {
                let data = rf.read_stream(&name)?;
                out.push((name, data));
            }
            out
        };

        // Build patches for the two targets, using their original bytes
        // grown by 1 KB each. That's a "changed content + grown size"
        // pair of patches — exercises multi-stream mutation without
        // adding complexity we can't assert against.
        let bfi_original = all_streams
            .iter()
            .find(|(n, _)| n == BASIC_FILE_INFO)
            .map(|(_, b)| b.clone());
        let pa_original = all_streams
            .iter()
            .find(|(n, _)| n == PART_ATOM)
            .map(|(_, b)| b.clone());

        let Some(bfi_original) = bfi_original else {
            continue;
        };
        let Some(pa_original) = pa_original else {
            continue;
        };

        let mut new_bfi = bfi_original.clone();
        new_bfi.extend(std::iter::repeat_n(0xAAu8, 1024));
        let mut new_pa = pa_original.clone();
        new_pa.extend(std::iter::repeat_n(0xBBu8, 1024));

        let patches = vec![
            StreamPatch {
                stream_name: BASIC_FILE_INFO.into(),
                new_decompressed: new_bfi.clone(),
                framing: StreamFraming::Verbatim,
            },
            StreamPatch {
                stream_name: PART_ATOM.into(),
                new_decompressed: new_pa.clone(),
                framing: StreamFraming::Verbatim,
            },
        ];
        write_with_patches(&src, &dst, &patches)?;

        // Both patches must round-trip.
        let mut rw = rvt::RevitFile::open(&dst)?;
        assert_eq!(
            rw.read_stream(BASIC_FILE_INFO)?,
            new_bfi,
            "{year}: BasicFileInfo didn't round-trip after multi-stream patch"
        );
        assert_eq!(
            rw.read_stream(PART_ATOM)?,
            new_pa,
            "{year}: PartAtom didn't round-trip after multi-stream patch"
        );

        // Every OTHER stream must read back identical to source.
        for (name, original) in &all_streams {
            if name == BASIC_FILE_INFO || name == PART_ATOM {
                continue;
            }
            let after = rw.read_stream(name)?;
            assert_eq!(
                &after,
                original,
                "{year}: unpatched stream {name} changed after multi-stream patch \
                 (src={} B, dst={} B)",
                original.len(),
                after.len()
            );
        }
    }

    let _ = fs::remove_dir_all(&tmp);
    Ok(())
}
