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
//! RE-139 (the same scan): for the copies of a repeated id that differ, the
//! byte positions that differ, counted by class and by position from the
//! record's start (#548).
//!
//! RE-140 (the same scan, run on Autodesk's sample projects of 2016 to 2027):
//! every group also prints the values of the trailer's flag word and how many
//! trailers do not repeat the record's size; a 2023-form file prints how many
//! `[Outline][0xFF x 4][ElementParents]` markers it holds, how many have the
//! 32-bit header 52 bytes before them, and how many are followed by a
//! well-formed bounding box. A release after 2026 is read as 2024 is, from the
//! header record alone, so it prints what is there without being admitted.
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

/// Most differing byte positions counted for one pair of copies, and most
/// (class, position) entries printed.
const DIFF_POSITIONS: usize = 64;
const DIFFERING_OFFSETS_SHOWN: usize = 60;

/// Bytes ahead of the first record field a 2023 header has: `u32` id, `u32`
/// size.
const HEADER_2023: usize = 12;

/// Bytes from a 2023 record's ElementId to its marker (RE-81), and from the
/// ElementId to the header tag.
const MARKER_AFTER_ID: usize = 52;
const HEADER_TAG_AFTER_ID: usize = 8;

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
    /// The record's bounding box, feet: min x, y, z then max x, y, z.
    bbox: Option<[f64; 6]>,
    /// The trailer's flag word, and whether its last word repeats the
    /// record's size (RE-140).
    trailer: Option<(u32, bool)>,
    /// Whether the element-record marker is where this layout puts it, 22
    /// bytes further for each declared parameter entry (RE-140): only such a
    /// record has the category and box read here.
    marked: bool,
    /// Where the marker is in the record's bytes, from its start, wherever
    /// that is (RE-140).
    marker_offset: Option<usize>,
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

fn f64_at(buf: &[u8], at: usize) -> Option<f64> {
    Some(f64::from_le_bytes(
        buf.get(at..at.checked_add(8)?)?.try_into().ok()?,
    ))
}

/// The size of a difference between two bounding-box values, feet, as a name.
fn delta_bucket(delta: f64) -> &'static str {
    if delta == 0.0 {
        "0"
    } else if delta < 1e-9 {
        "<1e-9"
    } else if delta < 1e-6 {
        "<1e-6"
    } else if delta < 1e-3 {
        "<1e-3"
    } else if delta < 1.0 {
        "<1"
    } else {
        ">=1"
    }
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

    /// The id and size words of the header at `start` (RE-144).
    fn header_words(self, buf: &[u8], start: usize) -> Option<(u64, u32)> {
        match self {
            Layout::V2023 => Some((u64::from(u32_at(buf, start)?), u32_at(buf, start + 4)?)),
            Layout::V2024 => Some((u64_at(buf, start)?, u32_at(buf, start + 8)?)),
        }
    }

    /// Bytes from a record's start to its element-record marker.
    fn marker_at(self) -> usize {
        match self {
            Layout::V2023 => MARKER_AFTER_ID,
            Layout::V2024 => per::BBOX_MARKER_OFFSET,
        }
    }

    /// The record that starts at `start`.
    fn read(
        self,
        stream: &str,
        buf: &[u8],
        start: usize,
        in_chain: bool,
        marker: Option<[u8; 8]>,
    ) -> Option<Record> {
        let (id, entries_at, class_at, flags_at, category_at, bbox_at) = match self {
            Layout::V2024 => (
                u64_at(buf, start)?,
                start + 0x0e,
                start + per::CLASS_TAG_OFFSET,
                start + FLAGS_OFFSET,
                start + per::CATEGORY_OFFSET,
                start + per::BBOX_OFFSET,
            ),
            Layout::V2023 => (
                u64::from(u32_at(buf, start)?),
                start + 10,
                start + 46,
                start + 42,
                start + 14,
                start + 60,
            ),
        };
        let entries = usize::from(u16_at(buf, entries_at).unwrap_or(0));
        let shift = ENTRY_BYTES * entries;
        let mut bbox = [0.0f64; 6];
        let bbox_read = bbox.iter_mut().enumerate().all(|(k, slot)| {
            match f64_at(buf, bbox_at + shift + 8 * k) {
                Some(value) if value.is_finite() => {
                    *slot = value;
                    true
                }
                _ => false,
            }
        });
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
            bbox: bbox_read.then_some(bbox),
            trailer: None,
            marked: marker.is_some_and(|marker| {
                let at = start + self.marker_at() + shift;
                buf.get(at..at + marker.len()) == Some(marker.as_slice())
            }),
            marker_offset: None,
        })
    }

    /// The flag word of the trailer that ends the record `start..end`, and
    /// whether the word after it repeats the size (both layouts end a record
    /// with the flag word and the size echo).
    fn trailer(self, buf: &[u8], start: usize, end: usize) -> Option<(u32, bool)> {
        let size_at = match self {
            Layout::V2023 => start + 4,
            Layout::V2024 => start + 8,
        };
        let size = u32_at(buf, size_at)?;
        let flag = u32_at(buf, end.checked_sub(8)?)?;
        let echo = u32_at(buf, end.checked_sub(4)?)?;
        Some((flag, echo == size))
    }
}

/// What one partition holds: every header record in stream order, where its
/// leading chain ends, how many positions outside an accepted record carried
/// the header tag, and the id and size words of those that were not accepted
/// (RE-144).
struct Scanned {
    records: Vec<Record>,
    chain_end: usize,
    candidates: usize,
    rejected: Vec<(u64, u32)>,
}

fn scan(
    layout: Layout,
    stream: &str,
    buf: &[u8],
    header_tag: u16,
    marker: Option<[u8; 8]>,
) -> Scanned {
    let mut chain_end = 0usize;
    while let Some(next) = layout.record_end(buf, chain_end, header_tag) {
        chain_end = next;
    }
    let mut out = Vec::new();
    let mut next_free = 0usize;
    let mut candidates = 0usize;
    let mut rejected = Vec::new();
    for hit in memchr::memmem::find_iter(buf, &header_tag.to_le_bytes()) {
        let Some(start) = hit.checked_sub(layout.tag_at()) else {
            continue;
        };
        if start < next_free {
            continue;
        }
        candidates += 1;
        let Some(end) = layout.record_end(buf, start, header_tag) else {
            rejected.extend(layout.header_words(buf, start));
            continue;
        };
        let Some(mut record) = layout.read(stream, buf, start, start < chain_end, marker) else {
            continue;
        };
        record.span = end - start;
        record.trailer = layout.trailer(buf, start, end);
        record.marker_offset = marker.and_then(|marker| {
            buf.get(start..end)
                .and_then(|body| memchr::memmem::find(body, &marker))
        });
        out.push(record);
        next_free = end;
    }
    Scanned {
        records: out,
        chain_end,
        candidates,
        rejected,
    }
}

fn declared_in(declared: &BTreeSet<u32>, id: u64) -> bool {
    u32::try_from(id).is_ok_and(|id| declared.contains(&id))
}

/// The ids of a partition's leading chain read by the rule a strict reader
/// would use (RE-140): the header tag, and a trailer whose last word repeats
/// the record's size. The flag word is returned, not checked, so a record
/// with an odd flag does not end the chain. Each id comes with its flag.
fn strict_chain(layout: Layout, buf: &[u8], header_tag: u16) -> Vec<(u64, u32)> {
    let (size_at, trailer) = match layout {
        Layout::V2023 => (4, HEADER_2023),
        Layout::V2024 => (8, per::PARTITION_RECORD_TRAILER_LEN),
    };
    let mut out = Vec::new();
    let mut at = 0usize;
    while u16_at(buf, at + layout.tag_at()) == Some(header_tag) {
        let (Some(size), Some(id)) = (
            u32_at(buf, at + size_at),
            match layout {
                Layout::V2023 => u32_at(buf, at).map(u64::from),
                Layout::V2024 => u64_at(buf, at),
            },
        ) else {
            break;
        };
        let end = at + size as usize;
        let (Some(flag), Some(echo)) = (
            u32_at(buf, end + trailer - 8),
            u32_at(buf, end + trailer - 4),
        ) else {
            break;
        };
        if echo != size || size < 16 {
            break;
        }
        out.push((id, flag));
        at = end + trailer;
    }
    out
}

/// Both ids of every `Global/ElemTable` record, read from `0x06` where the
/// first record starts (RE-140; `parse_records` reads from the frame 24 bytes
/// on, so it never reads the first record's ids): 28-byte records to 2023 and
/// 40-byte from 2024, the second id at `+4` or `+20`.
fn declared_from_origin(rf: &mut RevitFile, version: u32) -> Option<BTreeSet<u32>> {
    const TABLE: &str = rvt::streams::GLOBAL_ELEM_TABLE;
    let raw = rf.read_stream(TABLE).ok()?;
    let table = rvt::compression::inflate_stream_at(TABLE, &raw, 8)
        .or_else(|_| rvt::compression::inflate_stream_at(TABLE, &raw, 0))
        .ok()?;
    let count = usize::from(u16_at(&table, 2)?);
    let (stride, second) = if version >= 2024 { (40, 20) } else { (28, 4) };
    let mut ids = BTreeSet::new();
    for record in 0..count {
        let at = 6 + record * stride;
        ids.extend(u32_at(&table, at));
        ids.extend(u32_at(&table, at + second).filter(|&id| id != 0));
    }
    Some(ids)
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
    /// The trailer flag word of every record (RE-140).
    trailer_flags: BTreeMap<String, usize>,
    /// Trailers whose last word is not the record's size.
    echo_mismatch: usize,
    /// Where each record holds the element-record marker, from its start.
    marker_offsets: BTreeMap<String, usize>,
}

impl Tally {
    fn add(&mut self, record: &Record, declared: bool, class: String) {
        *self
            .marker_offsets
            .entry(
                record
                    .marker_offset
                    .map_or_else(|| "none".to_string(), |offset| offset.to_string()),
            )
            .or_default() += 1;
        match record.trailer {
            Some((flag, echo_matches)) => {
                *self.trailer_flags.entry(format!("{flag:#x}")).or_default() += 1;
                self.echo_mismatch += usize::from(!echo_matches);
            }
            None => *self.trailer_flags.entry("none".to_string()).or_default() += 1,
        }
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
            "{{\"group\":{group:?},\"records\":{},\"declared\":{},\"undeclared\":{},\"bit_0x10\":{{\"declared\":{},\"undeclared\":{}}},\"with_entries\":{},\"class_unresolved\":{},\"classes\":{},\"classes_declared\":{},\"classes_undeclared\":{},\"flags_declared\":{},\"flags_undeclared\":{},\"trailer_flags\":{},\"echo_mismatch\":{},\"marker_offsets\":{}}}",
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
            top_pairs(&self.trailer_flags, TOP),
            self.echo_mismatch,
            top_pairs(&self.marker_offsets, TOP),
        )
    }
}

fn run() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let digest = sha256_hex(&std::fs::read(&path).expect("read the model"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    // A release after 2026 is read as 2024 is (RE-140); it is printed, not admitted.
    let layout = if version <= 2023 {
        Layout::V2023
    } else {
        Layout::V2024
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
    let mut candidates = 0usize;
    // RE-140: the element-record marker from the file's schema; in a 2023-form
    // file the markers, the 32-bit header 52 bytes before each, and a
    // well-formed box after it.
    let marker = rvt::partition_element_records_2023::record_marker(&mut rf);
    let (mut markers, mut marker_headers, mut marker_boxes) = (0usize, 0usize, 0usize);
    let mut strict: Vec<(u64, u32)> = Vec::new();
    let mut rejected: Vec<(u64, u32)> = Vec::new();
    let streams = rf.partition_stream_names();
    for stream in &streams {
        let Ok(inflated) = rf.inflated_partition(stream) else {
            continue;
        };
        let buf = inflated.bytes();
        strict.extend(strict_chain(layout, buf, header_tag));
        if let (Layout::V2023, Some((marker, _))) = (layout, marker) {
            for at in memchr::memmem::find_iter(buf, &marker) {
                markers += 1;
                marker_headers += usize::from(
                    at.checked_sub(MARKER_AFTER_ID)
                        .and_then(|id_at| u16_at(buf, id_at + HEADER_TAG_AFTER_ID))
                        == Some(header_tag),
                );
                let bbox: Option<Vec<f64>> = (0..6).map(|k| f64_at(buf, at + 8 + 8 * k)).collect();
                marker_boxes += usize::from(bbox.is_some_and(|b| {
                    b.iter().all(|x| x.is_finite()) && (0..3).all(|axis| b[axis] <= b[axis + 3])
                }));
            }
        }
        let Scanned {
            records: found,
            chain_end,
            candidates: stream_candidates,
            rejected: stream_rejected,
        } = scan(
            layout,
            stream,
            buf,
            header_tag,
            marker.map(|(marker, _)| marker),
        );
        candidates += stream_candidates;
        rejected.extend(stream_rejected);
        stream_lines.push(format!(
            "{{\"stream\":{stream:?},\"bytes\":{},\"chain_end\":{chain_end},\"records\":{},\"in_chain\":{}}}",
            buf.len(),
            found.len(),
            found.iter().filter(|record| record.in_chain).count(),
        ));
        records.extend(found);
    }
    // RE-144: the positions that carry the header tag and are not a record, by
    // their id and size words: whether they come in repeated groups.
    let mut by_pair: BTreeMap<(u64, u32), usize> = BTreeMap::new();
    let mut by_size: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_id: BTreeMap<String, usize> = BTreeMap::new();
    for &(id, size) in &rejected {
        *by_pair.entry((id, size)).or_default() += 1;
        *by_size.entry(size.to_string()).or_default() += 1;
        *by_id.entry(id.to_string()).or_default() += 1;
    }
    let groups = by_pair.values().filter(|&&count| count >= 3).count();
    let in_groups: usize = by_pair.values().filter(|&&count| count >= 3).sum();
    println!(
        "{{\"rejected\":{{\"count\":{},\"distinct_id_and_size\":{},\"groups_of_3_or_more\":{groups},\"positions_in_those_groups\":{in_groups},\"by_size\":{},\"by_id\":{}}}}}",
        rejected.len(),
        by_pair.len(),
        top_pairs(&by_size, TOP),
        top_pairs(&by_id, TOP)
    );
    if layout == Layout::V2023 && marker.is_some() {
        println!(
            "{{\"markers\":{markers},\"markers_with_header\":{marker_headers},\"markers_with_box\":{marker_boxes}}}"
        );
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
    let mut differing_at: BTreeMap<String, usize> = BTreeMap::new();
    let mut bbox_deltas: BTreeMap<String, usize> = BTreeMap::new();
    let mut diff_examples: Vec<String> = Vec::new();
    let mut moved_walls: Vec<u64> = Vec::new();
    let mut stream_pairs: BTreeMap<String, usize> = BTreeMap::new();
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
            let class = class_name(first.class_tag);
            *differing.entry(class.clone()).or_default() += 1;
            if blobs.len() == indexes.len() {
                for blob in &blobs[1..] {
                    if blob.len() != blobs[0].len() {
                        *differing_at.entry(format!("{class}@size")).or_default() += 1;
                        continue;
                    }
                    let positions = (0..blob.len())
                        .filter(|&at| blob[at] != blobs[0][at])
                        .take(DIFF_POSITIONS);
                    for position in positions {
                        *differing_at
                            .entry(format!("{class}@{position}"))
                            .or_default() += 1;
                    }
                }
            }
            let boxes: Vec<Option<[f64; 6]>> =
                indexes.iter().map(|&index| records[index].bbox).collect();
            if let Some(first_box) = boxes[0] {
                let mut delta = 0.0f64;
                let mut every_box_read = true;
                for other in &boxes[1..] {
                    match other {
                        Some(other) => {
                            delta = first_box
                                .iter()
                                .zip(other)
                                .fold(delta, |most, (a, b)| most.max((a - b).abs()));
                        }
                        None => every_box_read = false,
                    }
                }
                if every_box_read {
                    *bbox_deltas
                        .entry(format!("{class}:{}", delta_bucket(delta)))
                        .or_default() += 1;
                    if delta >= 1e-6 {
                        if class == "SWall" {
                            moved_walls.push(*id);
                        }
                        let streams: Vec<&str> = indexes
                            .iter()
                            .map(|&index| records[index].stream.as_str())
                            .collect();
                        *stream_pairs.entry(streams.join("|")).or_default() += 1;
                    }
                    if delta >= 1e-6 && diff_examples.len() < EXAMPLES {
                        let copies: Vec<String> = indexes
                            .iter()
                            .map(|&index| {
                                let record = &records[index];
                                format!(
                                    "{{\"stream\":{:?},\"offset\":{},\"bbox\":{}}}",
                                    record.stream,
                                    record.offset,
                                    record
                                        .bbox
                                        .map_or_else(|| "null".to_string(), |b| format!("{b:?}")),
                                )
                            })
                            .collect();
                        diff_examples.push(format!(
                            "{{\"id\":{id},\"class\":{class:?},\"delta_ft\":{delta:?},\"copies\":[{}]}}",
                            copies.join(",")
                        ));
                    }
                }
            }
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
        "{{\"repeated_chain_ids\":{repeated_ids},\"adjacent\":{adjacent},\"in_elem_table_primary\":{in_primary},\"in_elem_table_secondary_only\":{in_secondary_only},\"in_one_stream\":{same_stream},\"same_class\":{same_class},\"same_size\":{same_span},\"identical_bytes\":{identical},\"differing_classes\":{},\"differing_offsets\":{},\"bbox_delta_by_class\":{},\"differing_examples\":[{}],\"signatures\":{},\"examples\":[{}]}}",
        top_pairs(&differing, TOP),
        top_pairs(&differing_at, DIFFERING_OFFSETS_SHOWN),
        top_pairs(&bbox_deltas, DIFFERING_OFFSETS_SHOWN),
        diff_examples.join(","),
        top_pairs(&signatures, TOP),
        shown.join(",")
    );
    let mut table_lengths: BTreeMap<String, usize> = BTreeMap::new();
    for record in &table {
        *table_lengths
            .entry(record.raw.len().to_string())
            .or_default() += 1;
    }
    println!(
        "{{\"scan\":{{\"candidates\":{candidates},\"accepted\":{}}},\"elem_table\":{{\"records\":{},\"record_lengths\":{}}}}}",
        records.len(),
        table.len(),
        top_pairs(&table_lengths, TOP)
    );
    println!(
        "{{\"moved\":{{\"walls\":{moved_walls:?},\"stream_pairs\":{}}}}}",
        top_pairs(&stream_pairs, TOP)
    );
    println!("{}", all.json("all"));
    println!("{}", in_chain.json("in_chain"));
    println!("{}", after_chain.json("after_chain"));

    // RE-140: the ids of the table read from where its first record starts.
    if let Some(exact) = declared_from_origin(&mut rf, version) {
        let chain_ids: BTreeSet<u64> = records
            .iter()
            .filter(|record| record.in_chain)
            .map(|record| record.id)
            .collect();
        let chain_not_declared = chain_ids
            .iter()
            .filter(|&&id| !declared_in(&exact, id))
            .count();
        let declared_without_record = exact
            .iter()
            .filter(|&&id| !chain_ids.contains(&u64::from(id)))
            .count();
        println!(
            "{{\"declared_from_origin\":{{\"ids\":{},\"chain_ids\":{},\"chain_not_declared\":{chain_not_declared},\"declared_without_chain_record\":{declared_without_record}}}}}",
            exact.len(),
            chain_ids.len()
        );
        // The same, for the chain read by the size echo alone.
        let strict_ids: BTreeSet<u64> = strict.iter().map(|(id, _)| *id).collect();
        let mut strict_flags: BTreeMap<String, usize> = BTreeMap::new();
        for (_, flag) in &strict {
            *strict_flags.entry(format!("{flag:#x}")).or_default() += 1;
        }
        let strict_not_declared = strict_ids
            .iter()
            .filter(|&&id| !declared_in(&exact, id))
            .count();
        let declared_without_strict = exact
            .iter()
            .filter(|&&id| !strict_ids.contains(&u64::from(id)))
            .count();
        println!(
            "{{\"strict_chain\":{{\"records\":{},\"ids\":{},\"not_declared\":{strict_not_declared},\"declared_without_record\":{declared_without_strict},\"flags\":{}}}}}",
            strict.len(),
            strict_ids.len(),
            top_pairs(&strict_flags, TOP)
        );
    }
    // RE-140: every chain record, one line each, for a comparison across
    // releases of the same model: `R id class category M|- min x y z max x y z`,
    // `M` when the element-record marker is where the layout puts it.
    if std::env::args().any(|arg| arg == "--records") {
        for record in records.iter().filter(|record| record.in_chain) {
            let bbox = record.bbox.map_or_else(
                || "-".to_string(),
                |bbox| {
                    bbox.iter()
                        .map(|value| format!("{value:.6}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                },
            );
            println!(
                "R {} {} {} {} {bbox}",
                record.id,
                class_name(record.class_tag),
                record
                    .category
                    .map_or_else(|| "-".to_string(), |category| category.to_string()),
                if record.marked { "M" } else { "-" }
            );
        }
    }

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
