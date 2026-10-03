//! RE-153 (probe): the text parameters stored after a room's number and name
//! (#35), against the `Pset_SpaceCommon` Revit's export writes.
//!
//! RE-117 reads a room's number and name from its parameter entries, `id ·
//! u32 n · UTF-16 × n` with a BuiltInParameter id as wide as the release's
//! ElementIds. Revit's IFC4 export of RE1 Architecture gives each of its 11
//! rooms a `Pset_SpaceCommon` with `FloorCovering`, the room's Floor Finish.
//! This lists every entry of that shape whose id is a BuiltInParameter
//! (-1,000,000 to -1,200,000) within [`WINDOW`] bytes past a room's name
//! entry, and the room's `Pset_SpaceCommon` from the reference export next
//! to the model (`<model>_slim.ifc`, else `<model>.ifc`), matched by room
//! number (the space's `Name`). It prints, per parameter id, how many rooms
//! carry it and on how many its value is each Revit property's, then the
//! first rooms in full.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re153_room_finishes -- MODEL.rvt ...

use rvt::RevitFile;
use rvt::partition_element_records as per;
use rvt::partition_room_parameters::{ROOM_NAME_PARAMETER, ROOM_NUMBER_PARAMETER};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// How far past a room's name entry its other entries are looked for.
const WINDOW: usize = 0x800;
const EXAMPLES: usize = 4;

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

fn probe(path: &str) -> anyhow::Result<Vec<String>> {
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
    let Some(layout) = rvt::partition_names::element_data_layout(revit) else {
        return Ok(vec![format!(
            "{{\"file\":{path:?},\"skipped\":\"no element-data layout\"}}"
        )]);
    };
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let rooms: BTreeSet<u32> =
        per::scan_category_records_multi(&mut rf, revit, &[per::OST_ROOMS], &declared)?
            .iter()
            .map(|r| r.element_id)
            .collect();
    let id_len = if layout.wide_id { 8 } else { 4 };
    let tag = |id: i64| -> Vec<u8> {
        if layout.wide_id {
            id.to_le_bytes().to_vec()
        } else {
            (id as i32).to_le_bytes().to_vec()
        }
    };
    let (number_tag, name_tag) = (tag(ROOM_NUMBER_PARAMETER), tag(ROOM_NAME_PARAMETER));
    let param_at = |buf: &[u8], at: usize| -> Option<i64> {
        let id = if layout.wide_id {
            i64::from_le_bytes(buf.get(at..at + 8)?.try_into().ok()?)
        } else {
            i64::from(i32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?))
        };
        (-1_200_000..=-1_000_000).contains(&id).then_some(id)
    };
    let string_at = |buf: &[u8], at: usize| -> Option<(String, usize)> {
        let n = u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?) as usize;
        if !(1..=256).contains(&n) {
            return None;
        }
        let units: Vec<u16> = buf
            .get(at + 4..at + 4 + 2 * n)?
            .chunks_exact(2)
            .map(|p| u16::from_le_bytes([p[0], p[1]]))
            .collect();
        let text = String::from_utf16(&units).ok()?;
        (!text.chars().any(char::is_control)).then_some((text, at + 4 + 2 * n))
    };
    // room -> (number, name, [(param, value, offset past the name entry)])
    let mut found: BTreeMap<u32, (String, String, Vec<(i64, String, usize)>)> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for hit in memchr::memmem::find_iter(buf, &[1u8, 0, 0, 0]) {
            let Some(id) = layout.id_at(buf, hit + 4).filter(|id| rooms.contains(id)) else {
                continue;
            };
            if found.contains_key(&id) {
                continue;
            }
            let start = hit + 4 + id_len;
            let end = (start + 0x400).min(buf.len());
            let Some(window) = buf.get(start..end) else {
                continue;
            };
            let entry = memchr::memmem::find_iter(window, &number_tag).find_map(|at| {
                let at = start + at;
                let (number, after) = string_at(buf, at + number_tag.len())?;
                (buf.get(after..after + name_tag.len())? == &name_tag[..]).then_some(())?;
                let (name, past) = string_at(buf, after + name_tag.len())?;
                Some((number, name, past))
            });
            let Some((number, name, past)) = entry else {
                continue;
            };
            let mut entries = Vec::new();
            let stop = (past + WINDOW).min(buf.len());
            let mut at = past;
            while at + id_len + 4 <= stop {
                if let Some(param) = param_at(buf, at) {
                    if let Some((value, next)) = string_at(buf, at + id_len) {
                        entries.push((param, value, at - past));
                        at = next;
                        continue;
                    }
                }
                at += 1;
            }
            found.insert(id, (number, name, entries));
        }
    }
    // param -> (rooms carrying it, matches per Revit property)
    let mut by_param: BTreeMap<i64, (usize, BTreeMap<String, usize>)> = BTreeMap::new();
    let mut matched_rooms = 0;
    for (number, _, entries) in found.values() {
        let revit_props = revit_spaces.get(number);
        matched_rooms += usize::from(revit_props.is_some());
        for (param, value, _) in entries {
            let row = by_param.entry(*param).or_default();
            row.0 += 1;
            for (prop, revit_value) in revit_props.into_iter().flatten() {
                if revit_value == value {
                    *row.1.entry(prop.clone()).or_default() += 1;
                }
            }
        }
    }
    let params: Vec<String> = by_param
        .iter()
        .map(|(p, (n, m))| {
            let m: Vec<String> = m.iter().map(|(k, v)| format!("{k:?}:{v}")).collect();
            format!(
                "\"{p}\":{{\"rooms\":{n},\"equals_revit\":{{{}}}}}",
                m.join(",")
            )
        })
        .collect();
    let mut out = vec![format!(
        "{{\"file\":{path:?},\"revit\":{revit},\"rooms\":{},\"rooms_read\":{},\"revit_spaces_with_pset\":{},\
         \"rooms_matched_by_number\":{matched_rooms},\"params\":{{{}}}}}",
        rooms.len(),
        found.len(),
        revit_spaces.len(),
        params.join(",")
    )];
    for (id, (number, name, entries)) in found.iter().take(EXAMPLES) {
        let e: Vec<String> = entries
            .iter()
            .map(|(p, v, at)| format!("[{p},{v:?},{at}]"))
            .collect();
        let revit_props = revit_spaces
            .get(number)
            .map(|m| {
                m.iter()
                    .map(|(k, v)| format!("{k:?}:{v:?}"))
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .unwrap_or_default();
        out.push(format!(
            "{{\"room\":{id},\"number\":{number:?},\"name\":{name:?},\"entries\":[{}],\"revit\":{{{revit_props}}}}}",
            e.join(",")
        ));
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
        match probe(path) {
            Ok(lines) => lines.iter().for_each(|l| println!("{l}")),
            Err(error) => println!("{{\"file\":{path:?},\"error\":{:?}}}", format!("{error:#}")),
        }
    }
}
