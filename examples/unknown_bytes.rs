//! Q5.1 probe: dump the first-byte histogram for fields that currently
//! classify as FieldType::Unknown, and alongside it a sample field name
//! per byte so we can reason about which C++ type each discriminator
//! represents.

use rvt::{compression, formats, streams::FORMATS_LATEST, RevitFile};

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: unknown_bytes <file.rfa>");
    let mut rf = RevitFile::open(&path)?;
    let raw = rf.read_stream(FORMATS_LATEST)?;
    let d = compression::inflate_at(&raw, 0)?;
    let schema = formats::parse_schema(&d)?;

    let mut hist: std::collections::BTreeMap<Vec<u8>, (u32, Vec<(String, String)>)> =
        std::collections::BTreeMap::new();
    for c in &schema.classes {
        for f in &c.fields {
            if let Some(ft) = &f.field_type {
                if let formats::FieldType::Unknown { bytes } = ft {
                    // Take first 4 bytes as a "signature"
                    let sig: Vec<u8> = bytes.iter().take(4).copied().collect();
                    let entry = hist.entry(sig).or_insert((0, Vec::new()));
                    entry.0 += 1;
                    if entry.1.len() < 3 {
                        entry.1.push((c.name.clone(), f.name.clone()));
                    }
                }
            }
        }
    }

    let mut rows: Vec<_> = hist.iter().collect();
    rows.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));

    println!(
        "Unknown field_type first-4-byte histogram (top 30 patterns, {} distinct):",
        rows.len()
    );
    println!("  signature               count   sample fields");
    for (sig, (count, samples)) in rows.iter().take(30) {
        let sig_hex: Vec<String> = sig.iter().map(|b| format!("{b:02x}")).collect();
        let joined = sig_hex.join(" ");
        let padded = format!("{joined:<23}");
        let sample_str: Vec<String> = samples
            .iter()
            .map(|(c, f)| format!("{c}.{f}"))
            .collect();
        println!("  {padded} {count:5}   {}", sample_str.join(", "));
    }

    Ok(())
}
