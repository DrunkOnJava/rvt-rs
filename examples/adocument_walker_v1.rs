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

fn find_table_b_end(d: &[u8]) -> usize {
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
            if expect - 1 >= 5 {
                last_end = end + 32;
                i = end;
                continue;
            }
        }
        i += 1;
    }
    last_end
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
        // Guess: Pointer wire = u32 (target ref or NULL marker)
        Pointer { kind } => {
            if bytes.len() < 4 {
                return (0, format!("<short buffer for Pointer kind={kind}>"));
            }
            let v = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            (
                4,
                format!(
                    "[{:02x} {:02x} {:02x} {:02x}] Pointer{{kind:{kind}}} -> 0x{v:08x}",
                    bytes[0], bytes[1], bytes[2], bytes[3]
                ),
            )
        }
        // Guess: ElementId wire = 8 bytes (full u64 or [u32 tag][u32 id])
        ElementId | ElementIdRef { .. } => {
            if bytes.len() < 8 {
                return (0, format!("<short buffer for ElementId>"));
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
        // Guess: Container wire = [u32 count][count * ...] — but element
        // size depends on kind. Try [u32 count][count * 6-byte records].
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
            // Assume each element is 6 bytes `[u16 id][u32 sentinel_or_data]`.
            let elem_size = 6;
            let total = 4 + count * elem_size;
            if bytes.len() < total {
                return (
                    0,
                    format!(
                        "Container{{kind:{kind}, count={count}}} — need {total} bytes, have {}",
                        bytes.len()
                    ),
                );
            }
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
            (
                total,
                format!(
                    "Container{{kind:{kind}, count:{count}}} = [{}{}]",
                    summary.join(", "),
                    if count > 3 { ", ..." } else { "" }
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
    let cutoff = find_table_b_end(&d);
    println!("Post-Table-B entry point: 0x{cutoff:06x}");
    println!();

    // Try two attack shapes: (a) start reading fields DIRECTLY at
    // cutoff; (b) skip an 8-byte preamble first.
    for skip in [0usize, 8] {
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
