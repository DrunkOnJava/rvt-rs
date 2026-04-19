//! Phase 4c.2 / Q5 probe: dump the raw bytes immediately after every
//! field name in a tagged-class record. Goal: find the structure of
//! `type_encoding` and map its bytes to a typed value class.
//!
//! Strategy:
//! 1. Parse Formats/Latest with our existing schema walker.
//! 2. For each tagged class, re-walk its fields at the raw-byte level.
//! 3. For each field, record:
//!    - name
//!    - offset
//!    - 32 bytes of raw type_encoding (trimmed at the next field start)
//!
//! Output: a deterministic table we can diff across releases.

use rvt::{compression, streams::FORMATS_LATEST, RevitFile};

fn looks_like_class_name(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && bytes[0].is_ascii_uppercase()
        && bytes[1..].iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_')
}
fn looks_like_field_name(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && (bytes[0].is_ascii_alphanumeric() || bytes[0] == b'_')
        && bytes.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_')
}

struct FieldSite {
    class_name: String,
    class_tag: u16,
    parent: Option<String>,
    declared_count: u32,
    field_name: String,
    field_name_offset: usize,
    type_enc: Vec<u8>, // up to 32 bytes, trimmed at next field or class boundary
    cpp_type_guess: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: field_type_probe <file.rfa>");
    let mut rf = RevitFile::open(&path)?;
    let raw = rf.read_stream(FORMATS_LATEST)?;
    let d = compression::inflate_at(&raw, 0)?;
    let scan_limit = (64 * 1024).min(d.len());
    let data = &d[..scan_limit];

    let mut results: Vec<FieldSite> = Vec::new();

    // Repeat the class-record walker here but record field byte ranges.
    let mut i = 0;
    while i + 2 < data.len() {
        let nlen = u16::from_le_bytes([data[i], data[i + 1]]) as usize;
        if !(3..=60).contains(&nlen) || i + 2 + nlen + 2 > data.len() {
            i += 1;
            continue;
        }
        let name_bytes = &data[i + 2..i + 2 + nlen];
        if !looks_like_class_name(name_bytes) {
            i += 1;
            continue;
        }
        let class_name = std::str::from_utf8(name_bytes).unwrap().to_string();
        let after_name = i + 2 + nlen;
        let tag_raw = u16::from_le_bytes([data[after_name], data[after_name + 1]]);
        let is_tagged = tag_raw & 0x8000 != 0;
        if !is_tagged {
            i += 1;
            continue;
        }
        let tag = tag_raw & 0x7fff;
        // Walk past the tag + pad + parent + flag + field_count × 2
        let mut cursor = after_name + 2;
        // pad
        if cursor + 2 > data.len() {
            i += 1;
            continue;
        }
        let _pad = u16::from_le_bytes([data[cursor], data[cursor + 1]]);
        cursor += 2;
        // parent
        if cursor + 2 > data.len() {
            i += 1;
            continue;
        }
        let plen = u16::from_le_bytes([data[cursor], data[cursor + 1]]) as usize;
        if !(3..=40).contains(&plen) || cursor + 2 + plen + 10 > data.len() {
            i += 1;
            continue;
        }
        let p_bytes = &data[cursor + 2..cursor + 2 + plen];
        if !looks_like_class_name(p_bytes) {
            i += 1;
            continue;
        }
        let parent_name = std::str::from_utf8(p_bytes).unwrap().to_string();
        cursor += 2 + plen;
        // Check preamble
        let flag = u16::from_le_bytes([data[cursor], data[cursor + 1]]);
        let fc = u32::from_le_bytes([
            data[cursor + 2],
            data[cursor + 3],
            data[cursor + 4],
            data[cursor + 5],
        ]);
        let fc2 = u32::from_le_bytes([
            data[cursor + 6],
            data[cursor + 7],
            data[cursor + 8],
            data[cursor + 9],
        ]);
        if flag & 0x8000 != 0 || fc != fc2 || fc > 200 {
            i += 1;
            continue;
        }
        cursor += 10;

        // Now walk fc fields, capturing type_encoding bytes.
        let mut class_fields: Vec<FieldSite> = Vec::new();
        for _ in 0..fc {
            if cursor + 4 > data.len() {
                break;
            }
            let fname_len = u32::from_le_bytes([
                data[cursor],
                data[cursor + 1],
                data[cursor + 2],
                data[cursor + 3],
            ]) as usize;
            if !(1..=60).contains(&fname_len) || cursor + 4 + fname_len > data.len() {
                break;
            }
            let fname_bytes = &data[cursor + 4..cursor + 4 + fname_len];
            if !looks_like_field_name(fname_bytes) {
                break;
            }
            let fname = std::str::from_utf8(fname_bytes).unwrap().to_string();
            let name_off = cursor;
            cursor += 4 + fname_len;

            // type_encoding: capture bytes until the next plausible field name
            // start (u32 len in [1..60] + looks_like_field_name) or next class
            // candidate or end of buffer, capped at 32 bytes.
            let enc_start = cursor;
            let mut enc_end = enc_start;
            let cap = (enc_start + 32).min(data.len());
            while enc_end + 4 < cap {
                // Check for next field record
                let maybe_len = u32::from_le_bytes([
                    data[enc_end],
                    data[enc_end + 1],
                    data[enc_end + 2],
                    data[enc_end + 3],
                ]) as usize;
                if (1..=60).contains(&maybe_len)
                    && enc_end + 4 + maybe_len <= data.len()
                    && looks_like_field_name(&data[enc_end + 4..enc_end + 4 + maybe_len])
                {
                    break;
                }
                // Check for next class candidate (u16 len + class-name-like bytes)
                let maybe_u16_len =
                    u16::from_le_bytes([data[enc_end], data[enc_end + 1]]) as usize;
                if (3..=40).contains(&maybe_u16_len)
                    && enc_end + 2 + maybe_u16_len + 2 <= data.len()
                    && looks_like_class_name(
                        &data[enc_end + 2..enc_end + 2 + maybe_u16_len],
                    )
                    && u16::from_le_bytes([
                        data[enc_end + 2 + maybe_u16_len],
                        data[enc_end + 2 + maybe_u16_len + 1],
                    ]) & 0x8000
                        != 0
                {
                    break;
                }
                enc_end += 1;
            }
            let type_enc = data[enc_start..enc_end].to_vec();

            // Heuristic C++ type extraction: look inside type_enc for a
            // `[u32 len][ASCII]` block that looks like a C++ type.
            let mut cpp_type = None;
            let mut k = 0;
            while k + 4 < type_enc.len() {
                let tlen = u32::from_le_bytes([
                    type_enc[k],
                    type_enc[k + 1],
                    type_enc[k + 2],
                    type_enc[k + 3],
                ]) as usize;
                if (3..=120).contains(&tlen) && k + 4 + tlen <= type_enc.len() {
                    let body = &type_enc[k + 4..k + 4 + tlen];
                    if body.iter().all(|b| {
                        b.is_ascii_graphic() || *b == b' '
                    }) {
                        let s = std::str::from_utf8(body).unwrap_or_default();
                        if s.chars().any(|c| c.is_ascii_uppercase())
                            || s.contains("std::")
                            || s.contains("double")
                            || s.contains("int")
                        {
                            cpp_type = Some(s.to_string());
                            break;
                        }
                    }
                }
                k += 1;
            }

            class_fields.push(FieldSite {
                class_name: class_name.clone(),
                class_tag: tag,
                parent: Some(parent_name.clone()),
                declared_count: fc,
                field_name: fname,
                field_name_offset: name_off,
                type_enc,
                cpp_type_guess: cpp_type,
            });
            cursor = enc_end;
        }
        results.extend(class_fields);
        i = cursor.max(i + 1);
    }

    // Group by class, report every tagged class with ≥1 field.
    let mut seen_classes: Vec<(String, u16, Option<String>, u32)> = Vec::new();
    for f in &results {
        let key = (
            f.class_name.clone(),
            f.class_tag,
            f.parent.clone(),
            f.declared_count,
        );
        if !seen_classes.contains(&key) {
            seen_classes.push(key);
        }
    }
    println!(
        "Tagged classes with ≥1 decoded field: {}\n",
        seen_classes.len()
    );

    // First-byte histogram of type_encoding across every field.
    let mut histogram: std::collections::BTreeMap<u8, u32> = std::collections::BTreeMap::new();
    for f in &results {
        if let Some(&b0) = f.type_enc.first() {
            *histogram.entry(b0).or_insert(0) += 1;
        }
    }
    println!("type_encoding[0] histogram across all fields:");
    let total: u32 = histogram.values().sum();
    for (b, c) in &histogram {
        let pct = 100.0 * *c as f64 / total.max(1) as f64;
        println!("  0x{b:02x}: {c:4} fields ({pct:.1}%)");
    }
    println!();

    // Length histogram of type_encoding.
    let mut len_histogram: std::collections::BTreeMap<usize, u32> =
        std::collections::BTreeMap::new();
    for f in &results {
        *len_histogram.entry(f.type_enc.len()).or_insert(0) += 1;
    }
    println!("type_encoding length histogram:");
    for (len, c) in &len_histogram {
        let pct = 100.0 * *c as f64 / total.max(1) as f64;
        println!("  {len:3} bytes: {c:4} fields ({pct:.1}%)");
    }
    println!();

    // Dump the first 25 classes with fields.
    for (class_name, tag, parent, declared) in seen_classes.iter().take(25) {
        println!(
            "═══ {class_name} (tag=0x{tag:04x}, parent={parent:?}, declared={declared}) ═══"
        );
        for f in results.iter().filter(|f| f.class_name == *class_name) {
            print!("  '{}' [enc {}]: ", f.field_name, f.type_enc.len());
            for b in f.type_enc.iter().take(24) {
                print!("{:02x} ", b);
            }
            if f.type_enc.len() > 24 {
                print!("…");
            }
            if let Some(t) = &f.cpp_type_guess {
                print!(" | cpp={t:?}");
            }
            println!();
        }
        println!();
    }

    Ok(())
}
