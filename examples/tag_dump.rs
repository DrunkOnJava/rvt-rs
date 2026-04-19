//! Dump (name, offset, next_u16, next_u32) for every class-name record found
//! in Formats/Latest. Used to discover the tag encoding by statistical
//! analysis across all ~400 classes in a 2024 RFA.
//!
//! Key finding (see `docs/rvt-moat-break-reconnaissance.md` §Phase D link
//! proof): the u16 immediately after a class name is the class tag, with
//! the 0x8000 bit set to mark "this is a definition, not a reference." 79
//! of 398 candidates carry the flag in the 2024 family file.
use rvt::{RevitFile, compression, streams::FORMATS_LATEST};
use std::collections::HashMap;

fn looks_like_class_name(bytes: &[u8]) -> bool {
    if bytes.is_empty() || !bytes[0].is_ascii_uppercase() {
        return false;
    }
    bytes[1..]
        .iter()
        .all(|c| c.is_ascii_alphanumeric() || *c == b'_')
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("path");
    let mut rf = RevitFile::open(&path)?;
    let raw = rf.read_stream(FORMATS_LATEST)?;
    let decomp = compression::inflate_at(&raw, 0)?;
    let scan_limit = 64 * 1024usize.min(decomp.len());
    let data = &decomp[..scan_limit.min(decomp.len())];

    let mut tag_frequency: HashMap<u16, u32> = HashMap::new();
    let mut classes = Vec::new();
    let mut i = 0;
    while i + 2 < data.len() {
        let len = u16::from_le_bytes([data[i], data[i + 1]]) as usize;
        if !(3..=60).contains(&len) {
            i += 1;
            continue;
        }
        if i + 2 + len + 2 > data.len() {
            i += 1;
            continue;
        }
        let name_bytes = &data[i + 2..i + 2 + len];
        if !looks_like_class_name(name_bytes) {
            i += 1;
            continue;
        }
        let name = std::str::from_utf8(name_bytes).unwrap().to_string();
        let after_name = i + 2 + len;
        let next_u16 = u16::from_le_bytes([data[after_name], data[after_name + 1]]);
        let next_u32 = if after_name + 6 <= data.len() {
            u32::from_le_bytes([
                data[after_name + 2],
                data[after_name + 3],
                data[after_name + 4],
                data[after_name + 5],
            ])
        } else {
            0
        };
        classes.push((name.clone(), i, next_u16, next_u32));
        *tag_frequency.entry(next_u16).or_insert(0) += 1;
        i += 2 + len; // skip past name
    }

    println!("Total class candidates: {}", classes.len());
    println!("\nDistribution of u16 immediately after class name:");
    let mut freq: Vec<_> = tag_frequency.iter().collect();
    freq.sort_by_key(|e| std::cmp::Reverse(*e.1));
    for (val, count) in freq.iter().take(20) {
        let flag = if **val & 0x8000 != 0 {
            " [0x8000 SET]"
        } else {
            ""
        };
        println!(
            "  u16 0x{:04x} ({:5}): {} occurrences{}",
            **val, **val, count, flag
        );
    }

    let flagged = classes.iter().filter(|c| c.2 & 0x8000 != 0).count();
    let zero = classes.iter().filter(|c| c.2 == 0).count();
    println!("\nSummary:");
    println!("  Total class candidates: {}", classes.len());
    println!(
        "  With 0x8000 flag SET:  {} ({:.1}%)",
        flagged,
        100.0 * flagged as f64 / classes.len() as f64
    );
    println!(
        "  With u16=0x0000:       {} ({:.1}%)",
        zero,
        100.0 * zero as f64 / classes.len() as f64
    );

    println!("\n15 sample entries (name, offset, u16_after, u32_after_that):");
    for (name, off, u16v, u32v) in classes.iter().take(15) {
        let flag = if u16v & 0x8000 != 0 { " [F]" } else { "" };
        println!(
            "  {name:<25}  @0x{off:05x}  u16=0x{:04x}{flag}  u32=0x{:08x}",
            u16v, u32v
        );
    }

    // Specifically look at flagged-only records
    println!("\n15 sample entries WITH 0x8000 flag (class tags with actual IDs):");
    for (name, off, u16v, u32v) in classes.iter().filter(|c| c.2 & 0x8000 != 0).take(15) {
        println!(
            "  {name:<25}  @0x{off:05x}  tag=0x{:04x}  u32_next=0x{:08x}",
            u16v & 0x7fff,
            u32v
        );
    }

    Ok(())
}
