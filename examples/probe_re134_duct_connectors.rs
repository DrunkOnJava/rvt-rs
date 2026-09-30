//! RE-134 (probe): the curve fields at a duct's or pipe's connector-manager
//! anchor (#96).
//!
//! RE-131 read a pipe's two ends from connector entries (`u64 element`,
//! `u32 index`, `u32 1`, `f64 x 3`). A duct or pipe is an `RbsCurve`, whose
//! first serialized field, `m_pConnectorManager`, is written as
//! `RbsCurveConnectorManager`'s tag, `0xFF`x4 and the next tag. The fields
//! that follow it in the schema (`m_dWidthOrDiameter`, `m_dHeight`, `m_vNormal`,
//! `m_dOffsetStart`, `m_dOffsetEnd`, `m_idType`, ...) are the fact under test:
//! RE1 Mechanical (Revit 2025, MIT) holds 25 rectangular ducts and 6 pipes,
//! RE1 Plumbing 63 pipes, and Revit's own IFC gives each one's type and size,
//! so `tools/re/curve_fields_vs_ifc.py` can check what this prints.
//!
//! It prints the schema's duct, connector and `Rbs` classes with their
//! declared fields, and every anchor with the bytes around it. For every duct
//! and pipe record it prints the record's fields, its reference list and its
//! first bytes, then each entry whose point lies in the record's box with the
//! bytes before it and after it, as hex and as `f64`. For every type a
//! reference list names it prints the header before the name and the bytes
//! after it.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re134_duct_connectors -- MODEL.rvt > mep.jsonl

use rvt::RevitFile;
use rvt::partition_element_records as per;
use rvt::partition_names::{MEP_CURVE_TYPE_ID_REPEAT, MEP_CURVE_TYPE_NAME_OFFSET};
use std::collections::BTreeSet;

/// How far outside a record's box an entry's point may fall, feet.
const TOLERANCE: f64 = 1e-3;
/// Most entries printed per element and stream.
const MAX_ENTRIES: usize = 16;
/// Bytes of the record printed.
const RECORD_BYTES: usize = 384;
/// Bytes printed before an entry's `u64 element`.
const BEFORE: usize = 48;
/// Bytes printed from the end of an entry's point.
const AFTER: usize = 224;
/// Bytes printed before a type's ElementId and after its name.
const TYPE_AROUND: usize = 192;
/// Longest type name read, UTF-16 units.
const NAME_MAX_UNITS: u32 = 256;
/// Bytes printed from each anchor on, and before it.
const ANCHOR_BYTES: usize = 1700;
const ANCHOR_BEFORE: usize = 96;
/// Most anchors printed per stream.
const MAX_ANCHORS: usize = 4_000;
/// Schema classes whose name holds one of these are printed.
const SCHEMA_WORDS: &[&str] = &["Duct", "Rbs", "Connector", "Pipe", "Mep", "MEP"];

fn hex(bytes: &[u8]) -> String {
    let digits: Vec<String> = bytes.iter().map(|b| format!("{b:02x}")).collect();
    digits.join("")
}

fn read_u32(buf: &[u8], at: usize) -> Option<u32> {
    buf.get(at..at + 4)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
}

fn read_u64(buf: &[u8], at: usize) -> Option<u64> {
    buf.get(at..at + 8)
        .map(|s| u64::from_le_bytes(s.try_into().expect("8 bytes")))
}

fn read_f64(buf: &[u8], at: usize) -> Option<f64> {
    buf.get(at..at + 8)
        .map(|s| f64::from_le_bytes(s.try_into().expect("8 bytes")))
}

/// A JSON string literal.
fn quote(value: &str) -> String {
    let mut out = String::from("\"");
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A JSON value for an optional string.
fn text(value: Option<&str>) -> String {
    value.map_or_else(|| "null".to_string(), quote)
}

/// The `count` `f64` from `at` on as a JSON array, `null` where JSON has no
/// number for the value.
fn floats(buf: &[u8], at: usize, count: usize) -> String {
    let items: Vec<String> = (0..count)
        .map(|index| match read_f64(buf, at + 8 * index) {
            Some(v) if v.is_finite() => format!("{v:?}"),
            _ => "null".to_string(),
        })
        .collect();
    format!("[{}]", items.join(","))
}

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    println!("{{\"revit\":{version}}}");

    let table = rf.schema()?;
    for class in &table.classes {
        if !SCHEMA_WORDS.iter().any(|word| class.name.contains(word)) {
            continue;
        }
        let fields: Vec<String> = class
            .fields
            .iter()
            .map(|field| {
                format!(
                    "[{},{}]",
                    quote(&field.name),
                    text(field.cpp_type.as_deref())
                )
            })
            .collect();
        println!(
            "{{\"schema_class\":{},\"tag\":{},\"parent\":{},\"fields\":[{}]}}",
            quote(&class.name),
            class
                .tag
                .map_or_else(|| "null".to_string(), |t| t.to_string()),
            text(class.parent.as_deref()),
            fields.join(",")
        );
    }
    let classes = rf.schema_classes()?;
    for name in ["RbsCurve", "RbsDuctCurve", "RbsPipeCurve"] {
        for class in classes.classes.iter().filter(|class| class.name == name) {
            let base = class
                .base
                .and_then(|tag| classes.by_tag(tag))
                .map(|base| base.name.as_str());
            println!(
                "{{\"curve_class\":{},\"tag\":{},\"base\":{}}}",
                quote(name),
                class.tag,
                text(base)
            );
        }
    }

    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let records = per::scan_category_records_multi(
        &mut rf,
        version,
        &[per::OST_DUCT_CURVES, per::OST_PIPE_CURVES],
        &declared,
    )?;
    let streams = rf.partition_stream_names();
    let mut types: BTreeSet<u32> = BTreeSet::new();

    // The curve object: `m_pConnectorManager`'s two tags, and what follows.
    let manager = classes
        .classes
        .iter()
        .find(|class| class.name == "RbsCurveConnectorManager")
        .map(|class| class.tag);
    if let Some(tag) = manager {
        let mut anchor = tag.to_le_bytes().to_vec();
        anchor.extend_from_slice(&[0xff; 4]);
        anchor.extend_from_slice(&(tag + 1).to_le_bytes());
        for stream in &streams {
            let Ok(inflated) = rf.inflated_partition(stream) else {
                continue;
            };
            let buf = inflated.bytes();
            for at in memchr::memmem::find_iter(buf, anchor.as_slice()).take(MAX_ANCHORS) {
                println!(
                    "{{\"anchor\":true,\"stream\":{},\"at\":{at},\"before\":\"{}\",\"tail\":\"{}\"}}",
                    quote(stream),
                    hex(&buf[at.saturating_sub(ANCHOR_BEFORE)..at]),
                    hex(&buf[at..(at + ANCHOR_BYTES).min(buf.len())])
                );
            }
        }
    }

    for record in &records {
        let id = record.element_id;
        let kind = if record.builtin_category == per::OST_DUCT_CURVES {
            "duct"
        } else {
            "pipe"
        };
        let class = classes
            .by_tag(record.class_tag)
            .map(|class| class.name.as_str());
        let b = record.bbox_feet;
        if let Ok(inflated) = rf.inflated_partition(&record.stream) {
            let own = inflated.bytes();
            let from = record.offset.min(own.len());
            let to = (record.offset + RECORD_BYTES).min(own.len());
            println!(
                "{{\"mep\":{id},\"kind\":\"{kind}\",\"class\":{},\"stream\":{},\"offset\":{},\"class_tag\":{},\"flags\":{},\"bbox\":{b:?},\"references\":{:?},\"record\":\"{}\"}}",
                text(class),
                quote(&record.stream),
                record.offset,
                record.class_tag,
                record.flags,
                record.references,
                hex(&own[from..to])
            );
        }
        for &reference in record.references.iter().skip(1) {
            if let Ok(reference) = u32::try_from(reference) {
                types.insert(reference);
            }
        }
        for stream in &streams {
            let Ok(inflated) = rf.inflated_partition(stream) else {
                continue;
            };
            let buf = inflated.bytes();
            let mut printed = 0;
            let last = buf.len().saturating_sub(24);
            for at in 16..last {
                if printed >= MAX_ENTRIES {
                    break;
                }
                let Some(x) = read_f64(buf, at) else {
                    break;
                };
                if !(x >= b[0] - TOLERANCE && x <= b[3] + TOLERANCE) {
                    continue;
                }
                if read_u32(buf, at - 4) != Some(1) {
                    continue;
                }
                let (Some(y), Some(z), Some(index), Some(key)) = (
                    read_f64(buf, at + 8),
                    read_f64(buf, at + 16),
                    read_u32(buf, at - 8),
                    read_u64(buf, at - 16),
                ) else {
                    continue;
                };
                if index > 255
                    || !(y >= b[1] - TOLERANCE
                        && y <= b[4] + TOLERANCE
                        && z >= b[2] - TOLERANCE
                        && z <= b[5] + TOLERANCE)
                {
                    continue;
                }
                printed += 1;
                let after_from = (at + 24).min(buf.len());
                let after_to = (at + 24 + AFTER).min(buf.len());
                println!(
                    "{{\"entry\":{id},\"kind\":\"{kind}\",\"stream\":{},\"at\":{at},\"key\":{key},\"index\":{index},\"point\":[{x:?},{y:?},{z:?}],\"before\":\"{}\",\"after\":\"{}\",\"after_f64\":{}}}",
                    quote(stream),
                    hex(&buf[(at - 16).saturating_sub(BEFORE)..at - 16]),
                    hex(&buf[after_from..after_to]),
                    floats(buf, after_from, AFTER / 8)
                );
            }
        }
    }

    for &type_id in &types {
        let needle = u64::from(type_id).to_le_bytes();
        for stream in &streams {
            let Ok(inflated) = rf.inflated_partition(stream) else {
                continue;
            };
            let buf = inflated.bytes();
            for at in memchr::memmem::find_iter(buf, &needle) {
                if read_u64(buf, at + MEP_CURVE_TYPE_ID_REPEAT) != Some(u64::from(type_id)) {
                    continue;
                }
                let Some(units) = read_u32(buf, at + MEP_CURVE_TYPE_NAME_OFFSET)
                    .filter(|units| (1..=NAME_MAX_UNITS).contains(units))
                else {
                    continue;
                };
                let name_at = at + MEP_CURVE_TYPE_NAME_OFFSET + 4;
                let name_end = name_at + units as usize * 2;
                let Some(name_bytes) = buf.get(name_at..name_end) else {
                    continue;
                };
                let code_units: Vec<u16> = name_bytes
                    .chunks_exact(2)
                    .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                    .collect();
                let Ok(name) = String::from_utf16(&code_units) else {
                    continue;
                };
                if name.trim().is_empty() || name.chars().any(char::is_control) {
                    continue;
                }
                println!(
                    "{{\"type\":{type_id},\"name\":{},\"stream\":{},\"at\":{at},\"before\":\"{}\",\"header\":\"{}\",\"after\":\"{}\"}}",
                    quote(&name),
                    quote(stream),
                    hex(&buf[at.saturating_sub(TYPE_AROUND)..at]),
                    hex(&buf[at..name_at - 4]),
                    hex(&buf[name_end..(name_end + TYPE_AROUND).min(buf.len())])
                );
            }
        }
    }
    eprintln!(
        "Revit {version}: {} ducts and pipes, {} types",
        records.len(),
        types.len()
    );
    Ok(())
}
