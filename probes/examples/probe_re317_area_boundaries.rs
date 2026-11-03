//! RE-317: where a Revit 2024 project keeps an Area's boundary.
//!
//! FACT under test: Revit's export draws each Area as an
//! `IfcExtrudedAreaSolid` of the Area's boundary, from its Level to the next
//! storey (RE-191 §3). The Area stores no boundary. Its chain record (RE-35)
//! names its Level, its area scheme (`OST_AreaSchemes`, -2003201) and two
//! elements of `OST_AreaSchemeLines` (-2000079), area boundary lines, each a
//! chain record with a bounding box of its own: for a straight line along an
//! axis, the line itself. The two `f64`s at byte 107 of the payload of the
//! Area's data object (RE-153) are a point of plan feet inside the Area. The
//! Area's region is the one of the arrangement of its scheme's lines on its
//! Level that holds the point, less the loops directly inside it. Verify with
//! `tools/re/area_boundaries_vs_ifc.py <probe output> <Revit's export>`, which
//! prints Revit's profile of each Area, its depth against the storeys, whether
//! the point lies in it, and the lines on its outline.
//!
//! This probe prints, tab-separated:
//!
//! - `level`: every recovered Level, with its elevation and its Building Story
//!   setting (`Some(true)`, `Some(false)` or `None` where it is not read);
//! - `cat`: a census of the chain records' categories;
//! - `area`: every Area's ElementId, GlobalId, number, name, the one Level its
//!   record names (`-` for none or several, and ids below 1000 are not read
//!   here), whether its placement word is the placed-instance kind, and the ids
//!   its record names with their categories;
//! - `rec`, `obj`: the bytes of its chain record and of each data object it
//!   owns (RE-153);
//! - `occ`: where else its ElementId appears as a `u64`, with the chain record
//!   that holds it;
//! - `solid`: the stored solids (RE-101's layout) keyed by an Area or by an
//!   element its record names;
//! - `chain`: the bytes of every chain record an Area names, and of a sample of
//!   the area boundary lines and polylines;
//! - `line`: every area boundary line and area polyline whose element record
//!   decodes: category, bounding box, references, owner.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re317_area_boundaries -- FILE.rvt

use rvt::RevitFile;
use rvt::research::partition_element_records::{
    BUILTIN_CATEGORY_MAX, BUILTIN_CATEGORY_MIN, CATEGORY_OFFSET, PLACEMENT_KIND_INSTANCE,
    PLACEMENT_KIND_OFFSET, builtin_category_name, file_bbox_marker, partition_record_chain,
    scan_category_records_multi,
};
use rvt::research::partition_level_records::recover_partition_levels;
use rvt::research::partition_room_boundaries::solid_headers_by_id;
use rvt::research::partition_room_parameters::{
    DATA_OBJECT_HEADER, data_objects, scan_room_parameter_entries,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const OST_AREAS: i64 = -2_003_200;
const OST_AREA_SCHEME_LINES: i64 = -2_000_079;
const OST_AREA_POLYLINES: i64 = -2_000_830;

const OCCURRENCES_PER_AREA: usize = 40;
const OBJECT_HEX_BYTES: usize = 2048;
const CHAIN_HEX_BYTES: usize = 640;
const UNREFERENCED_LINE_SAMPLE: usize = 30;

/// A chain record: start, end, ElementId, category.
type Span = (usize, usize, u32, i64);

/// One copy of an Area's chain record: stream, offset, bytes.
type AreaCopy = (String, usize, Vec<u8>);

/// A chain record of an area boundary line or polyline: ElementId, category,
/// stream, offset, size, its first bytes.
type LineRecord = (u32, i64, String, usize, usize, Vec<u8>);

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    buf.get(at..at.checked_add(4)?)
        .map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")))
}

fn i64_at(buf: &[u8], at: usize) -> Option<i64> {
    buf.get(at..at.checked_add(8)?)
        .map(|b| i64::from_le_bytes(b.try_into().expect("8 bytes")))
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

fn main() -> rvt::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("usage: FILE.rvt"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let Some(marker) = file_bbox_marker(&mut rf, version) else {
        println!("release {version}: no record marker");
        return Ok(());
    };
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let recovered = recover_partition_levels(&mut rf, version)?;
    let stories: BTreeMap<u32, Option<bool>> = recovered
        .iter()
        .map(|level| (level.element_id, level.building_story))
        .collect();
    let levels: BTreeMap<u32, (String, f64)> = recovered
        .into_iter()
        .map(|level| (level.element_id, (level.name, level.elevation_feet)))
        .collect();
    let global_ids = rvt::research::revit_global_ids::revit_global_ids(&mut rf).unwrap_or_default();
    println!("release {version}");
    for (id, (name, elevation)) in &levels {
        let story = stories.get(id).copied().flatten();
        println!("level\t{id}\t{name}\t{elevation:.6}\t{story:?}");
    }

    let streams = rf.partition_stream_names();
    let mut census: BTreeMap<i64, (usize, usize, usize)> = BTreeMap::new();
    let mut category_of: BTreeMap<u32, i64> = BTreeMap::new();
    let mut spans_by_stream: BTreeMap<String, Vec<Span>> = BTreeMap::new();
    let mut area_copies: BTreeMap<u32, Vec<AreaCopy>> = BTreeMap::new();
    let mut line_chain: Vec<LineRecord> = Vec::new();
    for stream in &streams {
        let Ok(inflated) = rf.inflated_partition(stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let spans = spans_by_stream.entry(stream.clone()).or_default();
        for span in partition_record_chain(buf, &marker) {
            let Ok(id) = u32::try_from(span.element_id) else {
                continue;
            };
            if !declared.contains(&id) {
                continue;
            }
            let Some(category) = i64_at(buf, span.start + CATEGORY_OFFSET)
                .filter(|c| (BUILTIN_CATEGORY_MIN..=BUILTIN_CATEGORY_MAX).contains(c))
            else {
                continue;
            };
            let size = span.end - span.start;
            let entry = census.entry(category).or_insert((0, usize::MAX, 0));
            entry.0 += 1;
            entry.1 = entry.1.min(size);
            entry.2 = entry.2.max(size);
            category_of.entry(id).or_insert(category);
            spans.push((span.start, span.end, id, category));
            if category == OST_AREAS {
                area_copies.entry(id).or_default().push((
                    stream.clone(),
                    span.start,
                    buf[span.start..span.end].to_vec(),
                ));
            }
            if category == OST_AREA_SCHEME_LINES || category == OST_AREA_POLYLINES {
                line_chain.push((
                    id,
                    category,
                    stream.clone(),
                    span.start,
                    size,
                    buf[span.start..span.end.min(span.start + CHAIN_HEX_BYTES)].to_vec(),
                ));
            }
        }
    }
    for (category, (count, smallest, largest)) in &census {
        println!(
            "cat\t{category}\t{}\t{count}\t{smallest}\t{largest}",
            builtin_category_name(*category).unwrap_or("?")
        );
    }

    let area_ids: BTreeSet<u32> = area_copies.keys().copied().collect();
    println!(
        "areas\t{}\tarea boundary line chain records {}",
        area_ids.len(),
        line_chain
            .iter()
            .filter(|l| l.1 == OST_AREA_SCHEME_LINES)
            .count()
    );
    let entries = scan_room_parameter_entries(&mut rf, version, &area_ids);
    let mut referenced: BTreeSet<u32> = BTreeSet::new();
    for (id, copies) in &area_copies {
        let Some(latest) = copies.iter().max_by_key(|c| partition_number(&c.0)) else {
            continue;
        };
        let bytes = &latest.2;
        let mut named_levels = BTreeSet::new();
        let mut refs = Vec::new();
        for at in CATEGORY_OFFSET..bytes.len().saturating_sub(7) {
            let Some(value) = i64_at(bytes, at)
                .and_then(|v| u32::try_from(v).ok())
                .filter(|v| *v > 1000 && v != id && declared.contains(v))
            else {
                continue;
            };
            if levels.contains_key(&value) {
                named_levels.insert(value);
                refs.push(format!("{at:#x}:{value}:level"));
            } else if let Some(category) = category_of.get(&value) {
                referenced.insert(value);
                refs.push(format!("{at:#x}:{value}:{category}"));
            } else {
                refs.push(format!("{at:#x}:{value}:-"));
            }
        }
        let level = match named_levels.iter().collect::<Vec<_>>().as_slice() {
            [only] => format!("{only}\t{:.6}", levels[*only].1),
            _ => "-\t-".to_string(),
        };
        let (number, name) = entries
            .get(id)
            .map_or(("-", "-"), |(n, m)| (n.as_str(), m.as_str()));
        let placed = copies
            .iter()
            .all(|c| u32_at(&c.2, PLACEMENT_KIND_OFFSET) == Some(PLACEMENT_KIND_INSTANCE));
        println!(
            "area\t{id}\t{}\t{number}\t{name}\t{level}\t{}\t{}",
            global_ids.get(id).map_or("-", String::as_str),
            if placed { "placed" } else { "not placed" },
            refs.join(",")
        );
        println!("rec\t{id}\t{}\t{}\t{}", latest.0, latest.1, hex(bytes));
    }

    let mut interesting: BTreeSet<u32> = area_ids.clone();
    interesting.extend(&referenced);
    let mut occurrences: BTreeMap<u32, usize> = BTreeMap::new();
    for stream in &streams {
        let Ok(inflated) = rf.inflated_partition(stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let spans = spans_by_stream
            .get(stream)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        for (at, object) in data_objects(buf) {
            if area_ids.contains(&object.element_id) {
                let payload = &buf[at + DATA_OBJECT_HEADER..object.end];
                println!(
                    "obj\t{}\t{stream}\t{at}\t{:#x}\t{}\t{}",
                    object.element_id,
                    object.class,
                    payload.len(),
                    hex(&payload[..payload.len().min(OBJECT_HEX_BYTES)])
                );
            }
        }
        for &id in &area_ids {
            let needle = u64::from(id).to_le_bytes();
            for at in memchr::memmem::find_iter(buf, &needle) {
                let count = occurrences.entry(id).or_insert(0);
                if *count >= OCCURRENCES_PER_AREA {
                    break;
                }
                *count += 1;
                let holder = spans
                    .partition_point(|s| s.0 <= at)
                    .checked_sub(1)
                    .and_then(|i| spans.get(i))
                    .filter(|s| at < s.1);
                let (record, category, relative) = match holder {
                    Some(s) => (s.2.to_string(), s.3.to_string(), (at - s.0).to_string()),
                    None => ("-".to_string(), "-".to_string(), "-".to_string()),
                };
                println!(
                    "occ\t{id}\t{stream}\t{at}\t{record}\t{category}\t{relative}\t{}",
                    hex(&buf[at.saturating_sub(16)..buf.len().min(at + 56)])
                );
            }
        }
        let solids = solid_headers_by_id(buf);
        for &id in &interesting {
            if let Some(headers) = solids.get(&u64::from(id)) {
                println!("solid\t{id}\t{stream}\t{}", headers.len());
            }
        }
        for &(start, end, id, category) in spans {
            if referenced.contains(&id) && category != OST_AREAS {
                println!(
                    "chain\t{id}\t{category}\t{stream}\t{start}\t{}\t{}",
                    end - start,
                    hex(&buf[start..end.min(start + CHAIN_HEX_BYTES)])
                );
            }
        }
    }
    let mut sampled = 0;
    for (id, category, stream, start, size, bytes) in &line_chain {
        if referenced.contains(id) || sampled >= UNREFERENCED_LINE_SAMPLE {
            continue;
        }
        sampled += 1;
        println!(
            "chain\t{id}\t{category}\t{stream}\t{start}\t{size}\t{}",
            hex(bytes)
        );
    }

    let lines = scan_category_records_multi(
        &mut rf,
        version,
        &[OST_AREA_SCHEME_LINES, OST_AREA_POLYLINES],
        &declared,
    )?;
    println!("lines\t{}", lines.len());
    for record in &lines {
        let join = |ids: &[u64]| ids.iter().map(u64::to_string).collect::<Vec<_>>().join(",");
        println!(
            "line\t{}\t{}\t{}\t{}\t{}\t{}\t{:#x}\t{}\t{}\t{}\t{}",
            record.element_id,
            record.builtin_category,
            record.stream,
            record.offset,
            record.class_tag,
            record
                .container_element_id()
                .map_or("-".to_string(), |c| c.to_string()),
            record.placement_kind,
            record
                .bbox_feet
                .iter()
                .map(|v| format!("{v:.6}"))
                .collect::<Vec<_>>()
                .join(","),
            record
                .owner_reference
                .map_or("-".to_string(), |o| o.to_string()),
            join(&record.references),
            record
                .design_option
                .map_or("-".to_string(), |d| d.to_string()),
        );
    }
    Ok(())
}
