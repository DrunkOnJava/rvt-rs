//! #2085: which saved-graphics graphs does the saved-mesh reader's budget of 1,000,000 values
//! refuse, and how large are they?
//!
//! FACT under test: `ifc::saved_meshes` decodes each saved-graphics graph with at most
//! 1,000,000 values and 1,000,000 objects, and refuses a graph past either whole. On a
//! contributor's save every chair's symbol graph (1,789,525 values) is refused so, before any
//! face is read. Whether raising the budget would let anything else through is a count of the
//! graphs it refuses in the files Measure holds. This decodes the graphs of every element the
//! export writes, and of every symbol their instances name, followed to 16 levels, as the
//! saved-mesh reader reads them (`value_source` "saved_graphics"), at 1,000,000, 2,000,000,
//! 4,000,000 and 16,000,000 values, objects held at 1,000,000. It prints:
//! ```text
//!   budget values=<n> read=<n> refused=<n>: per budget, the graphs decoded and refused;
//!   over id=<ElementId> class=<class> refused_at=<n,...> [objects_or_other]: each graph the
//!     1,000,000 budget refuses, with every budget that refuses it, so its size lies between the
//!     largest of those and the next; `objects_or_other` where 16,000,000 refuses it too (its
//!     objects, or another fault);
//!   largest values=<n>: the largest graph decoded at 16,000,000, in values.
//! ```
//! ElementIds and counts only. The selection is the probe's own: the elements of the export's
//! building elements and their symbols, not every id the saved-mesh reader asks for.
//!
//! Usage:
//! ```text
//!   cargo run --profile ci -p rvt-probes --example probe_2085_graph_budget -- MODEL.rvt
//! ```

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{self, Record};
use rvt::research::native_parameters::ObjectGraph;

const BUDGETS: [usize; 4] = [1_000_000, 2_000_000, 4_000_000, 16_000_000];

fn identifier(value: &Fv) -> Option<i64> {
    if let Some(n) = value.as_i64() {
        return Some(n);
    }
    ["m_id", "m_id64"]
        .iter()
        .find_map(|key| value.get(*key))
        .and_then(identifier)
}

fn instance_symbols(graph: &ObjectGraph) -> Vec<u64> {
    graph
        .objects
        .iter()
        .filter(|object| object.class_name == "InstanceInfo")
        .filter_map(|object| {
            object
                .fields
                .get("m_symbolId")
                .and_then(identifier)
                .and_then(|id| u64::try_from(id).ok())
                .filter(|id| *id > 0)
        })
        .collect()
}

fn options(ids: BTreeSet<u64>, values: usize) -> native_document::Options {
    native_document::Options {
        selected_ids: ids,
        max_graph_values: values,
        max_graph_objects: 1_000_000,
        value_source: Some("saved_graphics"),
        ..Default::default()
    }
}

fn main() -> Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|a| !a.starts_with("--"))
        .ok_or_else(|| anyhow::anyhow!("usage: probe_2085_graph_budget MODEL.rvt"))?;
    let mut rf = RevitFile::open_with_limits(&path, rvt::cli::open_limits(None, None, false))?;
    let model = rvt::ifc::RvtDocExporter.export_with_mode_and_limits(
        &mut rf,
        rvt::ifc::ExportQualityMode::Scaffold,
        rvt::walker::WalkerLimits::default(),
    )?;
    let ids: BTreeSet<u64> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            rvt::ifc::entities::IfcEntity::BuildingElement {
                type_guid: Some(tag),
                ..
            } => tag.parse().ok(),
            _ => None,
        })
        .collect();
    // At the largest budget, follow instances to their symbols, so every budget below reads
    // the same ids.
    let mut all = ids.clone();
    let mut wanted = ids;
    let mut classes: BTreeMap<u64, String> = BTreeMap::new();
    let mut largest = 0usize;
    for _ in 0..16 {
        if wanted.is_empty() {
            break;
        }
        let mut found: BTreeSet<u64> = BTreeSet::new();
        let summary = native_document::extract_graphs(
            &mut rf,
            &options(wanted.clone(), BUDGETS[3]),
            |r: Record| {
                if let Some(class) = &r.class_name {
                    classes.insert(r.identity.element_id, class.clone());
                }
                if let Some(graph) = &r.graph {
                    found.extend(instance_symbols(graph));
                }
                Ok(())
            },
        )?;
        largest = largest.max(summary.max_graph_values_observed);
        wanted = found.into_iter().filter(|id| all.insert(*id)).collect();
    }
    let mut refused: BTreeMap<usize, BTreeSet<u64>> = BTreeMap::new();
    for values in BUDGETS {
        let summary =
            native_document::extract_graphs(&mut rf, &options(all.clone(), values), |_| Ok(()))?;
        let set: BTreeSet<u64> = summary.refused_graph_ids.iter().copied().collect();
        println!(
            "budget values={values} read={} refused={}",
            summary.complete_graph_records,
            set.len()
        );
        refused.insert(values, set);
    }
    for id in &refused[&BUDGETS[0]] {
        let at: Vec<String> = BUDGETS
            .iter()
            .filter(|b| refused[b].contains(id))
            .map(|b| b.to_string())
            .collect();
        let class = classes.get(id).map_or("-", String::as_str);
        let tail = if refused[&BUDGETS[3]].contains(id) {
            " objects_or_other"
        } else {
            ""
        };
        println!(
            "over id={id} class={class} refused_at={}{tail}",
            at.join(",")
        );
    }
    println!("largest values={largest}");
    Ok(())
}
