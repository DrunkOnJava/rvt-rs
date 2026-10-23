//! RE-191: what a Revit 2024 project stores about each of its Areas (#763).
//!
//! FACT: an Area's partition chain record (RE-35) carries `OST_Areas`
//! (-2003200) at `+0x12` and no bounding-box marker at `+0x50`. Its container
//! slot at `+0x32` is unset and its placement word at `+0x42` is RE-21's
//! placed-instance kind on every Area Revit's export writes. Its record names
//! exactly one Level, and its data object holds RE-117's number and name
//! entries. On the BIM4LCA office the Areas matched with Revit's `IfcSpace` by
//! GlobalId have Revit's number, name and storey: 30 of 30 as built, and on
//! the building-permit file the 30 placed Areas, whose latest-partition
//! copies (#800) hold the names. The permit's 2 others carry `0xffff8020` and
//! are not in Revit's export. Verify with
//! `tools/re/area_spaces_vs_ifc.py <probe output> <Revit's export>`.
//!
//! This probe prints, for every Area:
//!
//! - its chain records, with each `u64` slot that holds a declared ElementId
//!   (named by the category of that id's own chain record, or as a Level);
//! - the byte offsets at which the Areas' first records differ;
//! - each verified data object (RE-153) the Area owns, with every
//!   `BuiltInParameter · u32 n · UTF-16 × n` string entry and every Level id
//!   in its payload;
//! - what the room readers return for the Area ids: the RE-29 parameter
//!   block and the RE-117 `ROOM_NUMBER` / `ROOM_NAME` entries;
//! - one tab-separated `area` line per Area: ElementId, Revit GlobalId,
//!   number, name, the one Level its records name (`-` for none or more than
//!   one), and whether its placement word (`+0x42`) is RE-21's placed
//!   instance kind. `tools/re/area_spaces_vs_ifc.py` pairs these lines with
//!   Revit's `IfcSpace` by GlobalId.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re191_area_records -- FILE.rvt

use rvt::RevitFile;
use rvt::research::partition_element_records::{
    BUILTIN_CATEGORY_MAX, BUILTIN_CATEGORY_MIN, CATEGORY_OFFSET, PLACEMENT_KIND_INSTANCE,
    PLACEMENT_KIND_OFFSET, builtin_category_name, file_bbox_marker, partition_record_chain,
};
use rvt::research::partition_level_records::recover_partition_levels;
use rvt::research::partition_room_parameters::{
    DATA_OBJECT_HEADER, data_objects, scan_room_parameter_entries, scan_room_parameters,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const OST_AREAS: i64 = -2_003_200;

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

fn utf16(buf: &[u8], at: usize, n: usize) -> Option<String> {
    let units: Vec<u16> = buf
        .get(at..at.checked_add(n.checked_mul(2)?)?)?
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let text = String::from_utf16(&units).ok()?;
    (!text.chars().any(char::is_control)).then_some(text)
}

/// One copy of an Area's chain record.
struct RecordCopy {
    stream: String,
    start: usize,
    bytes: Vec<u8>,
}

/// One data object an Area owns.
struct Object {
    stream: String,
    at: usize,
    class: u32,
    payload: Vec<u8>,
}

struct Names<'a> {
    category_of: &'a BTreeMap<u32, i64>,
    levels: &'a BTreeMap<u32, (String, f64)>,
    declared: &'a BTreeSet<u32>,
}

impl Names<'_> {
    fn describe(&self, id: u32) -> Option<String> {
        if let Some((name, elevation)) = self.levels.get(&id) {
            return Some(format!("{id} Level '{name}' {elevation:.4} ft"));
        }
        if let Some(category) = self.category_of.get(&id) {
            return Some(format!(
                "{id} {category} ({})",
                builtin_category_name(*category).unwrap_or("?")
            ));
        }
        self.declared
            .contains(&id)
            .then(|| format!("{id} declared, no chain record"))
    }

    /// Every `u64` slot in `bytes` that holds a declared id above 1000.
    fn references(&self, bytes: &[u8], base: usize, skip: Option<u32>) -> Vec<String> {
        let mut out = Vec::new();
        for at in 0..bytes.len().saturating_sub(7) {
            let Some(value) = i64_at(bytes, at) else {
                break;
            };
            let Ok(id) = u32::try_from(value) else {
                continue;
            };
            if id <= 1000 || Some(id) == skip {
                continue;
            }
            if let Some(text) = self.describe(id) {
                out.push(format!("+{:#x}: {text}", base + at));
            }
        }
        out
    }
}

/// Every `BuiltInParameter (i64) · u32 n · UTF-16 × n` entry in `bytes`.
fn string_entries(bytes: &[u8], base: usize) -> Vec<String> {
    let mut out = Vec::new();
    for at in 0..bytes.len().saturating_sub(12) {
        let Some(parameter) = i64_at(bytes, at) else {
            break;
        };
        if !(-1_300_000..=-1_000_000).contains(&parameter) {
            continue;
        }
        let Some(n) = u32_at(bytes, at + 8).map(|n| n as usize) else {
            continue;
        };
        if !(1..=256).contains(&n) {
            continue;
        }
        if let Some(text) = utf16(bytes, at + 12, n) {
            out.push(format!("+{:#x}: {parameter} = '{text}'", base + at));
        }
    }
    out
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
    let levels: BTreeMap<u32, (String, f64)> = recover_partition_levels(&mut rf, version)?
        .into_iter()
        .map(|level| (level.element_id, (level.name, level.elevation_feet)))
        .collect();
    let global_ids = rvt::research::revit_global_ids::revit_global_ids(&mut rf).unwrap_or_default();

    let mut category_of: BTreeMap<u32, i64> = BTreeMap::new();
    let mut area_records: BTreeMap<u32, Vec<RecordCopy>> = BTreeMap::new();
    let streams = rf.partition_stream_names();
    for stream in &streams {
        let Ok(inflated) = rf.inflated_partition(stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for span in partition_record_chain(buf, &marker) {
            let Ok(id) = u32::try_from(span.element_id) else {
                continue;
            };
            let Some(category) = i64_at(buf, span.start + CATEGORY_OFFSET)
                .filter(|c| (BUILTIN_CATEGORY_MIN..=BUILTIN_CATEGORY_MAX).contains(c))
            else {
                continue;
            };
            category_of.entry(id).or_insert(category);
            if category == OST_AREAS {
                area_records.entry(id).or_default().push(RecordCopy {
                    stream: stream.clone(),
                    start: span.start,
                    bytes: buf[span.start..span.end].to_vec(),
                });
            }
        }
    }
    let areas: BTreeSet<u32> = area_records.keys().copied().collect();
    let names = Names {
        category_of: &category_of,
        levels: &levels,
        declared: &declared,
    };
    println!(
        "release {version}: {} Areas, {} Levels recovered",
        areas.len(),
        levels.len()
    );
    for (id, (name, elevation)) in &levels {
        println!("  Level {id} '{name}' {elevation:.4} ft");
    }

    let firsts: Vec<&Vec<u8>> = area_records
        .values()
        .map(|copies| &copies[0].bytes)
        .collect();
    if let Some(shortest) = firsts.iter().map(|r| r.len()).min() {
        let varying: Vec<usize> = (0..shortest)
            .filter(|&at| firsts.iter().any(|r| r[at] != firsts[0][at]))
            .collect();
        let sizes: BTreeSet<usize> = firsts.iter().map(|r| r.len()).collect();
        println!("first records: sizes {sizes:?}; offsets that differ between Areas: {varying:x?}");
        if let Some(first) = firsts.first() {
            println!("first Area's record:");
            for (row, chunk) in first.chunks(16).enumerate() {
                println!("  +{:#05x} {}", row * 16, hex(chunk));
            }
        }
    }

    let mut objects: BTreeMap<u32, Vec<Object>> = BTreeMap::new();
    for stream in &streams {
        let Ok(inflated) = rf.inflated_partition(stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for (at, object) in data_objects(buf) {
            if areas.contains(&object.element_id) {
                objects.entry(object.element_id).or_default().push(Object {
                    stream: stream.clone(),
                    at,
                    class: object.class,
                    payload: buf[at + DATA_OBJECT_HEADER..object.end].to_vec(),
                });
            }
        }
    }

    let blocks = scan_room_parameters(&mut rf, version, &areas)?;
    let entries = scan_room_parameter_entries(&mut rf, version, &areas);
    println!(
        "room readers on the Area ids: RE-29 blocks {}, RE-117 entries {}",
        blocks.len(),
        entries.len()
    );

    for (id, copies) in &area_records {
        println!(
            "Area {id} GlobalId {}",
            global_ids.get(id).map_or("-", String::as_str)
        );
        if let Some(block) = blocks.get(id) {
            println!(
                "  RE-29 block: number '{}' name '{}' level slot {}",
                block.number,
                block.name,
                names
                    .describe(u32::try_from(block.level_slot).unwrap_or(0))
                    .unwrap_or_else(|| block.level_slot.to_string())
            );
        }
        if let Some((number, name)) = entries.get(id) {
            println!("  RE-117 entries: number '{number}' name '{name}'");
        }
        let level_ids = &levels;
        let named: BTreeSet<u32> = copies
            .iter()
            .flat_map(|copy| {
                let bytes = &copy.bytes;
                (0..bytes.len().saturating_sub(7)).filter_map(move |at| {
                    i64_at(bytes, at)
                        .and_then(|v| u32::try_from(v).ok())
                        .filter(|v| level_ids.contains_key(v))
                })
            })
            .collect();
        let level = match named.iter().collect::<Vec<_>>().as_slice() {
            [only] => levels[*only].0.clone(),
            _ => "-".to_string(),
        };
        let placed = copies.iter().all(|copy| {
            u32_at(&copy.bytes, PLACEMENT_KIND_OFFSET) == Some(PLACEMENT_KIND_INSTANCE)
        });
        let (number, name) = entries
            .get(id)
            .map_or(("-", "-"), |(n, m)| (n.as_str(), m.as_str()));
        println!(
            "area\t{id}\t{}\t{number}\t{name}\t{level}\t{}",
            global_ids.get(id).map_or("-", String::as_str),
            if placed { "placed" } else { "not placed" }
        );
        for copy in copies {
            println!(
                "  chain record {} {:#x} size {}",
                copy.stream,
                copy.start,
                copy.bytes.len()
            );
            for line in names.references(&copy.bytes, 0, Some(*id)) {
                println!("    {line}");
            }
            if copy.bytes.len() > 0x16 {
                println!("    tail +0x16: {}", hex(&copy.bytes[0x16..]));
            }
        }
        for object in objects.get(id).map(Vec::as_slice).unwrap_or(&[]) {
            println!(
                "  data object {} {:#x} class {:#x} payload {} bytes",
                object.stream,
                object.at,
                object.class,
                object.payload.len()
            );
            for line in string_entries(&object.payload, DATA_OBJECT_HEADER) {
                println!("    {line}");
            }
            for line in names.references(&object.payload, DATA_OBJECT_HEADER, Some(*id)) {
                if line.contains("Level") || line.contains("declared") {
                    println!("    {line}");
                }
            }
        }
    }
    Ok(())
}
