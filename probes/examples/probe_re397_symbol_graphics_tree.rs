//! RE-397 (#1635): the saved-graphics tree of the symbols a railing names, node
//! by node.
//!
//! FACT under test: the symbols of the `KAI` railings of the BIM4LCA office
//! keep a reachable `GBiFlipControl` (RE-303), which the traversal does not
//! take, so the mesh of each is partial; yet a railing's symbols' volumes add
//! up to Revit's body of it within 0.2% on all ten `KAI-4` railings and
//! `KAI-1`, and are 19% over it on `KAI-3`. This prints the whole graph of the
//! channel 103 record of a symbol of each of those and of one of the hidden
//! symbols a `KAI-4` railing names, to find what the flip control gates.
//!
//! Output, one record per line (JSON has no spaces):
//!   gsym <id> objects=<n> edges=<n> <class:count,...>
//!   gnode <id> <index> <class> flags=<m_GInfo.m_flags> tag=<m_GInfo.m_tag> <fields, capped>
//!     for every node except the surfaces' topology (faces, loops, edges, vertices, planes)
//!   gedge <id> <source index> <source class> -> <target index> <target class>
//!     for every pointer between two nodes that are not topology
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re397_symbol_graphics_tree -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};

/// The visible symbol of a `KAI-4` railing (1467318), of `KAI-1` (1485880), of
/// `KAI-3` (2614260) and of the small `KAI-5` (2903645), and the hidden symbol
/// of the first of them.
const RAILINGS: [u64; 5] = [1467319, 1485881, 2614260, 2903646, 2620342];
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

fn is_mesh(class: &str) -> bool {
    matches!(class, "Geometry" | "GPolyMesh")
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
        // A symbol holds thousands of meshes: the first two of each kind show
        // the shape, the census above counts them all.
        let mut meshes_shown = 0usize;
        for (index, object) in graph.objects.iter().enumerate() {
            let class = object.class_name.as_str();
            if skipped(class) {
                continue;
            }
            if is_mesh(class) {
                meshes_shown += 1;
                if meshes_shown > 2 {
                    continue;
                }
            }
            println!(
                "gnode {id} {index} {class} flags={} tag={} {}",
                object.fields["m_GInfo"]["m_flags"],
                object.fields["m_GInfo"]["m_tag"],
                cap(object.fields.to_string(), 2400)
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
            // The pointers of the structural nodes, and a flip control's to its
            // meshes: the rest are one per mesh.
            if (is_mesh(&source.class_name) || is_mesh(&target.class_name))
                && source.class_name != "GBiFlipControl"
            {
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
