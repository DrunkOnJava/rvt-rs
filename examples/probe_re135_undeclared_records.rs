//! RE-135 (probe): the header records outside a project's own element chain
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
//! not, and splits them by the leading record chain (RE-35: the project's own
//! records come first, back to back, and what follows is the loaded families'
//! own documents). For all records, those in the chain and those after it, it
//! prints:
//!
//! - how many ids are declared in `Global/ElemTable` and how many are not;
//! - the classes of each, and the flag values;
//! - how many carry bit `0x10`;
//!
//! and, for the file:
//!
//! - how many records after the chain have an id a record in the chain also
//!   has, and how many of those ids are declared;
//! - how many section headers `Global/ContentDocuments` holds;
//! - the records of the ids he names, with the three records either side.
//!
//! A 2024, 2025 or 2026 record is found as the chain does (RE-35): its header
//! constant, the size at `+0x08`, and a trailer that repeats the size. A 2023
//! record is `[u32 id][u32 size][u16 tag][u16 entries]` followed by `size`
//! bytes (Steffen's measurement), and counts as a record when the next one
//! follows at once, so the last record of each run is not counted. Unlike the
//! chain walk, the scan does not stop at the first position that is not a
//! record. The offsets of the class, flags and category shift by 22 bytes per
//! declared parameter entry, as 2024 and Steffen's 2023 measurement have it;
//! the number of 2023 records with entries is printed.
//!
//! RE-136 (the same scan): a Revit release before 2023 is read by the 2023
//! envelope (Steffen measured it on 2014 to 2018 files), and every result
//! carries how many records name a class the file's schema does not hold. For
//! the ids that open two or more records in the chain (#548) it prints the
//! classes, the sizes, whether the records are adjacent, and whether the id is
//! one `Global/ElemTable` writes as a primary id or only as a secondary one.
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

/// Bytes of one declared parameter entry.
const ENTRY_BYTES: usize = 22;

/// Offset of `m_abFlags4Bytes` in a 2024 record with no declared entries
/// (RE-128).
const FLAGS_OFFSET: usize = 0x46;

/// The bit Steffen reads as the host-stub marker.
const FLAG_HOST_STUB: u32 = 0x10;

/// The longest run of top entries printed for a histogram.
const TOP: usize = 15;

/// What a record's class is called when the file's schema has no such tag.
const UNRESOLVED: &str = "?";

/// Repeated chain ids shown in full.
const EXAMPLES: usize = 8;

/// Bytes ahead of the first record field a 2023 header has: `u32` id, `u32`
/// size.
const HEADER_2023: usize = 12;

#[derive(Clone, Copy, PartialEq)]
enum Layout {
    V2023,
    V2024,
}

struct Record {
    stream: String,
    offset: usize,
    id: u64,
    in_chain: bool,
    /// Bytes from the record's start to the next, trailer included.
    span: usize,
    entries: usize,
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

impl Layout {
    /// Bytes from a record's start to its header-tag `u16`.
    fn tag_at(self) -> usize {
        match self {
            Layout::V2023 => 8,
            Layout::V2024 => 0x0c,
        }
    }

    /// The offset one past the record that starts at `start`, trailer
    /// included, when one does.
    fn record_end(self, buf: &[u8], start: usize, header_tag: u16) -> Option<usize> {
        if u16_at(buf, start.checked_add(self.tag_at())?)? != header_tag {
            return None;
        }
        match self {
            Layout::V2024 => {
                let size = u32_at(buf, start + 8)? as usize;
                if size < per::PARTITION_RECORD_HEADER_LEN {
                    return None;
                }
                let end = start.checked_add(size)?;
                let flag_word = u32_at(buf, end.checked_add(8)?)?;
                let echo = u32_at(buf, end.checked_add(12)?)? as usize;
                (matches!(flag_word, 0 | 0x0100_0000) && echo == size)
                    .then_some(end + per::PARTITION_RECORD_TRAILER_LEN)
            }
            Layout::V2023 => {
                let size = u32_at(buf, start + 4)? as usize;
                let next = start.checked_add(HEADER_2023)?.checked_add(size)?;
                if next > buf.len() {
                    return None;
                }
                (next == buf.len() || u16_at(buf, next + self.tag_at()) == Some(header_tag))
                    .then_some(next)
            }
        }
    }

    /// The record that starts at `start`.
    fn read(self, stream: &str, buf: &[u8], start: usize, in_chain: bool) -> Option<Record> {
        let (id, entries_at, class_at, flags_at, category_at) = match self {
            Layout::V2024 => (
                u64_at(buf, start)?,
                start + 0x0e,
                start + per::CLASS_TAG_OFFSET,
                start + FLAGS_OFFSET,
                start + per::CATEGORY_OFFSET,
            ),
            Layout::V2023 => (
                u64::from(u32_at(buf, start)?),
                start + 10,
                start + 46,
                start + 42,
                start + 14,
            ),
        };
        let entries = usize::from(u16_at(buf, entries_at).unwrap_or(0));
        let shift = ENTRY_BYTES * entries;
        Some(Record {
            stream: stream.to_string(),
            offset: start,
            id,
            in_chain,
            span: 0,
            entries,
            class_tag: u16_at(buf, class_at + shift),
            flags: u32_at(buf, flags_at + shift),
            category: i64_at(buf, category_at + shift),
        })
    }
}

/// Every header record of one partition, in stream order, and where its
/// leading chain ends.
fn scan(layout: Layout, stream: &str, buf: &[u8], header_tag: u16) -> (Vec<Record>, usize) {
    let mut chain_end = 0usize;
    while let Some(next) = layout.record_end(buf, chain_end, header_tag) {
        chain_end = next;
    }
    let mut out = Vec::new();
    let mut next_free = 0usize;
    for hit in memchr::memmem::find_iter(buf, &header_tag.to_le_bytes()) {
        let Some(start) = hit.checked_sub(layout.tag_at()) else {
            continue;
        };
        if start < next_free {
            continue;
        }
        let Some(end) = layout.record_end(buf, start, header_tag) else {
            continue;
        };
        let Some(mut record) = layout.read(stream, buf, start, start < chain_end) else {
            continue;
        };
        record.span = end - start;
        out.push(record);
        next_free = end;
    }
    (out, chain_end)
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

/// What a group of records holds.
#[derive(Default)]
struct Tally {
    declared: usize,
    undeclared: usize,
    stub_declared: usize,
    stub_undeclared: usize,
    with_entries: usize,
    /// Records whose class tag the file's schema does not hold.
    unresolved: usize,
    classes: BTreeMap<String, usize>,
    classes_declared: BTreeMap<String, usize>,
    classes_undeclared: BTreeMap<String, usize>,
    flags_declared: BTreeMap<String, usize>,
    flags_undeclared: BTreeMap<String, usize>,
}

impl Tally {
    fn add(&mut self, record: &Record, declared: bool, class: String) {
        let flag_key = record
            .flags
            .map_or_else(|| "none".to_string(), |flags| format!("{flags:#x}"));
        let stub = record
            .flags
            .is_some_and(|flags| flags & FLAG_HOST_STUB != 0);
        self.with_entries += usize::from(record.entries > 0);
        self.unresolved += usize::from(class == UNRESOLVED);
        *self.classes.entry(class.clone()).or_default() += 1;
        if declared {
            self.declared += 1;
            self.stub_declared += usize::from(stub);
            *self.classes_declared.entry(class).or_default() += 1;
            *self.flags_declared.entry(flag_key).or_default() += 1;
        } else {
            self.undeclared += 1;
            self.stub_undeclared += usize::from(stub);
            *self.classes_undeclared.entry(class).or_default() += 1;
            *self.flags_undeclared.entry(flag_key).or_default() += 1;
        }
    }

    fn json(&self, group: &str) -> String {
        format!(
            "{{\"group\":{group:?},\"records\":{},\"declared\":{},\"undeclared\":{},\"bit_0x10\":{{\"declared\":{},\"undeclared\":{}}},\"with_entries\":{},\"class_unresolved\":{},\"classes\":{},\"classes_declared\":{},\"classes_undeclared\":{},\"flags_declared\":{},\"flags_undeclared\":{}}}",
            self.declared + self.undeclared,
            self.declared,
            self.undeclared,
            self.stub_declared,
            self.stub_undeclared,
            self.with_entries,
            self.unresolved,
            top_pairs(&self.classes, TOP),
            top_pairs(&self.classes_declared, TOP),
            top_pairs(&self.classes_undeclared, TOP),
            top_pairs(&self.flags_declared, TOP),
            top_pairs(&self.flags_undeclared, TOP),
        )
    }
}

fn run() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let digest = sha256_hex(&std::fs::read(&path).expect("read the model"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let layout = if version <= 2023 {
        Layout::V2023
    } else if per::supports_revit_version(version) {
        Layout::V2024
    } else {
        println!("{{\"revit\":{version},\"skipped\":\"release not measured\"}}");
        return Ok(());
    };
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
            .map_or_else(|| UNRESOLVED.to_string(), |class| class.name.clone())
    };
    let table = rvt::elem_table::parse_records(&mut rf).unwrap_or_default();
    let declared = rvt::elem_table::declared_ids(&table);
    let primary: BTreeSet<u32> = table.iter().map(|record| record.id_primary).collect();
    let secondary_only: BTreeSet<u32> = table
        .iter()
        .filter(|record| {
            record.raw.len() == rvt::elem_table::RECORD_LEN_40
                && record.id_secondary != 0
                && record.id_secondary != record.id_primary
        })
        .map(|record| record.id_secondary)
        .collect();

    let mut records: Vec<Record> = Vec::new();
    let mut stream_lines: Vec<String> = Vec::new();
    let streams = rf.partition_stream_names();
    for stream in &streams {
        let Ok(inflated) = rf.inflated_partition(stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let (found, chain_end) = scan(layout, stream, buf, header_tag);
        stream_lines.push(format!(
            "{{\"stream\":{stream:?},\"bytes\":{},\"chain_end\":{chain_end},\"records\":{},\"in_chain\":{}}}",
            buf.len(),
            found.len(),
            found.iter().filter(|record| record.in_chain).count(),
        ));
        records.extend(found);
    }

    let mut all = Tally::default();
    let mut in_chain = Tally::default();
    let mut after_chain = Tally::default();
    let mut chain_id_counts: BTreeMap<u64, usize> = BTreeMap::new();
    let mut after_id_counts: BTreeMap<u64, usize> = BTreeMap::new();
    for record in &records {
        let is_declared = declared_in(&declared, record.id);
        let name = class_name(record.class_tag);
        all.add(record, is_declared, name.clone());
        if record.in_chain {
            in_chain.add(record, is_declared, name);
            *chain_id_counts.entry(record.id).or_default() += 1;
        } else {
            after_chain.add(record, is_declared, name);
            *after_id_counts.entry(record.id).or_default() += 1;
        }
    }
    let chain_repeated = chain_id_counts.values().filter(|&&count| count > 1).count();
    let after_repeated = after_id_counts.values().filter(|&&count| count > 1).count();
    let colliding: Vec<u64> = after_id_counts
        .keys()
        .copied()
        .filter(|id| chain_id_counts.contains_key(id))
        .collect();
    let colliding_declared = colliding
        .iter()
        .filter(|&&id| declared_in(&declared, id))
        .count();

    let mut chain_indexes: BTreeMap<u64, Vec<usize>> = BTreeMap::new();
    for (index, record) in records
        .iter()
        .enumerate()
        .filter(|(_, record)| record.in_chain)
    {
        chain_indexes.entry(record.id).or_default().push(index);
    }
    let mut signatures: BTreeMap<String, usize> = BTreeMap::new();
    let mut differing: BTreeMap<String, usize> = BTreeMap::new();
    let mut shown: Vec<String> = Vec::new();
    let (mut repeated_ids, mut adjacent) = (0usize, 0usize);
    let (mut in_primary, mut in_secondary_only) = (0usize, 0usize);
    let (mut same_stream, mut same_class, mut same_span, mut identical) =
        (0usize, 0usize, 0usize, 0usize);
    for (id, indexes) in chain_indexes
        .iter()
        .filter(|(_, indexes)| indexes.len() > 1)
    {
        repeated_ids += 1;
        let names: Vec<String> = indexes
            .iter()
            .map(|&index| class_name(records[index].class_tag))
            .collect();
        *signatures.entry(names.join("+")).or_default() += 1;
        adjacent += usize::from(indexes.windows(2).all(|pair| pair[1] == pair[0] + 1));
        let id32 = u32::try_from(*id).ok();
        in_primary += usize::from(id32.is_some_and(|id| primary.contains(&id)));
        in_secondary_only += usize::from(id32.is_some_and(|id| secondary_only.contains(&id)));
        let streams_of: BTreeSet<&str> = indexes
            .iter()
            .map(|&index| records[index].stream.as_str())
            .collect();
        same_stream += usize::from(streams_of.len() < indexes.len());
        let first = &records[indexes[0]];
        same_class += usize::from(
            indexes
                .iter()
                .all(|&index| records[index].class_tag == first.class_tag),
        );
        same_span += usize::from(
            indexes
                .iter()
                .all(|&index| records[index].span == first.span),
        );
        let blobs: Vec<Vec<u8>> = indexes
            .iter()
            .filter_map(|&index| {
                let record = &records[index];
                let inflated = rf.inflated_partition(&record.stream).ok()?;
                inflated
                    .bytes()
                    .get(record.offset..record.offset + record.span)
                    .map(<[u8]>::to_vec)
            })
            .collect();
        if blobs.len() == indexes.len() && blobs.windows(2).all(|pair| pair[0] == pair[1]) {
            identical += 1;
        } else {
            *differing.entry(class_name(first.class_tag)).or_default() += 1;
        }
        if shown.len() < EXAMPLES {
            let parts: Vec<String> = indexes
                .iter()
                .map(|&index| {
                    let record = &records[index];
                    format!(
                        "{{\"stream\":{:?},\"offset\":{},\"span\":{},\"class\":{:?},\"flags\":{}}}",
                        record.stream,
                        record.offset,
                        record.span,
                        class_name(record.class_tag),
                        record
                            .flags
                            .map_or_else(|| "null".to_string(), |flags| format!("\"{flags:#x}\"")),
                    )
                })
                .collect();
            shown.push(format!(
                "{{\"id\":{id},\"declared\":{},\"records\":[{}]}}",
                declared_in(&declared, *id),
                parts.join(",")
            ));
        }
    }

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
        "{{\"revit\":{version},\"sha256\":\"{digest}\",\"header_tag\":{header_tag},\"declared_ids\":{},\"streams\":[{}],\"ids_repeated_in_chain\":{chain_repeated},\"ids_repeated_after_chain\":{after_repeated},\"after_chain_ids_also_in_chain\":{},\"of_them_declared\":{colliding_declared},\"content_documents\":{content_documents}}}",
        declared.len(),
        stream_lines.join(","),
        colliding.len(),
    );
    println!(
        "{{\"repeated_chain_ids\":{repeated_ids},\"adjacent\":{adjacent},\"in_elem_table_primary\":{in_primary},\"in_elem_table_secondary_only\":{in_secondary_only},\"in_one_stream\":{same_stream},\"same_class\":{same_class},\"same_size\":{same_span},\"identical_bytes\":{identical},\"differing_classes\":{},\"signatures\":{},\"examples\":[{}]}}",
        top_pairs(&differing, TOP),
        top_pairs(&signatures, TOP),
        shown.join(",")
    );
    println!("{}", all.json("all"));
    println!("{}", in_chain.json("in_chain"));
    println!("{}", after_chain.json("after_chain"));

    let describe = |record: &Record| {
        format!(
            "{{\"id\":{},\"class\":{:?},\"declared\":{},\"in_chain\":{},\"flags\":{},\"category\":{}}}",
            record.id,
            class_name(record.class_tag),
            declared_in(&declared, record.id),
            record.in_chain,
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
        "Revit {version}: {} records, {} in the chain, {} after it",
        records.len(),
        in_chain.declared + in_chain.undeclared,
        after_chain.declared + after_chain.undeclared,
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        println!("{{\"error\":{:?}}}", error.to_string());
    }
}
