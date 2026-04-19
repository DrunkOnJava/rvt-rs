//! Q4 probe: collect the `flag` word (the u16 between parent-class and
//! field-count) for every tagged class across the 11-version corpus.
//! Look for a distinguishing property that explains why the value
//! differs between classes.

use rvt::{compression, streams::FORMATS_LATEST, RevitFile};
use std::path::PathBuf;

fn looks_like_class_name(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && bytes[0].is_ascii_uppercase()
        && bytes[1..].iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_')
}

fn parse_flag_words(d: &[u8]) -> Vec<(String, u16, u16)> {
    // Returns (class_name, tag, flag) for every tagged class that has a
    // recognisable parent-class + preamble.
    let scan_limit = (64 * 1024).min(d.len());
    let data = &d[..scan_limit];
    let mut out = Vec::new();
    let mut i = 0;
    while i + 2 < data.len() {
        let nlen = u16::from_le_bytes([data[i], data[i + 1]]) as usize;
        if !(3..=60).contains(&nlen) || i + 2 + nlen + 2 > data.len() {
            i += 1;
            continue;
        }
        let name = &data[i + 2..i + 2 + nlen];
        if !looks_like_class_name(name) {
            i += 1;
            continue;
        }
        let after_name = i + 2 + nlen;
        let tag_raw = u16::from_le_bytes([data[after_name], data[after_name + 1]]);
        if tag_raw & 0x8000 == 0 {
            i += 1;
            continue;
        }
        let tag = tag_raw & 0x7fff;
        // pad + parent
        let after_tag = after_name + 2;
        if after_tag + 4 > data.len() {
            i += 1;
            continue;
        }
        let pad = u16::from_le_bytes([data[after_tag], data[after_tag + 1]]);
        let plen = u16::from_le_bytes([data[after_tag + 2], data[after_tag + 3]]) as usize;
        if pad != 0 || !(3..=40).contains(&plen) {
            i += 1;
            continue;
        }
        let parent_start = after_tag + 4;
        if parent_start + plen + 10 > data.len() {
            i += 1;
            continue;
        }
        let p_bytes = &data[parent_start..parent_start + plen];
        if !looks_like_class_name(p_bytes) {
            i += 1;
            continue;
        }
        let preamble_at = parent_start + plen;
        let flag = u16::from_le_bytes([data[preamble_at], data[preamble_at + 1]]);
        let fc = u32::from_le_bytes([
            data[preamble_at + 2],
            data[preamble_at + 3],
            data[preamble_at + 4],
            data[preamble_at + 5],
        ]);
        let fc2 = u32::from_le_bytes([
            data[preamble_at + 6],
            data[preamble_at + 7],
            data[preamble_at + 8],
            data[preamble_at + 9],
        ]);
        if flag & 0x8000 != 0 || fc != fc2 || fc > 200 {
            i += 1;
            continue;
        }
        let name_str = std::str::from_utf8(name).unwrap().to_string();
        out.push((name_str, tag, flag));
        i += 2 + nlen;
    }
    out
}

fn main() -> anyhow::Result<()> {
    let sample_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "samples/_phiag/examples/Autodesk".into());

    let mut all: Vec<(String, u16, u16, String)> = Vec::new(); // (class, tag, flag, release)
    for year in [2016, 2020, 2024, 2026] {
        for filename in [
            format!("racbasicsamplefamily-{year}.rfa"),
            format!("rac_basic_sample_family-{year}.rfa"),
        ] {
            let path = PathBuf::from(&sample_dir).join(&filename);
            if !path.exists() { continue; }
            let mut rf = RevitFile::open(&path)?;
            let raw = rf.read_stream(FORMATS_LATEST)?;
            let d = compression::inflate_at(&raw, 0)?;
            for (name, tag, flag) in parse_flag_words(&d) {
                all.push((name, tag, flag, year.to_string()));
            }
            break;
        }
    }

    // Histogram of flag values
    let mut freq: std::collections::BTreeMap<u16, u32> = std::collections::BTreeMap::new();
    for (_, _, flag, _) in &all {
        *freq.entry(*flag).or_insert(0) += 1;
    }
    let total: u32 = freq.values().sum();
    println!("Flag-word distribution across {} tagged-class records:", total);
    for (flag, c) in &freq {
        let pct = 100.0 * *c as f64 / total.max(1) as f64;
        println!("  0x{flag:04x} ({flag:5}): {c:4}  ({pct:.1}%)");
    }
    println!();

    // For each flag value, sample which classes use it
    println!("Sample classes per flag value:");
    for flag in freq.keys() {
        let samples: Vec<String> = all
            .iter()
            .filter(|r| r.2 == *flag)
            .take(5)
            .map(|r| r.0.clone())
            .collect();
        println!("  0x{flag:04x}: {samples:?}");
    }

    Ok(())
}
