//! RE-354 (#1185, W7): where the texts and the number Revit's export of the
//! BIM4LCA office writes on its door type objects are in the file.
//!
//! FACT under test: Revit's export writes `Pset_DoorCommon.FireRating`
//! (`EI30`, `EI60`, `-`) on six door type objects and `ThermalTransmittance`
//! (3.7021) on a seventh, and none of the saved parameters of those types
//! (RE-207, `probe_re354_door_type_parameters`) holds any of them. This lists
//! every `EI30`, `EI60` and `EI90` text and every 3.7021 of the partitions
//! with the bytes around it and the value block it lies in, and the
//! printable texts of the seven types' own value blocks.
//!
//! Output:
//!   text <stream> <offset> <text> block <owner> +<distance> prev <hex 16 bytes> next <hex 24 bytes>
//!   real <stream> <offset> 3.7021 block <owner> +<distance> prev <hex 16 bytes> next <hex 24 bytes>
//!   block <stream> <owner> at <offset> len <bytes>
//!   str <stream> <owner> +<offset in block> prev_i64 <id> <text>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re354_door_type_blocks -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::partition_type_parameters::VALUE_BLOCK_MARK;

/// The door types whose objects carry a rating or a transmittance in Revit's
/// export of the office, and one that carries neither.
const TYPES: [u64; 8] = [
    2803674, 1110711, 1106810, 1106812, 2324339, 2391770, 3261757, 394296,
];

const TEXTS: [&str; 3] = ["EI30", "EI60", "EI90"];

fn utf16(text: &str) -> Vec<u8> {
    let mut out = (text.encode_utf16().count() as u32).to_le_bytes().to_vec();
    out.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
    out
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn i64_at(buf: &[u8], at: usize) -> Option<i64> {
    let bytes: [u8; 8] = buf.get(at..at + 8)?.try_into().ok()?;
    Some(i64::from_le_bytes(bytes))
}

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    let bytes: [u8; 4] = buf.get(at..at + 4)?.try_into().ok()?;
    Some(u32::from_le_bytes(bytes))
}

fn text_at(buf: &[u8], at: usize) -> Option<String> {
    let count = u32_at(buf, at)? as usize;
    if !(1..=60).contains(&count) {
        return None;
    }
    let bytes = buf.get(at + 4..at + 4 + count * 2)?;
    let mut text = String::new();
    for pair in bytes.chunks_exact(2) {
        if pair[1] != 0 || !(0x20..0x7f).contains(&pair[0]) {
            return None;
        }
        text.push(char::from(pair[0]));
    }
    Some(text)
}

fn context(buf: &[u8], at: usize, length: usize) -> String {
    let from = at.saturating_sub(16);
    let to = (at + length + 24).min(buf.len());
    format!(
        "prev {} next {}",
        hex(&buf[from..at]),
        hex(&buf[at + length..to])
    )
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let real = 3.7021f64.to_le_bytes();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let blocks: Vec<(usize, u64)> = memchr::memmem::find_iter(buf, &VALUE_BLOCK_MARK)
            .filter(|&at| at >= 8 && buf[at - 1] != 0xff)
            .filter_map(|at| Some((at, u64::try_from(i64_at(buf, at - 8)?).ok()?)))
            .collect();
        let owner = |at: usize| {
            let index = blocks.partition_point(|(start, _)| *start < at);
            index.checked_sub(1).and_then(|i| blocks.get(i)).copied()
        };
        let show_owner = |at: usize| match owner(at) {
            Some((start, id)) => format!("block {id} +{}", at - start),
            None => "block - +0".to_string(),
        };
        for text in TEXTS {
            let pattern = utf16(text);
            for at in memchr::memmem::find_iter(buf, &pattern) {
                println!(
                    "text {stream} {at} {text} {} {}",
                    show_owner(at),
                    context(buf, at, pattern.len())
                );
            }
        }
        for at in memchr::memmem::find_iter(buf, &real) {
            println!(
                "real {stream} {at} 3.7021 {} {}",
                show_owner(at),
                context(buf, at, real.len())
            );
        }
        for (index, &(start, id)) in blocks.iter().enumerate() {
            if !TYPES.contains(&id) {
                continue;
            }
            let end = blocks.get(index + 1).map_or(buf.len(), |next| next.0);
            println!("block {stream} {id} at {start} len {}", end - start);
            let mut at = start;
            while at < end {
                if let Some(text) = text_at(buf, at) {
                    let width = 4 + text.len() * 2;
                    println!(
                        "str {stream} {id} +{} prev_i64 {} {}",
                        at - start,
                        i64_at(buf, at.saturating_sub(8)).unwrap_or(0),
                        text.replace(' ', "_")
                    );
                    at += width;
                } else {
                    at += 1;
                }
            }
        }
    }
    Ok(())
}
