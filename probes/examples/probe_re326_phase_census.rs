//! RE-326 (probe): every phased record's created and demolished phase, by
//! class, with its ElementIds (#1194).
//!
//! `probe_re326_room_phases` found that Revit's export of a Revit 2018 copy
//! of Autodesk's `rac_basic_sample_project` holds all 14 of its rooms, which
//! sit in the second of the project's three phases (Existing, Working
//! Drawings, Learning Content). Revit's exporter skips a room whose own phase
//! is not the export phase, and RE-184 took the export phase to be the last
//! phase, so one of the two readings is wrong for that export. The elements
//! tell which: an element created in the last phase is in an export of the
//! last phase and in none of an earlier one, and one demolished in the last
//! phase is in an export of an earlier phase only.
//!
//! FACT under test: the records created in (or demolished in) the last phase
//! of that sample are, or are not, in Revit's export of it.
//!
//! Output, one JSON object per line:
//!   {"kind":"file", ...}    the file, its release and its phases
//!   {"kind":"group", ...}   one per (class, created phase, demolished phase):
//!                           the number of records and their ElementIds
//!
//! `tools/re/phase_census_vs_ifc.py PROBE.txt REVIT.ifc` reads each group's
//! ElementIds against the `Tag` of Revit's building elements.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re326_phase_census -- MODEL.rvt ...

use rvt::RevitFile;
use rvt::research::graph_value::Fv;
use rvt::research::native_document;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

type Group = (String, i64, i64);

/// An ElementId field's value, as either shape the schema saves it.
fn ident(value: &Fv) -> Option<i64> {
    let identifier = value.get("m_id")?;
    identifier
        .get("m_id64")
        .or_else(|| identifier.get("m_id"))
        .and_then(Fv::as_i64)
}

fn probe(path: &str) -> anyhow::Result<()> {
    let file = std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut rf = RevitFile::open(path)?;
    let revit = rf.basic_file_info()?.version;
    let options = native_document::Options {
        graph_classes: Some(BTreeSet::from([
            "AllProjectPhases".to_string(),
            "ProjectPhase".to_string(),
        ])),
        releases: native_document::NATIVE_RELEASES,
        ..native_document::Options::default()
    };
    let mut order: Option<Vec<i64>> = None;
    let mut names: BTreeMap<i64, String> = BTreeMap::new();
    let mut groups: BTreeMap<Group, Vec<u64>> = BTreeMap::new();
    native_document::extract_graphs(&mut rf, &options, |record| {
        let class = record.class_name.clone().unwrap_or_default();
        if let Some(base) = &record.base {
            if base.created_phase_id != -1 {
                groups
                    .entry((class, base.created_phase_id, base.demolished_phase_id))
                    .or_default()
                    .push(record.identity.element_id);
            }
            return Ok(());
        }
        let Some(object) = record.graph.as_ref().and_then(|g| g.objects.first()) else {
            return Ok(());
        };
        match class.as_str() {
            "AllProjectPhases" if ident(&object.fields["m_famId"]) == Some(-1) => {
                let ids: Vec<i64> = object.fields["m_phaseIds"]
                    .as_array()
                    .map(|a| a.iter().filter_map(ident).collect())
                    .unwrap_or_default();
                order = Some(ids);
            }
            "ProjectPhase" => {
                if let Some(name) = object.fields["m_name"].as_str() {
                    names.insert(record.identity.element_id as i64, name.to_string());
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    let order = order.unwrap_or_default();
    let phase_list: Vec<Fv> = order.iter().map(|id| json!([id, names.get(id)])).collect();
    let phased: usize = groups.values().map(Vec::len).sum();
    println!(
        "{}",
        json!({
            "kind": "file",
            "file": file,
            "revit": revit,
            "phase_order": phase_list,
            "phased_records": phased,
        })
    );
    for ((class, created, demolished), ids) in &groups {
        println!(
            "{}",
            json!({
                "kind": "group",
                "file": file,
                "class": class,
                "created": created,
                "demolished": demolished,
                "records": ids.len(),
                "ids": ids,
            })
        );
    }
    Ok(())
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: probe_re326_phase_census MODEL.rvt ...");
        std::process::exit(2);
    }
    for path in &paths {
        if let Err(err) = probe(path) {
            println!(
                "{{\"kind\":\"error\",\"file\":{path:?},\"error\":{:?}}}",
                err.to_string()
            );
        }
    }
}
