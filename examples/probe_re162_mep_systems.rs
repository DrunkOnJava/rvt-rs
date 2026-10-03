//! RE-162 (probe): where an MEP system and its members are stored (B33,
//! #528).
//!
//! Revit's IFC4 exports of the RE1 MEP models write one `IfcSystem` per MEP
//! system (5 on Mechanical, 8 on Plumbing, 13 on Electrical), named as in
//! Revit (`Mechanical Supply Air 1`, `OTH 1`), each grouping its elements by
//! `IfcRelAssignsToGroup`. rvt-rs writes none.
//!
//! For each system of the reference export the probe looks for its name as
//! a length-prefixed UTF-16 string in every partition, and takes the
//! verified data object holding each hit (RE-153) as a candidate system
//! element. For each candidate id it counts, among the system's members
//! (their `Tag`s), how many have a verified data object of their own that
//! holds the id as a `u32` or `u64`, and how many hold it anywhere in the
//! partition outside their objects.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re162_mep_systems -- MODEL.rvt ...

use rvt::RevitFile;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

fn split_args(args: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let (mut quoted, mut depth) = (false, 0usize);
    for c in args.chars() {
        match c {
            '\'' => {
                quoted = !quoted;
                current.push(c);
            }
            '(' if !quoted => {
                depth += 1;
                current.push(c);
            }
            ')' if !quoted => {
                depth = depth.saturating_sub(1);
                current.push(c);
            }
            ',' if !quoted && depth == 0 => out.push(std::mem::take(&mut current)),
            _ => current.push(c),
        }
    }
    out.push(current);
    out
}

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
        let args = args.trim_end().trim_end_matches(';');
        let args = args.strip_suffix(')').unwrap_or(args);
        if let Ok(id) = id.trim().parse() {
            out.insert(id, (entity.trim().to_string(), args.to_string()));
        }
    }
    out
}

fn refs(field: &str) -> Vec<u64> {
    field
        .split('#')
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

/// Each system's name and its members' `Tag`s.
fn systems(step: &str) -> Vec<(String, BTreeSet<u32>)> {
    let ents = entities(step);
    let mut out = Vec::new();
    for (entity, args) in ents.values() {
        if entity != "IFCRELASSIGNSTOGROUP" {
            continue;
        }
        let f = split_args(args);
        let Some((group_entity, group_args)) =
            f.get(6).and_then(|g| refs(g).first().copied()).and_then(|g| ents.get(&g))
        else {
            continue;
        };
        if group_entity != "IFCSYSTEM" {
            continue;
        }
        let name = split_args(group_args)
            .get(2)
            .map(|n| n.trim().trim_matches('\'').to_string())
            .unwrap_or_default();
        let members: BTreeSet<u32> = f
            .get(4)
            .map(|m| refs(m))
            .unwrap_or_default()
            .iter()
            .filter_map(|m| ents.get(m))
            .filter_map(|(_, a)| split_args(a).get(7)?.trim().trim_matches('\'').parse().ok())
            .collect();
        out.push((name, members));
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

/// Every verified data object in `b` (RE-153): `(start, end, ElementId)`.
fn objects(b: &[u8]) -> Vec<(usize, usize, u32, u32)> {
    let mut out = Vec::new();
    let mut p = 0;
    while p + 20 <= b.len() {
        if u32_at(b, p + 4) == Some(0) {
            if let (Some(id), Some(sum), Some(size), Some(class)) = (
                u32_at(b, p),
                u32_at(b, p + 8),
                u32_at(b, p + 12),
                u32_at(b, p + 16),
            ) {
                let size = size as usize;
                let end = p + 20 + size;
                if size >= 4 && end <= b.len() && u32_at(b, end - 4) == Some(size as u32) {
                    let mut data = class.to_le_bytes().to_vec();
                    data.extend_from_slice(&b[p + 20..end]);
                    data.truncate(size);
                    if adler32(&data) == sum {
                        out.push((p, end, id, class));
                    }
                }
            }
        }
        p += 1;
    }
    out
}

/// The ElementId and class word of the innermost object holding `at`.
fn owner(objs: &[(usize, usize, u32, u32)], at: usize) -> Option<(u32, u32)> {
    let index = objs.partition_point(|(start, ..)| *start <= at);
    objs[..index]
        .iter()
        .rev()
        .find(|(_, end, ..)| *end > at)
        .map(|(_, _, id, class)| (*id, *class))
}

fn probe(path: &str) -> anyhow::Result<Vec<String>> {
    let model = Path::new(path);
    let stem = model.file_stem().unwrap_or_default().to_string_lossy();
    let dir = model.parent().unwrap_or(Path::new("."));
    let reference = [format!("{stem}_slim.ifc"), format!("{stem}.ifc")]
        .into_iter()
        .map(|n| dir.join(n))
        .find(|p| p.exists());
    let Some(reference) = reference else {
        return Ok(vec![format!(
            "{{\"file\":{path:?},\"skipped\":\"no reference export\"}}"
        )]);
    };
    let systems = systems(&std::fs::read_to_string(&reference)?);
    if systems.is_empty() {
        return Ok(vec![format!("{{\"file\":{path:?},\"systems\":0}}")]);
    }
    let mut rf = RevitFile::open(path)?;
    let class_names: BTreeMap<u32, String> = rf
        .schema_classes()?
        .classes
        .into_iter()
        .map(|class| (u32::from(class.tag), class.name))
        .collect();
    let names = rf.element_names();
    let mut streams = Vec::new();
    for name in rf.partition_stream_names() {
        if let Ok(inflated) = rf.inflated_partition(&name) {
            let bytes = inflated.bytes().to_vec();
            let objs = objects(&bytes);
            streams.push((name, bytes, objs));
        }
    }
    let mut out = vec![format!(
        "{{\"file\":{path:?},\"systems\":{}}}",
        systems.len()
    )];
    for (name, members) in &systems {
        let mut needle = (name.encode_utf16().count() as u32).to_le_bytes().to_vec();
        needle.extend(name.encode_utf16().flat_map(u16::to_le_bytes));
        // candidate system id -> (name hits, class word)
        let mut candidates: BTreeMap<u32, (usize, u32)> = BTreeMap::new();
        for (_, b, objs) in &streams {
            for at in memchr::memmem::find_iter(b, &needle) {
                if let Some((id, class)) = owner(objs, at) {
                    candidates.entry(id).or_insert((0, class)).0 += 1;
                }
            }
        }
        let mut rows = Vec::new();
        for (candidate, (hits, class)) in &candidates {
            let mut in_own = BTreeSet::new();
            let mut anywhere = 0usize;
            for (_, b, objs) in &streams {
                for at in memchr::memmem::find_iter(b, &candidate.to_le_bytes()) {
                    anywhere += 1;
                    if let Some((id, _)) = owner(objs, at) {
                        if members.contains(&id) {
                            in_own.insert(id);
                        }
                    }
                }
            }
            if in_own.is_empty() {
                continue;
            }
            let class_name = class_names
                .get(&(class & 0xffff))
                .cloned()
                .unwrap_or_default();
            let entry = names
                .entries
                .get(candidate)
                .map(|e| format!("{e:?}"))
                .unwrap_or_default();
            rows.push(format!(
                "{{\"candidate\":{candidate},\"name_hits\":{hits},\"members_holding_it\":{},\"occurrences\":{anywhere},\"class\":\"{class:#x} {class_name}\",\"name_entry\":{:?}}}",
                in_own.len(),
                entry
            ));
        }
        out.push(format!(
            "{{\"system\":{name:?},\"members\":{},\"candidates\":[{}]}}",
            members.len(),
            rows.join(",")
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
