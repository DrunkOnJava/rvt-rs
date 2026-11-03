//! RE-318 (#810): the saved-graphics tree of the symbols behind the BIM4LCA
//! office's two hatch doors and its O-16 door, node by node.
//!
//! FACT under test: the extent along X of a symbol's saved surfaces, hidden
//! branches included, is Revit's `OverallWidth` on 267 of the office's 269
//! doors and all its windows (`probe_re318_hidden_geometry`, Measure
//! 41720185112). The O-16 door 1722215 (symbol 1727541) is 5 mm short: 2750
//! mm against Revit's 2755 mm. This prints the whole tree of that symbol's
//! channel 103 record, and of the hatch symbols (2058852, 1766487) and a
//! door that is right (symbol 2393482) beside it, to find what carries the
//! last 5 mm: a curve, an arc, a nested family, a filter condition on a
//! parameter value.
//!
//! Output, one record per line (JSON has no spaces):
//!   gsym <symbol> objects=<n> <class:count,...>
//!   gnode <symbol> <index> <class> flags=<m_GInfo.m_flags> tag=<m_GInfo.m_tag> <fields, capped>
//!     for every node except the surfaces' topology (faces, loops, edges, vertices, planes)
//!   gcond <symbol> <index> <class> <fields>      (each condition object of a filter)
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re318_symbol_graphics_tree -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};

const SYMBOLS: [u64; 4] = [1727541, 2058852, 1766487, 2393482];
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

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let options = Options {
        selected_ids: SYMBOLS.iter().copied().collect::<BTreeSet<u64>>(),
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
            "gsym {id} objects={} {}",
            graph.objects.len(),
            census
                .iter()
                .map(|(class, n)| format!("{class}:{n}"))
                .collect::<Vec<_>>()
                .join(",")
        );
        for (index, object) in graph.objects.iter().enumerate() {
            let class = object.class_name.as_str();
            if SKIPPED.iter().any(|skipped| class.contains(*skipped)) {
                continue;
            }
            if class.starts_with("GCondition") {
                println!(
                    "gcond {id} {index} {class} {}",
                    cap(object.fields.to_string(), 400)
                );
                continue;
            }
            println!(
                "gnode {id} {index} {class} flags={} tag={} {}",
                object.fields["m_GInfo"]["m_flags"],
                object.fields["m_GInfo"]["m_tag"],
                cap(object.fields.to_string(), 700)
            );
        }
        Ok(())
    })?;
    Ok(())
}
