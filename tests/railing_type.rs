//! B45: a railing is typed as Revit's export types it.
//!
//! Revit's export of RE1 Architecture writes railing 462556 with the type
//! `IfcRailingType` 'Railing:-', `Tag` 446543 and GlobalId
//! `2gD5pNbA99PxygXN$rUlWU`, and gives it `Pset_RailingCommon.Reference`
//! '-', its type's name. rvt-rs must write the railing with a type object of
//! that `Tag`, GlobalId and name, and the same Reference.
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR` (`RE1-Architecture.rvt` and Revit's
//! `RE1-Architecture.ifc`). Skips what is absent.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// `#id` -> (entity name, raw argument text) for every entity line.
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
        let Ok(id) = id.trim().parse() else {
            continue;
        };
        out.insert(id, (entity.trim().to_string(), args.to_string()));
    }
    out
}

/// The `#n` references in a piece of argument text.
fn refs(args: &str) -> Vec<u64> {
    args.split('#')
        .skip(1)
        .filter_map(|s| {
            let digits: String = s.chars().take_while(char::is_ascii_digit).collect();
            digits.parse().ok()
        })
        .collect()
}

/// The quoted strings of an argument list, in order.
fn quoted(args: &str) -> Vec<String> {
    args.split('\'')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

/// What an export says of the element tagged `tag`: its type's entity,
/// GlobalId, name and `Tag`, and its `Pset_RailingCommon.Reference`.
#[derive(Debug, PartialEq, Default)]
struct Railing {
    type_entity: Option<String>,
    type_global_id: Option<String>,
    type_name: Option<String>,
    type_tag: Option<String>,
    reference: Option<String>,
}

fn railing(step: &str, tag: &str) -> Railing {
    let ents = entities(step);
    let Some((&element, _)) = ents.iter().find(|(_, (entity, args))| {
        entity == "IFCRAILING" && quoted(args).last().map(String::as_str) == Some(tag)
    }) else {
        return Railing::default();
    };
    let mut out = Railing::default();
    for (entity, args) in ents.values() {
        let all = refs(args);
        let Some((&last, related)) = all.split_last() else {
            continue;
        };
        if !related.iter().skip(1).any(|r| *r == element) {
            continue;
        }
        let Some((target, target_args)) = ents.get(&last) else {
            continue;
        };
        if entity == "IFCRELDEFINESBYTYPE" {
            let strings = quoted(target_args);
            out.type_entity = Some(target.clone());
            out.type_global_id = strings.first().cloned();
            out.type_name = strings.get(1).cloned();
            out.type_tag = strings.last().cloned();
        }
        if entity == "IFCRELDEFINESBYPROPERTIES" && target_args.contains("'Pset_RailingCommon'") {
            for property in refs(target_args) {
                if let Some((_, property_args)) = ents.get(&property) {
                    if property_args.starts_with("'Reference'") {
                        out.reference = quoted(property_args).get(1).cloned();
                    }
                }
            }
        }
    }
    out
}

#[test]
fn re1_railing_is_typed_as_revit_types_it() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let rvt = dir.join("RE1-Architecture.rvt");
    let reference = dir.join("RE1-Architecture.ifc");
    if !rvt.exists() || !reference.exists() {
        eprintln!("skipping: no RE1 Architecture model and reference export");
        return;
    }
    let theirs = railing(
        &std::fs::read_to_string(&reference).expect("reference IFC"),
        "462556",
    );
    assert_eq!(
        theirs.type_tag.as_deref(),
        Some("446543"),
        "Revit's export types railing 462556 with 446543: {theirs:?}"
    );
    let mut rf = RevitFile::open(&rvt).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let ours = railing(&write_step(&result.model), "462556");
    assert_eq!(ours.type_tag, theirs.type_tag, "type Tag; rvt-rs {ours:?}");
    assert_eq!(
        ours.type_global_id, theirs.type_global_id,
        "type GlobalId; rvt-rs {ours:?}"
    );
    assert_eq!(
        ours.type_name, theirs.type_name,
        "type name; rvt-rs {ours:?}"
    );
    assert_eq!(
        ours.reference, theirs.reference,
        "Pset_RailingCommon.Reference; rvt-rs {ours:?}"
    );
}
