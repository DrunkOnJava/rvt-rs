//! Phase D link proof: map class tags parsed from Formats/Latest to their
//! occurrence frequency in decompressed Global/Latest, and show the result
//! is extremely non-uniform (~340× concentrated on top tags in a 2024 family
//! file) — confirming the schema is the live type dictionary for the object
//! graph, not a separate artifact.
//!
//! Full writeup in `docs/rvt-moat-break-reconnaissance.md` §Phase D link
//! proof.
use rvt::{
    RevitFile, compression,
    streams::{FORMATS_LATEST, GLOBAL_LATEST},
};
use std::collections::HashMap;

fn looks_like_class_name(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && bytes[0].is_ascii_uppercase()
        && bytes[1..]
            .iter()
            .all(|c| c.is_ascii_alphanumeric() || *c == b'_')
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("path");
    let mut rf = RevitFile::open(&path)?;

    // Step 1: Parse Formats/Latest → list of (class_name, class_tag) where 0x8000 was set.
    let formats_raw = rf.read_stream(FORMATS_LATEST)?;
    let formats = compression::inflate_at(&formats_raw, 0)?;
    let scan_limit = (64 * 1024).min(formats.len());
    let fdata = &formats[..scan_limit];

    let mut tagged_classes: Vec<(String, u16)> = Vec::new();
    let mut i = 0;
    while i + 2 < fdata.len() {
        let len = u16::from_le_bytes([fdata[i], fdata[i + 1]]) as usize;
        if !(3..=60).contains(&len) || i + 2 + len + 2 > fdata.len() {
            i += 1;
            continue;
        }
        let name_bytes = &fdata[i + 2..i + 2 + len];
        if !looks_like_class_name(name_bytes) {
            i += 1;
            continue;
        }
        let after = i + 2 + len;
        let tag = u16::from_le_bytes([fdata[after], fdata[after + 1]]);
        if tag & 0x8000 != 0 {
            let name = std::str::from_utf8(name_bytes).unwrap().to_string();
            tagged_classes.push((name, tag & 0x7fff));
        }
        i += 2 + len;
    }
    tagged_classes.sort_by_key(|c| c.1);
    tagged_classes.dedup_by_key(|c| c.0.clone());
    println!("Tagged classes (0x8000 flag set): {}", tagged_classes.len());
    if tagged_classes.len() >= 5 {
        println!(
            "Tag range: 0x{:04x} ({}) to 0x{:04x} ({})",
            tagged_classes[0].1,
            tagged_classes[0].0,
            tagged_classes.last().unwrap().1,
            tagged_classes.last().unwrap().0,
        );
    }

    // Step 2: Decompress Global/Latest.
    let global_raw = rf.read_stream(GLOBAL_LATEST)?;
    let global = compression::inflate_at(&global_raw, 8)?;
    println!("\nGlobal/Latest decompressed: {} bytes", global.len());

    // Step 3: For each tag, count occurrences as u16 LE AND as u32 LE (with high bytes zero).
    //         Only count tag values that look like real class IDs (> 0 and < 0x4000 to avoid noise).
    let mut hits_u16: HashMap<u16, u32> = HashMap::new();
    let mut u16_pos = 0;
    while u16_pos + 2 <= global.len() {
        let v = u16::from_le_bytes([global[u16_pos], global[u16_pos + 1]]);
        if v > 0 && v < 0x4000 {
            *hits_u16.entry(v).or_insert(0) += 1;
        }
        u16_pos += 1; // overlapping search
    }

    // Show top 20 classes by u16-tag occurrence
    let mut counts: Vec<(&str, u16, u32)> = tagged_classes
        .iter()
        .map(|(n, t)| (n.as_str(), *t, *hits_u16.get(t).unwrap_or(&0)))
        .collect();
    counts.sort_by_key(|c| std::cmp::Reverse(c.2));

    println!("\nTop 25 tagged classes by u16 LE occurrences in Global/Latest:");
    println!(
        "  (reminder: overlapping u16 search yields a noise floor; look at relative distribution)"
    );
    for (name, tag, hits) in counts.iter().take(25) {
        println!("  tag=0x{:04x} ({:5})  hits={:6}  {}", tag, tag, hits, name);
    }

    let total_tagged_hits: u32 = counts.iter().map(|c| c.2).sum();
    println!(
        "\nTotal u16 hits across all {} tagged classes: {}",
        tagged_classes.len(),
        total_tagged_hits
    );
    println!(
        "Global/Latest has {} possible u16 positions; {:.2}% are tagged-class hits",
        global.len().saturating_sub(1),
        100.0 * total_tagged_hits as f64 / global.len().saturating_sub(1) as f64
    );

    // Baseline noise: how often does a random u16 in the same range appear?
    let expected_per_tag_if_random = (global.len() as f64 - 1.0) / (0x4000 as f64);
    println!(
        "Expected hits per tag if uniform random over [0, 0x4000): {:.1}",
        expected_per_tag_if_random
    );

    Ok(())
}
