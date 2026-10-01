//! RE-135 (probe): the header records `Global/ElemTable` does not declare
//! (Discussion #112, STE1200).
//!
//! rvt-rs reads a partition record only where its ElementId is declared in
//! `Global/ElemTable`. RE-132 left two Einhoven walls (2921 and 3637) outside
//! what it returns and could not say what they are. Steffen (SLIK Architekten)
//! reads every undeclared header record as part of the inventory of a family
//! document embedded in the project: the class mix of a standalone family, the
//! host-wall stub of a wall-hosted family among them, flagged with bit `0x10`
//! of `m_abFlags4Bytes`, as many documents as `Global/ContentDocuments` has
//! section headers.
//!
//! This scans every header record of every `Partitions/*` stream, declared or
//! not, and prints what that reading predicts:
//!
//! - how many records are declared and how many are not;
//! - the classes of each group, and the flag values;
//! - how many records of each group carry bit `0x10`;
//! - how many undeclared records sit before the last declared record of their
//!   stream, and how many separate runs of undeclared records there are;
//! - how many section headers `Global/ContentDocuments` holds;
//! - the records of the ids he names, with the three records either side.
//!
//! A 2024, 2025 or 2026 record is found as the chain does (RE-35): its header
//! constant, the size at `+0x08`, and a trailer that repeats the size. Unlike
//! the chain walk, this does not stop at the first position that is not a
//! record. A 2023 record is found at its bounding-box marker (RE-81), so a
//! record with declared parameter entries (its count not zero) is counted
//! apart and not read. The offsets of the class, flags and category shift by
//! 22 bytes per declared parameter entry on 2024 and later, and are read so.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re135_undeclared_records -- MODEL.rvt

use rvt::RevitFile;
use rvt::compression::InflatedStream;
use rvt::partition_element_records as per;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const EINHOVEN: &str = "d3a0c6d37d3f47a1726bc5aa7fe3880ed3c13bbe819b5e64680f6710b15aa948";
const CORE_INTERIOR: &str = "c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014";

/// The ids Steffen names as undeclared, by file.
const TARGETS: &[(&str, &[u64])] = &[
    (EINHOVEN, &[2921, 3637]),
    (CORE_INTERIOR, &[23_857, 17_716, 18_429]),
];

/// Records shown either side of a target.
const NEIGHBOURS: isize = 3;

/// Bytes from a 2023 record's ElementId to its marker (RE-81).
const ID_TO_MARKER_2023: usize = 52;

/// Bytes from a 2023 record's `BuiltInCategory` to its marker (RE-81).
const CATEGORY_BEFORE_MARKER_2023: usize = 38;

/// Bytes of one declared parameter entry, which a 2024 or later record writes
/// ahead of its category.
const ENTRY_BYTES: usize = 22;

/// Offset of `m_abFlags4Bytes` in a record with no declared entries (RE-128).
const FLAGS_OFFSET: usize = 0x46;

/// The bit Steffen reads as the host-stub marker.
const FLAG_HOST_STUB: u32 = 0x10;

/// The longest run of top entries printed for a histogram.
const TOP: usize = 15;

struct Record {
    stream: String,
    offset: usize,
    id: u64,
    class_tag: Option<u16>,
    flags: Option<u32>,
    category: Option<i64>,
}

fn u16_at(buf: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        buf.get(at..at.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        buf.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn u64_at(buf: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        buf.get(at..at.checked_add(8)?)?.try_into().ok()?,
    ))
}

fn i64_at(buf: &[u8], at: usize) -> Option<i64> {
    Some(i64::from_le_bytes(
        buf.get(at..at.checked_add(8)?)?.try_into().ok()?,
    ))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digits: Vec<String> = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    digits.join("")
}

/// Every header record of a 2024 or later partition, in stream order.
fn scan_2024(stream: &str, buf: &[u8], header_tag: u16) -> Vec<Record> {
    let mut out = Vec::new();
    let mut next_free = 0usize;
    for hit in memchr::memmem::find_iter(buf, &header_tag.to_le_bytes()) {
        let Some(start) = hit.checked_sub(0x0c) else {
            continue;
        };
        if start < next_free {
            continue;
        }
        let Some(size) = u32_at(buf, start + 8).map(|v| v as usize) else {
            continue;
        };
        if size < per::PARTITION_RECORD_HEADER_LEN {
            continue;
        }
        let end = start + size;
        let flag_word = u32_at(buf, end + 8);
        let echo = u32_at(buf, end + 12).map(|v| v as usize);
        if !matches!(flag_word, Some(0 | 0x0100_0000)) || echo != Some(size) {
            continue;
        }
        let Some(id) = u64_at(buf, start) else {
            continue;
        };
        let entries = usize::from(u16_at(buf, start + 0x0e).unwrap_or(0));
        let shift = ENTRY_BYTES * entries;
        out.push(Record {
            stream: stream.to_string(),
            offset: start,
            id,
            class_tag: u16_at(buf, start + per::CLASS_TAG_OFFSET + shift),
            flags: u32_at(buf, start + FLAGS_OFFSET + shift),
            category: i64_at(buf, start + per::CATEGORY_OFFSET + shift),
        });
        next_free = end + per::PARTITION_RECORD_TRAILER_LEN;
    }
    out
}

/// What a scan of one 2023 partition counted besides its records.
#[derive(Default)]
struct Scan2023 {
    /// Positions of the bounding-box marker.
    marker_hits: usize,
    /// Markers behind a header with declared parameter entries, not read.
    with_entries: usize,
}

/// Every header record of a 2023 partition that has no declared entries, in
/// stream order.
fn scan_2023(
    stream: &str,
    buf: &[u8],
    marker: &[u8; 8],
    header_tag: u16,
    counts: &mut Scan2023,
) -> Vec<Record> {
    let mut out = Vec::new();
    for at in memchr::memmem::find_iter(buf, marker) {
        counts.marker_hits += 1;
        let (Some(id_at), Some(base)) = (
            at.checked_sub(ID_TO_MARKER_2023),
            at.checked_sub(per::BBOX_MARKER_OFFSET),
        ) else {
            continue;
        };
        if u16_at(buf, id_at + 8) != Some(header_tag) {
            continue;
        }
        if u16_at(buf, id_at + 10) != Some(0) {
            counts.with_entries += 1;
            continue;
        }
        let Some(id) = u32_at(buf, id_at) else {
            continue;
        };
        out.push(Record {
            stream: stream.to_string(),
            offset: id_at,
            id: u64::from(id),
            class_tag: u16_at(buf, base + per::CLASS_TAG_OFFSET),
            flags: u32_at(buf, base + FLAGS_OFFSET),
            category: at
                .checked_sub(CATEGORY_BEFORE_MARKER_2023)
                .and_then(|from| i64_at(buf, from)),
        });
    }
    out
}

fn declared_in(declared: &BTreeSet<u32>, id: u64) -> bool {
    u32::try_from(id).is_ok_and(|id| declared.contains(&id))
}

/// The `limit` most frequent keys, as a JSON array of `[key, count]`.
fn top_pairs(histogram: &BTreeMap<String, usize>, limit: usize) -> String {
    let mut pairs: Vec<(&String, &usize)> = histogram.iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    let items: Vec<String> = pairs
        .iter()
        .take(limit)
        .map(|(key, count)| format!("[{key:?},{count}]"))
        .collect();
    format!("[{}]", items.join(","))
}

fn run() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let digest = sha256_hex(&std::fs::read(&path).expect("read the model"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    if version != 2023 && !per::supports_revit_version(version) {
        println!("{{\"revit\":{version},\"skipped\":\"release not measured\"}}");
        return Ok(());
    }
    let layout = if version == 2023 { "2023" } else { "2024" };
    let classes = rf.schema_classes()?;
    let tag_of = |name: &str| {
        classes
            .classes
            .iter()
            .find(|class| class.name == name)
            .map(|class| class.tag)
    };
    let Some(header_tag) = tag_of("ElementHeader") else {
        println!("{{\"revit\":{version},\"skipped\":\"no ElementHeader class\"}}");
        return Ok(());
    };
    let class_name = |tag: Option<u16>| {
        tag.and_then(|tag| classes.by_tag(tag))
            .map_or_else(|| "?".to_string(), |class| class.name.clone())
    };
    let declared = rvt::elem_table::parse_records(&mut rf)
        .map(|records| rvt::elem_table::declared_ids(&records))
        .unwrap_or_default();

    let marker_2023 = if version == 2023 {
        rvt::partition_element_records_2023::record_marker(&mut rf).map(|(marker, _)| marker)
    } else {
        None
    };
    let mut records: Vec<Record> = Vec::new();
    let mut counts_2023 = Scan2023::default();
    let streams = rf.partition_stream_names();
    for stream in &streams {
        let Ok(inflated) = rf.inflated_partition(stream) else {
            continue;
        };
        let buf = inflated.bytes();
        match &marker_2023 {
            Some(marker) => records.extend(scan_2023(
                stream,
                buf,
                marker,
                header_tag,
                &mut counts_2023,
            )),
            None if version != 2023 => records.extend(scan_2024(stream, buf, header_tag)),
            None => {}
        }
    }

    let mut classes_all: BTreeMap<String, usize> = BTreeMap::new();
    let mut classes_declared: BTreeMap<String, usize> = BTreeMap::new();
    let mut classes_undeclared: BTreeMap<String, usize> = BTreeMap::new();
    let mut flags_declared: BTreeMap<String, usize> = BTreeMap::new();
    let mut flags_undeclared: BTreeMap<String, usize> = BTreeMap::new();
    let mut id_counts: BTreeMap<u64, usize> = BTreeMap::new();
    let (mut declared_n, mut undeclared_n) = (0usize, 0usize);
    let (mut stub_declared, mut stub_undeclared) = (0usize, 0usize);
    for record in &records {
        let name = class_name(record.class_tag);
        let flag_key = record
            .flags
            .map_or_else(|| "none".to_string(), |flags| format!("{flags:#x}"));
        let stub = record.flags.is_some_and(|flags| flags & FLAG_HOST_STUB != 0);
        *id_counts.entry(record.id).or_default() += 1;
        *classes_all.entry(name.clone()).or_default() += 1;
        if declared_in(&declared, record.id) {
            declared_n += 1;
            stub_declared += usize::from(stub);
            *classes_declared.entry(name).or_default() += 1;
            *flags_declared.entry(flag_key).or_default() += 1;
        } else {
            undeclared_n += 1;
            stub_undeclared += usize::from(stub);
            *classes_undeclared.entry(name).or_default() += 1;
            *flags_undeclared.entry(flag_key).or_default() += 1;
        }
    }

    let bounds = |want_declared: bool| {
        let ids = records
            .iter()
            .filter(|record| declared_in(&declared, record.id) == want_declared)
            .map(|record| record.id);
        match (ids.clone().min(), ids.max()) {
            (Some(low), Some(high)) => format!("[{low},{high}]"),
            _ => "null".to_string(),
        }
    };

    let mut interleaved = 0usize;
    let mut start = 0usize;
    while start < records.len() {
        let stream = &records[start].stream;
        let len = records[start..]
            .iter()
            .take_while(|record| &record.stream == stream)
            .count();
        let run = &records[start..start + len];
        if let Some(last) = run
            .iter()
            .rposition(|record| declared_in(&declared, record.id))
        {
            interleaved += run[..last]
                .iter()
                .filter(|record| !declared_in(&declared, record.id))
                .count();
        }
        start += len;
    }

    let mut undeclared_runs = 0usize;
    let mut previous: Option<(&str, bool)> = None;
    for record in &records {
        let undeclared = !declared_in(&declared, record.id);
        let continues = undeclared
            && previous.is_some_and(|(stream, was)| was && stream == record.stream.as_str());
        if undeclared && !continues {
            undeclared_runs += 1;
        }
        previous = Some((record.stream.as_str(), undeclared));
    }

    let repeated = id_counts.values().filter(|&&count| count > 1).count();
    let repeated_declared = id_counts
        .iter()
        .filter(|&(&id, &count)| count > 1 && declared_in(&declared, id))
        .count();

    let content_documents = rf
        .read_stream("Global/ContentDocuments")
        .ok()
        .and_then(|raw| {
            let marker_tag = tag_of("ContentMarker")?;
            let inflated = InflatedStream::from_stored("Global/ContentDocuments", &raw);
            let mut pattern = Vec::with_capacity(12);
            pattern.extend_from_slice(&marker_tag.to_le_bytes());
            pattern.extend_from_slice(&[0xff; 4]);
            pattern.extend_from_slice(&marker_tag.wrapping_sub(1).to_le_bytes());
            pattern.extend_from_slice(&[0xff; 4]);
            let sections = memchr::memmem::find_iter(inflated.bytes(), &pattern).count();
            Some(format!(
                "{{\"marker_tag\":{marker_tag},\"stream_bytes\":{},\"section_headers\":{sections}}}",
                inflated.bytes().len()
            ))
        })
        .unwrap_or_else(|| "null".to_string());

    println!(
        "{{\"revit\":{version},\"layout\":{layout:?},\"sha256\":\"{digest}\",\"header_tag\":{header_tag},\"streams\":{},\"records\":{},\"declared_ids\":{},\"declared\":{declared_n},\"undeclared\":{undeclared_n},\"marker_hits_2023\":{},\"with_entries_2023\":{},\"bit_0x10\":{{\"declared\":{stub_declared},\"undeclared\":{stub_undeclared}}},\"undeclared_before_last_declared\":{interleaved},\"undeclared_runs\":{undeclared_runs},\"declared_id_bounds\":{},\"undeclared_id_bounds\":{},\"ids_in_two_or_more_records\":{repeated},\"of_them_declared\":{repeated_declared},\"content_documents\":{content_documents}}}",
        streams.len(),
        records.len(),
        declared.len(),
        counts_2023.marker_hits,
        counts_2023.with_entries,
        bounds(true),
        bounds(false),
    );
    println!(
        "{{\"classes_all\":{},\"classes_declared\":{},\"classes_undeclared\":{},\"flags_declared\":{},\"flags_undeclared\":{}}}",
        top_pairs(&classes_all, TOP),
        top_pairs(&classes_declared, TOP),
        top_pairs(&classes_undeclared, TOP),
        top_pairs(&flags_declared, TOP),
        top_pairs(&flags_undeclared, TOP),
    );

    let describe = |record: &Record| {
        format!(
            "{{\"id\":{},\"class\":{:?},\"declared\":{},\"flags\":{},\"category\":{}}}",
            record.id,
            class_name(record.class_tag),
            declared_in(&declared, record.id),
            record
                .flags
                .map_or_else(|| "null".to_string(), |flags| format!("\"{flags:#x}\"")),
            record
                .category
                .map_or_else(|| "null".to_string(), |category| category.to_string()),
        )
    };
    for (_, ids) in TARGETS.iter().filter(|(file, _)| *file == digest) {
        for &id in *ids {
            let mut found = 0usize;
            for (index, record) in records
                .iter()
                .enumerate()
                .filter(|(_, record)| record.id == id)
            {
                found += 1;
                let around: Vec<String> = (-NEIGHBOURS..=NEIGHBOURS)
                    .filter(|&step| step != 0)
                    .filter_map(|step| {
                        let other = records.get(index.checked_add_signed(step)?)?;
                        (other.stream == record.stream)
                            .then(|| format!("{{\"at\":{step},\"record\":{}}}", describe(other)))
                    })
                    .collect();
                println!(
                    "{{\"target\":{id},\"stream\":{:?},\"offset\":{},\"record\":{},\"around\":[{}]}}",
                    record.stream,
                    record.offset,
                    describe(record),
                    around.join(",")
                );
            }
            if found == 0 {
                println!("{{\"target\":{id},\"found\":0}}");
            }
        }
    }
    eprintln!(
        "Revit {version}: {} records, {declared_n} declared, {undeclared_n} not",
        records.len()
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        println!("{{\"error\":{:?}}}", error.to_string());
    }
}
