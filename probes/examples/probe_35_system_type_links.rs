//! #35: which field of a wall's, floor's, ceiling's or roof's native record
//! names its type.
//!
//! RE-207 lists a family instance's type parameters through
//! `FamilyInstance.m_masterSymbolId`; system-family elements get none. This
//! prints, for each element id given, its record's root class and every
//! ElementId-valued field of its root object, then, for each id those
//! fields name, the class of that record:
//!
//!   elem <id> <root class> <field>=<id> ...
//!   target <id> <root class>
//!
//! Usage:
//!   cargo run --release -p rvt-probes --example probe_35_system_type_links -- MODEL.rvt IDS.txt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::{self, Options};

/// A saved ElementId, `{"m_id":{"m_id64":n}}` or `{"m_id":n}`, as `n`.
fn element_id(value: &serde_json::Value) -> Option<i64> {
    let inner = value.get("m_id")?;
    inner.get("m_id64").unwrap_or(inner).as_i64()
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: MODEL.rvt IDS.txt");
    let ids: BTreeSet<u64> = std::fs::read_to_string(args.next().expect("IDS.txt"))?
        .split_whitespace()
        .filter_map(|word| word.parse().ok())
        .collect();
    let mut rf = RevitFile::open(&path)?;
    let mut targets: BTreeSet<u64> = BTreeSet::new();
    let options = Options {
        selected_ids: ids.clone(),
        ..Default::default()
    };
    native_document::extract(&mut rf, &options, |record| {
        let Some(graph) = &record.graph else {
            return Ok(());
        };
        let root = &graph.objects[0];
        let mut line = format!("elem {} {}", record.identity.element_id, root.class_name);
        for (field, value) in root.fields.as_object().into_iter().flatten() {
            if let Some(id) = element_id(value) {
                line.push_str(&format!(" {field}={id}"));
                if let Ok(id) = u64::try_from(id) {
                    targets.insert(id);
                }
            }
        }
        println!("{line}");
        Ok(())
    })?;
    let mut classes: BTreeMap<u64, String> = BTreeMap::new();
    let options = Options {
        selected_ids: targets,
        ..Default::default()
    };
    native_document::extract(&mut rf, &options, |record| {
        if let Some(graph) = &record.graph {
            classes.insert(
                record.identity.element_id,
                graph.objects[0].class_name.clone(),
            );
        }
        Ok(())
    })?;
    for (id, class) in classes {
        println!("target {id} {class}");
    }
    Ok(())
}
