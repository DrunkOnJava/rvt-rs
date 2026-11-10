//! RE-397 (#1635): the saved-graphics tree of a railing, node by node.
//!
//! FACT under test: the saved mesh of a `KAI` railing of the BIM4LCA office is
//! partial (a `GBiFlipControl` is reachable, RE-303) and Revit's body reaches up
//! to a foot beyond it at 0.39 to 0.54 of Revit's volume. This prints the whole
//! graph of the channel 103 record of one railing of each type the office has
//! (`KAI-1` to `KAI-6` and the stair railing `KJO-1`), to find what the flip
//! control gates and what the parts the mesh lacks are.
//!
//! Output, one record per line (JSON has no spaces):
//!   gsym <id> objects=<n> edges=<n> <class:count,...>
//!   gnode <id> <index> <class> flags=<m_GInfo.m_flags> tag=<m_GInfo.m_tag> <fields, capped>
//!     for every node except the surfaces' topology (faces, loops, edges, vertices, planes)
//!   gedge <id> <source index> <source class> -> <target index> <target class>
//!     for every pointer between two nodes that are not topology
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re397_railing_graphics_tree -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};

/// One railing of each type of the office as built: KAI-4, KAI-1, KAI-2,
/// KAI-3, KAI-5, KAI-6 and KJO-1.
const RAILINGS: [u64; 7] = [
    1467318, 1485880, 1749890, 2614259, 2598350, 3076853, 2598353,
];
const SKIPPED: [&str; 8] = [
    "Face",
    "Loop",
    "Edge",
    "Vertex",
    "Plane",
    "Cylinder",
    "CoEdge",
    "FacetedTopology0",
];

fn cap(text: String, limit: usize) -> String {
    if text.chars().count() <= limit {
        text
    } else {
        text.chars().take(limit).collect::<String>() + "..."
    }
}

fn skipped(class: &str) -> bool {
    SKIPPED.iter().any(|name| class.contains(*name))
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let options = Options {
        selected_ids: RAILINGS.iter().copied().collect::<BTreeSet<u64>>(),
        channels: BTreeSet::from([103]),
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let id = record.identity.element_id;
        let mut census: BTreeMap<&str, usize> = BTreeMap::new();
        for object in &graph.objects {
            *census.entry(object.class_name.as_str()).or_default() += 1;
        }
        println!(
            "gsym {id} objects={} edges={} {}",
            graph.objects.len(),
            graph.edges.len(),
            census
                .iter()
                .map(|(class, n)| format!("{class}:{n}"))
                .collect::<Vec<_>>()
                .join(",")
        );
        for (index, object) in graph.objects.iter().enumerate() {
            let class = object.class_name.as_str();
            if skipped(class) {
                continue;
            }
            println!(
                "gnode {id} {index} {class} flags={} tag={} {}",
                object.fields["m_GInfo"]["m_flags"],
                object.fields["m_GInfo"]["m_tag"],
                cap(object.fields.to_string(), 700)
            );
        }
        for edge in &graph.edges {
            let (Some(source), Some(target)) = (
                graph.objects.get(edge.source_object_index),
                graph.objects.get(edge.target_object_index),
            ) else {
                continue;
            };
            if skipped(&source.class_name) || skipped(&target.class_name) {
                continue;
            }
            println!(
                "gedge {id} {} {} -> {} {}",
                edge.source_object_index,
                source.class_name,
                edge.target_object_index,
                target.class_name
            );
        }
        Ok(())
    })?;
    Ok(())
}
