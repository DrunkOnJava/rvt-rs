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
//!   gap surface=<Plane|CylSurf> graph=<ElementId> face=<n> loop=<n> edges=<n>
//!     stored=<d>/<ft>ft/<n> refined=<d>/<ft>ft/<n> du=<d> dv=<d>: for a face refused as
//!     `disconnected UV boundary`, each of its rings whose own join fails
//!     (`native_saved_mesh::ring_gaps`): the largest gap between neighbouring edges' ends
//!     in the surface's parameters, the same gaps' largest in feet and how many are not
//!     within the join's 1e-7, with the edges as the graph stores them and as refined, and
//!     the largest refined gap's difference in each parameter (a gap near 2π in `du` with
//!     no feet is a seam, not noise), `back=<n>`, the refined gaps with a component back
//!     along the edge before them, `step=<ft>`, the largest refined gap as a straight step in
//!     the parameters measures along the surface (a chord on a plane, a helix on a cylinder),
//!     and `shortest=<ft>`, the shortest step between one stored edge's own ends;
//!   gapfrom surface=Plane graph=<ElementId> face=<n> loop=<n> neighbours=[<n>, ...]
//!     why=<reason>: a ring whose refinement fails, with the cylinder faces beside it whose
//!     own ring is refused as `disconnected UV boundary` (a plane meshes such a neighbour to
//!     share their edge, so the cylinder's refusal becomes the plane's);
//!   refused surface=<Plane|CylSurf> graph=<ElementId> face=<n> why=<reason, digits masked>:
//!     every refused plane or cylinder face, so two commits' faces pair by graph and face;
//!   gapnone, gapunread surface=<class> graph=<ElementId> face=<n>: no ring found failing,
//!     or the read itself refused;
//!   ringreach owner=<ElementId> graphs=[<ElementId>, ...]: an element whose graphics reach
//!     a graph holding such a face; then by class:
//!   gapsrc surface=<class> disconnected=<n> own=<n> neighbour=<n> other=<n> neither=<n>
//!     unread=<n>: the faces by where their first failing ring fails, which add up;
//!   gaploops surface=<class> loops=<n> stored_open=<n> seam_like=<n> backward=<n>: the rings
//!     whose own join fails, those whose stored edges already leave a gap open, those with a
//!     gap of 1e-2 or more in the parameters but under 1e-4 ft, and those with a gap that runs
//!     back along the edge before it;
//!   gaphist surface=<class> unit=<parameters|feet> under_1e-7=<n> ... 1e-2_or_more=<n>:
//!     those rings by their largest refined gap, by decade;
//!   chords surface=<class> ends=<n> moved=<n> largest=<ft>: the ends of chord edges (drawn
//!     straight to a plane) the polygons hold, those a merged join moved off the edge's own end,
//!     which the plane beside it keeps, and the largest move
//!   piece graph=<ElementId> face=<n> surface=<Plane|CylSurf> loop=<n> at=<k>/<edges>
//!     gap=<ft> du_ft=<ft> dv_ft=<ft> sig=<one|diagonal> tenth_mm=<bool> ends=<edge|none>
//!     through=<edge|none> mid=<ft>@<edge>:<faces> orphans=<n> orphan_near=<ft|none>
//!     sides_a=<ft|unread> sides_b=<ft|unread> unread_edges=<n>: does the graph store the edge
//!     a failing ring is missing? For each gap of 0.00001 ft (Griffin's touching bound) or more
//!     between the ring's stored edges, each joined at its nearer end in model coordinates,
//!     from the ring's point P before the gap to the next edge's end Q: its size and each
//!     parameter's share in feet (`one` where one is under 1% of the other; `tenth_mm` where
//!     one is 0.1 mm); `ends`, an edge of the graph (not the two beside the gap) with one end
//!     at P and the other at Q within the bound; `through`, one whose points and spans pass
//!     within it of both; `mid`, the nearest edge to the gap's midpoint and its faces (`F` for
//!     this face); `orphans`, the edges that name this face but that none of its loops reaches,
//!     and the nearest of them to P or Q; `sides_*`, how far apart the two faces' surfaces put
//!     the ends of the edge before (`a`) and after (`b`) the gap. An edge is placed through the
//!     first of its faces that is a plane or a cylinder; `unread_edges` have neither. Pointers
//!     and the graph's own points only: nothing is joined.
//!   piecenbr graph=<ElementId> face=<n> loop=<n> at=<k> a=<face>:<class>:<edges> b=...: the
//!     edges either side of the edge before (`a`) and after (`b`) the gap on the other face's
//!     own loop, each as `<edge>[<faces>]:<ft to P>/<ft to Q>`;
//!   piecenone, pieceunread graph=<ElementId> face=<n> loop=<n>: a failing ring with no gap of
//!     the bound or more in model coordinates, or one whose chain or points cannot be read;
//!   keys loop=[..] edge=[..] endpoint=[..] interior=[..] face=[..]: once, the field names of
//!     the first gap's loop, edge, edge points and face;
//!   pieces surface=<class> gaps=<n> one=<n> diagonal=<n> tenth_mm=<n> ends=<n> through=<n>
//!     mid_covered=<n> orphan_near=<n> sides_disagree=<n> rings_under=<n> unread=<n>: by class;
//!   graphs read=<n> refused=<n>: the graphs decoded, and those refused whole for the budget.
//! ```
//! ElementIds, counts and distances in feet only.
//!
//! Usage:
//! ```text
//!   cargo run --profile ci -p rvt-probes --example probe_2085_trim_breaks -- MODEL.rvt
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{self, Record};
use rvt::research::native_parameters::ObjectGraph;
use rvt::research::native_parametric_mesh::{
    JOIN_BOUNDS, face_grid, merged_chord_ends, polygon_breaks, trim_breaks,
};
use rvt::research::native_saved_mesh::{pointer, ring_gaps};

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
    gap_disconnected: usize,
    gap_own: usize,
    gap_neighbour: usize,
    gap_other: usize,
    gap_neither: usize,
    gap_unread: usize,
    gap_loops: usize,
    gap_stored_open: usize,
    gap_seam_like: usize,
    gap_backward: usize,
    gap_parameters: [usize; 7],
    gap_feet: [usize; 7],
    piece_gaps: usize,
    piece_one: usize,
    piece_tenth: usize,
    piece_ends: usize,
    piece_through: usize,
    piece_mid: usize,
    piece_orphan: usize,
    piece_sides: usize,
    piece_unread: usize,
    piece_rings_under: usize,
}

/// The decade a gap falls in: under 1e-7, then up to 1e-6, 1e-5, 1e-4, 1e-3, 1e-2, beyond.
fn decade(x: f64) -> usize {
    [1e-7, 1e-6, 1e-5, 1e-4, 1e-3, 1e-2]
        .iter()
        .filter(|t| x >= **t)
        .count()
}

/// Two points this close in feet are one point: Griffin's bound for a ring's ends (#2085).
const TOUCHING: f64 = 1e-5;
/// 0.1 mm in feet: the gap three housing graphs' rings leave in one parameter.
const TENTH_MM: f64 = 0.1 / 304.8;
static KEYS_SHOWN: AtomicBool = AtomicBool::new(false);

type Polyline = Vec<[f64; 3]>;

/// A plane or cylinder surface, as `native_saved_mesh::ring_gaps` evaluates it.
#[derive(Clone, Copy)]
enum Surf {
    Plane {
        o: [f64; 3],
        x: [f64; 3],
        y: [f64; 3],
    },
    Cyl {
        c: [f64; 3],
        x: [f64; 3],
        y: [f64; 3],
        z: [f64; 3],
        r: f64,
    },
}

impl Surf {
    fn at(&self, uv: [f64; 2]) -> [f64; 3] {
        match *self {
            Surf::Plane { o, x, y } => std::array::from_fn(|k| o[k] + uv[0] * x[k] + uv[1] * y[k]),
            Surf::Cyl { c, x, y, z, r } => {
                let (s, co) = uv[0].sin_cos();
                std::array::from_fn(|k| c[k] + r * (co * x[k] + s * y[k]) + uv[1] * z[k])
            }
        }
    }
    /// Feet per unit of each parameter.
    fn scales(&self) -> [f64; 2] {
        let norm = |v: [f64; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        match *self {
            Surf::Plane { x, y, .. } => [norm(x), norm(y)],
            Surf::Cyl { z, r, .. } => [r, norm(z)],
        }
    }
}

fn coords<const N: usize>(v: &Fv) -> Option<[f64; N]> {
    let a = v.as_array()?;
    if a.len() != N {
        return None;
    }
    let mut p = [0.0; N];
    for (i, x) in a.iter().enumerate() {
        p[i] = x.as_f64().filter(|x| x.is_finite())?;
    }
    Some(p)
}

fn surf(graph: &ObjectGraph, face: usize) -> Option<Surf> {
    let si = pointer(graph, face, graph.objects.get(face)?.fields.get("m_pSurf")?).ok()??;
    let object = graph.objects.get(si)?;
    let s = &object.fields;
    match object.class_name.as_str() {
        "Plane" => Some(Surf::Plane {
            o: coords(s.get("m_origin")?)?,
            x: coords(s.get("m_xVec")?)?,
            y: coords(s.get("m_yVec")?)?,
        }),
        "CylSurf" => Some(Surf::Cyl {
            c: coords(s.get("m_center")?)?,
            x: coords(s.get("m_xVec")?)?,
            y: coords(s.get("m_yVec")?)?,
            z: coords(s.get("m_zVec")?)?,
            r: s.get("m_radius")?.as_f64()?,
        }),
        _ => None,
    }
}

fn surface_class(graph: &ObjectGraph, face: usize) -> String {
    graph
        .objects
        .get(face)
        .and_then(|f| f.fields.get("m_pSurf"))
        .and_then(|p| pointer(graph, face, p).ok().flatten())
        .and_then(|si| graph.objects.get(si))
        .map_or_else(|| "unknown".to_string(), |s| s.class_name.clone())
}

fn edge_faces(graph: &ObjectGraph, e: usize) -> [Option<usize>; 2] {
    let faces = graph.objects[e]
        .fields
        .get("m_pFace")
        .and_then(|f| f.as_array());
    std::array::from_fn(|k| {
        faces
            .and_then(|f| f.get(k))
            .and_then(|p| pointer(graph, e, p).ok().flatten())
    })
}

/// An edge's stored points in the parameters of its face on `side`, ends included.
fn edge_uv(graph: &ObjectGraph, e: usize, side: usize) -> Option<Vec<[f64; 2]>> {
    let f = &graph.objects[e].fields;
    let ends = f.get("m_firstAndLastEdgePnts")?.as_array()?;
    if ends.len() != 2 {
        return None;
    }
    let mut pts = vec![coords::<2>(ends[0].get("uv")?.get(side)?)?];
    for p in f.get("m_interiorEdgePnts")?.as_array()? {
        pts.push(coords(p.get("uv")?.get(side)?)?);
    }
    pts.push(coords(ends[1].get("uv")?.get(side)?)?);
    Some(pts)
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// From `p` to the polyline `pts`, its spans included.
fn polyline_dist(p: [f64; 3], pts: &[[f64; 3]]) -> f64 {
    let mut best = pts
        .iter()
        .map(|q| dist(p, *q))
        .fold(f64::INFINITY, f64::min);
    for w in pts.windows(2) {
        let ab: [f64; 3] = std::array::from_fn(|k| w[1][k] - w[0][k]);
        let len2 = ab.iter().map(|x| x * x).sum::<f64>();
        if len2 > 0.0 {
            let t = ((0..3).map(|k| (p[k] - w[0][k]) * ab[k]).sum::<f64>() / len2).clamp(0.0, 1.0);
            best = best.min(dist(p, std::array::from_fn(|k| w[0][k] + t * ab[k])));
        }
    }
    best
}

/// One graph's edges in model coordinates, each through the first of its faces that is a
/// plane or a cylinder (`None` where neither is), built once per graph.
struct EdgeCache {
    xyz: Vec<(usize, Option<Polyline>)>,
    surfs: BTreeMap<usize, Option<Surf>>,
    naming: BTreeMap<usize, Vec<usize>>,
}

impl EdgeCache {
    fn new(graph: &ObjectGraph) -> EdgeCache {
        let mut surfs: BTreeMap<usize, Option<Surf>> = BTreeMap::new();
        let mut naming: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        let mut xyz = Vec::new();
        for (e, object) in graph.objects.iter().enumerate() {
            if object.class_name != "Edge" {
                continue;
            }
            let faces = edge_faces(graph, e);
            for face in faces.iter().flatten() {
                naming.entry(*face).or_default().push(e);
            }
            let mut line = None;
            for (side, face) in faces.iter().enumerate() {
                let Some(face) = face else { continue };
                let s = *surfs.entry(*face).or_insert_with(|| surf(graph, *face));
                if let Some(s) = s {
                    line = edge_uv(graph, e, side).map(|p| p.iter().map(|q| s.at(*q)).collect());
                    break;
                }
            }
            xyz.push((e, line));
        }
        EdgeCache { xyz, surfs, naming }
    }
    fn surf(&mut self, graph: &ObjectGraph, face: usize) -> Option<Surf> {
        *self.surfs.entry(face).or_insert_with(|| surf(graph, face))
    }
    /// How far an edge's ends lie apart as its two faces' surfaces place them, where both
    /// are planes or cylinders.
    fn sides(&mut self, graph: &ObjectGraph, e: usize) -> Option<f64> {
        let faces = edge_faces(graph, e);
        let mut ends = Vec::new();
        for (side, face) in faces.iter().enumerate() {
            let s = self.surf(graph, (*face)?)?;
            let pts = edge_uv(graph, e, side)?;
            ends.push([s.at(pts[0]), s.at(pts[pts.len() - 1])]);
        }
        Some(dist(ends[0][0], ends[1][0]).max(dist(ends[0][1], ends[1][1])))
    }
}

fn face_loops(graph: &ObjectGraph, fi: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let mut next = graph.objects[fi]
        .fields
        .get("m_pFirstLoop")
        .and_then(|p| pointer(graph, fi, p).ok().flatten());
    while let Some(li) = next {
        if out.contains(&li) {
            break;
        }
        out.push(li);
        next = graph.objects[li]
            .fields
            .get("m_nextLoop")
            .and_then(|p| pointer(graph, li, p).ok().flatten());
    }
    out
}

/// A loop's edges in chain order, each with the side of it that names the face.
fn loop_chain(graph: &ObjectGraph, fi: usize, li: usize) -> Option<Vec<(usize, usize)>> {
    let mut cur = pointer(graph, li, graph.objects[li].fields.get("m_next")?).ok()??;
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    while cur != li {
        if !seen.insert(cur) || graph.objects.get(cur)?.class_name != "Edge" {
            return None;
        }
        let faces = edge_faces(graph, cur);
        let side = (0..2).find(|k| faces[*k] == Some(fi))?;
        out.push((cur, side));
        cur = pointer(
            graph,
            cur,
            graph.objects[cur].fields.get("m_next")?.get(side)?,
        )
        .ok()??;
    }
    Some(out)
}

/// One gap of a loop's stored edges of `TOUCHING` or more, in model coordinates.
struct Gap {
    at: usize,
    a: usize,
    a_side: usize,
    b: usize,
    b_side: usize,
    p: [f64; 3],
    q: [f64; 3],
    duv: [f64; 2],
    d: f64,
}

/// The gaps of `TOUCHING` or more the loop's stored edges leave, each edge joined at its
/// nearer end in model coordinates, of the two ways round the first edge the one whose
/// largest gap is smaller.
fn open_gaps(graph: &ObjectGraph, chain: &[(usize, usize)], s: Surf) -> Option<Vec<Gap>> {
    let segs = chain
        .iter()
        .map(|(e, side)| edge_uv(graph, *e, *side))
        .collect::<Option<Vec<_>>>()?;
    if segs.is_empty() {
        return None;
    }
    let mut best: Option<(f64, Vec<Gap>)> = None;
    for reverse in [false, true] {
        let p0 = &segs[0];
        let (first, mut last) = if reverse {
            (p0[p0.len() - 1], p0[0])
        } else {
            (p0[0], p0[p0.len() - 1])
        };
        let mut prev = 0usize;
        let mut gaps = Vec::new();
        let mut worst: f64 = 0.0;
        let gap = |at: usize, prev: usize, from: [f64; 2], to: [f64; 2], gaps: &mut Vec<Gap>| {
            let d = dist(s.at(from), s.at(to));
            if d >= TOUCHING {
                gaps.push(Gap {
                    at,
                    a: chain[prev].0,
                    a_side: chain[prev].1,
                    b: chain[at].0,
                    b_side: chain[at].1,
                    p: s.at(from),
                    q: s.at(to),
                    duv: [to[0] - from[0], to[1] - from[1]],
                    d,
                });
            }
            d
        };
        for (k, pts) in segs.iter().enumerate().skip(1) {
            let (h, t) = (pts[0], pts[pts.len() - 1]);
            let (near, far) = if dist(s.at(last), s.at(h)) <= dist(s.at(last), s.at(t)) {
                (h, t)
            } else {
                (t, h)
            };
            worst = worst.max(gap(k, prev, last, near, &mut gaps));
            last = far;
            prev = k;
        }
        worst = worst.max(gap(0, prev, last, first, &mut gaps));
        if best.as_ref().is_none_or(|(w, _)| worst < *w) {
            best = Some((worst, gaps));
        }
    }
    best.map(|(_, gaps)| gaps)
}

fn keys(v: Option<&Fv>) -> Vec<String> {
    v.and_then(|v| v.as_object())
        .map(|o| o.keys().map(str::to_string).collect())
        .unwrap_or_default()
}

/// An edge beside `e` on the loop of `face` that holds it: its two neighbours in chain order.
fn chain_neighbours(graph: &ObjectGraph, face: usize, e: usize) -> Option<[usize; 2]> {
    for li in face_loops(graph, face) {
        let Some(chain) = loop_chain(graph, face, li) else {
            continue;
        };
        if let Some(i) = chain.iter().position(|(x, _)| *x == e) {
            let n = chain.len();
            return Some([chain[(i + n - 1) % n].0, chain[(i + 1) % n].0]);
        }
    }
    None
}

/// For each gap of `TOUCHING` or more in a failing ring's stored edges: whether the graph
/// stores an edge that fills it, by pointers first and then by distance (#2085).
#[allow(clippy::too_many_arguments)]
fn read_pieces(
    id: u64,
    fi: usize,
    class: &str,
    loop_index: usize,
    graph: &ObjectGraph,
    cache: &mut Option<EdgeCache>,
    t: &mut Tally,
) {
    let loops = face_loops(graph, fi);
    let (Some(&li), Some(s)) = (loops.get(loop_index), surf(graph, fi)) else {
        t.piece_unread += 1;
        println!("pieceunread graph={id} face={fi} loop={loop_index} why=loop_or_surface");
        return;
    };
    let Some(chain) = loop_chain(graph, fi, li) else {
        t.piece_unread += 1;
        println!("pieceunread graph={id} face={fi} loop={loop_index} why=chain");
        return;
    };
    let Some(gaps) = open_gaps(graph, &chain, s) else {
        t.piece_unread += 1;
        println!("pieceunread graph={id} face={fi} loop={loop_index} why=points");
        return;
    };
    if gaps.is_empty() {
        t.piece_rings_under += 1;
        println!("piecenone graph={id} face={fi} loop={loop_index}");
        return;
    }
    if !KEYS_SHOWN.swap(true, Ordering::Relaxed) {
        let a = &graph.objects[gaps[0].a].fields;
        let end = a
            .get("m_firstAndLastEdgePnts")
            .and_then(|v| v.as_array())
            .and_then(|v| v.first());
        let interior = a
            .get("m_interiorEdgePnts")
            .and_then(|v| v.as_array())
            .and_then(|v| v.first());
        println!(
            "keys loop={:?} edge={:?} endpoint={:?} interior={:?} face={:?}",
            keys(Some(&graph.objects[li].fields)),
            keys(Some(a)),
            keys(end),
            keys(interior),
            keys(Some(&graph.objects[fi].fields)),
        );
    }
    let cache = cache.get_or_insert_with(|| EdgeCache::new(graph));
    let reached: BTreeSet<usize> = loops
        .iter()
        .filter_map(|l| loop_chain(graph, fi, *l))
        .flatten()
        .map(|(e, _)| e)
        .collect();
    let orphans: Vec<usize> = cache
        .naming
        .get(&fi)
        .into_iter()
        .flatten()
        .filter(|e| !reached.contains(*e))
        .copied()
        .collect();
    let unread = cache.xyz.iter().filter(|(_, l)| l.is_none()).count();
    let scales = s.scales();
    for g in &gaps {
        t.piece_gaps += 1;
        let comp = [g.duv[0] * scales[0], g.duv[1] * scales[1]];
        let (lo, hi) = if comp[0].abs() <= comp[1].abs() {
            (comp[0].abs(), comp[1].abs())
        } else {
            (comp[1].abs(), comp[0].abs())
        };
        let sig = if hi > 0.0 && lo / hi < 0.01 {
            "one"
        } else {
            "diagonal"
        };
        let tenth = comp.iter().any(|c| (c.abs() - TENTH_MM).abs() < 1e-7);
        t.piece_one += usize::from(sig == "one");
        t.piece_tenth += usize::from(tenth);
        let mid: [f64; 3] = std::array::from_fn(|k| (g.p[k] + g.q[k]) / 2.0);
        let mut ends = None;
        let mut through = None;
        let mut nearest_mid = (f64::INFINITY, 0usize);
        for (e, line) in &cache.xyz {
            if *e == g.a || *e == g.b {
                continue;
            }
            let Some(line) = line else { continue };
            let (e0, e1) = (line[0], line[line.len() - 1]);
            if (dist(e0, g.p) < TOUCHING && dist(e1, g.q) < TOUCHING)
                || (dist(e0, g.q) < TOUCHING && dist(e1, g.p) < TOUCHING)
            {
                ends.get_or_insert(*e);
            }
            if polyline_dist(g.p, line) < TOUCHING && polyline_dist(g.q, line) < TOUCHING {
                through.get_or_insert(*e);
            }
            let dm = polyline_dist(mid, line);
            if dm < nearest_mid.0 {
                nearest_mid = (dm, *e);
            }
        }
        t.piece_ends += usize::from(ends.is_some());
        t.piece_through += usize::from(through.is_some());
        t.piece_mid += usize::from(nearest_mid.0 < TOUCHING);
        let orphan_near = orphans
            .iter()
            .filter_map(|e| {
                cache
                    .xyz
                    .iter()
                    .find(|(x, _)| x == e)
                    .and_then(|(_, l)| l.as_ref())
                    .map(|l| polyline_dist(g.p, l).min(polyline_dist(g.q, l)))
            })
            .fold(f64::INFINITY, f64::min);
        t.piece_orphan += usize::from(orphan_near < TOUCHING);
        let (sa, sb) = (cache.sides(graph, g.a), cache.sides(graph, g.b));
        t.piece_sides +=
            usize::from(sa.is_some_and(|x| x >= TOUCHING) || sb.is_some_and(|x| x >= TOUCHING));
        let shown = |x: Option<f64>| x.map_or_else(|| "unread".to_string(), |x| format!("{x:.9}"));
        let named = |x: Option<usize>| x.map_or_else(|| "none".to_string(), |x| x.to_string());
        let faces_of = |e: usize| {
            edge_faces(graph, e)
                .iter()
                .map(|f| match f {
                    Some(f) if *f == fi => "F".to_string(),
                    Some(f) => format!("{f}:{}", surface_class(graph, *f)),
                    None => "-".to_string(),
                })
                .collect::<Vec<_>>()
                .join(",")
        };
        println!(
            "piece graph={id} face={fi} surface={class} loop={loop_index} at={}/{} gap={:.9} du_ft={:.9} dv_ft={:.9} sig={sig} tenth_mm={tenth} ends={} through={} mid={:.9}@{}:{} orphans={} orphan_near={} sides_a={} sides_b={} unread_edges={unread}",
            g.at,
            chain.len(),
            g.d,
            comp[0],
            comp[1],
            named(ends),
            named(through),
            nearest_mid.0,
            nearest_mid.1,
            faces_of(nearest_mid.1),
            orphans.len(),
            if orphan_near.is_finite() {
                format!("{orphan_near:.9}")
            } else {
                "none".to_string()
            },
            shown(sa),
            shown(sb),
        );
        // The neighbour faces' chains at the two edges either side of the gap: an edge there
        // with an end at both P and Q would be the missing piece, stored on the other face.
        let mut beside = Vec::new();
        for (label, e, side) in [("a", g.a, g.a_side), ("b", g.b, g.b_side)] {
            let Some(other) = edge_faces(graph, e)[1 - side] else {
                beside.push(format!("{label}=none"));
                continue;
            };
            let neighbours = chain_neighbours(graph, other, e);
            let text = neighbours.map_or_else(
                || "unread".to_string(),
                |pair| {
                    pair.iter()
                        .map(|n| {
                            let line = cache
                                .xyz
                                .iter()
                                .find(|(x, _)| x == n)
                                .and_then(|(_, l)| l.as_ref());
                            match line {
                                Some(l) => format!(
                                    "{n}[{}]:{:.9}/{:.9}",
                                    faces_of(*n),
                                    polyline_dist(g.p, l),
                                    polyline_dist(g.q, l)
                                ),
                                None => format!("{n}[{}]:unread", faces_of(*n)),
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(";")
                },
            );
            beside.push(format!(
                "{label}={other}:{}:{text}",
                surface_class(graph, other)
            ));
        }
        println!(
            "piecenbr graph={id} face={fi} loop={loop_index} at={} {}",
            g.at,
            beside.join(" ")
        );
    }
}

/// The rings of a face refused as `disconnected UV boundary`: prints where each fails.
fn read_gaps(
    id: u64,
    fi: usize,
    class: &str,
    graph: &ObjectGraph,
    cache: &mut Option<EdgeCache>,
    t: &mut Tally,
) {
    let short = |text: &str| text.chars().take(160).collect::<String>();
    let rings = match ring_gaps(graph, fi) {
        Ok(rings) => rings,
        Err(error) => {
            t.gap_unread += 1;
            println!(
                "gapunread surface={class} graph={id} face={fi} why={}",
                short(format!("{error:#}").as_str())
            );
            return;
        }
    };
    let mut first = None;
    for r in &rings {
        match &r.refined {
            Ok(j) if j.open > 0 => {
                println!(
                    "gap surface={class} graph={id} face={fi} loop={} edges={} stored={:e}/{:e}ft/{} refined={:e}/{:e}ft/{} du={:e} dv={:e} back={} step={:e} shortest={:e}",
                    r.loop_index,
                    r.edges,
                    r.stored.parameters,
                    r.stored.feet,
                    r.stored.open,
                    j.parameters,
                    j.feet,
                    j.open,
                    j.components[0],
                    j.components[1],
                    j.backward,
                    j.step,
                    r.shortest_edge
                );
                if j.backward > 0 {
                    t.gap_backward += 1;
                }
                t.gap_loops += 1;
                t.gap_parameters[decade(j.parameters)] += 1;
                t.gap_feet[decade(j.feet)] += 1;
                if r.stored.open > 0 {
                    t.gap_stored_open += 1;
                }
                if j.parameters >= 1e-2 && j.feet < 1e-4 {
                    t.gap_seam_like += 1;
                }
                first.get_or_insert("own");
                if r.stored.open > 0 {
                    read_pieces(id, fi, class, r.loop_index, graph, cache, t);
                }
            }
            Ok(_) => {}
            Err(why) => {
                println!(
                    "gapfrom surface={class} graph={id} face={fi} loop={} neighbours={:?} why={}",
                    r.loop_index,
                    r.disconnected_neighbours,
                    short(why.as_str())
                );
                first.get_or_insert(if r.disconnected_neighbours.is_empty() {
                    "other"
                } else {
                    "neighbour"
                });
            }
        }
    }
    match first {
        Some("own") => t.gap_own += 1,
        Some("neighbour") => t.gap_neighbour += 1,
        Some(_) => t.gap_other += 1,
        None => {
            t.gap_neither += 1;
            println!(
                "gapnone surface={class} graph={id} face={fi} loops={}",
                rings.len()
            );
        }
    }
}

/// The faces of one graph: prints its breaks and adds to the tally; whether it holds one,
/// and whether it holds a plane or cylinder face refused as `disconnected UV boundary`.
fn scan(id: u64, graph: &ObjectGraph, tally: &mut BTreeMap<String, Tally>) -> (bool, bool) {
    let mut any = false;
    let mut ringed = false;
    let mut cache: Option<EdgeCache> = None;
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
                let full = format!("{error:#}");
                let text: String = full
                    .chars()
                    .map(|c| if c.is_ascii_digit() { '#' } else { c })
                    .take(200)
                    .collect();
                println!(
                    "refused surface={class} graph={id} face={fi} why={}",
                    text.chars().take(120).collect::<String>()
                );
                *t.ring_refused.entry(text).or_default() += 1;
                if full.contains("disconnected UV boundary") {
                    t.gap_disconnected += 1;
                    ringed = true;
                    read_gaps(id, fi, &class, graph, &mut cache, t);
                }
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
    (any, ringed)
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
    let mut ringed: BTreeSet<u64> = BTreeSet::new();
    let mut scanned: BTreeSet<u64> = BTreeSet::new();
    let mut symbols_of: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
    let mut owners: BTreeSet<u64> = BTreeSet::new();
    let mut read_graphs = 0usize;
    let mut refused_graphs = 0usize;
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
        // A graph past the budget is refused whole, never read in part.
        if r.status == "unsupported_graph" {
            refused_graphs += 1;
        }
        if let Some(graph) = &r.graph {
            read_graphs += 1;
            symbols_of.insert(id, instance_symbols(graph));
            let (any, ring) = scan(id, graph, tally);
            if any {
                broken.insert(id);
            }
            if ring {
                ringed.insert(id);
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
    println!("graphs read={read_graphs} refused={refused_graphs}");
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
        let graphs: Vec<u64> = seen.intersection(&ringed).copied().collect();
        if !graphs.is_empty() {
            println!("ringreach owner={owner} graphs={graphs:?}");
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
            println!(
                "gapsrc surface={class} disconnected={} own={} neighbour={} other={} neither={} unread={}",
                t.gap_disconnected,
                t.gap_own,
                t.gap_neighbour,
                t.gap_other,
                t.gap_neither,
                t.gap_unread
            );
            println!(
                "gaploops surface={class} loops={} stored_open={} seam_like={} backward={}",
                t.gap_loops, t.gap_stored_open, t.gap_seam_like, t.gap_backward
            );
            for (unit, bins) in [("parameters", t.gap_parameters), ("feet", t.gap_feet)] {
                println!(
                    "gaphist surface={class} unit={unit} under_1e-7={} 1e-7_to_1e-6={} 1e-6_to_1e-5={} 1e-5_to_1e-4={} 1e-4_to_1e-3={} 1e-3_to_1e-2={} 1e-2_or_more={}",
                    bins[0], bins[1], bins[2], bins[3], bins[4], bins[5], bins[6]
                );
            }
            println!(
                "pieces surface={class} gaps={} one={} diagonal={} tenth_mm={} ends={} through={} mid_covered={} orphan_near={} sides_disagree={} rings_under={} unread={}",
                t.piece_gaps,
                t.piece_one,
                t.piece_gaps - t.piece_one,
                t.piece_tenth,
                t.piece_ends,
                t.piece_through,
                t.piece_mid,
                t.piece_orphan,
                t.piece_sides,
                t.piece_rings_under,
                t.piece_unread
            );
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
