//! Deep variant of `unknown_bytes`: for every Unknown field_type, group by
//! the first 4 bytes (the "signature") and also dump the next 4 bytes so we
//! can see variable-length headers. For each signature emit up to 5 sample
//! `Class.field` names + the full body-byte histogram of bytes 4..8.

#![allow(clippy::type_complexity)]

use rvt::{RevitFile, compression, formats, streams::FORMATS_LATEST};
use std::collections::BTreeMap;

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: unknown_bytes_deep <file.rfa>");
    let mut rf = RevitFile::open(&path)?;
    let raw = rf.read_stream(FORMATS_LATEST)?;
    let d = compression::inflate_at(&raw, 0)?;
    let schema = formats::parse_schema(&d)?;

    // sig4 -> (count, sig8_histogram, sample fields)
    let mut hist: BTreeMap<Vec<u8>, (u32, BTreeMap<Vec<u8>, u32>, Vec<(String, String, usize)>)> =
        BTreeMap::new();
    let mut total_fields = 0u32;
    let mut total_unknown = 0u32;

    for c in &schema.classes {
        for f in &c.fields {
            total_fields += 1;
            if let Some(formats::FieldType::Unknown { bytes }) = &f.field_type {
                total_unknown += 1;
                let sig4: Vec<u8> = bytes.iter().take(4).copied().collect();
                let sig8: Vec<u8> = bytes.iter().take(8).copied().collect();
                let entry = hist.entry(sig4).or_insert((0, BTreeMap::new(), Vec::new()));
                entry.0 += 1;
                *entry.1.entry(sig8).or_insert(0) += 1;
                if entry.2.len() < 5 {
                    entry.2.push((c.name.clone(), f.name.clone(), bytes.len()));
                }
            }
        }
    }

    let pct = if total_fields > 0 {
        100.0 * total_unknown as f64 / total_fields as f64
    } else {
        0.0
    };
    println!("Schema total fields: {total_fields}  Unknown: {total_unknown}  ({pct:.2}%)");
    println!("Classification coverage: {:.2}%", 100.0 - pct);
    println!();

    let mut rows: Vec<_> = hist.iter().collect();
    rows.sort_by_key(|r| std::cmp::Reverse(r.1.0));

    for (sig, (count, sig8_hist, samples)) in rows.iter() {
        let sig_hex: Vec<String> = sig.iter().map(|b| format!("{b:02x}")).collect();
        println!("── sig4 = {}  ({} fields)", sig_hex.join(" "), count);

        let mut sub: Vec<_> = sig8_hist.iter().collect();
        sub.sort_by_key(|r| std::cmp::Reverse(*r.1));
        for (s8, n) in sub.iter().take(5) {
            let h: Vec<String> = s8.iter().map(|b| format!("{b:02x}")).collect();
            println!("    sig8 = {}  ×{}", h.join(" "), n);
        }
        for (c, f, len) in samples.iter() {
            println!("    example: {c}.{f}  (body len={len})");
        }
        println!();
    }
    Ok(())
}
