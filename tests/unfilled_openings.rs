//! RE-151 (#227): the openings Revit's export leaves unfilled.
//!
//! Revit's IFC4 export of `2024_Core_Interior.rvt` writes 63
//! `IfcOpeningElement`s that no door or window fills: 42 void floor slabs,
//! 20 void shading devices and 1 voids a wall. A slab's or shading device's
//! opening is a hole in its sketch, and its `Tag` is the ElementId of a
//! sketch line of that hole. rvt-rs must write every one of the slab and
//! shading-device openings, with Revit's `Tag`, voiding the element with the
//! host's `Tag`, and no unfilled opening Revit does not write.
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR`: `2024_Core_Interior.rvt` with
//! `../IFC Exports/2024_Core_Interior_slim.ifc`. Skips what is absent.

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

/// The last quoted string of an element's arguments: its `Tag`.
fn tag(args: &str) -> Option<u32> {
    args.rsplit('\'').nth(1)?.parse().ok()
}

/// `opening Tag -> (host entity, host Tag)` for every opening nothing fills.
fn unfilled_openings(step: &str) -> BTreeMap<u32, (String, Option<u32>)> {
    let ents = entities(step);
    let mut host_of = BTreeMap::new();
    let mut filled = BTreeSet::new();
    for (entity, args) in ents.values() {
        let r = refs(args);
        match entity.as_str() {
            "IFCRELVOIDSELEMENT" if r.len() >= 2 => {
                host_of.insert(r[r.len() - 1], r[r.len() - 2]);
            }
            "IFCRELFILLSELEMENT" if r.len() >= 2 => {
                filled.insert(r[r.len() - 2]);
            }
            _ => {}
        }
    }
    ents.iter()
        .filter(|(id, (entity, _))| entity == "IFCOPENINGELEMENT" && !filled.contains(*id))
        .filter_map(|(id, (_, args))| {
            let host = host_of.get(id).and_then(|h| ents.get(h));
            Some((
                tag(args)?,
                (
                    host.map_or_else(|| "none".to_string(), |(e, _)| e.clone()),
                    host.and_then(|(_, a)| tag(a)),
                ),
            ))
        })
        .collect()
}

#[test]
fn core_interior_unfilled_openings_are_revits() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let rvt = dir.join("2024_Core_Interior.rvt");
    let reference = dir.join("../IFC Exports/2024_Core_Interior_slim.ifc");
    if !rvt.exists() || !reference.exists() {
        eprintln!("skipping: no Core Interior model and reference export");
        return;
    }
    let theirs = unfilled_openings(&std::fs::read_to_string(&reference).expect("reference IFC"));
    let mut rf = RevitFile::open(&rvt).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let ours = unfilled_openings(&write_step(&result.model));

    let wanted: BTreeMap<u32, Option<u32>> = theirs
        .iter()
        .filter(|(_, (host, _))| host == "IFCSLAB" || host == "IFCSHADINGDEVICE")
        .map(|(opening, (_, host))| (*opening, *host))
        .collect();
    assert_eq!(wanted.len(), 62, "Revit's slab and shading-device openings");
    let missing: Vec<(u32, Option<u32>)> = wanted
        .iter()
        .filter(|(opening, host)| ours.get(*opening).map(|(_, h)| h) != Some(*host))
        .map(|(opening, host)| (*opening, *host))
        .collect();
    assert!(
        missing.is_empty(),
        "{} of Revit's {} unfilled slab and shading-device openings (Tag, host Tag) \
         are not in rvt-rs's export: {missing:?}",
        missing.len(),
        wanted.len()
    );
    let extra: Vec<(&u32, &(String, Option<u32>))> = ours
        .iter()
        .filter(|(opening, (_, host))| theirs.get(*opening).map(|(_, h)| h) != Some(host))
        .collect();
    assert!(
        extra.is_empty(),
        "unfilled openings Revit's export does not write: {extra:?}"
    );
}
