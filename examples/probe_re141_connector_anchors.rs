//! RE-141 (probe): which records hold a connector anchor, and what follows it
//! (#528).
//!
//! RE-138 reads a duct's or pipe's joins from the lists after its
//! `RbsCurveConnectorManager` anchor. A join is also a connection of a
//! fitting's, and the fittings' own objects are not read. This prints, for
//! every schema class whose name holds `Connector` or `Port`:
//!
//! - the class and its tag;
//! - for each such class that a record carries as an anchor
//!   (`[tag][0xFF x 4][tag + 1]`, as RE-134 found `RbsCurveConnectorManager`),
//!   how many anchors each class of record holds;
//! - for the first records of each (class, host class), the bytes from the
//!   anchor to the end of the record, in hex, and where the anchor sits in the
//!   record;
//! - the whole record of the first fittings of each kind, in hex.
//!
//! One JSON object per line, for `tools/re/` to search for the ids Revit's own
//! export joins.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re141_connector_anchors -- MODEL.rvt

use rvt::RevitFile;
use rvt::partition_element_records as per;
use std::collections::{BTreeMap, BTreeSet};

/// Anchors dumped in full per (anchor class, host class).
const EXAMPLES: usize = 3;
/// Fittings dumped in full per category.
const FITTINGS: usize = 4;
/// Most bytes dumped after an anchor.
const AFTER_ANCHOR: usize = 4096;

fn u16_at(buf: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(buf.get(at..at + 2)?.try_into().ok()?))
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let classes = rf.schema_classes()?;
    let connector_classes: Vec<(String, u16)> = classes
        .classes
        .iter()
        .filter(|class| class.name.contains("Connector") || class.name.contains("Port"))
        .map(|class| (class.name.clone(), class.tag))
        .collect();
    let class_rows: Vec<String> = connector_classes
        .iter()
        .map(|(name, tag)| format!("[{name:?},{tag}]"))
        .collect();
    println!(
        "{{\"revit\":{version},\"classes\":[{}]}}",
        class_rows.join(",")
    );
    let Some(marker) = per::file_bbox_marker(&mut rf, version) else {
        println!("{{\"skipped\":\"no record marker\"}}");
        return Ok(());
    };
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let host_class = |buf: &[u8], start: usize| {
        u16_at(buf, start + per::CLASS_TAG_OFFSET)
            .and_then(|tag| classes.by_tag(tag))
            .map_or_else(|| "?".to_string(), |class| class.name.clone())
    };

    let mut census: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut shown: BTreeMap<(String, String), usize> = BTreeMap::new();
    let fittings = per::scan_category_records_multi(
        &mut rf,
        version,
        &[per::OST_DUCT_FITTING, per::OST_PIPE_FITTING],
        &declared,
    )?;
    let fitting_ids: BTreeSet<u64> = fittings
        .iter()
        .map(|record| u64::from(record.element_id))
        .collect();
    let mut fitting_shown: BTreeMap<i64, usize> = BTreeMap::new();
    let categories: BTreeMap<u64, i64> = fittings
        .iter()
        .map(|record| (u64::from(record.element_id), record.builtin_category))
        .collect();
    println!("{{\"fittings\":{}}}", fitting_ids.len());

    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let chain = per::partition_record_chain(buf, &marker);
        for (name, tag) in &connector_classes {
            let mut anchor = tag.to_le_bytes().to_vec();
            anchor.extend_from_slice(&[0xff; 4]);
            anchor.extend_from_slice(&tag.wrapping_add(1).to_le_bytes());
            for at in memchr::memmem::find_iter(buf, &anchor) {
                let Some(span) = per::enclosing_record(&chain, at) else {
                    *census
                        .entry((name.clone(), "outside the chain".to_string()))
                        .or_default() += 1;
                    continue;
                };
                let host = host_class(buf, span.start);
                let key = (name.clone(), host.clone());
                *census.entry(key.clone()).or_default() += 1;
                let count = shown.entry(key).or_default();
                if *count < EXAMPLES {
                    *count += 1;
                    let end = (at + AFTER_ANCHOR).min(span.end);
                    println!(
                        "{{\"anchor\":{name:?},\"host\":{},\"host_class\":{host:?},\"stream\":{stream:?},\"anchor_at\":{},\"record_len\":{},\"from_anchor\":\"{}\"}}",
                        span.element_id,
                        at - span.start,
                        span.end - span.start,
                        hex(&buf[at..end])
                    );
                }
            }
        }
        for span in &chain {
            let Some(&category) = categories.get(&span.element_id) else {
                continue;
            };
            let count = fitting_shown.entry(category).or_default();
            if *count < FITTINGS {
                *count += 1;
                println!(
                    "{{\"fitting\":{},\"category\":{category},\"class\":{:?},\"stream\":{stream:?},\"record\":\"{}\"}}",
                    span.element_id,
                    host_class(buf, span.start),
                    hex(&buf[span.start..span.end])
                );
            }
        }
    }
    let rows: Vec<String> = census
        .iter()
        .map(|((name, host), count)| format!("[{name:?},{host:?},{count}]"))
        .collect();
    println!("{{\"census\":[{}]}}", rows.join(","));
    Ok(())
}
