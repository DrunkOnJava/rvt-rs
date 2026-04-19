//! Q6.5 verification probe: first-pass ADocument walker. Starts at the
//! post-Table-B boundary (dynamically located via `find_table_b_end`)
//! and attempts to read ADocument's 13 schema-declared fields in
//! order, interpreting bytes per each field's `FieldType`.
//!
//! The goal is NOT to be a finished walker. The goal is to put the
//! §Q6.5-B hypothesis to a concrete test: does the post-Table-B region
//! actually parse as ADocument's schema? Partial success — where the
//! walker runs for N fields before hitting a byte-shape it can't
//! interpret — still tightens understanding. Total failure also
//! teaches us something: either the entry point is wrong, or the
//! field-framing assumption is wrong.
//!
//! No library changes. This is a probe, not the walker module.

#![allow(clippy::needless_range_loop, clippy::type_complexity)]

use rvt::{RevitFile, compression, formats, streams};

/// Locate ADocument's instance start. Two strategies, in order:
///   (1) Scan for the ADocument signature — 8 zero bytes followed by a
///       small u32 count ≤ 100 and a plausible mask. This is the
///       strongest cross-version signal IF it's unique.
///   (2) If (1) finds multiple candidates, prefer the one AFTER the
///       end of the longest sequential-id run (Table A or Table B).
///
/// Today (2026-04-19) this works reliably on 2024 only; for other
/// releases, the candidate search finds additional false positives
/// inside the history block. Cross-version detection is documented
/// in §Q6.5-D of the recon report as the next open sub-task.
fn find_adocument_start(d: &[u8]) -> usize {
    // Fallback: reuse the old sequential-id-table detector — for 2024
    // it returns the right offset (0x0f67).
    let mut last_table_end = 0usize;
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
            if expect >= 6 {
                last_table_end = end + 32;
                i = end;
                continue;
            }
        }
        i += 1;
    }
    // Search for the 8-zero signature AFTER the last table's end.
    let min_start = last_table_end.max(0x200);
    let mut k = min_start;
    while k + 16 <= d.len() {
        if d[k..k + 8].iter().all(|&b| b == 0) {
            let next_u32 = u32::from_le_bytes([d[k + 8], d[k + 9], d[k + 10], d[k + 11]]);
            let next_next = u32::from_le_bytes([d[k + 12], d[k + 13], d[k + 14], d[k + 15]]);
            if (1..=100).contains(&next_u32) && (next_next == 0xffffffff || next_next <= 0x10000) {
                return k;
            }
        }
        k += 1;
    }
    last_table_end
}

/// Attempt to read one field. Returns the number of bytes consumed
/// plus a human-readable interpretation. This is deliberately
/// best-effort — for each FieldType we use the simplest plausible
/// wire encoding (Pointer = 4 bytes u32, ElementId = 8 bytes, etc.).
fn read_field(ft: &formats::FieldType, bytes: &[u8]) -> (usize, String) {
    use formats::FieldType::*;
    match ft {
        Primitive { kind, size } => {
            let s = *size as usize;
            if bytes.len() < s {
                return (0, format!("<short buffer: need {s}, have {}>", bytes.len()));
            }
            let hex: ::std::string::String =
                bytes[..s].iter().map(|b| format!("{b:02x} ")).collect();
            let decoded = match (*kind, s) {
                (0x01, 1) => format!("bool({})", bytes[0] != 0),
                (0x02, 2) => format!("u16({})", u16::from_le_bytes([bytes[0], bytes[1]])),
                (0x04 | 0x05, 4) => format!(
                    "u32({})",
                    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
                ),
                (0x06, 4) => format!(
                    "f32({})",
                    f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
                ),
                (0x07, 8) => {
                    let b = [
                        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                        bytes[7],
                    ];
                    format!("f64({})", f64::from_le_bytes(b))
                }
                (0x0b, 8) => {
                    let b = [
                        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                        bytes[7],
                    ];
                    format!("u64({})", u64::from_le_bytes(b))
                }
                _ => format!("primitive(kind=0x{kind:02x}, size={s})"),
            };
            (s, format!("[{hex:<24}] {decoded}"))
        }
        // Refined guess (v2): Pointer wire = 8 bytes. Motivation: 8-byte
        // preamble at ADocument entry + subsequent `0c 00 00 00 ff ff ff ff`
        // pair reads cleanly as two 8-byte words where the second is
        // [u32 count=12][u32 metadata] — consistent with Pointer-then-
        // Container sequence.
        Pointer { kind } => {
            if bytes.len() < 8 {
                return (0, format!("<short buffer for Pointer kind={kind}>"));
            }
            let v = u64::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]);
            let hex: ::std::string::String =
                bytes[..8].iter().map(|b| format!("{b:02x} ")).collect();
            (8, format!("[{hex}] Pointer{{kind:{kind}}} -> 0x{v:016x}"))
        }
        // Guess: ElementId wire = 8 bytes (full u64 or [u32 tag][u32 id])
        ElementId | ElementIdRef { .. } => {
            if bytes.len() < 8 {
                return (0, "<short buffer for ElementId>".into());
            }
            let tag = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            let id = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
            let hex: ::std::string::String =
                bytes[..8].iter().map(|b| format!("{b:02x} ")).collect();
            (
                8,
                format!("[{hex}] ElementId{{tag:0x{tag:08x}, id:0x{id:08x}}}"),
            )
        }
        // Refined guess (v4): Container wire = TWO parallel columns of
        // [u32 count][count × 6B records]. Observed in the 2024 sample:
        // m_appInfoArr's data at 0x0f6f is
        //     [u32 count=12][12×[u16 0x0bc8][u32 ff]][u32 count=12][12×[u16 0x0bc7][u32 ff]]
        // That's 2 × (4 + 72) = 152 bytes, which when consumed lands
        // field 2 (m_oContentTable) at 0x1007 where a pair of NULL
        // Pointers (0x0000000040200000-ish bytes) appear, consistent
        // with Pointer wire = 8 bytes, null-first-u32.
        // Hypothesis: every reference-container (kind=0x0e) serializes
        // as a 2-column [id-array][mask-array] table.
        Container { kind, .. } => {
            if bytes.len() < 4 {
                return (0, format!("<short buffer for Container kind={kind}>"));
            }
            let count = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
            if count > 1000 {
                return (
                    0,
                    format!(
                        "[{:02x} {:02x} {:02x} {:02x}] Container{{kind:{kind}, count={count}}} — count looks unreasonable, abort",
                        bytes[0], bytes[1], bytes[2], bytes[3]
                    ),
                );
            }
            let elem_size = 6;
            // Two-column layout: header + col1 + header + col2
            let col_bytes = 4 + count * elem_size;
            let total = 2 * col_bytes;
            if bytes.len() < total {
                return (
                    0,
                    format!(
                        "Container{{kind:{kind}, count={count}}} — need {total} bytes for 2-column layout, have {}",
                        bytes.len()
                    ),
                );
            }
            // Verify second column starts with the same count marker
            // (supports the hypothesis; if mismatched, fall back to
            // single-column 4+6*count bytes and warn).
            let col2_count = u32::from_le_bytes([
                bytes[col_bytes],
                bytes[col_bytes + 1],
                bytes[col_bytes + 2],
                bytes[col_bytes + 3],
            ]) as usize;
            if col2_count != count {
                // Single-column fallback.
                let fallback_total = col_bytes;
                let mut summary = Vec::new();
                for k in 0..count.min(3) {
                    let base = 4 + k * elem_size;
                    let id = u16::from_le_bytes([bytes[base], bytes[base + 1]]);
                    let mask = u32::from_le_bytes([
                        bytes[base + 2],
                        bytes[base + 3],
                        bytes[base + 4],
                        bytes[base + 5],
                    ]);
                    summary.push(format!("[0x{id:04x}, 0x{mask:08x}]"));
                }
                return (
                    fallback_total,
                    format!(
                        "Container{{kind:{kind}, count:{count}, 1-col}} = [{}{}] ({fallback_total}B, no 2nd col)",
                        summary.join(", "),
                        if count > 3 { ", ..." } else { "" }
                    ),
                );
            }
            let mut col1 = Vec::new();
            let mut col2 = Vec::new();
            for k in 0..count.min(3) {
                let base = 4 + k * elem_size;
                let id1 = u16::from_le_bytes([bytes[base], bytes[base + 1]]);
                let base2 = col_bytes + 4 + k * elem_size;
                let id2 = u16::from_le_bytes([bytes[base2], bytes[base2 + 1]]);
                col1.push(format!("0x{id1:04x}"));
                col2.push(format!("0x{id2:04x}"));
            }
            (
                total,
                format!(
                    "Container{{kind:{kind}, count:{count}, 2-col}} col1=[{}] col2=[{}] ({total}B)",
                    col1.join(","),
                    col2.join(","),
                ),
            )
        }
        Vector { kind, .. } => (
            0,
            format!("Vector{{kind:{kind}}} — decoding not implemented in v1 walker"),
        ),
        Guid => {
            if bytes.len() < 16 {
                return (0, "<short buffer for Guid>".into());
            }
            let hex: ::std::string::String =
                bytes[..16].iter().map(|b| format!("{b:02x}")).collect();
            (16, format!("Guid({hex})"))
        }
        String => {
            // String encoding unknown — speculate [u32 len][len * 2 UTF-16 code units]
            if bytes.len() < 4 {
                return (0, "<short for String>".into());
            }
            let len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
            if len > 10_000 {
                return (
                    0,
                    format!(
                        "String — len prefix {len} looks unreasonable (expected UTF-16 code units)"
                    ),
                );
            }
            let byte_len = len * 2;
            if bytes.len() < 4 + byte_len {
                return (
                    0,
                    format!(
                        "String len={len} — need {byte_len}+4 bytes, have {}",
                        bytes.len()
                    ),
                );
            }
            let mut s = ::std::string::String::new();
            for k in 0..len {
                let c = u16::from_le_bytes([bytes[4 + 2 * k], bytes[5 + 2 * k]]);
                if let Some(ch) = char::from_u32(c as u32) {
                    s.push(ch);
                }
            }
            (4 + byte_len, format!("String(len={len}) = {s:?}"))
        }
        Unknown { .. } => (0, "Unknown FieldType — cannot walk".into()),
    }
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: adocument_walker_v1 <file>");
    let mut rf = RevitFile::open(&path)?;

    let formats_raw = rf.read_stream(streams::FORMATS_LATEST)?;
    let formats_d = compression::inflate_at(&formats_raw, 0)?;
    let schema = formats::parse_schema(&formats_d)?;
    let adoc = schema
        .classes
        .iter()
        .find(|c| c.name == "ADocument")
        .ok_or_else(|| anyhow::anyhow!("ADocument not in schema"))?;
    println!("ADocument schema: {} fields declared", adoc.fields.len());

    let raw_gl = rf.read_stream(streams::GLOBAL_LATEST)?;
    let d = compression::inflate_at(&raw_gl, 8)?;
    let cutoff = find_adocument_start(&d);
    if cutoff == 0 {
        println!("ADocument-start signature not found.");
        return Ok(());
    }
    println!("ADocument entry point: 0x{cutoff:06x}");
    println!();

    // Single attempt now that find_adocument_start lands exactly on
    // m_elemTable (the 8-zero signature). No preamble to skip.
    for skip in [0usize] {
        let start = cutoff + skip;
        if start >= d.len() {
            continue;
        }
        println!("=== Attempt: skip preamble = {skip} bytes (start=0x{start:06x}) ===");
        let mut offset = start;
        let mut fields_read = 0;
        for (idx, field) in adoc.fields.iter().enumerate() {
            let Some(ft) = &field.field_type else {
                println!("  #{idx:2} {name} :: <no FieldType>", name = field.name);
                break;
            };
            let slice = &d[offset..];
            let (n, interp) = read_field(ft, slice);
            println!(
                "  #{idx:2} {name:<32}  (offset 0x{offset:06x}, +{n}B)  {interp}",
                name = field.name
            );
            if n == 0 {
                println!("    ^-- walker aborted on this field");
                break;
            }
            offset += n;
            fields_read += 1;
            if offset > d.len() {
                println!("    (ran off end of stream)");
                break;
            }
        }
        println!(
            "  fields successfully read: {fields_read} / {}",
            adoc.fields.len()
        );
        println!();
    }

    Ok(())
}
