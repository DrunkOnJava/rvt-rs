//! RE-389 follow-up (#1630): the saved-graphics tree of a roof that stays a
//! level block, node by node.
//!
//! FACT under test: of the 20 roofs of Autodesk's Snowdon Towers Architectural
//! (Revit 2024), 10 decode whole and #1612 draws them from their saved mesh;
//! of the other 10, 9 report `unsupported reachable graphics class GGTag`, 4 of
//! them nothing else (`probe_re389_roof_blocks`). This prints the whole graph
//! of the channel 103 record of two of those four and of two roofs that report
//! something else, to find what a `GGTag` holds and where it sits.
//!
//! Output, one record per line (JSON has no spaces):
//!   gsym <id> objects=<n> edges=<n> <class:count,...>
//!   gnode <id> <index> <class> flags=<m_GInfo.m_flags> tag=<m_GInfo.m_tag> <fields, capped>
//!     for every node except the surfaces' topology and all but the first two
//!     meshes
//!   gedge <id> <source index> <source class> -> <target index> <target class>
//!     for every pointer between two nodes that are not topology, and a flip
//!     control's or tag's to its meshes
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re389_roof_graphics_tree -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};

/// Roofs of Snowdon Towers Architectural 2024 that report only `GGTag` (1935345
/// and 1924305), a curved edge only (1916596) and a cylinder trim and `GGTag`
/// (964559).
const RAILINGS: [u64; 4] = [1935345, 1924305, 1916596, 964559];
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
                && !matches!(source.class_name.as_str(), "GBiFlipControl" | "GGTag")
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
