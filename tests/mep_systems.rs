//! RE-162 (B33, #528): rvt-rs writes Revit's MEP systems, each grouping
//! Revit's members.
//!
//! Revit's IFC4 exports of the RE1 MEP models write one `IfcSystem` per MEP
//! system (5 on Mechanical, 8 on Plumbing, 13 on Electrical), named as in
//! Revit (`Mechanical Supply Air 1`, `CW 1`, the circuit `1`), each grouping
//! its members by `IfcRelAssignsToGroup`. For each of them rvt-rs must write
//! an `IfcSystem` of the same name grouping the same elements, among the
//! elements both exports hold (by `Tag`).
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR`: `RE1-<discipline>.rvt` with
//! `RE1-<discipline>.ifc`. Skips what is absent.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

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
        let args = args.trim_end().trim_end_matches(';');
        let args = args.strip_suffix(')').unwrap_or(args);
        if let Ok(id) = id.trim().parse() {
            out.insert(id, (entity.trim().to_string(), args.to_string()));
        }
    }
    out
}

/// Split a STEP argument list on top-level commas.
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

fn reference(field: &str) -> Option<u64> {
    field.trim().strip_prefix('#')?.parse().ok()
}

fn list(field: &str) -> Vec<u64> {
    split_args(field.trim().trim_start_matches('(').trim_end_matches(')'))
        .iter()
        .filter_map(|f| reference(f))
        .collect()
}

fn tag_of(ents: &BTreeMap<u64, (String, String)>, id: u64) -> Option<String> {
    let (_, args) = ents.get(&id)?;
    let tag = split_args(args)
        .get(7)?
        .trim()
        .trim_matches('\'')
        .to_string();
    tag.parse::<u64>().is_ok().then_some(tag)
}

/// Each `IfcSystem`'s name and the `Tag`s of the elements it groups (one
/// entry per system: Revit names several circuits `1`), and every element
/// `Tag` in the file.
fn systems(step: &str) -> (Vec<(String, BTreeSet<String>)>, BTreeSet<String>) {
    let ents = entities(step);
    let mut by_group: BTreeMap<u64, (String, BTreeSet<String>)> = BTreeMap::new();
    for (entity, args) in ents.values() {
        if entity != "IFCRELASSIGNSTOGROUP" {
            continue;
        }
        let f = split_args(args);
        let Some(group) = f.get(6).and_then(|g| reference(g)) else {
            continue;
        };
        let Some((group_entity, group_args)) = ents.get(&group) else {
            continue;
        };
        if group_entity != "IFCSYSTEM" {
            continue;
        }
        let name = split_args(group_args)
            .get(2)
            .map(|n| n.trim().trim_matches('\'').to_string())
            .unwrap_or_default();
        let members = list(f.get(4).map(String::as_str).unwrap_or(""))
            .into_iter()
            .filter_map(|m| tag_of(&ents, m));
        let entry = by_group
            .entry(group)
            .or_insert_with(|| (name, BTreeSet::new()));
        entry.1.extend(members);
    }
    let tags = ents.keys().filter_map(|id| tag_of(&ents, *id)).collect();
    (by_group.into_values().collect(), tags)
}

#[test]
fn re1_mep_systems_are_revits() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    for (model, expected) in [("Mechanical", 5), ("Plumbing", 8), ("Electrical", 13)] {
        let rvt = dir.join(format!("RE1-{model}.rvt"));
        let reference_ifc = dir.join(format!("RE1-{model}.ifc"));
        if !rvt.exists() || !reference_ifc.exists() {
            eprintln!("skipping RE1 {model}: model or reference export absent");
            continue;
        }
        let (theirs, theirs_written) =
            systems(&std::fs::read_to_string(&reference_ifc).expect("reference IFC"));
        assert_eq!(theirs.len(), expected, "RE1 {model}: Revit's systems");
        let mut rf = RevitFile::open(&rvt).expect("open");
        let result = RvtDocExporter
            .export_with_diagnostics(&mut rf)
            .expect("export");
        let (ours, written) = systems(&write_step(&result.model));
        // rvt-rs's systems, each restricted to the elements both exports hold.
        let mut unmatched: Vec<(String, BTreeSet<String>)> = ours
            .into_iter()
            .map(|(name, members)| {
                let shared = members
                    .into_iter()
                    .filter(|tag| theirs_written.contains(tag))
                    .collect();
                (name, shared)
            })
            .collect();
        let mut off = Vec::new();
        for (name, members) in &theirs {
            let shared: BTreeSet<String> = members
                .iter()
                .filter(|tag| written.contains(*tag))
                .cloned()
                .collect();
            match unmatched
                .iter()
                .position(|(n, m)| n == name && *m == shared)
            {
                Some(i) => {
                    unmatched.swap_remove(i);
                }
                None => {
                    let same_name: Vec<usize> = unmatched
                        .iter()
                        .filter(|(n, _)| n == name)
                        .map(|(_, m)| m.len())
                        .collect();
                    off.push((name.clone(), shared.len(), same_name));
                }
            }
        }
        assert!(
            off.is_empty(),
            "RE1 {model}: {} of Revit's {} systems are not rvt-rs's (name, Revit's members rvt-rs writes, the member counts of rvt-rs's unmatched systems of that name): {off:?}",
            off.len(),
            theirs.len()
        );
    }
}
