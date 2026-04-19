//! Q6.1 probe: find every u32-LE occurrence of a chosen class tag in
//! Global/Latest and compare the 24 bytes that follow each occurrence.
//! If instances share a layout, the byte columns will show repeating
//! patterns (same bytes at same offsets across many hits).
//!
//! Test target: HostObjAttr (tag 0x006b). Schema says 3 fields:
//!   m_symbolInfo    (Pointer)
//!   m_renderStyleId (ElementId)
//!   m_previewElemId (ElementId)
//! Expected instance size: ~20 bytes (4-byte tag + 3 fields × ~5-8 bytes).

use rvt::{compression, streams::GLOBAL_LATEST, RevitFile};

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: instance_scan <file.rfa> [tag-hex]");
    let tag_hex = std::env::args().nth(2).unwrap_or_else(|| "006b".into());
    let tag = u32::from_str_radix(&tag_hex, 16)?;

    let mut rf = RevitFile::open(&path)?;
    let raw = rf.read_stream(GLOBAL_LATEST)?;
    let d = compression::inflate_at(&raw, 8)?;
    println!("Global/Latest: {} bytes decompressed", d.len());
    println!("Target tag: 0x{tag:04x} (searching as u32 LE)");

    let target_bytes = tag.to_le_bytes();
    let mut positions: Vec<usize> = Vec::new();
    for i in 0..d.len().saturating_sub(4) {
        if d[i..i + 4] == target_bytes {
            positions.push(i);
        }
    }
    println!("u32 LE occurrences: {}", positions.len());
    if positions.is_empty() {
        return Ok(());
    }

    // Dump 32 bytes of context after the FIRST 30 occurrences
    let show = positions.len().min(30);
    println!("\nFirst {show} occurrences (hex of 24 bytes AFTER the tag):");
    println!("    offset    tag   |  +0           +4           +8           +12          +16          +20");
    for &pos in positions.iter().take(show) {
        print!("  0x{pos:06x}  {tag:04x}  |  ");
        let end = (pos + 4 + 24).min(d.len());
        for j in pos + 4..end {
            print!("{:02x} ", d[j]);
            if (j - pos - 3) % 4 == 0 {
                print!(" ");
            }
        }
        println!();
    }

    // Byte-column consistency: for each offset delta (0..24), what is
    // the mode (most common byte value) and how often does it appear?
    println!("\nByte-column consistency across all {} hits:", positions.len());
    println!("  offset  mode  count  freq   (all values that appear ≥5% of the time)");
    for delta in 0..24 {
        let mut freq: std::collections::HashMap<u8, u32> = std::collections::HashMap::new();
        let mut total = 0u32;
        for &pos in &positions {
            let idx = pos + 4 + delta;
            if idx < d.len() {
                *freq.entry(d[idx]).or_insert(0) += 1;
                total += 1;
            }
        }
        let mut entries: Vec<_> = freq.iter().collect();
        entries.sort_by(|a, b| b.1.cmp(a.1));
        let (mode_b, mode_c) = entries.first().map(|(b, c)| (**b, **c)).unwrap_or((0, 0));
        let mode_pct = 100.0 * mode_c as f64 / total.max(1) as f64;
        print!("  +{delta:2}      0x{mode_b:02x}   {mode_c:5}  {mode_pct:5.1}%   ");
        let mut seen_enough = false;
        for (b, c) in entries.iter().take(8) {
            let pct = 100.0 * **c as f64 / total.max(1) as f64;
            if pct >= 5.0 {
                print!("0x{:02x}×{}({:.0}%)  ", b, c, pct);
                seen_enough = true;
            }
        }
        if !seen_enough {
            print!("(scattered — no byte value ≥ 5%)");
        }
        println!();
    }

    Ok(())
}
