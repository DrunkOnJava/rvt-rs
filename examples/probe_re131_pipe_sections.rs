//! RE-131 (probe): where a pipe's centreline is stored (#96).
//!
//! jakobhirn-bit (Discussion #112) reads a pipe's ends from entries of a
//! connected ElementId, two `u32` and a point of three `f64`, in a record of
//! its own, and its nominal size after the two tags that follow
//! `RbsCurveConnectorManager`. On RE1 the pipe's element record is 335 bytes
//! and holds only its box, so this looks for the rest.
//!
//! For every pipe record it prints each place in any partition where three
//! `f64` fall inside the pipe's box, with the bytes around it and the record
//! of the leading chain that contains it, and it prints every occurrence of
//! the anchor `RbsCurveConnectorManager`'s tag, `0xFF`x4 and the next tag,
//! with the two `f64` that follow.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re131_pipe_sections -- MODEL.rvt > pipes.jsonl

use rvt::RevitFile;
use rvt::partition_element_records as per;
use std::collections::BTreeMap;

/// How far outside the pipe's box a point may fall, feet.
const TOLERANCE: f64 = 1e-3;
/// Most point hits printed per pipe and stream.
const MAX_HITS: usize = 60;
/// Most anchor occurrences printed per stream.
const MAX_ANCHORS: usize = 4_000;

fn hex(bytes: &[u8]) -> String {
    let digits: Vec<String> = bytes.iter().map(|b| format!("{b:02x}")).collect();
    digits.join("")
}

fn read_f64(buf: &[u8], at: usize) -> Option<f64> {
    buf.get(at..at + 8)
        .map(|s| f64::from_le_bytes(s.try_into().expect("8 bytes")))
}

/// A JSON value for an optional number.
fn json<T: ToString>(value: Option<T>) -> String {
    value.map_or_else(|| "null".to_string(), |v| v.to_string())
}

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let classes = rf.schema_classes()?;
    let manager = classes
        .classes
        .iter()
        .find(|class| class.name == "RbsCurveConnectorManager")
        .map(|class| class.tag);
    println!(
        "{{\"revit\":{version},\"connector_manager_tag\":{}}}",
        json(manager)
    );
    let Some(marker) = per::file_bbox_marker(&mut rf, version) else {
        eprintln!("Revit {version}: no record marker");
        return Ok(());
    };
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let records =
        per::scan_category_records_multi(&mut rf, version, &[per::OST_PIPE_CURVES], &declared)?;
    let streams = rf.partition_stream_names();
    let mut chains: BTreeMap<String, Vec<per::PartitionRecordSpan>> = BTreeMap::new();

    // The anchor, wherever it occurs.
    if let Some(tag) = manager {
        let mut anchor = tag.to_le_bytes().to_vec();
        anchor.extend_from_slice(&[0xff; 4]);
        anchor.extend_from_slice(&(tag + 1).to_le_bytes());
        for stream in &streams {
            let Ok(inflated) = rf.inflated_partition(stream) else {
                continue;
            };
            let buf = inflated.bytes();
            let chain = chains
                .entry(stream.clone())
                .or_insert_with(|| per::partition_record_chain(buf, &marker));
            for at in memchr::memmem::find_iter(buf, anchor.as_slice()).take(MAX_ANCHORS) {
                let owner = per::enclosing_record(chain, at);
                println!(
                    "{{\"anchor\":true,\"stream\":{stream:?},\"at\":{at},\"nominal\":[{},{}],\"owner_start\":{},\"owner_id\":{},\"before\":\"{}\"}}",
                    json(read_f64(buf, at + 8)),
                    json(read_f64(buf, at + 16)),
                    json(owner.map(|span| span.start)),
                    json(owner.map(|span| span.element_id)),
                    hex(&buf[at.saturating_sub(64)..at])
                );
            }
        }
    }

    // The pipes' points.
    for record in &records {
        let b = record.bbox_feet;
        let id = record.element_id;
        println!(
            "{{\"pipe\":{id},\"stream\":{:?},\"offset\":{},\"bbox\":{:?}}}",
            record.stream, record.offset, b
        );
        for stream in &streams {
            let Ok(inflated) = rf.inflated_partition(stream) else {
                continue;
            };
            let buf = inflated.bytes();
            let chain = chains
                .entry(stream.clone())
                .or_insert_with(|| per::partition_record_chain(buf, &marker));
            let mut hits = 0;
            let last = buf.len().saturating_sub(24);
            for at in 0..last {
                if hits >= MAX_HITS {
                    break;
                }
                let Some(x) = read_f64(buf, at) else {
                    break;
                };
                if !(x >= b[0] - TOLERANCE && x <= b[3] + TOLERANCE) {
                    continue;
                }
                let (Some(y), Some(z)) = (read_f64(buf, at + 8), read_f64(buf, at + 16)) else {
                    continue;
                };
                if !(y >= b[1] - TOLERANCE
                    && y <= b[4] + TOLERANCE
                    && z >= b[2] - TOLERANCE
                    && z <= b[5] + TOLERANCE)
                {
                    continue;
                }
                hits += 1;
                let owner = per::enclosing_record(chain, at);
                println!(
                    "{{\"hit\":{id},\"stream\":{stream:?},\"at\":{at},\"point\":[{x:?},{y:?},{z:?}],\"owner_start\":{},\"owner_id\":{},\"before\":\"{}\",\"after\":\"{}\"}}",
                    json(owner.map(|span| span.start)),
                    json(owner.map(|span| span.element_id)),
                    hex(&buf[at.saturating_sub(48)..at]),
                    hex(&buf[(at + 24).min(buf.len())..(at + 72).min(buf.len())])
                );
            }
        }
    }
    eprintln!("Revit {version}: {} pipes", records.len());
    Ok(())
}
