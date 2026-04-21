//! RE-14.2 — record-prefix-filtered signal analysis per D14 of
//! `RE-14.1-synthesis.md`.
//!
//! RE-11 computed observed/expected ratios per tag and found 10-16
//! "strong signal" classes. RE-14.1 revealed that for tag 0x006b
//! (HostObjAttr) ~80% of hits are UTF-16LE text artifacts (letter 'k'
//! inside parameter-name strings), and only ~20% are real records.
//!
//! Hypothesis H11 from RE-14.1 (conf 0.5): all 10 top-signal tags
//! have similar bimodal populations. If true, filtering by a record-
//! prefix signature (bytes at +2 and +3 relative to the tag == 0x00,
//! 0x00) isolates real records from text artifacts.
//!
//! This probe:
//!   1. Scans all tagged classes (80 on 2023, 79 on 2024) across a
//!      partition stream.
//!   2. For each, counts raw occurrences AND record-prefix-filtered
//!      occurrences.
//!   3. Reports raw_count, filtered_count, filter_ratio (kept / raw),
//!      and re-ranks by filtered_count.
//!
//! Expected outcomes:
//!   - Text-artifact tags (e.g. 0x006b when the 'k' is textual): low
//!     filter_ratio (~15-25%).
//!   - True-record tags: high filter_ratio (>50%).
//!   - Re-ranking should surface different "top elements" than raw
//!     ranking.

use rvt::{RevitFile, compression, formats, streams};
use std::collections::BTreeMap;

fn main() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());

    for (file, partition) in [
        ("Revit_IFC5_Einhoven.rvt", "Partitions/0"),
        ("Revit_IFC5_Einhoven.rvt", "Partitions/5"),
        ("2024_Core_Interior.rvt", "Partitions/46"),
    ] {
        let path = format!("{project_dir}/{file}");
        let Ok(mut rf) = RevitFile::open(&path) else {
            continue;
        };
        let Ok(formats_raw) = rf.read_stream(streams::FORMATS_LATEST) else {
            continue;
        };
        let Ok(formats_d) = compression::inflate_at(&formats_raw, 0) else {
            continue;
        };
        let Ok(schema) = formats::parse_schema(&formats_d) else {
            continue;
        };
        let tag_to_name: BTreeMap<u16, &str> = schema
            .classes
            .iter()
            .filter_map(|c| c.tag.map(|t| (t, c.name.as_str())))
            .collect();

        let Ok(raw) = rf.read_stream(partition) else {
            continue;
        };
        let chunks = compression::inflate_all_chunks(&raw);
        let concat: Vec<u8> = chunks.into_iter().flatten().collect();
        if concat.len() < 6 {
            continue;
        }

        println!(
            "\n=== {file} {partition} — {} B, {} tagged classes ===",
            concat.len(),
            tag_to_name.len()
        );

        // For each tagged class, compute raw + filtered counts.
        // Filter: the u16 at position+2 must be 0x0000 (record-null,
        // not UTF-16 text continuation).
        let mut rows: Vec<(u16, &str, usize, usize)> = Vec::new();
        for (&tag, &name) in &tag_to_name {
            let mut raw_count = 0usize;
            let mut filtered_count = 0usize;
            for i in 0..concat.len().saturating_sub(3) {
                let v = u16::from_le_bytes([concat[i], concat[i + 1]]);
                if v != tag {
                    continue;
                }
                raw_count += 1;
                if concat[i + 2] == 0x00 && concat[i + 3] == 0x00 {
                    filtered_count += 1;
                }
            }
            if raw_count > 0 {
                rows.push((tag, name, raw_count, filtered_count));
            }
        }

        // Sort by filtered_count descending — this is the new ranking.
        rows.sort_by_key(|(_, _, _, filtered)| std::cmp::Reverse(*filtered));

        println!(
            "\n  Top-20 tags ranked by record-prefix-filtered count \
             (filter: buf[pos+2..pos+4] == 0x0000):"
        );
        println!(
            "    {:^8} {:<30} {:>10} {:>10} {:>10} {:>10}",
            "tag", "class", "raw", "filtered", "kept %", "expected"
        );
        println!("    {}", "-".repeat(90));
        let positions = concat.len().saturating_sub(3);
        let expected = (positions as f64) / 65536.0;
        for (tag, name, raw, filt) in rows.iter().take(20) {
            if *raw == 0 {
                continue;
            }
            let pct = 100.0 * (*filt as f64) / (*raw as f64);
            println!(
                "    0x{tag:04x}   {name:<30} {raw:>10} {filt:>10} {:>9.1}% {:>10.0}",
                pct, expected
            );
        }

        // Also: count how many tags have high filter ratio (real) vs
        // low filter ratio (text artifact) vs in-between.
        let real_tag_count = rows.iter().filter(|(_, _, _, f)| *f > 50).count();
        let text_tag_count = rows
            .iter()
            .filter(|(_, _, r, f)| *r > 100 && (*f as f64 / *r as f64) < 0.3)
            .count();
        let total_real_records: usize = rows
            .iter()
            .filter(|(_, _, _, f)| *f > 50)
            .map(|(_, _, _, f)| *f)
            .sum();
        println!(
            "\n  Summary: {real_tag_count} tags with filtered>50 records, \
             {text_tag_count} tags with >100 raw but <30% kept (text artifacts). \
             Total filtered records across strong tags: {total_real_records}"
        );
    }
}
