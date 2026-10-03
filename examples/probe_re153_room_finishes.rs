//! RE-153 (probe): a room's text parameters (#35), against the
//! `Pset_SpaceCommon` Revit's export writes.
//!
//! Revit's IFC4 export of RE1 Architecture gives each of its 11 rooms a
//! `Pset_SpaceCommon` with `FloorCovering`, the room's Floor Finish. A text
//! parameter is an entry `id · u32 n · UTF-16 × n` with a BuiltInParameter id
//! as wide as the release's ElementIds (RE-117). This finds every such entry
//! whose id is in [`PARAMETERS`] (the room parameters around `ROOM_NAME`,
//! -1006900) in every partition, and the data object holding it: the last
//! header `[i32 ElementId][i32 0][u32 Adler-32][i32 size][i32 class]` before
//! it whose `size` bytes end in the size again, enclose the entry, and whose
//! Adler-32 (over the class word and the payload less its last 4 bytes)
//! verifies (the #35 comment of 2026-09-29). It joins the object's ElementId
//! to the rooms (`OST_Rooms`), and each room by number to the space's
//! `Pset_SpaceCommon` in the reference export next to the model
//! (`<model>_slim.ifc`, else `<model>.ifc`). Per parameter id it prints the
//! entries, those inside a verified object, those on a room, and on how many
//! rooms the value equals each Revit property; then every room of the first
//! file with a reference in full. Revit 2024 and later (the header of 2023 has
//! no zero word).
//!
//! Usage:
//!   cargo run --profile ci --example probe_re153_room_finishes -- MODEL.rvt ...

use rvt::RevitFile;
use rvt::partition_element_records as per;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// The BuiltInParameter ids looked for: `ROOM_NAME` (-1006900) and its
/// neighbours.
const PARAMETERS: std::ops::RangeInclusive<i64> = -1_006_920..=-1_006_895;
/// How far before an entry its object's header is looked for.
const SEARCH: usize = 200_000;
/// Bytes of a data object's header.
const HEADER: usize = 20;

/// `#id -> (entity, args)` for every line of a STEP file.
fn entities(step: &str) -> BTreeMap<u64, (String, String)> {
    let mut out = BTreeMap::new();
    for line in step.lines() {
        let Some(rest) = line.strip_prefix('#') else {
            continue;
        };
        let Some((id, body)) = rest.split_once('=') else {
            continue;
        };
        let Some((entity, args)) = body.split_once('(') else {
            continue;
        };
        if let Ok(id) = id.trim().parse() {
            out.insert(id, (entity.trim().to_string(), args.to_string()));
        }
    }
    out
}

fn refs(args: &str) -> Vec<u64> {
    args.split('#')
        .skip(1)
        .filter_map(|s| {
            s.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse()
                .ok()
        })
        .collect()
}

fn quoted(args: &str) -> Vec<String> {
    args.split('\'')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

/// Room number -> property name -> value, for each space's
/// `Pset_SpaceCommon` in Revit's export.
fn space_common(step: &str) -> BTreeMap<String, BTreeMap<String, String>> {
    let ents = entities(step);
    let mut out: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for (entity, args) in ents.values() {
        if entity != "IFCRELDEFINESBYPROPERTIES" {
            continue;
        }
        let r = refs(args);
        let Some((set_entity, set_args)) = r.last().and_then(|id| ents.get(id)) else {
            continue;
        };
        if set_entity != "IFCPROPERTYSET"
            || quoted(set_args).get(1).map(String::as_str) != Some("Pset_SpaceCommon")
        {
            continue;
        }
        let props: BTreeMap<String, String> = refs(set_args)
            .iter()
            .skip(1)
            .filter_map(|id| ents.get(id))
            .filter_map(|(_, a)| {
                let q = quoted(a);
                Some((q.first()?.clone(), q.get(1)?.clone()))
            })
            .collect();
        for space in &r[1..r.len() - 1] {
            if let Some((e, a)) = ents.get(space) {
                if e == "IFCSPACE" {
                    if let Some(number) = quoted(a).get(1) {
                        out.insert(number.clone(), props.clone());
                    }
                }
            }
        }
    }
    out
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        b.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut s) = (1u32, 0u32);
    for chunk in data.chunks(5552) {
        for &x in chunk {
            a += u32::from(x);
            s += a;
        }
        a %= 65521;
        s %= 65521;
    }
    (s << 16) | a
}

/// The ElementId of the verified data object whose payload holds `at`.
fn enclosing_object(b: &[u8], at: usize) -> Option<u32> {
    let low = at.saturating_sub(SEARCH);
    let mut p = at.checked_sub(HEADER)?;
    loop {
        if let (Some(id), Some(0), Some(sum), Some(size), Some(class)) = (
            u32_at(b, p),
            u32_at(b, p + 4),
            u32_at(b, p + 8),
            u32_at(b, p + 12).map(|s| s as usize),
            u32_at(b, p + 16),
        ) {
            let end = p + HEADER + size;
            if size >= 4 && end > at && end <= b.len() && u32_at(b, end - 4) == Some(size as u32) {
                let mut data = class.to_le_bytes().to_vec();
                data.extend_from_slice(&b[p + HEADER..end]);
                data.truncate(size);
                if adler32(&data) == sum {
                    return Some(id);
                }
            }
        }
        if p <= low {
            return None;
        }
        p -= 1;
    }
}

fn string_at(buf: &[u8], at: usize) -> Option<String> {
    let n = u32_at(buf, at)? as usize;
    if !(1..=256).contains(&n) {
        return None;
    }
    let units: Vec<u16> = buf
        .get(at + 4..at + 4 + 2 * n)?
        .chunks_exact(2)
        .map(|p| u16::from_le_bytes([p[0], p[1]]))
        .collect();
    let text = String::from_utf16(&units).ok()?;
    (!text.chars().any(char::is_control)).then_some(text)
}

#[derive(Default)]
struct Tally {
    entries: usize,
    in_object: usize,
    on_room: usize,
    equals: BTreeMap<String, usize>,
}

fn probe(path: &str, full: bool) -> anyhow::Result<Vec<String>> {
    let model = Path::new(path);
    let stem = model.file_stem().unwrap_or_default().to_string_lossy();
    let dir = model.parent().unwrap_or(Path::new("."));
    let reference = [format!("{stem}_slim.ifc"), format!("{stem}.ifc")]
        .into_iter()
        .map(|n| dir.join(n))
        .find(|p| p.exists());
    let revit_spaces = match &reference {
        Some(p) => space_common(&std::fs::read_to_string(p)?),
        None => BTreeMap::new(),
    };
    let mut rf = RevitFile::open(path)?;
    let revit = rf.basic_file_info()?.version;
    if revit < 2024 {
        return Ok(vec![format!(
            "{{\"file\":{path:?},\"revit\":{revit},\"skipped\":\"before 2024\"}}"
        )]);
    }
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let rooms: BTreeSet<u32> =
        per::scan_category_records_multi(&mut rf, revit, &[per::OST_ROOMS], &declared)?
            .iter()
            .map(|r| r.element_id)
            .collect();
    // room -> param -> values
    let mut by_room: BTreeMap<u32, BTreeMap<i64, BTreeSet<String>>> = BTreeMap::new();
    let mut tallies: BTreeMap<i64, Tally> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for param in PARAMETERS {
            for hit in memchr::memmem::find_iter(buf, &param.to_le_bytes()) {
                let Some(value) = string_at(buf, hit + 8) else {
                    continue;
                };
                let tally = tallies.entry(param).or_default();
                tally.entries += 1;
                let Some(id) = enclosing_object(buf, hit) else {
                    continue;
                };
                tally.in_object += 1;
                if rooms.contains(&id) {
                    tally.on_room += 1;
                    by_room
                        .entry(id)
                        .or_default()
                        .entry(param)
                        .or_default()
                        .insert(value);
                }
            }
        }
    }
    let number_of = |values: &BTreeMap<i64, BTreeSet<String>>| {
        values
            .get(&rvt::partition_room_parameters::ROOM_NUMBER_PARAMETER)
            .filter(|v| v.len() == 1)
            .and_then(|v| v.iter().next().cloned())
    };
    let mut matched = 0;
    for values in by_room.values() {
        let Some(revit_props) = number_of(values).and_then(|n| revit_spaces.get(&n)) else {
            continue;
        };
        matched += 1;
        for (param, set) in values {
            for (prop, revit_value) in revit_props {
                if set.len() == 1 && set.contains(revit_value) {
                    *tallies
                        .entry(*param)
                        .or_default()
                        .equals
                        .entry(prop.clone())
                        .or_default() += 1;
                }
            }
        }
    }
    let params: Vec<String> = tallies
        .iter()
        .map(|(p, t)| {
            let eq: Vec<String> = t.equals.iter().map(|(k, v)| format!("{k:?}:{v}")).collect();
            format!(
                "\"{p}\":{{\"entries\":{},\"in_verified_object\":{},\"on_room\":{},\"equals_revit\":{{{}}}}}",
                t.entries,
                t.in_object,
                t.on_room,
                eq.join(",")
            )
        })
        .collect();
    let mut out = vec![format!(
        "{{\"file\":{path:?},\"revit\":{revit},\"rooms\":{},\"rooms_with_entries\":{},\
         \"revit_spaces_with_pset\":{},\"rooms_matched_by_number\":{matched},\"params\":{{{}}}}}",
        rooms.len(),
        by_room.len(),
        revit_spaces.len(),
        params.join(",")
    )];
    if full {
        for (room, values) in &by_room {
            let v: Vec<String> = values
                .iter()
                .map(|(p, set)| format!("\"{p}\":{:?}", set.iter().collect::<Vec<_>>()))
                .collect();
            let revit_props = number_of(values)
                .and_then(|n| revit_spaces.get(&n))
                .map(|m| {
                    m.iter()
                        .map(|(k, v)| format!("{k:?}:{v:?}"))
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .unwrap_or_default();
            out.push(format!(
                "{{\"room\":{room},\"values\":{{{}}},\"revit\":{{{revit_props}}}}}",
                v.join(",")
            ));
        }
    }
    Ok(out)
}

fn main() {
    // Measure passes flags such as `--records` after the paths.
    let paths: Vec<String> = std::env::args()
        .skip(1)
        .filter(|arg| !arg.starts_with("--"))
        .collect();
    for path in &paths {
        let full = !path.contains("Core_Interior");
        match probe(path, full) {
            Ok(lines) => lines.iter().for_each(|l| println!("{l}")),
            Err(error) => println!("{{\"file\":{path:?},\"error\":{:?}}}", format!("{error:#}")),
        }
    }
}
