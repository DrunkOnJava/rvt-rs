//! Q6.4 (sixth-pass, the one from the §Q6.4 addendum's "next contributor
//! probe" list): Phase-D-style class-tag density scan over the ~935 KB
//! region of `Global/Latest` that sits AFTER Table B ends. Phase D
//! originally found that class tags from `Formats/Latest` occur in the
//! full `Global/Latest` stream at ~340× uniform-random rate, proving
//! schema-is-live-type-dictionary. That was a whole-stream measurement.
//! This probe runs the same test but RESTRICTED to the post-Table-B
//! region, which is where the actual object payload must be hiding.
//!
//! If density in the post-Table-B region is ≥100× uniform, that IS
//! where schema-driven instance data lives, and Q6.5 reduces to finding
//! the first schema-tagged record in that region.

#![allow(clippy::needless_range_loop)]

use rvt::{RevitFile, compression, formats, streams};
use std::collections::HashSet;

fn find_table_b_end(d: &[u8]) -> usize {
    // Match the logic in second_table_probe.rs: scan for any `[u32 1]`
    // anchor, extend into a sequential run (>=5 records). Take the
    // LAST such run and report its end offset.
    let mut last_end = 0usize;
    let mut i = 0;
    while i + 4 < d.len() {
        if d[i..i + 4] == [1, 0, 0, 0] {
            let mut cursor = i + 4;
            let mut expect: u32 = 2;
            let mut end = i + 4;
            while cursor + 4 <= d.len() {
                let marker = [
                    (expect & 0xff) as u8,
                    ((expect >> 8) & 0xff) as u8,
                    ((expect >> 16) & 0xff) as u8,
                    ((expect >> 24) & 0xff) as u8,
                ];
                let window_end = (cursor + 64).min(d.len());
                if let Some(p) = d[cursor..window_end].windows(4).position(|w| w == marker) {
                    end = cursor + p + 4;
                    cursor = end;
                    expect += 1;
                } else {
                    break;
                }
            }
            let records = expect - 1;
            if records >= 5 {
                // record this run's end + pad for the last record's body
                last_end = end + 32; // 32-byte pad for trailing body
                i = end;
                continue;
            }
        }
        i += 1;
    }
    last_end
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: post_table_b_density <file>");
    let mut rf = RevitFile::open(&path)?;

    // 1. Pull the schema; collect all known class tags.
    let formats_raw = rf.read_stream(streams::FORMATS_LATEST)?;
    let formats_d = compression::inflate_at(&formats_raw, 0)?;
    let schema = formats::parse_schema(&formats_d)?;
    let tags: HashSet<u16> = schema.classes.iter().filter_map(|c| c.tag).collect();
    println!(
        "Schema: {} classes, {} have tags (using the tagged set for density probe)",
        schema.classes.len(),
        tags.len()
    );

    // 2. Decompress Global/Latest; locate the post-Table-B boundary.
    let raw_gl = rf.read_stream(streams::GLOBAL_LATEST)?;
    let d = compression::inflate_at(&raw_gl, 8)?;
    let cutoff = find_table_b_end(&d);
    if cutoff == 0 || cutoff >= d.len() {
        println!("Could not locate post-Table-B boundary — abort.");
        return Ok(());
    }
    let pre = &d[..cutoff];
    let post = &d[cutoff..];
    println!("Global/Latest: {} bytes decompressed", d.len());
    println!(
        "  Pre-Table-B (through end of Table B): {} bytes (0x000000..0x{:06x})",
        pre.len(),
        cutoff
    );
    println!(
        "  Post-Table-B (candidate object region): {} bytes (0x{:06x}..0x{:06x})",
        post.len(),
        cutoff,
        d.len()
    );

    // 3. Count class-tag occurrences in each region. A u16 tag `t` is
    //    counted once per position in the sliding 2-byte window where
    //    `t` matches (mirrors the Phase D method).
    fn count_tags(bytes: &[u8], tags: &HashSet<u16>) -> (usize, usize) {
        let mut total = 0usize;
        let mut distinct: HashSet<u16> = HashSet::new();
        for w in bytes.windows(2) {
            let v = u16::from_le_bytes([w[0], w[1]]);
            if tags.contains(&v) {
                total += 1;
                distinct.insert(v);
            }
        }
        (total, distinct.len())
    }

    let (pre_total, pre_distinct) = count_tags(pre, &tags);
    let (post_total, post_distinct) = count_tags(post, &tags);

    // 4. Expected under uniform-random.
    let expected_per_byte = (tags.len() as f64) / 65536.0;
    let pre_expected = (pre.len().saturating_sub(1) as f64) * expected_per_byte;
    let post_expected = (post.len().saturating_sub(1) as f64) * expected_per_byte;

    let pre_ratio = if pre_expected > 0.0 {
        pre_total as f64 / pre_expected
    } else {
        0.0
    };
    let post_ratio = if post_expected > 0.0 {
        post_total as f64 / post_expected
    } else {
        0.0
    };
    println!();
    println!(
        "{:36} {:>10} {:>10} {:>10} {:>8}",
        "region", "total_hits", "distinct", "expected", "ratio"
    );
    println!("{}", "-".repeat(80));
    println!(
        "{:36} {:>10} {:>10} {:>10.2} {:>7.1}×",
        "pre-Table-B (incl. directory)", pre_total, pre_distinct, pre_expected, pre_ratio
    );
    println!(
        "{:36} {:>10} {:>10} {:>10.2} {:>7.1}×",
        "post-Table-B (candidate object region)",
        post_total,
        post_distinct,
        post_expected,
        post_ratio
    );

    println!();
    println!("Interpretation:");
    println!("  ≥100×  → post-Table-B region IS schema-driven object data (Q6.5 mostly solved)");
    println!("  ~10×   → partial signal; other structure likely mixed in");
    println!("   ≈1×   → post-Table-B region is opaque (compressed / encrypted / other)");
    println!(
        "  For comparison: the whole-stream Phase D measurement found ~340× density in the ENTIRE Global/Latest,"
    );
    println!(
        "  so we expect the post-Table-B region specifically to show an even higher ratio if it's the payload."
    );
    Ok(())
}
