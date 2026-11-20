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
//! ```text
//!   break graph=<ElementId> face=<n> surface=<class> loop=<n> edges=<n> chained=<n>
//!     gap=<ft> closing=<true|false> other_meets=<true|false>
//!   - `chained`: the edges the attempt that joins the most joined; `closing` where every
//!     edge joined and only the step back to the first point fails;
//!   - `gap`: from the last joined point to the nearer end of the next edge (or back to the
//!     first point), in feet;
//!   - `other_meets`: an edge further on in the loop meets the last point instead, within
//!     the same 1e-7 every join fails, so its zeros do not show that the edges are in order;
//!   - then `param_gap=<d>`: the larger parameter difference across that gap;
//!     `joins_within=<ft|none>`: the smallest of 1e-6, 1e-5, 1e-4 and 1e-3 ft within which the
//!     whole loop joins, each edge at its nearer end and back to the first point, either way
//!     round; `shortest_edge=<ft>`: the shortest edge's span between its ends (an edge shorter
//!     than a bound joins at either end);
//!   - then, at that bound: `small_gap=<d>`, the largest of the joins' parameter gaps over the
//!     surface's parameter range that is not over 1e-2; `pole_joins=<n>`, the joins over it
//!     whose point halfway between in the parameters is the same point too (a pole);
//!     `seam_joins=<n>`, those over it where that point is not (a seam or another jump).
//!   unread graph=<ElementId> face=<n> why=<reason>: a refused face `trim_breaks` cannot read.
//!   reach owner=<ElementId> graphs=[<ElementId>, ...]: an element whose graphics reach a
//!     graph with a break.
//! then by the refused face's surface class:
//!   faces surface=<class> meshed=<n> disconnected=<n> other=<n>
//!   gaps surface=<class> loops=<n> under_1e-6=<n> 1e-6_to_1e-4=<n> 1e-4_to_0.01=<n>
//!     over_0.01=<n> closing=<n> other_meets=<n>
//!   joins surface=<class> within_1e-6=<n> within_1e-5=<n> within_1e-4=<n> within_1e-3=<n>
//!     none=<n> shortest_edge_under_bound=<n>: the loops by `joins_within`, and those whose
//!     shortest edge is shorter than the bound they join within.
//!   params surface=<class> small_under_1e-6=<n> 1e-6_to_1e-4=<n> 1e-4_to_1e-2=<n>
//!     loops_with_pole_joins=<n> loops_with_seam_joins=<n>: the joined loops by `small_gap`
//!     and by their jumps.
//!   polygon graph=<ElementId> face=<n> surface=<class> loop=<n> edges=<n> joined=<n>
//!     closing=<bool> bounded=<bool> meets=<bool> param_gap=<d> relative_gap=<d>
//!     halfway_same=<bool>: for a face refused as `a trim loop does not close in the
//!     surface's parameters`, where its polygon fails on the attempt that gets furthest:
//!     `bounded` where the loop joins only within a bound looser than 1e-7, `meets` where the
//!     two points meet in model coordinates within that bound, the parameters' difference
//!     there and over the surface's range, and whether the point halfway across is the same
//!     point (a side that collapses); then by surface class:
//!   polygons surface=<class> faces=<n> loops=<n> relative_under_1e-6=<n> 1e-6_to_1e-4=<n>
//!     1e-4_to_1e-2=<n> 1e-2_or_more=<n> closing=<n> bounded=<n> meets=<n> halfway_same=<n>
//!   ring surface=<Plane|CylSurf> faces=<n> refused=<n>: those faces put to their own ring
//!     (`native_saved_mesh::face`), and the refused ones;
//!   ringwhy surface=<Plane|CylSurf> faces=<n> why=<reason, digits masked>: the eight most
//!     common reasons, so one a neighbour's grid raises shows apart from the ring's own;
//!   chords surface=<class> ends=<n> moved=<n> largest=<ft>: the ends of chord edges (drawn
//!     straight to a plane) the polygons hold, those a merged join moved off the edge's own end,
//!     which the plane beside it keeps, and the largest move
//! ```
//! ElementIds, counts and distances in feet only.
//!
//! Usage:
//! ```text
//!   cargo run --profile ci -p rvt-probes --example probe_2085_trim_breaks -- MODEL.rvt
//! ```

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{self, Record};
use rvt::research::native_parameters::ObjectGraph;
use rvt::research::native_parametric_mesh::{
    JOIN_BOUNDS, face_grid, merged_chord_ends, polygon_breaks, trim_breaks,
};
use rvt::research::native_saved_mesh::pointer;

const DISCONNECTED: &str = "disconnected parametric trim in model coordinates";
const POLYGON: &str = "a trim loop does not close in the surface's parameters";

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
    ring_faces: usize,
    ring_refused: BTreeMap<String, usize>,
    meshed: usize,
    disconnected: usize,
    other: usize,
    loops: usize,
    bins: [usize; 4],
    closing: usize,
    other_meets: usize,
    joins: [usize; 5],
    short: usize,
    small: [usize; 3],
    with_poles: usize,
    with_seams: usize,
    polygon_faces: usize,
    polygon_loops: usize,
    polygon_bins: [usize; 4],
    polygon_closing: usize,
    polygon_bounded: usize,
    polygon_meets: usize,
    polygon_halfway: usize,
    chord_ends: usize,
    chord_moved: usize,
    chord_largest: f64,
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
            // Their own ring (`native_saved_mesh::face`): what each refused face says, digits
            // masked, so a reason a neighbour's grid raised can be told from the ring's own.
            let t = tally.entry(class.clone()).or_default();
            t.ring_faces += 1;
            if let Err(error) = rvt::research::native_saved_mesh::face(graph, fi) {
                let text: String = format!("{error:#}")
                    .chars()
                    .map(|c| if c.is_ascii_digit() { '#' } else { c })
                    .take(200)
                    .collect();
                *t.ring_refused.entry(text).or_default() += 1;
            }
            continue;
        }
        if let Ok((_, ends, moved, largest)) = merged_chord_ends(graph, fi) {
            let t = tally.entry(class.clone()).or_default();
            t.chord_ends += ends;
            t.chord_moved += moved;
            t.chord_largest = t.chord_largest.max(largest);
        }
        let why = face_grid(graph, fi).err().map(|e| format!("{e:#}"));
        let t = tally.entry(class.clone()).or_default();
        match &why {
            None => t.meshed += 1,
            Some(w) if w.contains(DISCONNECTED) => t.disconnected += 1,
            Some(_) => t.other += 1,
        }
        if why.as_deref().is_some_and(|w| w.contains(POLYGON)) {
            match polygon_breaks(graph, fi) {
                Ok((_, breaks)) => {
                    let t = tally.entry(class.clone()).or_default();
                    t.polygon_faces += 1;
                    for b in breaks {
                        any = true;
                        t.polygon_loops += 1;
                        let bin = match b.relative_gap {
                            g if g < 1e-6 => 0,
                            g if g < 1e-4 => 1,
                            g if g < 1e-2 => 2,
                            _ => 3,
                        };
                        t.polygon_bins[bin] += 1;
                        let closing = b.joined == b.edges;
                        t.polygon_closing += usize::from(closing);
                        t.polygon_bounded += usize::from(b.bounded);
                        t.polygon_meets += usize::from(b.meets);
                        t.polygon_halfway += usize::from(b.halfway_same);
                        println!(
                            "polygon graph={id} face={fi} surface={class} loop={} edges={} joined={} closing={closing} bounded={} meets={} param_gap={:.3e} relative_gap={:.3e} halfway_same={}",
                            b.loop_index,
                            b.edges,
                            b.joined,
                            b.bounded,
                            b.meets,
                            b.param_gap,
                            b.relative_gap,
                            b.halfway_same
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
                    let within = JOIN_BOUNDS.iter().position(|x| Some(*x) == b.joins_within);
                    t.joins[within.unwrap_or(4)] += 1;
                    if b.joins_within.is_some_and(|x| b.shortest_edge < x) {
                        t.short += 1;
                    }
                    if b.joins_within.is_some() {
                        let bin = match b.small_gap {
                            g if g < 1e-6 => 0,
                            g if g < 1e-4 => 1,
                            _ => 2,
                        };
                        t.small[bin] += 1;
                        t.with_poles += usize::from(b.pole_joins > 0);
                        t.with_seams += usize::from(b.seam_joins > 0);
                    }
                    let shown = b
                        .joins_within
                        .map_or_else(|| "none".to_string(), |x| format!("{x:e}"));
                    println!(
                        "break graph={id} face={fi} surface={class} loop={} edges={} chained={} gap={:.9} closing={closing} other_meets={} param_gap={:.3e} joins_within={shown} shortest_edge={:.9} small_gap={:.3e} pole_joins={} seam_joins={}",
                        b.loop_index,
                        b.edges,
                        b.chained,
                        b.gap,
                        b.other_meets,
                        b.param_gap,
                        b.shortest_edge,
                        b.small_gap,
                        b.pole_joins,
                        b.seam_joins
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
        if t.ring_faces > 0 {
            let refused: usize = t.ring_refused.values().sum();
            println!(
                "ring surface={class} faces={} refused={refused}",
                t.ring_faces
            );
            let mut reasons: Vec<(&String, &usize)> = t.ring_refused.iter().collect();
            reasons.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            for (why, n) in reasons.into_iter().take(8) {
                println!("ringwhy surface={class} faces={n} why={why}");
            }
            continue;
        }
        println!(
            "faces surface={class} meshed={} disconnected={} other={}",
            t.meshed, t.disconnected, t.other
        );
        if t.chord_ends > 0 {
            println!(
                "chords surface={class} ends={} moved={} largest={:.9}",
                t.chord_ends, t.chord_moved, t.chord_largest
            );
        }
        if t.polygon_loops > 0 {
            println!(
                "polygons surface={class} faces={} loops={} relative_under_1e-6={} 1e-6_to_1e-4={} 1e-4_to_1e-2={} 1e-2_or_more={} closing={} bounded={} meets={} halfway_same={}",
                t.polygon_faces,
                t.polygon_loops,
                t.polygon_bins[0],
                t.polygon_bins[1],
                t.polygon_bins[2],
                t.polygon_bins[3],
                t.polygon_closing,
                t.polygon_bounded,
                t.polygon_meets,
                t.polygon_halfway
            );
        }
        if t.loops > 0 {
            println!(
                "gaps surface={class} loops={} under_1e-6={} 1e-6_to_1e-4={} 1e-4_to_0.01={} over_0.01={} closing={} other_meets={}",
                t.loops, t.bins[0], t.bins[1], t.bins[2], t.bins[3], t.closing, t.other_meets
            );
            println!(
                "joins surface={class} within_1e-6={} within_1e-5={} within_1e-4={} within_1e-3={} none={} shortest_edge_under_bound={}",
                t.joins[0], t.joins[1], t.joins[2], t.joins[3], t.joins[4], t.short
            );
            println!(
                "params surface={class} small_under_1e-6={} 1e-6_to_1e-4={} 1e-4_to_1e-2={} loops_with_pole_joins={} loops_with_seam_joins={}",
                t.small[0], t.small[1], t.small[2], t.with_poles, t.with_seams
            );
        }
    }
    Ok(())
}
