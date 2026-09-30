//! RE-131 (probe): where a pipe's centreline is stored (#96).
//!
//! jakobhirn-bit (Discussion #112) reads a pipe's ends from a second record
//! keyed by the pipe's ElementId, as entries of a connected ElementId, two
//! `u32` and a point of three `f64`, and its nominal size after the two tags
//! that follow `RbsCurveConnectorManager`. This prints, for every pipe
//! record, the record's frame, every record of the partition's leading chain
//! that carries the pipe's ElementId, and every other place the id occurs
//! followed by the record header's constant, each as hex, so the layout can
//! be read off RE1's pipes and set against the extrusions in Revit's export.
//! It also prints the tags of the schema's `Rbs*` classes, which name the
//! anchors.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re131_pipe_sections -- MODEL.rvt > pipes.jsonl

use rvt::RevitFile;
use rvt::partition_element_records as per;
use std::collections::BTreeMap;

/// Most bytes printed for one frame or record.
const HEX_LIMIT: usize = 2_400;

fn hex(bytes: &[u8]) -> String {
    let digits: Vec<String> = bytes.iter().map(|b| format!("{b:02x}")).collect();
    digits.join("")
}

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let classes = rf.schema_classes()?;
    let rbs: Vec<String> = classes
        .classes
        .iter()
        .filter(|class| class.name.starts_with("Rbs"))
        .map(|class| format!("{:?}:{}", class.name, class.tag))
        .collect();
    println!(
        "{{\"revit\":{version},\"schema_rbs\":{{{}}}}}",
        rbs.join(",")
    );
    let Some(marker) = per::file_bbox_marker(&mut rf, version) else {
        eprintln!("Revit {version}: no record marker");
        return Ok(());
    };
    let constant = per::record_prologue_constant(&marker).to_le_bytes();
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let records =
        per::scan_category_records_multi(&mut rf, version, &[per::OST_PIPE_CURVES], &declared)?;
    let mut chains: BTreeMap<String, Vec<per::PartitionRecordSpan>> = BTreeMap::new();
    for record in &records {
        let Ok(inflated) = rf.inflated_partition(&record.stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let chain = chains
            .entry(record.stream.clone())
            .or_insert_with(|| per::partition_record_chain(buf, &marker));
        let id = u64::from(record.element_id);
        let same: Vec<String> = chain
            .iter()
            .filter(|span| span.element_id == id)
            .map(|span| {
                let shown = span.end.min(span.start + HEX_LIMIT);
                format!(
                    "{{\"start\":{},\"end\":{},\"hex\":\"{}\"}}",
                    span.start,
                    span.end,
                    hex(&buf[span.start..shown])
                )
            })
            .collect();
        let needle = id.to_le_bytes();
        let elsewhere: Vec<String> = memchr::memmem::find_iter(buf, needle.as_slice())
            .filter(|&at| buf.get(at + 0x0c..at + 0x0e) == Some(constant.as_slice()))
            .map(|at| {
                let size = buf
                    .get(at + 8..at + 12)
                    .map_or(0, |s| u32::from_le_bytes(s.try_into().expect("4 bytes")));
                format!("[{at},{size}]")
            })
            .collect();
        let frame_end = (record.offset + HEX_LIMIT).min(buf.len());
        let frame = buf.get(record.offset..frame_end).unwrap_or_default();
        println!(
            "{{\"id\":{id},\"stream\":{:?},\"offset\":{},\"bbox\":{:?},\"stream_len\":{},\"chain_records\":{},\"chain_end\":{},\"frame\":\"{}\",\"same_id_in_chain\":[{}],\"id_then_constant_at\":[{}]}}",
            record.stream,
            record.offset,
            record.bbox_feet,
            buf.len(),
            chain.len(),
            chain.last().map_or(0, |span| span.end),
            hex(frame),
            same.join(","),
            elsewhere.join(",")
        );
    }
    eprintln!("Revit {version}: {} pipes", records.len());
    Ok(())
}
