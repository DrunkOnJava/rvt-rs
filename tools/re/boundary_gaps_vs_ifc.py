#!/usr/bin/env python3
"""Where Revit's own body puts the boundary across each gap of a refused ring (#2085).

`probe_2085_trim_breaks` prints, for every element whose graphics reach a graph with a refused
plane or cylinder ring, each gap's two ends (`gapxyz ... p=x,y,z q=x,y,z`) and every edge end of
the same ring (`jointxyz ... at_gap=<bool> x=x,y,z`), in document feet, beside its census of the
same gaps (`piece`). This sets them against Revit's body for the same Tag and prints counts and
distances only, never a coordinate; `--per-gap` adds one line per gap with its ElementIds.

1. **Row 0.** Each `gapxyz` gap equals the `piece` gap of the same graph, face, loop and
   position, so both lines read one gap finder. A gap on a plane refused because of a
   neighbouring face (the probe's `gapfrom`), which the census does not count, is counted apart
   and left out. Any other difference or unmatched gap is printed and nothing after it is
   trusted.
2. **Coverage.** The gaps an element's graphics reach at detail level 3, of the census, split
   into the census's groups: both edges beside the gap placed alike by their two faces (`agree`:
   both distances under 0.00001 ft), one of them placed apart (`apart`), or either unread
   (`unread`). The distances are `piecex`'s `a_agree` and `b_agree` where it is printed, which
   place an edge through any face an evaluator reads, and `piece`'s `sides_a` and `sides_b`
   (planes and cylinders only) elsewhere.
3. **Bodies.** Only the Body representations of the Tag's elements but openings (which carry
   their host's Tag) and their aggregated parts. Revit's vertices are compared only where every
   Body item is polygonal: an `IfcPolygonalFaceSet`, `IfcTriangulatedFaceSet`, `IfcFacetedBrep`
   or `IfcShellBasedSurfaceModel` of polygon loops, alone or mapped. Revit's own coordinates
   decide: the points each item's faces use, placed into the probe's frame through the element's
   placement, each mapped item's target and its map's origin as IfcOpenShell applies them, the
   file's unit and the site frame, with no meshing between. IfcOpenShell's mesh, with
   `weld-vertices` off, is the cross-check: on the BIM4LCA housing IfcOpenShell 0.9.0.post1
   cannot process a basin's largest face set (`Failed to process shape`), and the element's mesh
   leaves it out without a reported failure.
4. **The floors.** For each element, every ring end that closes (`at_gap=false`) to Revit's
   nearest vertex: the median distance is its floor, from Revit's coordinates, with the mesh's
   beside it. Elements whose two floors disagree (one over twice the other and 0.00001 ft) are
   counted each way; a raw floor higher than the mesh's means Revit's coordinates are placed
   wrong, and nothing after it counts. Where every element's median vector is one nonzero vector
   it is a frame offset and is taken off. An element whose floor is 0.01 ft or more is a
   placement mismatch (`placement`), counted apart.
5. **Outcomes, per occurrence and by cohort, from Revit's coordinates.** With `tol` the larger of
   twice the element's floor and 0.00001 ft, a gap under twice `tol` (so at least 0.00002 ft) is
   `unresolved`. Otherwise Revit has a vertex within `tol` at P only (`p_only`), at Q only
   (`q_only`), at both (`both`), or none of these (`none`); or it has one strictly between them
   (within `tol` of the segment and farther than `tol` from both ends): alone (`between`, Revit
   merging the two ends into one vertex), or with P (`between_p`), Q (`between_q`) or both
   (`between_both`) there too. Per gap, the
   outcomes over its occurrences and whether they agree. The registered verdict per cohort: one
   end kept on every scored gap is the office's result; anything else is counted by outcome. The
   mesh's outcomes beside them, by cohort. Where Revit keeps one end, which end, by cohort,
   against the state of the two edges beside the gap: the edge before it, ending at P, and the
   edge after it, starting at Q, each placed alike or apart by its two faces, or unread; once by
   `piecex`'s reading and once by `piece`'s, which places an edge through planes and cylinders
   only.
6. **Corners and sizes.** The scored gaps grouped by their two ends in world coordinates,
   either way round, so one corner seen from two faces counts once; then one line per cohort,
   surface class and gap size, with the elements' IFC classes and the floor and `tol` they were
   read with (one line per corner with `--per-gap`).
7. **Between.** Where Revit has a point between the ends, how far along the gap it lies, from P
   (0) to Q (1), by cohort: 0.5 is the gap's middle.

8. **Shared edges whose faces' samples differ** (#2106, the probe's `edgesides` lines). For a
   plane refused as `shared edge surface samples disagree`, each edge it shares with a curved
   face, both faces' samples placed in the probe's frame, ends included. The plane's ring ends that
   lie on those samples are left out of the floors, since they are what is measured. At each
   sample the two put 2 `tol` or more apart, whether Revit's own points sit on the plane's sample, the curved face's,
   both or neither; an edge whose samples all say one thing is counted by it, and one that
   splits is `split`.

9. **Edge flags.** Where Revit's points are scored, the outcome against the stored `m_flags` of
   the edge before the gap (ending at P) and the edge after it (starting at Q), the probe's
   `piecemore` `a_flags` and `b_flags`: by whether bit 0x8 is set on one edge, both or neither,
   and by the two values. Where only one edge has it, whether Revit's point is at the other
   edge's end only, at the flagged edge's end only, or neither alone. Beside it, the outcome
   against the class of each edge's other face (the probe's `piecenbr` `a=` and `b=`), and
   against where that face's own loop meets the gap: whether an edge beside the edge in that
   loop passes within 0.00001 ft of P, of Q, of both or of neither, as `piecenbr` places it,
   and again within 1e-7 ft, the bound the probe's own loops join within. Where the bit is on
   one edge only, the same two readings for the flagged edge's side, each end named by whose
   it is, against the end Revit keeps. And the outcome against the gap's place in its ring
   (`piece`'s `at`: 0 is the step from the last edge back to the first) and against the signs of
   the step from P to Q in the surface's two parameters (`du_ft`, `dv_ft`; 0 for a component under
   1% of the other, as `piece`'s `sig=one` reads a step along one parameter). And against which of
   the two edges' other faces has the lower index in the decoded graph (`piecenbr`), in case the
   face a mesher takes first fixes the corner it shares; and, for a corner seen from more than one
   ring, whether that reading names the same end from every ring, since the rings of one corner
   compare different faces while Revit keeps one point. Where the probe prints `ownnbr` rows (#2136:
   each side's neighbouring edges placed through their own face), the sides on which a neighbour's
   end joins the shared edge within 1e-7 ft and lies at exactly one gap end within 1e-7 ft, against
   whether Revit keeps that end. And, where the bit is on one edge only, the stored gaps by graph,
   since a gap's drawings agree: whether the split between the two ends follows the graph. And,
   where the probe prints `pieceorient`, the outcome against whether P and Q are their edges'
   stored first or last points, and for the flagged gaps against the same for the flagged edge's
   end and the other edge's end, counted once per stored gap.

An empty or unreadable probe output fails row 0 rather than reading as nothing to score.

Needs IfcOpenShell and numpy.

    python3 tools/re/boundary_gaps_vs_ifc.py <probe output> <revit.ifc> [--per-gap]
"""

import collections
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ifc_common import site_frame  # noqa: E402
from mesh_bodies_vs_ifc import meshes  # noqa: E402

FACESETS = {"IfcPolygonalFaceSet", "IfcTriangulatedFaceSet"}
POLYGON_SOLIDS = {"IfcFacetedBrep", "IfcFacetedBrepWithVoids", "IfcShellBasedSurfaceModel"}
FACETED = FACESETS | POLYGON_SOLIDS
TOUCHING = 1e-5
SAME_OFFSET = 1e-6
PLACEMENT = 0.01
ONE_END = ("p_only", "q_only")
BETWEEN = ("between", "between_p", "between_q", "between_both")
SCORED = ("p_only", "q_only", "both", "none") + BETWEEN
COHORTS = ("agree", "apart", "unread")
# The `m_flags` bit that, on the office's washbasin and a Snowdon Towers hand sink, marks the
# edge whose end Revit does not keep where only one edge beside a gap has it.
FLAG_BIT = 0x8


def point(text):
    return np.array([float(x) for x in text.split(",")])


def fields(line):
    return dict(token.split("=", 1) for token in line.split()[1:] if "=" in token)


def side(value):
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


def states(a, b):
    """The census group of a gap and the state of each edge beside it, from two distances."""
    split = "unread" if a is None or b is None else "agree" if max(a, b) < TOUCHING else "apart"
    state = tuple("unread" if x is None else "placed alike" if x < TOUCHING else "placed apart" for x in (a, b))
    return split, state


def read_probe(path):
    """`piecex` places each edge through either face by any evaluator that reads it, parametric
    surfaces included, so its `a_agree` and `b_agree` give a gap's group where it is printed;
    `piece`'s `sides_a` and `sides_b` (planes and cylinders only) give it elsewhere."""
    pieces, gaps, joints, wider = {}, [], [], {}
    # utf-8-sig: Windows PowerShell's `Out-File -Encoding utf8` writes a byte-order mark.
    with open(path, encoding="utf-8-sig", errors="replace") as handle:
        for line in handle:
            if line.startswith("piece "):
                f = fields(line)
                key = (f["graph"], f["face"], f["loop"], f["at"].split("/")[0])
                PLACE[key] = ("closing" if f["at"].split("/")[0] == "0" else "inner",
                              signs(side(f.get("du_ft")), side(f.get("dv_ft"))))
                split, state = states(side(f.get("sides_a")), side(f.get("sides_b")))
                pieces[key] = (float(f["gap"]), split, f.get("surface", "-"), state, state)
            elif line.startswith("piecex "):
                f = fields(line)
                wider[(f["graph"], f["face"], f["loop"], f["at"])] = states(side(f.get("a_agree")), side(f.get("b_agree")))
            elif line.startswith("piecemore "):
                f = fields(line)
                FLAGS[(f["graph"], f["face"], f["loop"], f["at"])] = (f.get("a_flags"), f.get("b_flags"))
            elif line.startswith("pieceorient "):
                f = fields(line)
                ORIENT[(f["graph"], f["face"], f["loop"], f["at"])] = (f.get("p_is", "?"), f.get("q_is", "?"))
            elif line.startswith("ownnbr "):
                f = fields(line)
                OWN.setdefault((f["graph"], f["face"], f["loop"], f["at"]), []).append(f)
            elif line.startswith("piecenbr "):
                f = fields(line)
                key = (f["graph"], f["face"], f["loop"], f["at"])
                NEIGHBOURS[key] = tuple(
                    (f.get(s) or "-").split(":")[1] if ":" in (f.get(s) or "") else "-" for s in ("a", "b"))
                REACH[key] = tuple(reach(f.get(s), TOUCHING) for s in ("a", "b"))
                ORDER[key] = face_order(f.get("a"), f.get("b"))
                REACH_JOIN[key] = tuple(reach(f.get(s), JOIN) for s in ("a", "b"))
            elif line.startswith("gapxyz "):
                gaps.append(fields(line))
            elif line.startswith("jointxyz "):
                joints.append(fields(line))
            elif line.startswith("boundaryunread "):
                UNREAD_OWNERS.append(fields(line).get("owner"))
            elif line.startswith("gapfrom "):
                f = fields(line)
                NEIGHBOUR_REFUSED.add((f["graph"], f["face"]))
            elif line.startswith("edgesides "):
                f = fields(line)
                f["plane"] = [point(x) for x in f["plane"].split(";")]
                f["curved"] = [point(x) for x in f["curved"].split(";")]
                EDGES.append(f)
    for key, (split, state) in wider.items():
        if key in pieces:
            gap, _, surface, _, plane_state = pieces[key]
            pieces[key] = (gap, split, surface, state, plane_state)
    GROUP_SOURCE.update(piecex=sum(1 for k in pieces if k in wider), piece=sum(1 for k in pieces if k not in wider))
    return pieces, gaps, joints


UNREAD_OWNERS = []
# (graph, face) of each `gapfrom` line: a plane refused because a neighbouring face's ring is,
# whose own ring the census does not count.
NEIGHBOUR_REFUSED = set()
# Each `edgesides` line (#2106): a plane's shared edge with both faces' samples placed.
EDGES = []
GROUP_SOURCE = {}
# (graph, face, loop, at) -> the `m_flags` of the edges before and after the gap, as printed.
FLAGS = {}
# (graph, face, loop, at) -> the class of the other face of the edges before and after the gap.
NEIGHBOURS = {}
# (graph, face, loop, at) -> for the edges before and after the gap, which of P and Q an edge
# beside it in its other face's own loop passes within TOUCHING of, and within JOIN of.
REACH = {}
REACH_JOIN = {}
# (graph, face, loop, at) -> (`closing` or `inner`, the signs of the step from P to Q in u and v).
PLACE = {}
# (graph, face, loop, at) -> whose edge's other face comes first: `P's edge`, `Q's edge`, or unread.
ORDER = {}
# (graph, face, loop, at) -> the probe's `ownnbr` rows for the gap (#2136).
OWN = {}
# (graph, face, loop, at) -> whether P and Q are their edges' stored first or last points.
ORIENT = {}


def face_order(a, b):
    """Which side's other face has the lower index in the graph, from `piecenbr`'s `a=` and `b=`."""
    try:
        fa, fb = int(a.split(":", 1)[0]), int(b.split(":", 1)[0])
    except (AttributeError, ValueError):
        return "unread"
    return "P's edge" if fa < fb else "Q's edge" if fb < fa else "the same face"


def signs(du, dv):
    """The signs of a step's two parameter components, `0` for one under 1% of the other (as
    `piece`'s `sig=one` reads a step along one parameter) or under 1e-9 ft."""
    if du is None or dv is None:
        return "??"
    big = max(abs(du), abs(dv))
    return "".join("0" if abs(x) < max(1e-9, 0.01 * big) else "+" if x > 0 else "-" for x in (du, dv))
# The bound within which the probe's loops join (`native_saved_mesh`'s `close`).
JOIN = 1e-7


def reach(value, bound):
    """`P`, `Q`, `P+Q` or `-` from a `piecenbr` side, `<face>:<class>:<edge>[<faces>]:<to P>/<to Q>;...`;
    `unread` where no neighbour is placed, `none` where the edge has no other face."""
    if not value or value == "none" or value.count(":") < 2:
        return "none"
    near_p = near_q = read = False
    for neighbour in value.split(":", 2)[2].split(";"):
        distances = neighbour.rsplit("]:", 1)[-1]
        if "/" not in distances:
            continue
        to_p, to_q = (side(x) for x in distances.split("/", 1))
        if to_p is None or to_q is None:
            continue
        read = True
        near_p |= to_p < bound
        near_q |= to_q < bound
    if not read:
        return "unread"
    return "P+Q" if near_p and near_q else "P" if near_p else "Q" if near_q else "-"


def side_of(value):
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


def flags_of(key):
    a, b = FLAGS.get(key, (None, None))
    return (int(a), int(b)) if a and b and a.isdigit() and b.isdigit() else None


def key_of(g):
    return (g["graph"], g["face"], g["loop"], g["at"])


def parts_of(element):
    """The element and its aggregated parts that carry a representation, as `meshes` reads them."""
    out = [element] if element.Representation else []
    for rel in getattr(element, "IsDecomposedBy", None) or []:
        out.extend(part for part in rel.RelatedObjects if part.Representation)
    return out


def axis2(placement):
    """An IfcAxis2Placement3D as a 4x4 matrix in the file's unit."""
    m = np.eye(4)
    if placement is None:
        return m
    o = np.array(placement.Location.Coordinates, dtype=float)
    z = np.array(placement.Axis.DirectionRatios, dtype=float) if getattr(placement, "Axis", None) else np.array([0.0, 0.0, 1.0])
    x = np.array(placement.RefDirection.DirectionRatios, dtype=float) if getattr(placement, "RefDirection", None) else np.array([1.0, 0.0, 0.0])
    z = z / np.linalg.norm(z)
    x = x - (x @ z) * z
    x = x / np.linalg.norm(x)
    m[:3, 0], m[:3, 1], m[:3, 2], m[:3, 3] = x, np.cross(z, x), z, o
    return m


def operator(target):
    """An IfcCartesianTransformationOperator3D as a 4x4 matrix (its axes as given, then scaled)."""
    m = np.eye(4)
    if target is None:
        return m
    axes = [getattr(target, a, None) for a in ("Axis1", "Axis2", "Axis3")]
    base = np.eye(3)
    cols = [np.array(a.DirectionRatios, dtype=float) / np.linalg.norm(a.DirectionRatios) if a else base[:, k] for k, a in enumerate(axes)]
    scale = float(getattr(target, "Scale", None) or 1.0)
    for k in range(3):
        m[:3, k] = cols[k] * scale
    m[:3, 3] = np.array(target.LocalOrigin.Coordinates, dtype=float)
    return m


def body_items(element):
    """Each item of the Body representations of an element and its parts, with the matrix that
    places it. Axis, footprint and box representations are not the body and are left out."""
    import ifcopenshell.util.placement

    out = []
    for part in parts_of(element):
        top = np.array(ifcopenshell.util.placement.get_local_placement(part.ObjectPlacement), dtype=float)
        stack = [(item, top) for rep in part.Representation.Representations
                 if (rep.RepresentationIdentifier or "") == "Body" for item in rep.Items]
        while stack:
            item, m = stack.pop()
            if item.is_a("IfcMappedItem"):
                source = item.MappingSource
                # As IfcOpenShell maps an item: its target, then the map's origin as a placement.
                inner = m @ operator(item.MappingTarget) @ axis2(source.MappingOrigin)
                stack.extend((x, inner) for x in source.MappedRepresentation.Items)
            else:
                out.append((item, m))
    return out


def local_points(item):
    """Revit's own coordinates of a polygonal item in its local frame, or None where the item is
    not made of polygons (a face of a polygon solid bounded by anything but an IfcPolyLoop)."""
    kind = item.is_a()
    if kind in FACESETS:
        # Only the points a face uses: a coordinate list may hold more (on the BIM4LCA housing a
        # basin's largest face set lists 17,276 that no triangle indexes).
        coords = item.Coordinates.CoordList
        if kind == "IfcTriangulatedFaceSet":
            used = {i for tri in item.CoordIndex for i in tri}
        else:
            used = set()
            for face in item.Faces:
                used.update(face.CoordIndex)
                for inner in getattr(face, "InnerCoordIndices", None) or []:
                    used.update(inner)
        pn = getattr(item, "PnIndex", None)
        positions = sorted({pn[i - 1] for i in used} if pn else used)
        return np.array([coords[i - 1] for i in positions], dtype=float)
    shells = []
    if kind in ("IfcFacetedBrep", "IfcFacetedBrepWithVoids"):
        shells = [item.Outer] + list(getattr(item, "Voids", None) or [])
    elif kind == "IfcShellBasedSurfaceModel":
        shells = list(item.SbsmBoundary)
    else:
        return None
    pts = []
    for shell in shells:
        for face in shell.CfsFaces:
            for bound in face.Bounds:
                loop = bound.Bound
                if not loop.is_a("IfcPolyLoop"):
                    return None
                pts.extend(pt.Coordinates for pt in loop.Polygon)
    return np.array(pts, dtype=float) if pts else None


def segment_distance(points, a, b):
    ab = b - a
    t = np.clip(((points - a) @ ab) / max(float(ab @ ab), 1e-24), 0.0, 1.0)
    return np.linalg.norm(points - (a + t[:, None] * ab), axis=1)


def outcome(verts, gap, p, q, tol):
    if gap < 2 * tol:
        return "unresolved"
    dp = np.linalg.norm(verts - p, axis=1)
    dq = np.linalg.norm(verts - q, axis=1)
    at_p, at_q = bool((dp < tol).any()), bool((dq < tol).any())
    if bool(((segment_distance(verts, p, q) < tol) & (dp > tol) & (dq > tol)).any()):
        # A point between the ends, and which ends are there too: alone, it is Revit merging the
        # two ends into one vertex; with both, the gap is drawn with a vertex inside it.
        return "between_both" if at_p and at_q else "between_p" if at_p else "between_q" if at_q else "between"
    return "both" if at_p and at_q else "p_only" if at_p else "q_only" if at_q else "none"


def between_at(verts, p, q, tol):
    """Where along the gap, from P (0) to Q (1), the vertex nearest the segment lies among those
    within `tol` of it and farther than `tol` from both ends; None where there is none."""
    ab = q - p
    t = ((verts - p) @ ab) / max(float(ab @ ab), 1e-24)
    near = (segment_distance(verts, p, q) < tol) & (np.linalg.norm(verts - p, axis=1) > tol) & (np.linalg.norm(verts - q, axis=1) > tol)
    if not near.any():
        return None
    idx = np.flatnonzero(near)
    best = idx[int(np.argmin(segment_distance(verts[idx], p, q)))]
    return float(np.clip(t[best], 0.0, 1.0))


def floors_of(bodies, joints, tags, offset=None):
    out = {}
    for tag in tags:
        verts = bodies[tag]
        if offset is not None:
            verts = verts - offset
        closed = [point(j["x"]) for j in joints if j["owner"] == tag and j["at_gap"] == "false"]
        if not closed:
            continue
        vectors = []
        for c in closed:
            d = np.linalg.norm(verts - c, axis=1)
            vectors.append(verts[int(d.argmin())] - c)
        vectors = np.array(vectors)
        dist = np.linalg.norm(vectors, axis=1)
        out[tag] = {
            "floor": float(np.median(dist)),
            "vector": np.median(vectors, axis=0),
            "within": float((dist < TOUCHING).mean()),
            "n": len(dist),
        }
    return out


def score(gaps, bodies, faceted, floors, offset=None, present=None):
    rows = []
    for g in gaps:
        tag = g["owner"]
        if tag not in (bodies if present is None else present):
            o = "no_revit_body"
        elif tag not in faceted or tag not in bodies:
            o = "not_faceted"
        elif tag not in floors:
            o = "no_floor"
        elif floors[tag]["floor"] >= PLACEMENT:
            o = "placement"
        else:
            verts = bodies[tag] if offset is None else bodies[tag] - offset
            tol = max(2 * floors[tag]["floor"], TOUCHING)
            o = outcome(verts, float(g["gap"]), point(g["p"]), point(g["q"]), tol)
        rows.append((g, o))
    return rows


def counts(c):
    return ", ".join(f"{k} {v}" for k, v in sorted(c.items())) or "none"


def summary(label, rows, cohort):
    """Outcomes by cohort, per occurrence and per gap, and the registered verdict per cohort."""
    per_gap = collections.defaultdict(collections.Counter)
    for g, o in rows:
        per_gap[key_of(g)][o] += 1
    for c in COHORTS:
        occ = collections.Counter(o for g, o in rows if cohort(g) == c)
        gaps_c = {k: v for k, v in per_gap.items() if cohort_key(k, cohort) == c}
        if not occ:
            continue
        agree = sum(1 for v in gaps_c.values() if len(v) == 1)
        by_gap = collections.Counter(v.most_common(1)[0][0] for v in gaps_c.values())
        print(f"{label} {c}: occurrences {counts(occ)}")
        print(f"{label} {c}: gaps {len(gaps_c)} (occurrences all agree on {agree}); by outcome {counts(by_gap)}")
        scored = [k for k, v in gaps_c.items() if v.most_common(1)[0][0] in SCORED]
        one_end = [k for k in scored if gaps_c[k].most_common(1)[0][0] in ONE_END]
        if not scored:
            verdict = "not exercised: no gap scored"
        elif len(one_end) == len(scored):
            verdict = f"one end kept on all {len(scored)} scored gaps, as on the office"
        else:
            verdict = f"one end kept on {len(one_end)} of {len(scored)} scored gaps; the others: " + counts(
                collections.Counter(gaps_c[k].most_common(1)[0][0] for k in scored if k not in one_end))
        print(f"{label} {c}: verdict: {verdict}")
    return per_gap


def cohort_key(key, cohort):
    return cohort({"graph": key[0], "face": key[1], "loop": key[2], "at": key[3]})


def raw_points(model, elements, tags):
    """Tag -> Revit's own coordinates of the element's polygonal Body items, placed through the
    element's placement, each mapped item's target and its map's origin, the file's unit and the site frame,
    into the frame of the probe's points, with no meshing between; None where an item is not
    polygonal."""
    import ifcopenshell.util.unit

    unit = ifcopenshell.util.unit.calculate_unit_scale(model)
    frame = site_frame(model)
    out = {}
    for tag in tags:
        pts = []
        for item, m in (x for e in elements[tag] for x in body_items(e)):
            local = local_points(item)
            if local is None:
                pts = None
                break
            pts.append(((m[:3, :3] @ local.T).T + m[:3, 3]) * unit)
        if pts:
            out[tag] = frame(np.unique(np.concatenate(pts), axis=0))
    return out


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    per_gap_lines = "--per-gap" in sys.argv
    if len(args) < 2:
        print(__doc__)
        return 2
    import ifcopenshell
    import ifcopenshell.geom

    print(f"ifcopenshell {ifcopenshell.version}, numpy {np.__version__}")
    pieces, gaps, joints = read_probe(args[0])
    if not pieces:
        # An empty or unreadable probe output (in Windows PowerShell 5, `>` writes UTF-16) would
        # otherwise read as a holding row 0 over nothing.
        print("row 0 FAILS: no `piece` line read; the probe output is empty, incomplete or not UTF-8 "
              "(in Windows PowerShell write it with `| Out-File -Encoding utf8`)")
        return 1

    def cohort(g):
        return pieces.get(key_of(g), (None, "not in the census", "-", None, None))[1]

    def surface(g):
        return pieces.get(key_of(g), (None, "-", "-", None, None))[2]

    # 1. Row 0. A gap on a plane refused because of its neighbour (`gapfrom`) has no census line;
    # it is counted apart and left out, and any other gap not in the census fails the row.
    outside = [g for g in gaps if key_of(g) not in pieces and (g["graph"], g["face"]) in NEIGHBOUR_REFUSED]
    if outside:
        print(f"gaps on planes refused because of a neighbouring face (`gapfrom`), outside the census, left out: {len(outside)}")
        gaps = [g for g in gaps if not (key_of(g) not in pieces and (g["graph"], g["face"]) in NEIGHBOUR_REFUSED)]
    equal = differ = unmatched = 0
    for g in gaps:
        key = key_of(g)
        if key not in pieces:
            unmatched += 1
        elif abs(pieces[key][0] - float(g["gap"])) <= 1e-9:
            equal += 1
        else:
            differ += 1
    holds = differ == 0 and unmatched == 0
    print(f"row 0 {'holds' if holds else 'FAILS'}: gap occurrences {len(gaps)}, equal to their census gap {equal}, different {differ}, not in the census {unmatched}")
    if not holds:
        return 1
    # 2. Coverage.
    reached = {key_of(g) for g in gaps}
    census = collections.Counter(v[1] for v in pieces.values())
    hit = collections.Counter(pieces[k][1] for k in reached if k in pieces)
    print(f"coverage: census gaps {len(pieces)} ({counts(census)}); reached by an element at level 3 {len(reached)} ({counts(hit)}); "
          f"groups from `piecex` on {GROUP_SOURCE.get('piecex', 0)} gaps and from `piece` on {GROUP_SOURCE.get('piece', 0)}")
    tags = {g["owner"] for g in gaps} | {e["owner"] for e in EDGES}
    print(f"elements reaching a gap {len(tags)}; ring ends {len(joints)}; elements whose graphics the probe could not walk (`boundaryunread`) {len(UNREAD_OWNERS)}")
    if not gaps and not EDGES:
        print("not exercised: no gap or shared edge reached by an element's graphics")
        return 0
    # 3. Bodies: Revit's own coordinates decide; IfcOpenShell's mesh is the cross-check.
    model = ifcopenshell.open(args[1])
    kinds = collections.defaultdict(collections.Counter)
    elements = {}
    # Every element with the Tag but openings, which carry their host's Tag in Revit's export:
    # `meshes` reads the same set.
    for element in model.by_type("IfcElement"):
        if element.Tag in tags and not element.is_a("IfcOpeningElement"):
            elements.setdefault(element.Tag, []).append(element)
            for item, _ in body_items(element):
                kinds[element.Tag][item.is_a()] += 1
    faceted = {t for t, c in kinds.items() if c and set(c) <= FACETED}
    print("Body item types: " + counts(collections.Counter(k for t in tags for k in kinds.get(t, {}))))
    raw = raw_points(model, elements, sorted(faceted))
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    try:
        settings.set("weld-vertices", False)
        weld_note = "weld-vertices off"
    except Exception as e:  # the setting's name differs between IfcOpenShell releases
        weld_note = f"weld-vertices could not be set ({type(e).__name__})"
    try:
        got = meshes(args[1], tags=tags, model=model, settings=settings)
    except Exception as e:  # the mesh is a cross-check only; its failure leaves Revit's reading
        print(f"mesh cross-check failed ({type(e).__name__}); outcomes stand on Revit's coordinates alone")
        got = {}
    mesh = {t: np.unique(v[2].reshape(-1, 3), axis=0) for t, v in got.items()}
    print(f"bodies: elements with a Revit body by Tag {len(set(elements) & tags)} of {len(tags)}, made of polygons {len(faceted)}, "
          f"Revit's coordinates read on {len(raw)}; IfcOpenShell meshed {len(set(mesh) & faceted)} of them ({weld_note})")
    # A refused plane's ring ends include the ends of its disputed edges (`edgesides`), which are
    # what is being measured, so they are not controls: left in, a floor would rise to their
    # separation wherever Revit follows the curved face's samples.
    if EDGES:
        disputed = collections.defaultdict(list)
        for e in EDGES:
            disputed[e["owner"]].extend(e["plane"] + e["curved"])
        before = len(joints)
        joints = [j for j in joints if not any(np.linalg.norm(point(j["x"]) - d) < 1e-7 for d in disputed.get(j["owner"], ()))]
        print(f"ring-end controls on a disputed shared edge's samples, left out of the floors: {before - len(joints)}")
    # 4. The floors: Revit's coordinates (raw) decide; the mesh's is printed beside it.
    floors = floors_of(raw, joints, sorted(raw))
    mesh_floors = floors_of(mesh, joints, sorted(set(mesh) & faceted))
    placement = [t for t, f in floors.items() if f["floor"] >= PLACEMENT]
    usable = {t: f for t, f in floors.items() if f["floor"] < PLACEMENT}
    for label, fs in (("raw floor (decides)", floors), ("mesh floor (cross-check)", mesh_floors)):
        if fs:
            fl = sorted(f["floor"] for f in fs.values())
            within = sum(f["within"] * f["n"] for f in fs.values()) / sum(f["n"] for f in fs.values())
            off = sum(1 for f in fs.values() if f["floor"] >= PLACEMENT)
            print(f"{label} over {len(fs)} elements (0.01 ft or more on {off}): min {fl[0]:.9f} median {fl[len(fl) // 2]:.9f} max {fl[-1]:.9f} ft; "
                  f"closed ends within 0.00001 ft of a Revit vertex {within:.1%}")
    both = sorted(set(floors) & set(mesh_floors))
    mesh_higher = sum(1 for t in both if mesh_floors[t]["floor"] > max(2 * floors[t]["floor"], TOUCHING))
    raw_higher = sum(1 for t in both if floors[t]["floor"] > max(2 * mesh_floors[t]["floor"], TOUCHING))
    print(f"floors disagree (one over twice the other and 0.00001 ft): mesh higher on {mesh_higher}, raw higher on {raw_higher} of {len(both)} elements"
          + ("; raw higher means Revit's coordinates are placed wrong, and nothing after this counts" if raw_higher else ""))
    if floors and not usable:
        print("not exercised: every element is 0.01 ft or more off in Revit's coordinates, so the frames differ")
        return 0
    offset = None
    if usable:
        vecs = np.array([f["vector"] for f in usable.values()])
        common = np.median(vecs, axis=0)
        spread = float(np.abs(vecs - common).max())
        print(f"median offset vectors: common {common[0]:+.9f},{common[1]:+.9f},{common[2]:+.9f} ft, largest departure from it {spread:.9f} ft")
        offset = common if spread <= SAME_OFFSET and float(np.linalg.norm(common)) > SAME_OFFSET else None
        print("offset: one vector, taken off" if offset is not None else "offset: no single nonzero vector")
        if offset is not None:
            floors = floors_of(raw, joints, sorted(raw), offset)
    # 5. Outcomes, by cohort, from Revit's coordinates.
    rows = score(gaps, raw, faceted, floors, offset, present=elements)
    per_gap = summary("outcomes", rows, cohort)
    mesh_rows = score(gaps, mesh, faceted, mesh_floors, present=elements)
    cross = collections.Counter((cohort(g), o, m) for (g, o), (_, m) in zip(rows, mesh_rows) if o in SCORED or m in SCORED)
    for c in COHORTS:
        line = ", ".join(f"{o}->{m} {n}" for (cc, o, m), n in sorted(cross.items()) if cc == c)
        if line:
            print(f"mesh cross-check {c} (raw outcome -> mesh outcome): {line}")
    # 6. Corners and gap sizes, by cohort.
    scored = [(g, o) for g, o in rows if o in SCORED]
    corners = collections.defaultdict(list)
    for g, o in scored:
        ends = sorted([tuple(np.round(point(g["p"]), 6)), tuple(np.round(point(g["q"]), 6))])
        corners[tuple(ends)].append((g, o))
    print(f"scored occurrences {len(scored)}, distinct gaps {len({key_of(g) for g, _ in scored})}, "
          f"corners {len(corners)}, elements {len({g['owner'] for g, _ in scored})}, graphs {len({g['graph'] for g, _ in scored})}")
    by_size = collections.defaultdict(collections.Counter)
    corners_by_size = collections.Counter()
    classes = collections.defaultdict(collections.Counter)
    tols = collections.defaultdict(list)
    for ends, members in corners.items():
        g0 = members[0][0]
        size = (cohort(g0), surface(g0), round(float(g0["gap"]), 6))
        corners_by_size[size] += 1
        for g, o in members:
            by_size[size][o] += 1
            names = {e.is_a() + (f" {e.ObjectType}" if per_gap_lines and e.ObjectType else "") for e in elements[g["owner"]]}
            classes[size]["+".join(sorted(names))] += 1
            tols[size].append((floors[g["owner"]]["floor"], max(2 * floors[g["owner"]]["floor"], TOUCHING)))
        if per_gap_lines:
            print(f"corner {size[0]} {size[1]} gap {size[2]:.6f} ft: occurrences {len(members)}, " + counts(collections.Counter(o for _, o in members)))
    # Which end Revit keeps, against the state of the edge before the gap (ending at P) and the
    # edge after it (starting at Q): on the office it keeps the end of the edge both faces place.
    # Twice: by `piecex`'s reading (any evaluator), and by `piece`'s, which places an edge through
    # planes and cylinders only; on the office the end Revit keeps is on the edge `piece` reads.
    for which, label in ((3, "kept end by its edges"), (4, "kept end by its edges as planes and cylinders place them")):
        kept = collections.Counter()
        for g, o in scored:
            if o in ONE_END:
                state = pieces.get(key_of(g), (None, None, None, None, None))[which]
                if state:
                    kept[(cohort(g), o, state[0], state[1])] += 1
        for c in COHORTS:
            line = ", ".join(f"{o} with P's edge {a} and Q's edge {b} {n}" for (cc, o, a, b), n in sorted(kept.items()) if cc == c)
            if line:
                print(f"{label}, {c}: {line}")
    # 9. Which end Revit keeps against the stored `m_flags` of the edges beside the gap.
    by_bit, by_pair, other_end = collections.Counter(), collections.Counter(), collections.Counter()
    by_graph_gap = {}
    by_orient_flag = {}
    by_faces, by_reach, by_join = collections.Counter(), collections.Counter(), collections.Counter()
    flagged_side = collections.Counter()
    by_place, by_step, by_order = collections.Counter(), collections.Counter(), collections.Counter()
    orient_seen = {}
    for g, o in scored:
        if o in ONE_END and key_of(g) in ORIENT:
            orient_seen.setdefault(key_of(g), (cohort(g), "/".join(ORIENT[key_of(g)]), o))
        by_order[(cohort(g), ORDER.get(key_of(g), "unread"), o)] += 1
        place, step = PLACE.get(key_of(g), ("unread", "??"))
        by_place[(cohort(g), place, o)] += 1
        by_step[(cohort(g), f"du{step[0]} dv{step[1]}", o)] += 1
        by_faces[(cohort(g), NEIGHBOURS.get(key_of(g), ("unread", "unread")), o)] += 1
        by_reach[(cohort(g), REACH.get(key_of(g), ("unread", "unread")), o)] += 1
        by_join[(cohort(g), REACH_JOIN.get(key_of(g), ("unread", "unread")), o)] += 1
        fl = flags_of(key_of(g))
        if fl is not None and bool(fl[0] & FLAG_BIT) != bool(fl[1] & FLAG_BIT):
            on_p = bool(fl[0] & FLAG_BIT)
            ends = {"P": "the flagged edge's end" if on_p else "the other edge's end",
                    "Q": "the other edge's end" if on_p else "the flagged edge's end", "P+Q": "both ends", "-": "neither end"}
            kept = {("p_only" if on_p else "q_only"): "keeps the flagged edge's end", ("q_only" if on_p else "p_only"): "keeps the other edge's end"}
            for bound, table in (("0.00001 ft", REACH), ("1e-7 ft", REACH_JOIN)):
                side_reach = table.get(key_of(g), ("unread", "unread"))[0 if on_p else 1]
                flagged_side[(cohort(g), bound, ends.get(side_reach, side_reach), kept.get(o, o))] += 1
        fl = flags_of(key_of(g))
        if fl is None:
            by_bit[(cohort(g), "unread", o)] += 1
            continue
        a, b = (bool(x & FLAG_BIT) for x in fl)
        where = "neither edge" if not (a or b) else "both edges" if a and b else "P's edge only" if a else "Q's edge only"
        by_bit[(cohort(g), where, o)] += 1
        by_pair[(cohort(g), fl, o)] += 1
        if a != b:
            other = "q_only" if a else "p_only"
            flagged = "p_only" if a else "q_only"
            if o in (other, flagged):
                by_graph_gap.setdefault((cohort(g), g["graph"], key_of(g)), "other" if o == other else "flagged")
                orient = ORIENT.get(key_of(g))
                if orient:
                    flagged_is, other_is = (orient[0], orient[1]) if a else (orient[1], orient[0])
                    by_orient_flag.setdefault(key_of(g), (cohort(g), f"flagged end {flagged_is}, other end {other_is}",
                                                          "keeps the other edge's end" if o == other else "keeps the flagged edge's end",
                                                          other_is))
            other_end[(cohort(g), "at the other edge's end only" if o == other else "at the flagged edge's end only" if o == flagged else "otherwise")] += 1
    # The `ownnbr` rows: a side whose neighbour joins the shared edge and lies at one gap end.
    own_sides, own_seen = collections.Counter(), {}
    for g, o in scored:
        if o not in ONE_END:
            continue
        for side in ("a", "b"):
            ends = set()
            for row in OWN.get(key_of(g), []):
                if row.get("side") != side:
                    continue
                join, to_p, to_q = side_of(row.get("join")), side_of(row.get("end_p")), side_of(row.get("end_q"))
                if join is not None and to_p is not None and to_q is not None and join < JOIN and (to_p < JOIN) != (to_q < JOIN):
                    ends.add("p_only" if to_p < JOIN else "q_only")
            if len(ends) == 1:
                own_seen.setdefault((key_of(g), side), (cohort(g), "keeps that end" if o in ends else "keeps the other end"))
    # Counted once per stored gap's side, as the drawings of one stored gap agree.
    own_sides.update(own_seen.values())
    for c in COHORTS:
        groups = {}
        for (cc, w, o), n in sorted(by_bit.items()):
            if cc == c:
                groups.setdefault(w, []).append(f"{o} {n}")
        if groups:
            print(f"kept end by its edges' m_flags bit 0x{FLAG_BIT:x}, {c}: " + "; ".join(f"on {w}: {', '.join(v)}" for w, v in groups.items()))
        pairs = [f"{a}/{b} {o} {n}" for (cc, (a, b), o), n in sorted(by_pair.items()) if cc == c]
        if pairs:
            print(f"edge m_flags (P's edge/Q's edge), {c}: " + ", ".join(pairs))
        faces = [f"{a}/{b} {o} {n}" for (cc, (a, b), o), n in sorted(by_faces.items()) if cc == c]
        if faces:
            print(f"edges' other faces (P's edge/Q's edge), {c}: " + ", ".join(faces))
        reached = [f"{a}/{b} {o} {n}" for (cc, (a, b), o), n in sorted(by_reach.items()) if cc == c]
        if reached:
            print(f"where each edge's other face's loop meets P and Q (P's edge/Q's edge), {c}: " + ", ".join(reached))
        joined = [f"{a}/{b} {o} {n}" for (cc, (a, b), o), n in sorted(by_join.items()) if cc == c]
        if joined:
            print(f"where each edge's other face's loop meets P and Q within 1e-7 ft (P's edge/Q's edge), {c}: " + ", ".join(joined))
        ordered = [f"{w} first {o} {n}" for (cc, w, o), n in sorted(by_order.items()) if cc == c]
        if ordered:
            print(f"kept end by whose edge's other face comes first in the graph, {c}: " + ", ".join(ordered))
        own = {k: n for (cc, k), n in own_sides.items() if cc == c}
        if own:
            print(f"ownnbr sides of stored gaps whose neighbour joins the shared edge and lies at one gap end, within 1e-7 ft, {c}: "
                  + ", ".join(f"{k} {n}" for k, n in sorted(own.items())))
        corner_order = collections.Counter()
        for _, members in corners.items():
            views = [g for g, _ in members if cohort(g) == c and ORDER.get(key_of(g)) in ("P's edge", "Q's edge")]
            if len(views) < 2:
                continue
            picks = {tuple(np.round(point(g["p" if ORDER[key_of(g)] == "P's edge" else "q"]), 6)) for g in views}
            corner_order["the same end" if len(picks) == 1 else "different ends"] += 1
        if corner_order:
            print(f"whose edge's other face comes first, on a corner seen from more than one ring, {c}: names "
                  + ", ".join(f"{k} on {n} corners" for k, n in sorted(corner_order.items())))
        placed = [f"{w} {o} {n}" for (cc, w, o), n in sorted(by_place.items()) if cc == c]
        if placed:
            print(f"kept end by the gap's place in its ring, {c}: " + ", ".join(placed))
        stepped = [f"{w} {o} {n}" for (cc, w, o), n in sorted(by_step.items()) if cc == c]
        if stepped:
            print(f"kept end by the signs of the step from P to Q in the surface's parameters, {c}: " + ", ".join(stepped))
        for bound in ("0.00001 ft", "1e-7 ft"):
            sided = [f"meets {r}, {k} {n}" for (cc, bb, r, k), n in sorted(flagged_side.items()) if cc == c and bb == bound]
            if sided:
                print(f"bit 0x{FLAG_BIT:x} on one edge only, where the flagged edge's other face's loop meets the gap within {bound}, {c}: " + "; ".join(sided))
        # A corner's two rings label the kept point P in one and Q in the other, so a reading by the
        # stored orientation holds on a corner only where its two views read opposite ways.
        if c == COHORTS[0]:
            pairs = collections.Counter()
            for _, members in corners.items():
                views = sorted(f"{'/'.join(ORIENT[key_of(g)])} {o}" for g, o in members if key_of(g) in ORIENT and o in ONE_END)
                if len(views) >= 2:
                    pairs[" with ".join(views)] += 1
            if pairs:
                print("stored orientation of a corner's rings, on corners seen from more than one ring, every cohort: "
                      + "; ".join(f"{k}: {n}" for k, n in sorted(pairs.items())))
        # The coordinated join the chair save suggests (#2085): a gap two rings hold, one ring's P and Q
        # its edges' last and first points, the other's both first or both last, joined at the
        # first ring's Q. Every such corner Revit does not join there would make the rule wrong.
        if c == COHORTS[0]:
            judged = collections.Counter()
            distinct = collections.defaultdict(set)
            widest = 0.0
            # Every outcome, scored or not: a corner Revit's export cannot judge is still reach.
            every = collections.defaultdict(list)
            for g, o in rows:
                every[tuple(sorted([tuple(np.round(point(g["p"]), 6)), tuple(np.round(point(g["q"]), 6))]))].append((g, o))
            for _, members in every.items():
                views = [(g, o) for g, o in members if key_of(g) in ORIENT]
                if len(views) != 2:
                    continue
                cfg = ["/".join(ORIENT[key_of(g)]) for g, _ in views]
                lf = [i for i, k in enumerate(cfg) if k == "last/first"]
                if len(lf) != 1 or cfg[1 - lf[0]] not in ("first/first", "last/last"):
                    continue
                (g1, o1), (g2, o2) = views[lf[0]], views[1 - lf[0]]
                widest = max(widest, float(g1["gap"]), float(g2["gap"]))
                same = tuple(np.round(point(g1["q"]), 6)) == tuple(np.round(point(g2["p"]), 6))
                width = "under 0.0001 ft" if max(float(g1["gap"]), float(g2["gap"])) < 1e-4 else "0.0001 ft or more"
                if o1 not in SCORED or o2 not in SCORED:
                    verdict = f"{width}, not judged ({o1} with {o2})"
                elif (o1, o2) == ("q_only", "p_only") and same:
                    verdict = f"{width}, Revit keeps the last/first ring's Q"
                else:
                    verdict = f"{width}, otherwise, {o1} with {cfg[1 - lf[0]]} {o2}"
                judged[verdict] += 1
                distinct[verdict].add(tuple(sorted((key_of(g1), key_of(g2)))))
            if judged:
                print(f"coordinated join (#2085), corners two rings hold, one last/first and one first/first or last/last, "
                      f"widest gap {widest:.9f} ft: " + "; ".join(f"{k}: {n} corners, {len(distinct[k])} distinct gaps"
                                                            for k, n in sorted(judged.items())))
            else:
                print("coordinated join (#2085): no corner two rings hold meets it")
        oriented = collections.Counter((w, o) for cc, w, o in orient_seen.values() if cc == c)
        if oriented:
            print(f"kept end by whether P and Q are their edges' stored first or last points (P/Q), stored gaps, {c}: "
                  + ", ".join(f"{w} {o} {n}" for (w, o), n in sorted(oriented.items())))
        # The other (unflagged) edge is one stored edge shared by a corner's two rings, so its end's
        # orientation reads the same from both; the flagged edge is a different edge in each ring.
        other_oriented = collections.Counter((f"other end {oi}", k) for cc, _, k, oi in by_orient_flag.values() if cc == c)
        if other_oriented:
            print(f"bit 0x{FLAG_BIT:x} on one edge only, by the other edge's end's stored orientation, stored gaps, {c}: "
                  + "; ".join(f"{w}: {k} {n}" for (w, k), n in sorted(other_oriented.items())))
        flag_oriented = collections.Counter((w, k) for cc, w, k, _ in by_orient_flag.values() if cc == c)
        if flag_oriented:
            print(f"bit 0x{FLAG_BIT:x} on one edge only, by the two ends' stored orientation, stored gaps, {c}: "
                  + "; ".join(f"{w}: {k} {n}" for (w, k), n in sorted(flag_oriented.items())))
        per_graph = collections.defaultdict(collections.Counter)
        for (cc, graph, _), kept in by_graph_gap.items():
            if cc == c:
                per_graph[graph][kept] += 1
        if per_graph:
            placements = collections.defaultdict(set)
            for g, _ in scored:
                placements[g["graph"]].add(g["owner"])
            print(f"bit 0x{FLAG_BIT:x} on one edge only, stored gaps by graph, {c}: "
                  + "; ".join(f"{graph} ({len(placements[graph])} placements): the other edge's end {k['other']}, the flagged edge's end {k['flagged']}"
                              for graph, k in sorted(per_graph.items())))
        told = {k: n for (cc, k), n in other_end.items() if cc == c}
        if told:
            print(f"bit 0x{FLAG_BIT:x} on one edge only, {c}: {sum(told.values())} drawings; Revit's point "
                  + ", ".join(f"{k} {n}" for k, n in sorted(told.items())))
    for c, surf, size in sorted(by_size):
        k = (c, surf, size)
        fl = [f for f, _ in tols[k]]
        tl = [t for _, t in tols[k]]
        print(f"gap size {c} {surf} {size:.6f} ft ({size * 304.8:.3f} mm): corners {corners_by_size[k]}, occurrences {counts(by_size[k])}; "
              f"classes {counts(classes[k])}; floor {min(fl):.9f} to {max(fl):.9f} ft, tol {min(tl):.9f} to {max(tl):.9f} ft")
    # 8. Shared edges whose two faces' samples differ (#2106): at each interior sample the two
    # faces put far enough apart to tell, which of them Revit's own points sit on.
    if EDGES:
        verdicts = collections.Counter()
        largest = 0.0
        for e in EDGES:
            tag = e["owner"]
            largest = max(largest, float(e["apart"]))
            if tag not in raw or tag not in floors:
                verdicts[(e["other"], "no Revit body or floor")] += 1
                continue
            tol = max(2 * floors[tag]["floor"], TOUCHING)
            verts = raw[tag] if offset is None else raw[tag] - offset
            calls = collections.Counter()
            for a, c in zip(e["plane"], e["curved"]):
                if np.linalg.norm(a - c) < 2 * tol:
                    calls["unresolved"] += 1
                    continue
                on_a = bool((np.linalg.norm(verts - a, axis=1) < tol).any())
                on_c = bool((np.linalg.norm(verts - c, axis=1) < tol).any())
                calls["both" if on_a and on_c else "plane's" if on_a else "curved face's" if on_c else "neither"] += 1
            told = {k: v for k, v in calls.items() if k != "unresolved"}
            verdict = "unresolved" if not told else next(iter(told)) if len(told) == 1 else "split"
            verdicts[(e["other"], verdict)] += 1
            if per_gap_lines:
                print(f"edge owner={tag} graph={e['graph']} face={e['face']} edge={e['edge']} other={e['other']} n={e['n']} apart={e['apart']} "
                      + " ".join(f"{k}={v}" for k, v in sorted(calls.items())) + f" verdict={verdict}")
        samples = collections.Counter(int(e["n"]) for e in EDGES)
        print("shared edges by their sample count: " + ", ".join(f"{k} samples {v}" for k, v in sorted(samples.items())))
        print(f"shared edges whose faces' samples differ: {len(EDGES)}, largest separation {largest:.9f} ft; "
              "Revit's points on, by edge: " + ", ".join(f"{o} {v} {n}" for (o, v), n in sorted(verdicts.items())))
    # 7. Where a point between the ends lies, from P (0) to Q (1): 0.5 is the gap's middle.
    along = collections.defaultdict(list)
    for g, o in scored:
        if o in BETWEEN:
            tag = g["owner"]
            verts = raw[tag] if offset is None else raw[tag] - offset
            along[cohort(g)].append(between_at(verts, point(g["p"]), point(g["q"]), max(2 * floors[tag]["floor"], TOUCHING)))
    for c in COHORTS:
        if along.get(c):
            ts = sorted(along[c])
            print(f"between, {c}: {len(ts)} drawings, along the gap from P (0) to Q (1): min {ts[0]:.3f} median {ts[len(ts) // 2]:.3f} max {ts[-1]:.3f}")
    if per_gap_lines:
        for key, c in sorted(per_gap.items()):
            fl = flags_of(key)
            print(f"gap graph={key[0]} face={key[1]} loop={key[2]} at={key[3]} cohort={pieces.get(key, (None, '-', '-', None, None))[1]} "
                  f"flags={'unread' if fl is None else f'{fl[0]}/{fl[1]}'} faces={'/'.join(NEIGHBOURS.get(key, ('unread', 'unread')))} reach={'/'.join(REACH.get(key, ('unread', 'unread')))} joins={'/'.join(REACH_JOIN.get(key, ('unread', 'unread')))} first={ORDER.get(key, 'unread').replace("'s edge", '')} place={PLACE.get(key, ('unread', '??'))[0]} step=du{PLACE.get(key, ('unread', '??'))[1][0]}dv{PLACE.get(key, ('unread', '??'))[1][1]} "
                  + " ".join(f"{k}={v}" for k, v in sorted(c.items())))
    return 0


if __name__ == "__main__":
    sys.exit(main())
