//! RE-184 (probe): the project's phase order, and the elements a last-phase
//! export leaves out (#328).
//!
//! #328 needs two facts rvt-rs did not have: the order of a project's phases,
//! and which elements exist in the phase Revit's export shows. The order is
//! the project's `AllProjectPhases` record (`m_phaseIds`); a family
//! document's own `AllProjectPhases` (non-null `m_famId`, an empty list) is
//! skipped. Each phase is a `ProjectPhase` record (`m_name`,
//! `m_description`). Every element body carries `m_createdPhaseId` and
//! `m_demolishedPhaseId` (B46).
//!
//! For the last phase `P` it lists every record that does not exist in `P`:
//! created after `P`, or demolished in `P` or before it. Records with no
//! created phase (-1) are not phased and are not counted.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re184_phase_order -- MODEL.rvt ...

use rvt::RevitFile;
use rvt::research::native_document;
use std::collections::BTreeMap;

/// Elements printed per file.
const SHOWN: usize = 20;

fn id_at(value: &serde_json::Value, pointer: &str) -> Option<i64> {
    value.pointer(pointer).and_then(serde_json::Value::as_i64)
}

fn probe(path: &str) -> anyhow::Result<()> {
    let file = std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut rf = RevitFile::open(path)?;
    let revit = rf.basic_file_info()?.version;
    let mut order: Option<Vec<i64>> = None;
    let mut names: BTreeMap<i64, String> = BTreeMap::new();
    // (created, demolished) -> records, and the records not in the last phase.
    let mut phased: Vec<(u64, String, i64, i64)> = Vec::new();
    native_document::extract_graphs(&mut rf, &native_document::Options::default(), |record| {
        let Some(object) = record.graph.as_ref().and_then(|g| g.objects.first()) else {
            return Ok(());
        };
        let fields = serde_json::to_value(&object.fields)?;
        let class = record.class_name.clone().unwrap_or_default();
        match class.as_str() {
            "AllProjectPhases" if id_at(&fields, "/m_famId/m_id/m_id64") == Some(-1) => {
                let ids: Vec<i64> = fields["m_phaseIds"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|p| id_at(p, "/m_id/m_id64")).collect())
                    .unwrap_or_default();
                order = Some(ids);
            }
            "ProjectPhase" => {
                if let Some(name) = fields["m_name"].as_str() {
                    names.insert(record.identity.element_id as i64, name.to_string());
                }
            }
            _ => {}
        }
        let created = id_at(&fields, "/m_createdPhaseId/m_id/m_id64").unwrap_or(-1);
        let demolished = id_at(&fields, "/m_demolishedPhaseId/m_id/m_id64").unwrap_or(-1);
        if created != -1 {
            phased.push((record.identity.element_id, class, created, demolished));
        }
        Ok(())
    })?;
    let order = order.unwrap_or_default();
    let named: Vec<String> = order
        .iter()
        .map(|id| format!("[{id},{:?}]", names.get(id).map_or("?", String::as_str)))
        .collect();
    let index = |id: i64| order.iter().position(|p| *p == id);
    let last = order.len().checked_sub(1);
    let mut by_status: BTreeMap<&str, usize> = BTreeMap::new();
    let mut left_out = Vec::new();
    for (id, class, created, demolished) in &phased {
        let (Some(last), Some(c)) = (last, index(*created)) else {
            *by_status
                .entry("created phase not in the order")
                .or_default() += 1;
            continue;
        };
        let d = (*demolished != -1).then(|| index(*demolished));
        let status = match d {
            Some(None) => "demolished phase not in the order",
            _ if c > last => "created after the last phase",
            Some(Some(d)) if d <= last => "demolished by the last phase",
            _ => "exists in the last phase",
        };
        *by_status.entry(status).or_default() += 1;
        if status != "exists in the last phase" {
            left_out.push(format!(
                "[{id},{class:?},{:?},{:?}]",
                names.get(created).map_or("?", String::as_str),
                names.get(demolished).map_or("-", String::as_str)
            ));
        }
    }
    let statuses: Vec<String> = by_status
        .iter()
        .map(|(s, n)| format!("{s:?}:{n}"))
        .collect();
    println!(
        "{{\"file\":{file:?},\"revit\":{revit},\"phase_order\":[{}],\"phased_records\":{},\"status\":{{{}}},\"left_out\":{},\"shown\":[{}]}}",
        named.join(","),
        phased.len(),
        statuses.join(","),
        left_out.len(),
        left_out
            .iter()
            .take(SHOWN)
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    );
    Ok(())
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: probe_re184_phase_order MODEL.rvt ...");
        std::process::exit(2);
    }
    for path in &paths {
        if let Err(err) = probe(path) {
            println!("{{\"file\":{path:?},\"error\":{:?}}}", err.to_string());
        }
    }
}
