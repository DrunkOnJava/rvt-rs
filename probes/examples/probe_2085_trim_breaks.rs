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
//!   pieceorient graph=<ElementId> face=<n> loop=<n> at=<k> p_is=<first|last> q_is=<first|last>:
//!     whether the gap's P is the stored first or last point of the edge before it
//!     (`m_firstAndLastEdgePnts`), and Q of the edge after it. The ring's edges are in their
//!     stored loop order (`m_next`) and which end of each is P or Q is the one that joins best, so
//!     this is each edge's own stored direction at the gap;
//!   pairorient graph=<ElementId> views=<face>:<loop>:<k>,<face>:<loop>:<k> cfg=<P/Q>+<P/Q>
//!     candidate=<yes|no> gap=<ft>: a stored gap two rings of the graph hold, the same two
//!     ends in model coordinates to 1e-6 ft, with each ring's `pieceorient` reading. The
//!     candidate is the coordinated join the chair save suggests: one ring `last/first`, the
//!     other `first/first` or `last/last`;
//!   pairorients pairs=<n> candidate=<n> one_ring=<n> more_rings=<n> widest_candidate=<ft>:
//!     over the file, the stored gaps two rings hold, those that meet the candidate, those
//!     only one ring holds and those more than two do;
//!   ownnbr graph=<ElementId> face=<n> loop=<n> at=<k> side=<a|b> order=<prev|next>
//!     class=<class> poly_p=<ft> poly_q=<ft> end_p=<ft> end_q=<ft>
//!     shared_p=<ft> shared_q=<ft> join=<ft>: the two edges adjacent to the shared edge in
//!     its other face's loop, evaluated through that face. The shared-edge endpoint is
//!     the one nearest its own gap end. For each adjacent edge the probe reports polyline
//!     distances to P and Q, distances to P and Q from the two selected endpoints, and
//!     their separation (`join`). The adjacent endpoint is the one nearest the selected
//!     shared endpoint.
//!     `ownnbrunread` gives the missing face, loop or placement.
//!     Distances describe the stored samples; selecting a Revit endpoint still requires
//!     the same-save reference.
//!
//!   pieces surface=<class> gaps=<n> one=<n> diagonal=<n> tenth_mm=<n> ends=<n> through=<n>
//!     mid_covered=<n> orphan_near=<n> sides_disagree=<n> rings_under=<n> unread=<n>: by class;
//!   graphs read=<n> refused=<n>: the graphs decoded, and those refused whole for the budget.
//!   gapxyz owner=<ElementId> occ=<n> graph=<ElementId> oi=<n> level=3 face=<n> loop=<n> at=<k>
//!     gap=<ft> p=<x,y,z> q=<x,y,z>: for each element whose graphics reach a graph with a
//!     refused ring, walked at detail level 3, each gap of a selected face's rings, its ends
//!     placed by the element's transform in document feet (`occ` counts repeats of one
//!     `Geometry` in one element); jointxyz owner=<ElementId> occ=<n> graph=<ElementId>
//!     face=<n> loop=<n> at_gap=<bool> x=<x,y,z>: every edge end of the same ring, the control a
//!     scorer reads its floor from (ends at a gap marked); boundaryunread owner=<ElementId>.
//!     The coordinates are a file's own: keep them private for a save that is not shared.
//!   edgereach owner=<ElementId> graphs=[...]; edgesides owner=<ElementId> occ=<n>
//!     graph=<ElementId> face=<n> loop=<n> edge=<n> other=<class> n=<n> apart=<ft>
//!     plane=<x,y,z;...> curved=<x,y,z;...> (#2106): for an element whose graphics reach a plane
//!     refused as `shared edge surface samples disagree`, each edge of that plane's rings that it
//!     shares with a cylinder, surface of revolution, ruled surface or cone, both faces' samples
//!     of it placed by the element's transform, where they are more than 1e-7 ft apart; the
//!     plane's ring ends follow as `jointxyz` controls. `edgesidesunread` names an edge whose
//!     two sides could not both be placed.
//!   piecemore graph=<ElementId> face=<n> loop=<n> at=<k> loops_held=<n> loops_walked=<n>
//!     through_faces=<faces> through_body=<same|other|none> through_orphan=<bool|none>
//!     a_other_ends=<ft to P>/<ft to Q> a_ginfo=<..> a_flags=<..> b_...: for each gap, the
//!     faces of the `through` edge and whether one of them is joined to this face through
//!     edges that name two faces (its body), whether the edge is one of those that name this
//!     face but that no loop walked reaches, the face's loops against those the probe could
//!     walk, and for the edges before (`a`) and after (`b`) the gap: where the side on their
//!     other face puts its ends (the nearer to P, the nearer to Q), their `m_GInfo` (the
//!     class and fields of what it points to, or its value) and `m_flags`;
//!     then matches=<n> matches_same_body=<n> matches_naming_face=<n>
//!     matches_same_body_naming_face=<n> matches_same_body_between_others=<n>
//!     matches_unreached=<n> face_regions=<n|absent>: every edge passing within the bound of
//!     both P and Q (`through` names only the first, in the graph's order), by body, by whether
//!     it names this face, and by whether no loop walked reaches it; and how many regions the
//!     face lists (`m_faceRegions`), which the probe does not walk;
//!   pieces2 surface=<class> through=<n> through_same_body=<n> through_orphan=<n>: by class.
//!   piecex graph=<ElementId> face=<n> loop=<n> at=<k> placed=<n> unplaced=<n>
//!     unplaced_classes=<class+class:n,...> ends=<edge|none> matches=<n> matches_same_body=<n>
//!     matches_naming_face=<n> match_agree_max=<ft|none> a_agree=<ft|unread>
//!     b_agree=<ft|unread> a_other_ends=<ft>/<ft> b_other_ends=<ft>/<ft>: the same gap with
//!     every edge placed through either face by any evaluator that reads it (plane, cylinder,
//!     or a parametric surface the mesher reads): the graph's edges placed and not, the
//!     unplaced ones by their faces' classes, the matches as `piecemore` counts them, the
//!     largest distance at which a matching edge's two faces put it (a "no match" binds only
//!     where they agree), the same for the edges before and after the gap, and where their
//!     other face's side ends. `matches_one_face_only` counts the matches that reach both
//!     ends as one face places them and not as the other does; `unplaced_classes` keys each
//!     count by cause (`not_read`, `not_evaluated`) and the two faces' classes;
//!   edgeagree graph=<ElementId> both_faces=<n> over_bound=<n> largest=<ft>: once per graph
//!     with a gap, the edges both faces place, how many they put 0.00001 ft or more apart,
//!     and the largest.
//! ```
//! ElementIds, counts and distances in feet only.
//!
//! Usage:
//! ```text
//!   cargo run --profile ci -p rvt-probes --example probe_2085_trim_breaks -- MODEL.rvt
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{self, Record};
use rvt::research::native_graphics_traversal::select_graphics_with_resolver_at_detail;
use rvt::research::native_parameters::ObjectGraph;
use rvt::research::native_parametric_mesh::{
    JOIN_BOUNDS, face_grid, merged_chord_ends, polygon_breaks, trim_breaks,
};
use rvt::research::native_parametric_surface::ParametricSurface;
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
    piece_through_same: usize,
    piece_through_orphan: usize,
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
    /// Each face's body: the faces joined to it through edges that name two faces.
    body: BTreeMap<usize, usize>,
    /// Each edge placed through either of its faces by any evaluator that reads it (plane,
    /// cylinder, or the parametric surfaces the mesher reads), and how far apart its two
    /// faces put its points where both place it.
    xany: Vec<(usize, [Option<Polyline>; 2], Option<f64>)>,
    params: BTreeMap<usize, Option<ParametricSurface>>,
    /// The edges no evaluator places, by cause (`not_read`: neither face's surface is read;
    /// `not_evaluated`: a face's surface is read but a point of the edge does not evaluate)
    /// and their two faces' surface classes.
    unplaced: BTreeMap<String, usize>,
}

/// Why an edge's side could not be placed.
enum Placed {
    At(Polyline),
    NotRead,
    NotEvaluated,
}

fn param_surface(graph: &ObjectGraph, face: usize) -> Option<ParametricSurface> {
    let si = pointer(graph, face, graph.objects.get(face)?.fields.get("m_pSurf")?).ok()??;
    ParametricSurface::read(graph, si).ok()
}

/// An edge's points through its face on `side`, by whichever evaluator reads that face. A
/// parametric surface's point within 1e-7 of its envelope is clamped onto it and one
/// farther out is not placed, as `loop_segments` takes a loop's samples.
fn place_why(
    graph: &ObjectGraph,
    e: usize,
    side: usize,
    face: usize,
    surfs: &mut BTreeMap<usize, Option<Surf>>,
    params: &mut BTreeMap<usize, Option<ParametricSurface>>,
) -> Placed {
    let Some(uv) = edge_uv(graph, e, side) else {
        return Placed::NotRead;
    };
    let s = *surfs.entry(face).or_insert_with(|| surf(graph, face));
    if let Some(s) = s {
        return Placed::At(uv.iter().map(|q| s.at(*q)).collect());
    }
    let Some(ps) = params
        .entry(face)
        .or_insert_with(|| param_surface(graph, face))
        .as_ref()
    else {
        return Placed::NotRead;
    };
    let env = ps.bounds();
    // `clamp` panics on a bound that is not finite.
    if !env.iter().flatten().all(|x| x.is_finite()) {
        return Placed::NotEvaluated;
    }
    let points: Option<Polyline> = uv
        .iter()
        .map(|q| {
            // As `loop_segments` takes a trim sample: within 1e-7 of the envelope, clamped
            // onto it; farther out, not placed ("trim outside surface envelope").
            let mut uv = *q;
            for k in 0..2 {
                let (lo, hi) = (env[0][k].min(env[1][k]), env[0][k].max(env[1][k]));
                if uv[k] < lo - 1e-7 || uv[k] > hi + 1e-7 {
                    return None;
                }
                uv[k] = uv[k].clamp(lo, hi);
            }
            ps.evaluate(uv[0], uv[1]).ok()
        })
        .collect();
    points.map_or(Placed::NotEvaluated, Placed::At)
}

fn place(
    graph: &ObjectGraph,
    e: usize,
    side: usize,
    face: usize,
    surfs: &mut BTreeMap<usize, Option<Surf>>,
    params: &mut BTreeMap<usize, Option<ParametricSurface>>,
) -> Option<Polyline> {
    match place_why(graph, e, side, face, surfs, params) {
        Placed::At(points) => Some(points),
        _ => None,
    }
}

/// The root of `x` in a union-find over faces.
fn root(parent: &mut BTreeMap<usize, usize>, x: usize) -> usize {
    let mut r = x;
    while let Some(&p) = parent.get(&r) {
        if p == r {
            break;
        }
        r = p;
    }
    parent.insert(x, r);
    r
}

impl EdgeCache {
    fn new(graph: &ObjectGraph) -> EdgeCache {
        let mut surfs: BTreeMap<usize, Option<Surf>> = BTreeMap::new();
        let mut naming: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        let mut parent: BTreeMap<usize, usize> = BTreeMap::new();
        let mut xyz = Vec::new();
        let mut xany = Vec::new();
        let mut params: BTreeMap<usize, Option<ParametricSurface>> = BTreeMap::new();
        let mut unplaced: BTreeMap<String, usize> = BTreeMap::new();
        for (e, object) in graph.objects.iter().enumerate() {
            if object.class_name != "Edge" {
                continue;
            }
            let faces = edge_faces(graph, e);
            for face in faces.iter().flatten() {
                naming.entry(*face).or_default().push(e);
                parent.entry(*face).or_insert(*face);
            }
            if let [Some(f0), Some(f1)] = faces {
                let (r0, r1) = (root(&mut parent, f0), root(&mut parent, f1));
                if r0 != r1 {
                    parent.insert(r0.max(r1), r0.min(r1));
                }
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
            let mut placed: [Option<Polyline>; 2] = [None, None];
            let mut evaluated_somewhere = false;
            for (side, face) in faces.iter().enumerate() {
                if let Some(face) = face {
                    match place_why(graph, e, side, *face, &mut surfs, &mut params) {
                        Placed::At(points) => placed[side] = Some(points),
                        Placed::NotEvaluated => evaluated_somewhere = true,
                        Placed::NotRead => {}
                    }
                }
            }
            let agree = match (&placed[0], &placed[1]) {
                (Some(a), Some(b)) if a.len() == b.len() => Some(
                    a.iter()
                        .zip(b)
                        .map(|(p, q)| dist(*p, *q))
                        .fold(0.0, f64::max),
                ),
                _ => None,
            };
            if placed[0].is_none() && placed[1].is_none() {
                let classes = faces
                    .iter()
                    .map(|f| f.map_or_else(|| "-".to_string(), |f| surface_class(graph, f)))
                    .collect::<Vec<_>>()
                    .join("+");
                let cause = if evaluated_somewhere {
                    "not_evaluated"
                } else {
                    "not_read"
                };
                *unplaced.entry(format!("{cause}/{classes}")).or_default() += 1;
            }
            xany.push((e, placed, agree));
        }
        let faces: Vec<usize> = parent.keys().copied().collect();
        let body = faces
            .into_iter()
            .map(|f| (f, root(&mut parent, f)))
            .collect();
        EdgeCache {
            xyz,
            surfs,
            naming,
            body,
            xany,
            params,
            unplaced,
        }
    }
    /// As [`EdgeCache::other_side_ends`], placing the side by any evaluator that reads it.
    fn other_side_ends_any(
        &mut self,
        graph: &ObjectGraph,
        e: usize,
        face: usize,
        p: [f64; 3],
        q: [f64; 3],
    ) -> Option<(f64, f64)> {
        let faces = edge_faces(graph, e);
        let side = (0..2).find(|k| faces[*k].is_some_and(|f| f != face))?;
        let pts = place(
            graph,
            e,
            side,
            faces[side]?,
            &mut self.surfs,
            &mut self.params,
        )?;
        let ends = [pts[0], pts[pts.len() - 1]];
        Some((
            dist(ends[0], p).min(dist(ends[1], p)),
            dist(ends[0], q).min(dist(ends[1], q)),
        ))
    }
    /// Where the side of `e` on the face that is not `face` puts its ends: the nearer of them
    /// to `p`, and the nearer to `q`.
    fn other_side_ends(
        &mut self,
        graph: &ObjectGraph,
        e: usize,
        face: usize,
        p: [f64; 3],
        q: [f64; 3],
    ) -> Option<(f64, f64)> {
        let faces = edge_faces(graph, e);
        let side = (0..2).find(|k| faces[*k].is_some_and(|f| f != face))?;
        let s = self.surf(graph, faces[side]?)?;
        let pts = edge_uv(graph, e, side)?;
        let ends = [s.at(pts[0]), s.at(pts[pts.len() - 1])];
        Some((
            dist(ends[0], p).min(dist(ends[1], p)),
            dist(ends[0], q).min(dist(ends[1], q)),
        ))
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

/// A stored gap as `pieceorient` reads it, kept to pair the rings that hold the same gap.
struct OrientRow {
    graph: u64,
    face: usize,
    loop_index: usize,
    at: usize,
    ends: [[i64; 3]; 2],
    cfg: &'static str,
    d: f64,
}
/// Every stored gap read, across surface classes, since a gap's two rings can be on two.
static ORIENTS: Mutex<Vec<OrientRow>> = Mutex::new(Vec::new());
/// A point of model coordinates to 1e-6 ft, the scorer's corner key.
fn micro(p: [f64; 3]) -> [i64; 3] {
    p.map(|x| (x * 1e6).round() as i64)
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
    /// Whether P is the stored first point of the edge before the gap (`m_firstAndLastEdgePnts[0]`),
    /// and whether Q is the stored first point of the edge after it. The ring's edges are in their
    /// stored loop order (`m_next`); which end of each is P or Q is the one that joins best.
    p_first: bool,
    q_first: bool,
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
        // Whether `first` and `last` are their edge's stored first point.
        let first_is_first = !reverse;
        let mut last_is_first = reverse;
        let mut prev = 0usize;
        let mut gaps = Vec::new();
        let mut worst: f64 = 0.0;
        let gap = |at: usize,
                   prev: usize,
                   from: ([f64; 2], bool),
                   to: ([f64; 2], bool),
                   gaps: &mut Vec<Gap>| {
            let (from, p_first) = from;
            let (to, q_first) = to;
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
                    p_first,
                    q_first,
                });
            }
            d
        };
        for (k, pts) in segs.iter().enumerate().skip(1) {
            let (h, t) = (pts[0], pts[pts.len() - 1]);
            let ((near, near_is_first), (far, far_is_first)) =
                if dist(s.at(last), s.at(h)) <= dist(s.at(last), s.at(t)) {
                    ((h, true), (t, false))
                } else {
                    ((t, false), (h, true))
                };
            worst = worst.max(gap(
                k,
                prev,
                (last, last_is_first),
                (near, near_is_first),
                &mut gaps,
            ));
            last = far;
            last_is_first = far_is_first;
            prev = k;
        }
        worst = worst.max(gap(
            0,
            prev,
            (last, last_is_first),
            (first, first_is_first),
            &mut gaps,
        ));
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

/// Place the shared edge and its loop neighbours through the other face's own evaluator.
fn report_own_neighbours(
    graph: &ObjectGraph,
    cache: &EdgeCache,
    gap: &Gap,
    location: (u64, usize, usize),
) {
    let (id, face, loop_index) = location;
    let at = gap.at;
    let nearest_end = |line: &Polyline, point| {
        let first = line[0];
        let last = line[line.len() - 1];
        if dist(first, point) <= dist(last, point) {
            first
        } else {
            last
        }
    };
    for (label, edge, side, ring_end) in [
        ("a", gap.a, gap.a_side, gap.p),
        ("b", gap.b, gap.b_side, gap.q),
    ] {
        let unread = |reason| {
            println!(
                "ownnbrunread graph={id} face={face} loop={loop_index} at={at} side={label} reason={reason}"
            );
        };
        let Some(other) = edge_faces(graph, edge)[1 - side] else {
            unread("face");
            continue;
        };
        let Some(neighbours) = chain_neighbours(graph, other, edge) else {
            unread("loop");
            continue;
        };
        let line_on_other = |edge| {
            let own_side = edge_faces(graph, edge)
                .iter()
                .position(|face| *face == Some(other))?;
            let (_, placements, _) = cache
                .xany
                .iter()
                .find(|(candidate, _, _)| *candidate == edge)?;
            placements[own_side].as_ref()
        };
        let Some(shared) = line_on_other(edge) else {
            unread("shared_edge");
            continue;
        };
        let shared_end = nearest_end(shared, ring_end);
        let class = surface_class(graph, other);
        for (order, neighbour) in ["prev", "next"].into_iter().zip(neighbours) {
            let Some(line) = line_on_other(neighbour) else {
                println!(
                    "ownnbrunread graph={id} face={face} loop={loop_index} at={at} side={label} order={order} reason=adjacent_edge"
                );
                continue;
            };
            let endpoint = nearest_end(line, shared_end);
            println!(
                "ownnbr graph={id} face={face} loop={loop_index} at={at} side={label} order={order} class={class} poly_p={:.9} poly_q={:.9} end_p={:.9} end_q={:.9} shared_p={:.9} shared_q={:.9} join={:.9}",
                polyline_dist(gap.p, line),
                polyline_dist(gap.q, line),
                dist(endpoint, gap.p),
                dist(endpoint, gap.q),
                dist(shared_end, gap.p),
                dist(shared_end, gap.q),
                dist(endpoint, shared_end)
            );
        }
    }
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
    let fresh = cache.is_none();
    let cache = cache.get_or_insert_with(|| EdgeCache::new(graph));
    if fresh {
        let both: Vec<f64> = cache.xany.iter().filter_map(|(_, _, a)| *a).collect();
        println!(
            "edgeagree graph={id} both_faces={} over_bound={} largest={:.9}",
            both.len(),
            both.iter().filter(|a| **a >= TOUCHING).count(),
            both.iter().copied().fold(0.0, f64::max)
        );
    }
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
        // Every edge passing within the bound of both P and Q, not only the first.
        let mut matches: Vec<usize> = Vec::new();
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
                matches.push(*e);
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
        report_own_neighbours(graph, cache, g, (id, fi, loop_index));
        println!(
            "pieceorient graph={id} face={fi} loop={loop_index} at={} p_is={} q_is={}",
            g.at,
            if g.p_first { "first" } else { "last" },
            if g.q_first { "first" } else { "last" }
        );
        if let Ok(mut rows) = ORIENTS.lock() {
            let mut ends = [micro(g.p), micro(g.q)];
            ends.sort();
            rows.push(OrientRow {
                graph: id,
                face: fi,
                loop_index,
                at: g.at,
                ends,
                cfg: match (g.p_first, g.q_first) {
                    (true, true) => "first/first",
                    (true, false) => "first/last",
                    (false, true) => "last/first",
                    (false, false) => "last/last",
                },
                d: g.d,
            });
        }
        // Whose edge a `through` match is: this face's body or another, and whether it is one
        // of the edges that name this face but that no loop walked reaches.
        let own_body = cache.body.get(&fi).copied();
        let (through_body, through_orphan, through_faces) = match through {
            Some(e) => {
                let same = edge_faces(graph, e)
                    .iter()
                    .flatten()
                    .any(|f| own_body.is_some() && cache.body.get(f).copied() == own_body);
                t.piece_through_same += usize::from(same);
                t.piece_through_orphan += usize::from(orphans.contains(&e));
                (
                    if same { "same" } else { "other" },
                    orphans.contains(&e).to_string(),
                    faces_of(e),
                )
            }
            None => ("none", "none".to_string(), "-".to_string()),
        };
        let mut sides = Vec::new();
        for (label, e) in [("a", g.a), ("b", g.b)] {
            let text = cache.other_side_ends(graph, e, fi, g.p, g.q).map_or_else(
                || "unread".to_string(),
                |(dp, dq)| format!("{dp:.9}/{dq:.9}"),
            );
            let fields = &graph.objects[e].fields;
            let ginfo = fields.get("m_GInfo").map_or_else(
                || "absent".to_string(),
                |v| match pointer(graph, e, v).ok().flatten() {
                    Some(o) => format!(
                        "{}{:?}",
                        graph.objects[o].class_name,
                        keys(Some(&graph.objects[o].fields))
                    ),
                    None => v.to_json().to_string().chars().take(80).collect(),
                },
            );
            let flags: String = fields
                .get("m_flags")
                .map_or_else(|| "absent".to_string(), |v| v.to_json().to_string())
                .chars()
                .take(40)
                .collect();
            sides.push(format!(
                "{label}_other_ends={text} {label}_ginfo={} {label}_flags={flags}",
                ginfo.replace(' ', "")
            ));
        }
        // Every match, by body, by whether it names this face, and by whether no loop walked
        // reaches it; and how many of each kind name the face and sit in this body.
        let mut same_body = 0usize;
        let mut names_face = 0usize;
        let mut same_names = 0usize;
        let mut same_between_others = 0usize;
        let mut unreached = 0usize;
        for e in &matches {
            let faces = edge_faces(graph, *e);
            let same = faces
                .iter()
                .flatten()
                .any(|f| own_body.is_some() && cache.body.get(f).copied() == own_body);
            let named = faces.contains(&Some(fi));
            same_body += usize::from(same);
            names_face += usize::from(named);
            same_names += usize::from(same && named);
            same_between_others += usize::from(same && !named);
            unreached += usize::from(orphans.contains(e));
        }
        let regions = graph.objects[fi].fields.get("m_faceRegions").map_or_else(
            || "absent".to_string(),
            |v| {
                v.as_array()
                    .map_or_else(|| "not_a_list".to_string(), |a| a.len().to_string())
            },
        );
        println!(
            "piecemore graph={id} face={fi} loop={loop_index} at={} loops_held={} loops_walked={} through_faces={through_faces} through_body={through_body} through_orphan={through_orphan} {} matches={} matches_same_body={same_body} matches_naming_face={names_face} matches_same_body_naming_face={same_names} matches_same_body_between_others={same_between_others} matches_unreached={unreached} face_regions={regions}",
            g.at,
            loops.len(),
            loops
                .iter()
                .filter(|l| loop_chain(graph, fi, **l).is_some())
                .count(),
            sides.join(" "),
            matches.len()
        );
        // The same gap with every edge any evaluator places (#2085): what the plane and
        // cylinder placement left unread.
        let mut x_ends = None;
        let mut x_matches = 0usize;
        let mut x_one_face = 0usize;
        let mut x_same = 0usize;
        let mut x_naming = 0usize;
        let mut x_unreached = 0usize;
        let mut x_agree: f64 = 0.0;
        let mut x_agree_known = 0usize;
        for (e, lines, agree) in &cache.xany {
            if *e == g.a || *e == g.b {
                continue;
            }
            // Each placement is tested: an edge may reach both ends as one face puts it and
            // not as the other does.
            let mut passes = 0usize;
            for line in lines.iter().flatten() {
                let (e0, e1) = (line[0], line[line.len() - 1]);
                if (dist(e0, g.p) < TOUCHING && dist(e1, g.q) < TOUCHING)
                    || (dist(e0, g.q) < TOUCHING && dist(e1, g.p) < TOUCHING)
                {
                    x_ends.get_or_insert(*e);
                }
                if polyline_dist(g.p, line) < TOUCHING && polyline_dist(g.q, line) < TOUCHING {
                    passes += 1;
                }
            }
            if passes > 0 {
                x_matches += 1;
                let placements = lines.iter().flatten().count();
                x_one_face += usize::from(placements == 2 && passes == 1);
                x_unreached += usize::from(orphans.contains(e));
                let faces = edge_faces(graph, *e);
                if faces
                    .iter()
                    .flatten()
                    .any(|f| own_body.is_some() && cache.body.get(f).copied() == own_body)
                {
                    x_same += 1;
                }
                if faces.contains(&Some(fi)) {
                    x_naming += 1;
                }
                if let Some(a) = agree {
                    x_agree = x_agree.max(*a);
                    x_agree_known += 1;
                }
            }
        }
        let placed = cache
            .xany
            .iter()
            .filter(|(_, l, _)| l.iter().any(|x| x.is_some()))
            .count();
        let unplaced: usize = cache.unplaced.values().sum();
        let classes = cache
            .unplaced
            .iter()
            .map(|(k, v)| format!("{k}:{v}"))
            .collect::<Vec<_>>()
            .join(",");
        let agree_of = |e: usize| {
            cache
                .xany
                .iter()
                .find(|(x, _, _)| *x == e)
                .and_then(|(_, _, a)| *a)
                .map_or_else(|| "unread".to_string(), |a| format!("{a:.9}"))
        };
        let (a_agree, b_agree) = (agree_of(g.a), agree_of(g.b));
        let ends_shown = |x: Option<(f64, f64)>| {
            x.map_or_else(
                || "unread".to_string(),
                |(dp, dq)| format!("{dp:.9}/{dq:.9}"),
            )
        };
        let xa = cache.other_side_ends_any(graph, g.a, fi, g.p, g.q);
        let xb = cache.other_side_ends_any(graph, g.b, fi, g.p, g.q);
        println!(
            "piecex graph={id} face={fi} loop={loop_index} at={} placed={placed} unplaced={unplaced} unplaced_classes={} ends={} matches={x_matches} matches_one_face_only={x_one_face} matches_same_body={x_same} matches_naming_face={x_naming} matches_unreached={x_unreached} match_agree_max={} a_agree={a_agree} b_agree={b_agree} a_other_ends={} b_other_ends={}",
            g.at,
            if classes.is_empty() {
                "-".to_string()
            } else {
                classes
            },
            named(x_ends),
            if x_agree_known > 0 {
                format!("{x_agree:.9}")
            } else {
                "none".to_string()
            },
            ends_shown(xa),
            ends_shown(xb),
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
fn scan(id: u64, graph: &ObjectGraph, tally: &mut BTreeMap<String, Tally>) -> (bool, bool, bool) {
    let mut any = false;
    let mut ringed = false;
    let mut disagreed = false;
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
                if full.contains(SAMPLES_DISAGREE) {
                    disagreed = true;
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
    (any, ringed, disagreed)
}

/// `native_saved_mesh`'s refusal of a plane whose shared edge's two faces' samples differ.
const SAMPLES_DISAGREE: &str = "shared edge surface samples disagree";

/// The curved surfaces `refined_plane_edge` meets a plane along.
const PLANE_NEIGHBOURS: [&str; 4] = ["CylSurf", "SurfRev", "RuledSurf", "ConeSurf"];

fn xyz(p: [f64; 3]) -> String {
    format!("{:.9},{:.9},{:.9}", p[0], p[1], p[2])
}

/// #2106: for a plane refused as [`SAMPLES_DISAGREE`], each edge of its rings that it shares
/// with a curved face `refined_plane_edge` meets it along, both faces' samples of the edge,
/// placed by the element's own transform in document feet, where they are more than 1e-7 ft
/// apart (`edgesides`); and every edge end of the same rings as controls (`jointxyz`, never at a
/// gap), as `boundary_pass` prints them, so the scorer reads the element's floor.
fn edge_sides(
    sg: &ObjectGraph,
    fi: usize,
    surface: Surf,
    to_world: &dyn Fn([f64; 3]) -> [f64; 3],
    owner: u64,
    occ: usize,
    gid: u64,
) {
    let mut surfs: BTreeMap<usize, Option<Surf>> = BTreeMap::new();
    let mut params: BTreeMap<usize, Option<ParametricSurface>> = BTreeMap::new();
    let mut ends: Vec<[f64; 3]> = Vec::new();
    for (loop_index, li) in face_loops(sg, fi).iter().enumerate() {
        let Some(chain) = loop_chain(sg, fi, *li) else {
            continue;
        };
        for (e, side) in &chain {
            if let Some(pts) = edge_uv(sg, *e, *side) {
                for uv in [pts[0], pts[pts.len() - 1]] {
                    let p = surface.at(uv);
                    if !ends.iter().any(|q| dist(*q, p) < 1e-9) {
                        ends.push(p);
                    }
                }
            }
            let faces = edge_faces(sg, *e);
            let Some(other) = faces[1 - *side] else {
                continue;
            };
            let class = surface_class(sg, other);
            if !PLANE_NEIGHBOURS.contains(&class.as_str()) {
                continue;
            }
            let own = place(sg, *e, *side, fi, &mut surfs, &mut params);
            let theirs = place(sg, *e, 1 - *side, other, &mut surfs, &mut params);
            let (Some(own), Some(theirs)) = (own, theirs) else {
                println!(
                    "edgesidesunread owner={owner} graph={gid} face={fi} edge={e} other={class}"
                );
                continue;
            };
            if own.len() != theirs.len() {
                println!(
                    "edgesidesunread owner={owner} graph={gid} face={fi} edge={e} other={class} why=counts"
                );
                continue;
            }
            let apart = own
                .iter()
                .zip(&theirs)
                .map(|(a, b)| dist(*a, *b))
                .fold(0.0, f64::max);
            if apart <= 1e-7 {
                continue;
            }
            let join = |pts: &[[f64; 3]]| {
                pts.iter()
                    .map(|p| xyz(to_world(*p)))
                    .collect::<Vec<_>>()
                    .join(";")
            };
            println!(
                "edgesides owner={owner} occ={occ} graph={gid} face={fi} loop={loop_index} edge={e} other={class} n={} apart={apart:.9} plane={} curved={}",
                own.len(),
                join(&own),
                join(&theirs)
            );
        }
    }
    for p in ends {
        println!(
            "jointxyz owner={owner} occ={occ} graph={gid} face={fi} loop=0 at_gap=false x={}",
            xyz(to_world(p))
        );
    }
}

/// #2085: for every element whose graphics reach a graph with a refused ring, its graphics at
/// detail level 3 (the fitting probe's convention; which level Revit's export used is not in
/// the file), and for each selected `Geometry` face refused as `disconnected UV boundary`, each
/// gap of its rings and each of its rings' edge ends, placed by the element's own transform in
/// document feet, for a scorer to set against Revit's body by Tag.
fn boundary_pass(
    rf: &mut RevitFile,
    options: &native_document::Options,
    owners: &BTreeSet<u64>,
    symbols_of: &BTreeMap<u64, Vec<u64>>,
) -> Result<()> {
    let mut wanted = owners.clone();
    for owner in owners {
        let mut stack = vec![*owner];
        while let Some(next) = stack.pop() {
            for symbol in symbols_of.get(&next).into_iter().flatten() {
                if wanted.insert(*symbol) {
                    stack.push(*symbol);
                }
            }
        }
    }
    let mut graphs: BTreeMap<u64, ObjectGraph> = BTreeMap::new();
    let mut again = options.clone();
    again.selected_ids = wanted;
    native_document::extract_graphs(rf, &again, |r: Record| {
        if let Some(graph) = r.graph {
            graphs.insert(r.identity.element_id, graph);
        }
        Ok(())
    })?;
    for owner in owners {
        let Some(g) = graphs.get(owner) else {
            println!("boundaryunread owner={owner} why=no_graph");
            continue;
        };
        let resolver = |rid: u64| graphs.get(&rid);
        let selection = select_graphics_with_resolver_at_detail(g, &resolver, 3);
        let mut seen: BTreeMap<(u64, usize), usize> = BTreeMap::new();
        for s in &selection.selected {
            let gid = s.source_owner_id.unwrap_or(*owner);
            let Some(sg) = graphs.get(&gid) else {
                continue;
            };
            let oi = s.object_index;
            if sg.objects.get(oi).map(|o| o.class_name.as_str()) != Some("Geometry") {
                continue;
            }
            let occurrence = seen.entry((gid, oi)).or_default();
            *occurrence += 1;
            let occ = *occurrence;
            let m = &s.world_transform;
            let place = |p: [f64; 3]| -> [f64; 3] {
                std::array::from_fn(|i| m[i][3] + (0..3).map(|j| m[i][j] * p[j]).sum::<f64>())
            };
            let faces = sg.objects[oi].fields["m_pFaces"]
                .as_array()
                .map(<[Fv]>::to_vec)
                .unwrap_or_default();
            for ptr in &faces {
                let Ok(Some(fi)) = pointer(sg, oi, ptr) else {
                    continue;
                };
                let Some(surface) = surf(sg, fi) else {
                    continue;
                };
                let why = match rvt::research::native_saved_mesh::face(sg, fi) {
                    Err(e) => format!("{e:#}"),
                    Ok(_) => continue,
                };
                if why.contains(SAMPLES_DISAGREE) && matches!(surface, Surf::Plane { .. }) {
                    edge_sides(sg, fi, surface, &place, *owner, occ, gid);
                    continue;
                }
                if !why.contains("disconnected UV boundary") {
                    continue;
                }
                for (loop_index, li) in face_loops(sg, fi).iter().enumerate() {
                    let Some(chain) = loop_chain(sg, fi, *li) else {
                        continue;
                    };
                    let Some(gaps) = open_gaps(sg, &chain, surface) else {
                        continue;
                    };
                    if gaps.is_empty() {
                        continue;
                    }
                    for gp in &gaps {
                        println!(
                            "gapxyz owner={owner} occ={occ} graph={gid} oi={oi} level=3 face={fi} loop={loop_index} at={} gap={:.9} p={} q={}",
                            gp.at,
                            gp.d,
                            xyz(place(gp.p)),
                            xyz(place(gp.q))
                        );
                    }
                    // The control: every edge end of the same ring, through the same transform.
                    // An end within the touching bound of a gap's end is marked, so the scorer
                    // reads the floor from the ends that close.
                    let mut ends: Vec<[f64; 3]> = Vec::new();
                    for (e, side) in &chain {
                        let Some(pts) = edge_uv(sg, *e, *side) else {
                            continue;
                        };
                        for uv in [pts[0], pts[pts.len() - 1]] {
                            let p = surface.at(uv);
                            if !ends.iter().any(|q| dist(*q, p) < 1e-9) {
                                ends.push(p);
                            }
                        }
                    }
                    for p in ends {
                        let at_gap = gaps
                            .iter()
                            .any(|gp| dist(gp.p, p) < TOUCHING || dist(gp.q, p) < TOUCHING);
                        println!(
                            "jointxyz owner={owner} occ={occ} graph={gid} face={fi} loop={loop_index} at_gap={at_gap} x={}",
                            xyz(place(p))
                        );
                    }
                }
            }
        }
    }
    Ok(())
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
    let mut disagreed: BTreeSet<u64> = BTreeSet::new();
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
            let (any, ring, disagree) = scan(id, graph, tally);
            if any {
                broken.insert(id);
            }
            if ring {
                ringed.insert(id);
            }
            if disagree {
                disagreed.insert(id);
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
    let mut reaching: BTreeSet<u64> = BTreeSet::new();
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
            reaching.insert(*owner);
        }
        let graphs: Vec<u64> = seen.intersection(&disagreed).copied().collect();
        if !graphs.is_empty() {
            println!("edgereach owner={owner} graphs={graphs:?}");
            reaching.insert(*owner);
        }
    }
    boundary_pass(&mut rf, &options, &reaching, &symbols_of)?;
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
            println!(
                "pieces2 surface={class} through={} through_same_body={} through_orphan={}",
                t.piece_through, t.piece_through_same, t.piece_through_orphan
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
    report_orient_pairs();
    Ok(())
}

/// The stored gaps two rings of a graph hold, each with both rings' `pieceorient` reading,
/// and whether they meet the coordinated join the chair save suggests (#2085).
fn report_orient_pairs() {
    let Ok(rows) = ORIENTS.lock() else {
        return;
    };
    type GapKey = (u64, [[i64; 3]; 2]);
    let mut by_gap: BTreeMap<GapKey, Vec<&OrientRow>> = BTreeMap::new();
    for r in rows.iter() {
        by_gap.entry((r.graph, r.ends)).or_default().push(r);
    }
    let (mut pairs, mut candidates, mut one, mut more) = (0, 0, 0, 0);
    let mut widest = 0f64;
    for ((graph, _), views) in &by_gap {
        match views.len() {
            1 => one += 1,
            2 => {
                pairs += 1;
                let (a, b) = (views[0], views[1]);
                let same_end = |c: &str| c == "first/first" || c == "last/last";
                let candidate = (a.cfg == "last/first" && same_end(b.cfg))
                    || (b.cfg == "last/first" && same_end(a.cfg));
                if candidate {
                    candidates += 1;
                    widest = widest.max(a.d.max(b.d));
                }
                println!(
                    "pairorient graph={graph} views={}:{}:{},{}:{}:{} cfg={}+{} candidate={} gap={:.9}",
                    a.face,
                    a.loop_index,
                    a.at,
                    b.face,
                    b.loop_index,
                    b.at,
                    a.cfg,
                    b.cfg,
                    if candidate { "yes" } else { "no" },
                    a.d.max(b.d)
                );
            }
            _ => more += 1,
        }
    }
    println!(
        "pairorients pairs={pairs} candidate={candidates} one_ring={one} more_rings={more} widest_candidate={widest:.9}"
    );
}
