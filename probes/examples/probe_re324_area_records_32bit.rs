//! RE-324 (probe): Areas on Revit 2015 to 2023, against the same project saved
//! by Revit 2024 (#1194, #1261).
//!
//! FACT under test: the Area records RE-191 reads on Revit 2024 exist on the
//! 32-bit layout (RE-315) in the same shape with `u32` ElementIds: an Area's
//! chain record carries `OST_Areas` (-2003200) in the `BuiltInCategory` slot
//! of the 32-bit envelope (14 bytes after the record's ElementId), the
//! container, design option and placement words 16, 12 and 24 bytes after it
//! (RE-315's marker-relative constants turned into record-relative ones), and
//! counted lists of `u32` ElementIds that name its Level, its area scheme and
//! its area boundary lines. The project saved by Revit 2024 is the oracle:
//! every Area it reads, an older save of the same project reads with the same
//! number, name, Level, scheme, named lines and point.
//!
//! One file prints its census (`level`, `cat`, `areas`, `area`, `rec`,
//! `list`, `word`, `dump`, `line`, `reader` rows). Two files (the older
//! first, as Measure's paired probe runs them) print `pair` rows: the Areas,
//! Levels, area boundary lines and points both saves hold, side by side, and
//! the refusal or body the production reader gives each Area in each.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re324_area_records_32bit -- OLDER.rvt [NEWER.rvt]

use rvt::RevitFile;
use rvt::research::partition_area_records::{
    AREA_BODY_HEIGHT_FIELD, AREA_BODY_REFUSAL_FIELD, AREA_POINT_OFFSET, NARROW_POINT_OFFSET,
    areas_from_partition_chain_records,
};
use rvt::research::partition_design_options::compute_design_options;
use rvt::research::partition_element_records::{
    self as per, BUILTIN_CATEGORY_MAX, BUILTIN_CATEGORY_MIN, CATEGORY_OFFSET, CLASS_TAG_OFFSET,
    CONTAINER_OFFSET, PLACEMENT_KIND_OFFSET, PartitionElementRecord, builtin_category_name,
    file_bbox_marker, partition_record_chain, scan_category_records_multi,
};
use rvt::research::partition_element_records_2023 as per32;
use rvt::research::partition_level_records::{PartitionLevel, recover_partition_levels};
use rvt::research::partition_room_parameters::{
    DATA_OBJECT_HEADER, data_objects, scan_room_parameter_entries,
};
use std::collections::{BTreeMap, BTreeSet};

const OST_AREAS: i64 = -2_003_200;
const OST_AREA_SCHEMES: i64 = -2_003_201;
const OST_AREA_SCHEME_LINES: i64 = -2_000_079;
const OST_AREA_POLYLINES: i64 = -2_000_830;

/// The lowest ElementId read as an area boundary line, as the 2024 reader.
const LINE_ID_FLOOR: u32 = 1000;
/// Bytes of an Area's data searched for its point.
const POINT_WINDOW: usize = 0x2000;
/// How far before a point a data header is looked for.
const HEADER_SEARCH: usize = 0x2000;
/// Bytes of an Area's data printed after its header.
const DUMP_BYTES: usize = 512;
/// Areas whose data is dumped.
const DUMPED_AREAS: usize = 3;
/// Tolerance of a point found without its exact bytes, feet.
const POINT_TOLERANCE: f64 = 1e-6;
/// Tolerance of a boundary line's box, feet.
const LINE_TOLERANCE: f64 = 1e-4;
/// Offsets of the record searched for the category slot of a 32-bit record.
const CATEGORY_SEARCH: std::ops::RangeInclusive<usize> = 8..=40;

fn u16_at(buf: &[u8], at: usize) -> Option<u16> {
    buf.get(at..at.checked_add(2)?)
        .map(|b| u16::from_le_bytes(b.try_into().expect("2 bytes")))
}

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    buf.get(at..at.checked_add(4)?)
        .map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")))
}

fn u64_at(buf: &[u8], at: usize) -> Option<u64> {
    buf.get(at..at.checked_add(8)?)
        .map(|b| u64::from_le_bytes(b.try_into().expect("8 bytes")))
}

fn i64_at(buf: &[u8], at: usize) -> Option<i64> {
    buf.get(at..at.checked_add(8)?)
        .map(|b| i64::from_le_bytes(b.try_into().expect("8 bytes")))
}

fn f64_at(buf: &[u8], at: usize) -> Option<f64> {
    buf.get(at..at.checked_add(8)?)
        .map(|b| f64::from_le_bytes(b.try_into().expect("8 bytes")))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn partition_number(stream: &str) -> u32 {
    stream
        .rsplit('/')
        .next()
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

/// How a file's chain records are walked.
enum Walk {
    /// Revit 2024 and later: `u64` ElementIds, the file's bbox marker.
    Wide([u8; 8]),
    /// Revit 2023 and earlier: `u32` ElementIds, the tag of `ElementHeader`.
    Narrow(u16),
}

/// A declared chain record: start, end, ElementId.
struct Span {
    start: usize,
    end: usize,
    id: u32,
}

fn spans(buf: &[u8], walk: &Walk, declared: &BTreeSet<u32>) -> Vec<Span> {
    match walk {
        Walk::Wide(marker) => partition_record_chain(buf, marker)
            .into_iter()
            .filter_map(|span| {
                let id = u32::try_from(span.element_id).ok()?;
                declared.contains(&id).then_some(Span {
                    start: span.start,
                    end: span.end,
                    id,
                })
            })
            .collect(),
        Walk::Narrow(tag) => per32::record_spans(buf, *tag, declared)
            .into_iter()
            .map(|span| Span {
                start: span.start,
                end: span.end,
                id: span.element_id,
            })
            .collect(),
    }
}

fn each_partition(rf: &mut RevitFile, mut visit: impl FnMut(&str, &[u8])) {
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        visit(&stream, inflated.bytes());
    }
}

/// One Area as its latest partition holds it.
struct Area {
    stream: String,
    offset: usize,
    bytes: Vec<u8>,
    copies: usize,
    disagree: bool,
    number: Option<String>,
    name: Option<String>,
}

struct Census {
    saved: u32,
    release: u32,
    wide: bool,
    cat_at: usize,
    histogram: BTreeMap<usize, usize>,
    declared: BTreeSet<u32>,
    recovered: Vec<PartitionLevel>,
    levels: BTreeMap<u32, (String, f64, Option<bool>)>,
    categories: BTreeMap<i64, (usize, usize, usize)>,
    category_of: BTreeMap<u32, i64>,
    scheme_ids: BTreeSet<u32>,
    line_ids: BTreeSet<u32>,
    areas: BTreeMap<u32, Area>,
    area_marker_frames: usize,
    lines: Vec<PartitionElementRecord>,
    options: String,
    non_primary: BTreeSet<u32>,
}

struct AreaCopy {
    stream: String,
    start: usize,
    bytes: Vec<u8>,
}

fn build(path: &str) -> anyhow::Result<(RevitFile, Census)> {
    let mut rf = RevitFile::open(path)?;
    let saved = rf.basic_file_info()?.version;
    let release = rf.layout_release()?;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let wide = !per32::is_32bit_release(release);
    let walk = if wide {
        match file_bbox_marker(&mut rf, release) {
            Some(marker) => Walk::Wide(marker),
            None => anyhow::bail!("release {release}: no record marker"),
        }
    } else {
        match per32::record_marker(&mut rf) {
            Some((_, tag)) => Walk::Narrow(tag),
            None => anyhow::bail!("release {release}: no record marker"),
        }
    };
    let recovered = recover_partition_levels(&mut rf, release).unwrap_or_default();
    let levels: BTreeMap<u32, (String, f64, Option<bool>)> = recovered
        .iter()
        .map(|level| {
            (
                level.element_id,
                (
                    level.name.clone(),
                    level.elevation_feet,
                    level.building_story,
                ),
            )
        })
        .collect();

    let mut spans_by_stream: BTreeMap<String, Vec<Span>> = BTreeMap::new();
    let mut histogram: BTreeMap<usize, usize> = BTreeMap::new();
    each_partition(&mut rf, |stream, buf| {
        let found = spans(buf, &walk, &declared);
        if !wide {
            for span in &found {
                for offset in CATEGORY_SEARCH {
                    if i64_at(buf, span.start + offset) == Some(OST_AREAS) {
                        *histogram.entry(offset).or_default() += 1;
                    }
                }
            }
        }
        spans_by_stream.insert(stream.to_string(), found);
    });
    let cat_at = if wide {
        CATEGORY_OFFSET
    } else {
        histogram
            .iter()
            .max_by_key(|(_, count)| **count)
            .map_or(14, |(offset, _)| *offset)
    };

    let mut categories: BTreeMap<i64, (usize, usize, usize)> = BTreeMap::new();
    let mut category_of: BTreeMap<u32, i64> = BTreeMap::new();
    let mut scheme_ids = BTreeSet::new();
    let mut line_ids = BTreeSet::new();
    let mut copies: BTreeMap<u32, Vec<AreaCopy>> = BTreeMap::new();
    each_partition(&mut rf, |stream, buf| {
        let Some(found) = spans_by_stream.get(stream) else {
            return;
        };
        for span in found {
            let Some(category) = i64_at(buf, span.start + cat_at)
                .filter(|c| (BUILTIN_CATEGORY_MIN..=BUILTIN_CATEGORY_MAX).contains(c))
            else {
                continue;
            };
            let size = span.end - span.start;
            let entry = categories.entry(category).or_insert((0, usize::MAX, 0));
            entry.0 += 1;
            entry.1 = entry.1.min(size);
            entry.2 = entry.2.max(size);
            category_of.entry(span.id).or_insert(category);
            match category {
                OST_AREA_SCHEMES => {
                    scheme_ids.insert(span.id);
                }
                OST_AREA_SCHEME_LINES => {
                    line_ids.insert(span.id);
                }
                OST_AREAS => copies.entry(span.id).or_default().push(AreaCopy {
                    stream: stream.to_string(),
                    start: span.start,
                    bytes: buf[span.start..span.end].to_vec(),
                }),
                _ => {}
            }
        }
    });

    let area_ids: BTreeSet<u32> = copies.keys().copied().collect();
    let entries = scan_room_parameter_entries(&mut rf, release, &area_ids);
    let mut areas = BTreeMap::new();
    for (id, held) in copies {
        let latest = held
            .iter()
            .map(|c| partition_number(&c.stream))
            .max()
            .unwrap_or(0);
        let at_latest: Vec<&AreaCopy> = held
            .iter()
            .filter(|c| partition_number(&c.stream) == latest)
            .collect();
        let Some(first) = at_latest.first() else {
            continue;
        };
        let disagree = at_latest.iter().any(|c| c.bytes != first.bytes);
        let (number, name) = match entries.get(&id) {
            Some((number, name)) => (Some(number.clone()), Some(name.clone())),
            None => (None, None),
        };
        areas.insert(
            id,
            Area {
                stream: first.stream.clone(),
                offset: first.start,
                bytes: first.bytes.clone(),
                copies: held.len(),
                disagree,
                number,
                name,
            },
        );
    }

    let wanted = [OST_AREAS, OST_AREA_SCHEME_LINES, OST_AREA_POLYLINES];
    let records: Vec<PartitionElementRecord> = if wide {
        scan_category_records_multi(&mut rf, release, &wanted, &declared)?
    } else {
        per32::scan_records(&mut rf, release)
            .into_iter()
            .filter(|record| wanted.contains(&record.builtin_category))
            .collect()
    };
    let area_marker_frames = records
        .iter()
        .filter(|record| record.builtin_category == OST_AREAS)
        .count();
    let lines: Vec<PartitionElementRecord> = records
        .into_iter()
        .filter(|record| record.builtin_category != OST_AREAS)
        .collect();

    let (options, non_primary) = match compute_design_options(&mut rf) {
        Ok(options) => (format!("{:?}", options.sets), options.non_primary_options()),
        Err(error) => (format!("error {error}"), BTreeSet::new()),
    };

    Ok((
        rf,
        Census {
            saved,
            release,
            wide,
            cat_at,
            histogram,
            declared,
            recovered,
            levels,
            categories,
            category_of,
            scheme_ids,
            line_ids,
            areas,
            area_marker_frames,
            lines,
            options,
            non_primary,
        },
    ))
}

/// An Area record's container, design option and placement words.
fn words(c: &Census, bytes: &[u8]) -> (Option<u64>, Option<u32>, Option<u32>) {
    if c.wide {
        (
            u64_at(bytes, CONTAINER_OFFSET),
            per::frame_design_option(bytes, 0),
            u32_at(bytes, PLACEMENT_KIND_OFFSET),
        )
    } else {
        let at = c.cat_at;
        (
            u32_at(bytes, at + 16).map(|v| {
                if v == u32::MAX {
                    u64::MAX
                } else {
                    u64::from(v)
                }
            }),
            u32_at(bytes, at + 12).filter(|v| *v != u32::MAX),
            u32_at(bytes, at + 24),
        )
    }
}

fn is_placed(c: &Census, bytes: &[u8]) -> bool {
    let (container, _, placement) = words(c, bytes);
    container == Some(u64::MAX) && placement == Some(per::PLACEMENT_KIND_INSTANCE)
}

/// The ids of at least two entries that `bytes` holds as an ascending counted
/// list of `u32` declared ElementIds, from `from` on: `(offset, ids)`.
fn counted_lists(bytes: &[u8], from: usize, declared: &BTreeSet<u32>) -> Vec<(usize, Vec<u32>)> {
    let mut out = Vec::new();
    let mut at = from;
    while at + 12 <= bytes.len() {
        let count = u32_at(bytes, at).unwrap_or(0) as usize;
        if (2..=64).contains(&count) && at + 4 + 4 * count <= bytes.len() {
            let ids: Vec<u32> = (0..count)
                .filter_map(|i| u32_at(bytes, at + 4 + 4 * i))
                .collect();
            if ids.iter().all(|id| declared.contains(id)) && ids.windows(2).all(|w| w[0] < w[1]) {
                let next = at + 4 + 4 * count;
                out.push((at, ids));
                at = next;
                continue;
            }
        }
        at += 1;
    }
    out
}

#[derive(Default, PartialEq, Eq, Debug, Clone)]
struct Refs {
    levels: BTreeSet<u32>,
    schemes: BTreeSet<u32>,
    lines: BTreeSet<u32>,
}

impl Refs {
    fn offer(&mut self, c: &Census, value: u32) {
        if c.levels.contains_key(&value) {
            self.levels.insert(value);
        }
        if c.scheme_ids.contains(&value) {
            self.schemes.insert(value);
        }
        if value >= LINE_ID_FLOOR && c.line_ids.contains(&value) {
            self.lines.insert(value);
        }
    }
}

/// What every word of the record past the placement word names.
fn refs_by_scan(c: &Census, bytes: &[u8]) -> Refs {
    let mut refs = Refs::default();
    let from = if c.wide {
        CATEGORY_OFFSET
    } else {
        c.cat_at + 28
    };
    let width = if c.wide { 8 } else { 4 };
    for at in from..bytes.len().saturating_sub(width - 1) {
        let value = if c.wide {
            u64_at(bytes, at).and_then(|v| u32::try_from(v).ok())
        } else {
            u32_at(bytes, at)
        };
        if let Some(value) = value {
            refs.offer(c, value);
        }
    }
    refs
}

/// What the counted lists of a 32-bit record name, or the scan's answer on a
/// 2024 record, which keeps `u64` slots.
fn refs_of(c: &Census, bytes: &[u8]) -> (Refs, &'static str) {
    if c.wide {
        return (refs_by_scan(c, bytes), "scan");
    }
    let lists = counted_lists(bytes, c.cat_at + 28, &c.declared);
    if lists.is_empty() {
        return (refs_by_scan(c, bytes), "scan");
    }
    let mut refs = Refs::default();
    for (_, ids) in &lists {
        for id in ids {
            refs.offer(c, *id);
        }
    }
    (refs, "lists")
}

fn describe_ids(c: &Census, ids: &BTreeSet<u32>) -> String {
    ids.iter()
        .map(|id| match c.levels.get(id) {
            Some((name, elevation, story)) => format!("{id}={name}@{elevation:.4}/{story:?}"),
            None => id.to_string(),
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// The 2024 reader's point of each Area: two `f64`s at the payload offset
/// of its data object (RE-317).
fn points_wide(rf: &mut RevitFile, c: &Census) -> BTreeMap<u32, (f64, f64)> {
    let mut found: BTreeMap<u32, Option<(f64, f64)>> = BTreeMap::new();
    each_partition(rf, |stream, buf| {
        for (at, object) in data_objects(buf) {
            let Some(area) = c.areas.get(&object.element_id) else {
                continue;
            };
            if area.stream != stream
                || u16_at(&area.bytes, CLASS_TAG_OFFSET).map(u32::from) != Some(object.class)
            {
                continue;
            }
            let start = at + DATA_OBJECT_HEADER + AREA_POINT_OFFSET;
            if start + 16 > object.end {
                continue;
            }
            let point = match (f64_at(buf, start), f64_at(buf, start + 8)) {
                (Some(x), Some(y)) if x.is_finite() && y.is_finite() => Some((x, y)),
                _ => None,
            };
            match found.get(&object.element_id) {
                Some(held) if *held != point => {
                    found.insert(object.element_id, None);
                }
                Some(_) => {}
                None => {
                    found.insert(object.element_id, point);
                }
            }
        }
    });
    found
        .into_iter()
        .filter_map(|(id, point)| Some((id, point?)))
        .collect()
}

/// Where a point sits in an older partition.
struct Located {
    id: u32,
    stream: String,
    at: usize,
    header_distance: Option<usize>,
    exact: bool,
    context: String,
}

/// The distance from the end of the data header `01 00 00 00 · id` that
/// precedes `at` within [`HEADER_SEARCH`] bytes, to `at`.
fn header_before(buf: &[u8], at: usize, id: u32) -> Option<usize> {
    let wanted = id.to_le_bytes();
    let low = at.saturating_sub(HEADER_SEARCH);
    (low..at).rev().find_map(|q| {
        let head = buf.get(q..q + 4)?;
        let tail = buf.get(q + 4..q + 8)?;
        (head == [1u8, 0, 0, 0] && tail == wanted).then_some(at - (q + 8))
    })
}

/// Where each point of `wanted` sits in the partitions of an older save:
/// first its exact 16 bytes, then, for an Area with none, a point within
/// [`POINT_TOLERANCE`] in the [`POINT_WINDOW`] bytes after its data header.
fn locate_points(rf: &mut RevitFile, wanted: &BTreeMap<u32, (f64, f64)>) -> Vec<Located> {
    let mut found: Vec<Located> = Vec::new();
    each_partition(rf, |stream, buf| {
        for (&id, &(x, y)) in wanted {
            let mut needle = [0u8; 16];
            needle[..8].copy_from_slice(&x.to_le_bytes());
            needle[8..].copy_from_slice(&y.to_le_bytes());
            for at in memchr::memmem::find_iter(buf, &needle) {
                found.push(Located {
                    id,
                    stream: stream.to_string(),
                    at,
                    header_distance: header_before(buf, at, id),
                    exact: true,
                    context: hex(&buf[at.saturating_sub(48)..buf.len().min(at + 64)]),
                });
            }
        }
    });
    let exact: BTreeSet<u32> = found.iter().map(|l| l.id).collect();
    let missing: BTreeMap<u32, (f64, f64)> = wanted
        .iter()
        .filter(|(id, _)| !exact.contains(*id))
        .map(|(id, point)| (*id, *point))
        .collect();
    if missing.is_empty() {
        return found;
    }
    each_partition(rf, |stream, buf| {
        for (&id, &(x, y)) in &missing {
            let mut header = [0u8; 8];
            header[..4].copy_from_slice(&[1, 0, 0, 0]);
            header[4..].copy_from_slice(&id.to_le_bytes());
            for start in memchr::memmem::find_iter(buf, &header) {
                let from = start + 8;
                let to = buf.len().min(from + POINT_WINDOW);
                for at in from..to.saturating_sub(15) {
                    let (Some(px), Some(py)) = (f64_at(buf, at), f64_at(buf, at + 8)) else {
                        continue;
                    };
                    if (px - x).abs() <= POINT_TOLERANCE && (py - y).abs() <= POINT_TOLERANCE {
                        found.push(Located {
                            id,
                            stream: stream.to_string(),
                            at,
                            header_distance: Some(at - from),
                            exact: false,
                            context: hex(&buf[at.saturating_sub(48)..buf.len().min(at + 64)]),
                        });
                    }
                }
            }
        }
    });
    found
}

/// The first bytes of the data of the first [`DUMPED_AREAS`] Areas, after
/// their `01 00 00 00 · id` header.
fn dump_data(rf: &mut RevitFile, c: &Census) {
    let ids: Vec<u32> = c.areas.keys().copied().take(DUMPED_AREAS).collect();
    let mut printed: BTreeMap<u32, usize> = BTreeMap::new();
    each_partition(rf, |stream, buf| {
        for &id in &ids {
            let mut header = [0u8; 8];
            header[..4].copy_from_slice(&[1, 0, 0, 0]);
            header[4..].copy_from_slice(&id.to_le_bytes());
            for start in memchr::memmem::find_iter(buf, &header) {
                let count = printed.entry(id).or_default();
                if *count >= 2 {
                    break;
                }
                *count += 1;
                let from = start + 8;
                println!(
                    "dump\t{id}\t{stream}\t{start}\t{}",
                    hex(&buf[from..buf.len().min(from + DUMP_BYTES)])
                );
            }
        }
    });
}

/// What the production reader returns for one Area: a verdict, and every field
/// but the record's own stream and offset.
struct ReaderRow {
    verdict: String,
    fields: BTreeMap<String, String>,
}

/// The reader's verdict on each Area it returns.
fn reader_rows(rf: &mut RevitFile, c: &Census) -> BTreeMap<u32, ReaderRow> {
    let level_ids: BTreeSet<u32> = c.recovered.iter().map(|level| level.element_id).collect();
    let mut out = BTreeMap::new();
    for element in areas_from_partition_chain_records(rf, c.release, &level_ids, &c.recovered) {
        let Some(id) = element.id else {
            continue;
        };
        let refusal = element
            .fields
            .iter()
            .find(|field| field.0 == AREA_BODY_REFUSAL_FIELD)
            .map(|field| format!("{:?}", field.1));
        let height = element
            .fields
            .iter()
            .find(|field| field.0 == AREA_BODY_HEIGHT_FIELD)
            .map(|field| format!("{:?}", field.1));
        let verdict = match (refusal, height) {
            (Some(reason), _) => format!("refused {reason}"),
            (None, Some(height)) => format!("body {height}"),
            (None, None) => "no body, no refusal".to_string(),
        };
        let fields = element
            .fields
            .iter()
            .filter(|field| field.0 != "m_source_stream" && field.0 != "m_source_offset")
            .map(|field| (field.0.clone(), format!("{:?}", field.1)))
            .collect();
        out.insert(id, ReaderRow { verdict, fields });
    }
    out
}

/// A point read from an older save's element data (RE-324).
struct Point35 {
    stream: String,
    level_word: u32,
    x: f64,
    y: f64,
}

/// The two `f64`s [`NARROW_POINT_OFFSET`] bytes after each `01 00 00 00 · id`
/// header of the Areas of a 32-bit save, with the `u32` that follows the
/// header.
fn points_35(rf: &mut RevitFile, c: &Census) -> BTreeMap<u32, Vec<Point35>> {
    let mut out: BTreeMap<u32, Vec<Point35>> = BTreeMap::new();
    each_partition(rf, |stream, buf| {
        for hit in memchr::memmem::find_iter(buf, &[1u8, 0, 0, 0]) {
            let Some(id) = u32_at(buf, hit + 4).filter(|id| c.areas.contains_key(id)) else {
                continue;
            };
            let data = hit + 8;
            let (Some(level_word), Some(x), Some(y)) = (
                u32_at(buf, data),
                f64_at(buf, data + NARROW_POINT_OFFSET),
                f64_at(buf, data + NARROW_POINT_OFFSET + 8),
            ) else {
                continue;
            };
            out.entry(id).or_default().push(Point35 {
                stream: stream.to_string(),
                level_word,
                x,
                y,
            });
        }
    });
    out
}

fn print_census(rf: &mut RevitFile, c: &Census) {
    println!(
        "release saved {} layout {} ({})",
        c.saved,
        c.release,
        if c.wide { "u64 ids" } else { "u32 ids" }
    );
    println!("declared ids {}", c.declared.len());
    println!(
        "category slot at record+{:#x}; offsets holding OST_Areas: {:?}",
        c.cat_at, c.histogram
    );
    for (id, (name, elevation, story)) in &c.levels {
        println!("level\t{id}\t{name}\t{elevation:.6}\t{story:?}");
    }
    println!(
        "design option sets {}; non-primary options {:?}",
        c.options, c.non_primary
    );
    for (category, (count, smallest, largest)) in &c.categories {
        println!(
            "cat\t{category}\t{}\t{count}\t{smallest}\t{largest}",
            builtin_category_name(*category).unwrap_or("?")
        );
    }
    println!(
        "areas\t{}\tarea schemes {}\tarea boundary line chain records {}\tmarker frames of Areas {}",
        c.areas.len(),
        c.scheme_ids.len(),
        c.line_ids.len(),
        c.area_marker_frames
    );
    for (id, area) in &c.areas {
        let (container, design, placement) = words(c, &area.bytes);
        let (refs, how) = refs_of(c, &area.bytes);
        println!(
            "area\t{id}\tcopies {}\t{}\t{}\tsize {}\tdisagree {}\tcontainer {container:x?}\tdesign {design:?}\tplacement {placement:x?}\tplaced {}\tnumber {:?}\tname {:?}\tvia {how}\tlevels [{}]\tschemes {:?}\tlines {:?}",
            area.copies,
            area.stream,
            area.offset,
            area.bytes.len(),
            area.disagree,
            is_placed(c, &area.bytes),
            area.number,
            area.name,
            describe_ids(c, &refs.levels),
            refs.schemes,
            refs.lines,
        );
        println!(
            "rec\t{id}\t{}\t{}\t{}",
            area.stream,
            area.offset,
            hex(&area.bytes)
        );
        if !c.wide {
            for (at, ids) in counted_lists(&area.bytes, c.cat_at + 28, &c.declared) {
                println!("list\t{id}\t{at:#x}\t{ids:?}");
            }
        }
    }
    println!("lines\t{}", c.lines.len());
    for record in &c.lines {
        let join = |ids: &[u64]| ids.iter().map(u64::to_string).collect::<Vec<_>>().join(",");
        println!(
            "line\t{}\t{}\t{}\t{}\t{}\t{}\t{:#x}\t{}\t{}\t{}",
            record.element_id,
            record.builtin_category,
            record.stream,
            record.offset,
            record
                .container_element_id()
                .map_or("-".to_string(), |v| v.to_string()),
            record.class_tag,
            record.placement_kind,
            record
                .bbox_feet
                .iter()
                .map(|v| format!("{v:.6}"))
                .collect::<Vec<_>>()
                .join(","),
            join(&record.references),
            record
                .design_option
                .map_or("-".to_string(), |v| v.to_string()),
        );
    }
    if !c.wide {
        dump_data(rf, c);
    }
    for (id, row) in reader_rows(rf, c) {
        println!("reader\t{id}\t{}", row.verdict);
    }
}

struct LineInfo {
    bbox: [f64; 6],
    levels: BTreeSet<u32>,
    schemes: BTreeSet<u32>,
    copies: usize,
}

fn line_infos(c: &Census) -> BTreeMap<u32, LineInfo> {
    let mut out: BTreeMap<u32, LineInfo> = BTreeMap::new();
    for record in &c.lines {
        if record.builtin_category != OST_AREA_SCHEME_LINES {
            continue;
        }
        let named: BTreeSet<u32> = record
            .references
            .iter()
            .filter_map(|r| u32::try_from(*r).ok())
            .collect();
        match out.get_mut(&record.element_id) {
            Some(held) => held.copies += 1,
            None => {
                out.insert(
                    record.element_id,
                    LineInfo {
                        bbox: record.bbox_feet,
                        levels: named
                            .iter()
                            .copied()
                            .filter(|v| c.levels.contains_key(v))
                            .collect(),
                        schemes: named
                            .iter()
                            .copied()
                            .filter(|v| c.scheme_ids.contains(v))
                            .collect(),
                        copies: 1,
                    },
                );
            }
        }
    }
    out
}

fn same_box(a: &[f64; 6], b: &[f64; 6]) -> bool {
    a.iter()
        .zip(b)
        .all(|(x, y)| (x - y).abs() <= LINE_TOLERANCE)
}

fn compare(older_rf: &mut RevitFile, older: &Census, newer_rf: &mut RevitFile, newer: &Census) {
    println!(
        "pair older release {} (saved {}) newer release {} (saved {})",
        older.release, older.saved, newer.release, newer.saved
    );
    let both: Vec<u32> = older
        .areas
        .keys()
        .filter(|id| newer.areas.contains_key(*id))
        .copied()
        .collect();
    let only_older: Vec<u32> = older
        .areas
        .keys()
        .filter(|id| !newer.areas.contains_key(*id))
        .copied()
        .collect();
    let only_newer: Vec<u32> = newer
        .areas
        .keys()
        .filter(|id| !older.areas.contains_key(*id))
        .copied()
        .collect();
    println!(
        "pair areas older {} newer {} both {} only older {only_older:?} only newer {only_newer:?}",
        older.areas.len(),
        newer.areas.len(),
        both.len()
    );

    let points = points_wide(newer_rf, newer);
    let wanted: BTreeMap<u32, (f64, f64)> = points
        .iter()
        .filter(|(id, _)| older.areas.contains_key(*id))
        .map(|(id, point)| (*id, *point))
        .collect();
    let located = if older.wide {
        Vec::new()
    } else {
        locate_points(older_rf, &wanted)
    };
    let older_reader = reader_rows(older_rf, older);
    let newer_reader = reader_rows(newer_rf, newer);
    let older_points35 = if older.wide {
        BTreeMap::new()
    } else {
        points_35(older_rf, older)
    };
    let (older_lines, newer_lines) = (line_infos(older), line_infos(newer));
    let single = |set: &BTreeSet<u32>| -> Option<u32> {
        let mut ids = set.iter();
        match (ids.next(), ids.next()) {
            (Some(id), None) => Some(*id),
            _ => None,
        }
    };
    let group_ids = |lines: &BTreeMap<u32, LineInfo>, level: u32, scheme: u32| -> BTreeSet<u32> {
        lines
            .iter()
            .filter(|(_, line)| line.levels.contains(&level) && line.schemes.contains(&scheme))
            .map(|(id, _)| *id)
            .collect()
    };
    let mut bodies: BTreeMap<(bool, bool), usize> = BTreeMap::new();
    let mut changed_lines: BTreeSet<u32> = BTreeSet::new();
    let mut named_by_area: BTreeSet<u32> = BTreeSet::new();
    let mut inputs_differ: Vec<String> = Vec::new();

    let (mut number_eq, mut name_eq, mut level_eq, mut scheme_eq, mut lines_eq, mut placed_eq) =
        (0, 0, 0, 0, 0, 0);
    let (mut design_eq, mut point_found, mut point_exact, mut reader_eq) = (0, 0, 0, 0);
    let mut header_distances: BTreeMap<usize, usize> = BTreeMap::new();
    for id in &both {
        let (a, b) = (&older.areas[id], &newer.areas[id]);
        let (ra, how_a) = refs_of(older, &a.bytes);
        let (rb, how_b) = refs_of(newer, &b.bytes);
        let (_, design_a, _) = words(older, &a.bytes);
        let (_, design_b, _) = words(newer, &b.bytes);
        let (placed_a, placed_b) = (is_placed(older, &a.bytes), is_placed(newer, &b.bytes));
        number_eq += usize::from(a.number == b.number);
        name_eq += usize::from(a.name == b.name);
        level_eq += usize::from(ra.levels == rb.levels);
        scheme_eq += usize::from(ra.schemes == rb.schemes);
        lines_eq += usize::from(ra.lines == rb.lines);
        placed_eq += usize::from(placed_a == placed_b);
        design_eq += usize::from(design_a == design_b);
        let hits: Vec<&Located> = located.iter().filter(|l| l.id == *id).collect();
        if !hits.is_empty() {
            point_found += 1;
        }
        if hits.iter().any(|l| l.exact) {
            point_exact += 1;
        }
        for hit in &hits {
            if let Some(distance) = hit.header_distance {
                *header_distances.entry(distance).or_default() += 1;
            }
        }
        let (row_a, row_b) = (older_reader.get(id), newer_reader.get(id));
        let (verdict_a, verdict_b) = (
            row_a.map(|row| row.verdict.as_str()),
            row_b.map(|row| row.verdict.as_str()),
        );
        reader_eq += usize::from(verdict_a == verdict_b);
        let key = (single(&ra.levels), single(&ra.schemes));
        let (group_a, group_b) = match key {
            (Some(level), Some(scheme)) => (
                group_ids(&older_lines, level, scheme),
                group_ids(&newer_lines, level, scheme),
            ),
            _ => (BTreeSet::new(), BTreeSet::new()),
        };
        let changed: Vec<u32> = group_a
            .union(&group_b)
            .filter(
                |line| match (older_lines.get(*line), newer_lines.get(*line)) {
                    (Some(x), Some(y)) => !same_box(&x.bbox, &y.bbox),
                    _ => true,
                },
            )
            .copied()
            .collect();
        let named_changed: Vec<u32> = changed
            .iter()
            .copied()
            .filter(|line| ra.lines.contains(line))
            .collect();
        changed_lines.extend(changed.iter().copied());
        named_by_area.extend(ra.lines.iter().copied());
        let newer_point = points.get(id).copied();
        let older_point = key.0.and_then(|level| {
            older_points35
                .get(id)?
                .iter()
                .find(|p| p.stream == a.stream && p.level_word == level)
                .map(|p| (p.x, p.y))
        });
        let point_equal = older_point.is_some() && older_point == newer_point;
        let inputs_equal = changed.is_empty() && point_equal;
        let fields_equal = match (row_a, row_b) {
            (Some(x), Some(y)) => x.fields == y.fields,
            _ => false,
        };
        let differing_fields: Vec<&String> = match (row_a, row_b) {
            (Some(x), Some(y)) => x
                .fields
                .keys()
                .chain(y.fields.keys())
                .filter(|name| x.fields.get(*name) != y.fields.get(*name))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            _ => Vec::new(),
        };
        *bodies.entry((inputs_equal, fields_equal)).or_default() += 1;
        if !inputs_equal {
            inputs_differ.push(format!(
                "{id} (number {:?}): changed lines {changed:?}, named and changed {named_changed:?}, point read {older_point:?} against {newer_point:?}",
                a.number
            ));
        }
        println!(
            "pair-body\t{id}\tinputs equal {inputs_equal}\tgroup lines {}/{}\tchanged {changed:?}\tnamed changed {named_changed:?}\tpoint35 {:?}\t2024 point {newer_point:?}\tfields equal {fields_equal}\tdiffering {differing_fields:?}\tverdicts {verdict_a:?}/{verdict_b:?}",
            group_a.len(),
            group_b.len(),
            older_points35.get(id).map(|points| points
                .iter()
                .map(|p| format!("{}:{}:{:.9},{:.9}", p.stream, p.level_word, p.x, p.y))
                .collect::<Vec<_>>()),
        );
        println!(
            "pair-area\t{id}\tnumber {:?}/{:?}\tname {:?}/{:?}\tlevel [{}]/[{}]\tscheme {:?}/{:?}\tlines {}/{} ({how_a}/{how_b}) equal {}\tplaced {placed_a}/{placed_b}\tdesign {design_a:?}/{design_b:?}\tpoint {:?}\tfound {:?}\treader {verdict_a:?}/{verdict_b:?}",
            a.number,
            b.number,
            a.name,
            b.name,
            describe_ids(older, &ra.levels),
            describe_ids(newer, &rb.levels),
            ra.schemes,
            rb.schemes,
            ra.lines.len(),
            rb.lines.len(),
            ra.lines == rb.lines,
            points.get(id),
            hits.iter()
                .map(|l| format!(
                    "{}@{}{}+{:?}",
                    l.stream,
                    l.at,
                    if l.exact { "" } else { "~" },
                    l.header_distance
                ))
                .collect::<Vec<_>>(),
        );
        if let Some(first) = hits.first() {
            println!("pair-context\t{id}\t{}", first.context);
        }
    }
    println!(
        "pair areas both {}: number equal {number_eq}, name equal {name_eq}, level equal {level_eq}, scheme equal {scheme_eq}, lines equal {lines_eq}, placed equal {placed_eq}, design option equal {design_eq}, reader verdict equal {reader_eq}",
        both.len()
    );
    println!(
        "pair points: the newer save reads {} of its {} Areas' points, {} of those Areas are in the older; point found in the older {point_found} (exact bytes {point_exact}); header distances {header_distances:?}",
        points.len(),
        newer.areas.len(),
        wanted.len()
    );
    let count = |inputs: bool, fields: bool| bodies.get(&(inputs, fields)).copied().unwrap_or(0);
    println!(
        "pair bodies: inputs equal (the Area's group lines and point equal the newer save's) {}: reader fields equal {}, differ {}; inputs differ {}: reader fields equal {}, differ {}",
        count(true, true) + count(true, false),
        count(true, true),
        count(true, false),
        count(false, true) + count(false, false),
        count(false, true),
        count(false, false),
    );
    println!(
        "pair edit: {} lines differ in box between the saves {changed_lines:?}; of them {} are named by some Area of the older save {:?}",
        changed_lines.len(),
        changed_lines.intersection(&named_by_area).count(),
        changed_lines
            .intersection(&named_by_area)
            .collect::<Vec<_>>(),
    );
    for line in &inputs_differ {
        println!("pair inputs differ: {line}");
    }

    for (id, (name_a, elevation_a, story_a)) in &older.levels {
        match newer.levels.get(id) {
            Some((name_b, elevation_b, story_b)) => println!(
                "pair-level\t{id}\tname {}\televation {}\tstory {story_a:?}/{story_b:?}\t{name_a}@{elevation_a:.6}/{name_b}@{elevation_b:.6}",
                name_a == name_b,
                (elevation_a - elevation_b).abs() <= 1e-6,
            ),
            None => println!("pair-level\t{id}\tonly older\t{name_a}@{elevation_a:.6}"),
        }
    }
    for id in newer
        .levels
        .keys()
        .filter(|id| !older.levels.contains_key(*id))
    {
        println!("pair-level\t{id}\tonly newer");
    }

    let mut box_eq = 0;
    let mut key_eq = 0;
    let mut copies_eq = 0;
    let mut in_both = 0;
    for (id, a) in &older_lines {
        let Some(b) = newer_lines.get(id) else {
            continue;
        };
        in_both += 1;
        box_eq += usize::from(same_box(&a.bbox, &b.bbox));
        key_eq += usize::from(a.levels == b.levels && a.schemes == b.schemes);
        copies_eq += usize::from(a.copies == b.copies);
    }
    println!(
        "pair lines: older {} newer {} (chain records {}/{}) both {in_both}: box equal {box_eq}, level and scheme equal {key_eq}, copy count equal {copies_eq}; only older {:?} only newer {:?}",
        older_lines.len(),
        newer_lines.len(),
        older.line_ids.len(),
        newer.line_ids.len(),
        older_lines
            .keys()
            .filter(|id| !newer_lines.contains_key(*id))
            .take(12)
            .collect::<Vec<_>>(),
        newer_lines
            .keys()
            .filter(|id| !older_lines.contains_key(*id))
            .take(12)
            .collect::<Vec<_>>(),
    );
}

fn main() -> anyhow::Result<()> {
    let paths: Vec<String> = std::env::args()
        .skip(1)
        .filter(|arg| !arg.starts_with("--"))
        .collect();
    match paths.as_slice() {
        [one] => {
            let (mut rf, census) = build(one)?;
            print_census(&mut rf, &census);
        }
        [older, newer] => {
            let (mut older_rf, older_census) = build(older)?;
            let (mut newer_rf, newer_census) = build(newer)?;
            compare(&mut older_rf, &older_census, &mut newer_rf, &newer_census);
        }
        _ => anyhow::bail!("usage: probe_re324_area_records_32bit OLDER.rvt [NEWER.rvt]"),
    }
    Ok(())
}
