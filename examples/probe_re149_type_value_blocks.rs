//! RE-149 (probe): what the value block of a placed family type holds when
//! rvt-rs reads no geometry materials for it (#355).
//!
//! `partition_type_materials` reads a type's geometry-material map (RE-82,
//! puzzbobb's `FamilySymbol.m_geomTag2MaterialId`) from the type's value
//! block, and a map whose entries are all unset (`0xff` x 8) reads as no
//! map: the type then counts as drawing no geometry (RE-84). Revit's export
//! gives 107 of Core Interior's columns and almost every RE1 fitting and
//! terminal the material `<Unnamed>`.
//!
//! For every family type that placed columns or products use and that has
//! no `m_type_material`, this prints the instances, their class, the
//! number of value blocks the type owns, and every candidate map in them
//! (`u32 n` in 1..=64 followed by `n x (u32 key, u64 value)` that fits the
//! block), with its keys and how many values are unset, plus the first 96
//! bytes after the first block's mark, in hex.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re149_type_value_blocks -- MODEL.rvt

use rvt::RevitFile;
use rvt::partition_schema_mvp::{self, TYPE_ID_FIELD, TYPE_MATERIAL_FIELD};
use rvt::walker::{InstanceField, WalkerLimits};
use std::collections::BTreeMap;

/// The bytes that follow an owner id to open its value block (RE-77).
fn mark() -> [u8; 59] {
    let mut m = [0xffu8; 59];
    m[56] = 0;
    m[57] = 0;
    m[58] = 0;
    m
}

const TYPES: usize = 12;

fn probe(path: &str) -> rvt::Result<Vec<String>> {
    let file = std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut rf = RevitFile::open(path)?;
    let revit = rf.basic_file_info()?.version;
    let mvp = partition_schema_mvp::recover_partition_schema_mvp(
        &mut rf,
        revit,
        WalkerLimits::default(),
    )?;
    // type id -> (instances, classes) for elements with no type material.
    let mut types: BTreeMap<u32, (usize, BTreeMap<String, usize>)> = BTreeMap::new();
    for element in mvp.columns.iter().chain(mvp.products.iter()) {
        let has_material = element.fields.iter().any(|(n, _)| n == TYPE_MATERIAL_FIELD);
        if has_material {
            continue;
        }
        let Some(type_id) = element.fields.iter().find_map(|(n, v)| match v {
            InstanceField::ElementId { id, .. } if n == TYPE_ID_FIELD => Some(*id),
            _ => None,
        }) else {
            continue;
        };
        let entry = types.entry(type_id).or_default();
        entry.0 += 1;
        *entry.1.entry(element.class.clone()).or_default() += 1;
    }
    let mark = mark();
    let mut out = vec![format!(
        "{{\"file\":{file:?},\"revit\":{revit},\"types_without_material\":{},\"instances\":{}}}",
        types.len(),
        types.values().map(|t| t.0).sum::<usize>()
    )];
    let mut ranked: Vec<_> = types.into_iter().collect();
    ranked.sort_by_key(|(_, (n, _))| std::cmp::Reverse(*n));
    let streams = rf.partition_stream_names();
    for (type_id, (instances, classes)) in ranked.into_iter().take(TYPES) {
        let owner = u64::from(type_id).to_le_bytes();
        let mut blocks = 0;
        let mut maps = Vec::new();
        let mut head = String::new();
        for stream in &streams {
            let Ok(inflated) = rf.inflated_partition(stream) else {
                continue;
            };
            let buf = inflated.bytes();
            for at in memchr::memmem::find_iter(buf, &mark) {
                if at < 8 || buf[at - 8..at] != owner {
                    continue;
                }
                blocks += 1;
                let start = at + mark.len();
                let end = (start + 4096).min(buf.len());
                if head.is_empty() {
                    head = buf[start..(start + 96).min(buf.len())]
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect();
                }
                let mut p = start;
                while p + 4 <= end && maps.len() < 6 {
                    let n = u32::from_le_bytes(buf[p..p + 4].try_into().expect("4")) as usize;
                    let map_end = p + 4 + n * 12;
                    if (1..=64).contains(&n) && map_end <= end {
                        let mut keys = Vec::new();
                        let mut unset = 0;
                        for k in 0..n {
                            let e = p + 4 + k * 12;
                            keys.push(u32::from_le_bytes(buf[e..e + 4].try_into().expect("4")));
                            unset += usize::from(buf[e + 4..e + 12].iter().all(|&b| b == 0xff));
                        }
                        let distinct = {
                            let mut k = keys.clone();
                            k.sort_unstable();
                            k.dedup();
                            k.len()
                        };
                        if unset == n && distinct == n {
                            maps.push(format!(
                                "{{\"offset\":{},\"n\":{n},\"keys\":{keys:?},\"unset\":{unset}}}",
                                p - start
                            ));
                        }
                    }
                    p += 1;
                }
            }
        }
        out.push(format!(
            "{{\"type\":{type_id},\"instances\":{instances},\"classes\":{classes:?},\"blocks\":{blocks},\
             \"all_unset_maps_with_distinct_keys\":[{}],\"head\":\"{head}\"}}",
            maps.join(",")
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
            Err(error) => println!("{{\"file\":{path:?},\"error\":{:?}}}", error.to_string()),
        }
    }
}
