//! #2085: where does a parametric face's trim loop stop joining, on a face refused as
//! `disconnected parametric trim in model coordinates`?
//!
//! FACT under test: `native_parametric_mesh::loop_closes` evaluates each edge's ends on the
//! face's surface and needs each edge to meet the last within 1e-7 in model coordinates,
//! either way round, and the last to meet the first. A small gap says the tolerance is too
//! tight for the surface's evaluation; a large one, that the edges' parameters are not
//! where the surface puts them, or that the loop's edges are out of order. Built on
//! `research/2085-chairs` (its 4,000,000-value graph budget, and #1941's `HermiteSurf`).
//!
//! For every element the export writes and every symbol its graphics name, each face of
//! its saved graphics is put to `native_parametric_mesh::face_grid`; for a face refused with
//! that reason, `native_parametric_mesh::trim_breaks` says where each loop that does not
//! close stops. It prints:
//!   break graph=<ElementId> face=<n> surface=<class> loop=<n> edges=<n> chained=<n>
//!     gap=<ft> closing=<true|false> other_meets=<true|false>
//!   - `chained`: the edges the attempt that joins the most joined; `closing` where every
//!     edge joined and only the step back to the first point fails;
//!   - `gap`: from the last joined point to the nearer end of the next edge (or back to the
//!     first point), in feet;
//!   - `other_meets`: an edge further on in the loop meets the last point instead.
//!   unread graph=<ElementId> face=<n> why=<reason>: a refused face `trim_breaks` cannot read.
//!   reach owner=<ElementId> graphs=[<ElementId>, ...]: an element whose graphics reach a
//!     graph with a break.
//! then by the refused face's surface class:
//!   faces surface=<class> meshed=<n> disconnected=<n> other=<n>
//!   gaps surface=<class> loops=<n> under_1e-6=<n> 1e-6_to_1e-4=<n> 1e-4_to_0.01=<n>
//!     over_0.01=<n> closing=<n> other_meets=<n>
//! ElementIds, counts and distances in feet only.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_2085_trim_breaks -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{self, Record};
use rvt::research::native_parameters::ObjectGraph;
use rvt::research::native_parametric_mesh::{face_grid, trim_breaks};
use rvt::research::native_saved_mesh::pointer;

const DISCONNECTED: &str = "disconnected parametric trim in model coordinates";

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

/// Counts by the refused face's surface class: faces (meshed, disconnected, other) and
/// loops by gap bin, closing and other-meets.
#[derive(Default)]
struct Tally {
    meshed: usize,
    disconnected: usize,
    other: usize,
    loops: usize,
    bins: [usize; 4],
    closing: usize,
    other_meets: usize,
}

/// The faces of one graph: prints its breaks and adds to the tally; whether it holds one.
fn scan(id: u64, graph: &ObjectGraph, tally: &mut BTreeMap<String, Tally>) -> bool {
    let mut any = false;
    for (fi, object) in graph.objects.iter().enumerate() {
        if object.class_name != "Face" {
            continue;
        }
        // Planes and cylinders are meshed by `native_saved_mesh`, not on this grid.
        let class = object
            .fields
            .get("m_pSurf")
            .and_then(|p| pointer(graph, fi, p).ok().flatten())
            .and_then(|si| graph.objects.get(si))
            .map(|s| s.class_name.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        if class == "Plane" || class == "CylSurf" {
            continue;
        }
        let why = face_grid(graph, fi).err().map(|e| format!("{e:#}"));
        let t = tally.entry(class.clone()).or_default();
        match &why {
            None => t.meshed += 1,
            Some(w) if w.contains(DISCONNECTED) => t.disconnected += 1,
            Some(_) => t.other += 1,
        }
        if !why.as_deref().is_some_and(|w| w.contains(DISCONNECTED)) {
            continue;
        }
        match trim_breaks(graph, fi) {
            Ok((_, breaks)) => {
                let t = tally.entry(class.clone()).or_default();
                for b in breaks {
                    any = true;
                    t.loops += 1;
                    let bin = match b.gap {
                        g if g < 1e-6 => 0,
                        g if g < 1e-4 => 1,
                        g if g < 1e-2 => 2,
                        _ => 3,
                    };
                    t.bins[bin] += 1;
                    let closing = b.chained == b.edges;
                    t.closing += usize::from(closing);
                    t.other_meets += usize::from(b.other_meets);
                    println!(
                        "break graph={id} face={fi} surface={class} loop={} edges={} chained={} gap={:.9} closing={closing} other_meets={}",
                        b.loop_index, b.edges, b.chained, b.gap, b.other_meets
                    );
                }
            }
            Err(e) => {
                println!(
                    "unread graph={id} face={fi} why={}",
                    format!("{e:#}").replace(' ', "_")
                );
            }
        }
    }
    any
}

fn main() -> Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|a| !a.starts_with("--"))
        .ok_or_else(|| anyhow::anyhow!("usage: probe_2085_trim_breaks MODEL.rvt"))?;
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
    let mut options = native_document::Options {
        selected_ids: ids,
        channels: BTreeSet::from([103]),
        // The branch's saved-graphics budget, so the graphs it decodes are the ones read here.
        max_graph_values: 4_000_000,
        max_graph_objects: 1_000_000,
        record_extras: true,
        ..Default::default()
    };
    let mut tally: BTreeMap<String, Tally> = BTreeMap::new();
    let mut broken: BTreeSet<u64> = BTreeSet::new();
    let mut scanned: BTreeSet<u64> = BTreeSet::new();
    let mut symbols_of: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
    let mut owners: BTreeSet<u64> = BTreeSet::new();
    let mut note = |r: &Record,
                    owner: bool,
                    tally: &mut BTreeMap<String, Tally>,
                    symbols_of: &mut BTreeMap<u64, Vec<u64>>| {
        let id = r.identity.element_id;
        if owner {
            owners.insert(id);
        }
        if !scanned.insert(id) {
            return;
        }
        if let Some(graph) = &r.graph {
            symbols_of.insert(id, instance_symbols(graph));
            if scan(id, graph, tally) {
                broken.insert(id);
            }
        }
    };
    native_document::extract_graphs(&mut rf, &options, |r| {
        note(&r, true, &mut tally, &mut symbols_of);
        Ok(())
    })?;
    for _ in 0..16 {
        let wanted: BTreeSet<u64> = symbols_of
            .values()
            .flatten()
            .filter(|id| !symbols_of.contains_key(id))
            .copied()
            .collect();
        if wanted.is_empty() {
            break;
        }
        options.selected_ids = wanted.clone();
        native_document::extract_graphs(&mut rf, &options, |r| {
            note(&r, false, &mut tally, &mut symbols_of);
            Ok(())
        })?;
        // A symbol whose record holds no graph is not asked for again.
        for id in wanted {
            symbols_of.entry(id).or_default();
        }
    }
    for owner in &owners {
        let mut seen = BTreeSet::from([*owner]);
        let mut stack = vec![*owner];
        while let Some(next) = stack.pop() {
            for symbol in symbols_of.get(&next).into_iter().flatten() {
                if seen.insert(*symbol) {
                    stack.push(*symbol);
                }
            }
        }
        let graphs: Vec<u64> = seen.intersection(&broken).copied().collect();
        if !graphs.is_empty() {
            println!("reach owner={owner} graphs={graphs:?}");
        }
    }
    for (class, t) in &tally {
        println!(
            "faces surface={class} meshed={} disconnected={} other={}",
            t.meshed, t.disconnected, t.other
        );
        if t.loops > 0 {
            println!(
                "gaps surface={class} loops={} under_1e-6={} 1e-6_to_1e-4={} 1e-4_to_0.01={} over_0.01={} closing={} other_meets={}",
                t.loops, t.bins[0], t.bins[1], t.bins[2], t.bins[3], t.closing, t.other_meets
            );
        }
    }
    Ok(())
}
