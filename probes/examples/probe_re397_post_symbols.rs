//! RE-397 (#1635): the symbol the posts of a `KAI-3` railing name, and what its
//! saved graphics hold.
//!
//! FACT under test: in the saved mesh of the `KAI-3` railings of the BIM4LCA
//! office each of the 12 posts of 2618205 is a cylinder of diameter 0.328 ft and
//! height 2.133 ft (0.1793 ft3), the circle through the corners of the
//! rectangle, 0.256 by 0.204 ft, that Revit's own export draws (0.1125 ft3, 24
//! vertices to a cap). This follows the instances of the two railings' symbols
//! (2618206 and 2614260) to the symbols they name and prints each one's
//! surfaces, to find where the circle comes from.
//!
//! Output:
//!   instsym <owner> <symbol> <instances>
//!   gsym <id> objects=<n> edges=<n> <class:count,...>
//!   gnode <id> <index> <class> flags=<m_GInfo.m_flags> tag=<m_GInfo.m_tag> <fields, capped>
//!     every node but the topology (faces, loops, edges, vertices); the planes and
//!     cylinders are counted in the line above, and the first six of each printed
//!   gedge <id> <source index> <source class> -> <target index> <target class>
//!     the pointers between nodes that are not topology
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re397_post_symbols -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};

/// The symbols of `KAI-3` railings 2618205 and 2614259.
const OWNERS: [u64; 2] = [2618206, 2614260];
const TOPOLOGY: [&str; 6] = [
    "Face",
    "Loop",
    "Edge",
    "Vertex",
    "CoEdge",
    "FacetedTopology0",
];
const SURFACES: [&str; 3] = ["Plane", "CylSurf", "SurfRev"];

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

fn topology(class: &str) -> bool {
    TOPOLOGY.iter().any(|name| class.contains(*name))
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let first = Options {
        selected_ids: OWNERS.iter().copied().collect::<BTreeSet<u64>>(),
        channels: BTreeSet::from([103]),
        ..Options::default()
    };
    let mut named: BTreeSet<u64> = BTreeSet::new();
    extract_graphs(&mut rf, &first, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let owner = record.identity.element_id;
        let mut counts: BTreeMap<u64, usize> = BTreeMap::new();
        for object in &graph.objects {
            if object.class_name != "InstanceInfo" {
                continue;
            }
            if let Some(symbol) = object.fields["m_symbolId"]["m_id"]["m_id64"].as_u64() {
                *counts.entry(symbol).or_default() += 1;
            }
        }
        for (symbol, n) in counts {
            println!("instsym {owner} {symbol} {n}");
            named.insert(symbol);
        }
        Ok(())
    })?;
    let second = Options {
        selected_ids: named,
        channels: BTreeSet::from([103]),
        ..Options::default()
    };
    extract_graphs(&mut rf, &second, |record| {
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
        let mut meshes_shown = 0usize;
        let mut surfaces_shown: BTreeMap<&str, usize> = BTreeMap::new();
        for (index, object) in graph.objects.iter().enumerate() {
            let class = object.class_name.as_str();
            if topology(class) {
                continue;
            }
            if is_mesh(class) {
                meshes_shown += 1;
                if meshes_shown > 2 {
                    continue;
                }
            }
            if let Some(kind) = SURFACES.iter().find(|name| class.contains(**name)) {
                let shown = surfaces_shown.entry(kind).or_default();
                *shown += 1;
                if *shown > 6 {
                    continue;
                }
            }
            println!(
                "gnode {id} {index} {class} flags={} tag={} {}",
                object.fields["m_GInfo"]["m_flags"],
                object.fields["m_GInfo"]["m_tag"],
                cap(object.fields.to_string(), 1600)
            );
        }
        for edge in &graph.edges {
            let (Some(source), Some(target)) = (
                graph.objects.get(edge.source_object_index),
                graph.objects.get(edge.target_object_index),
            ) else {
                continue;
            };
            if topology(&source.class_name)
                || topology(&target.class_name)
                || SURFACES.iter().any(|name| {
                    source.class_name.contains(*name) || target.class_name.contains(*name)
                })
                || is_mesh(&source.class_name)
                || is_mesh(&target.class_name)
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
