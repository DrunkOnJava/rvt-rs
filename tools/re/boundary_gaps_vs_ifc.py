#!/usr/bin/env python3
"""Where Revit's own body puts the boundary across each gap of a refused ring (#2085).

`probe_2085_trim_breaks` prints, for every element whose graphics reach a graph with a refused
plane or cylinder ring, each gap's two ends (`gapxyz ... p=x,y,z q=x,y,z`) and every edge end of
the same ring (`jointxyz ... at_gap=<bool> x=x,y,z`), in document feet, beside its census of the
same gaps (`piece`). This sets them against Revit's body for the same Tag and prints counts and
distances only, never a coordinate; `--per-gap` adds one line per gap with its ElementIds.

1. **Row 0.** Each `gapxyz` gap equals the `piece` gap of the same graph, face, loop and
   position, so both lines read one gap finder. Any difference or unmatched gap is printed
   and nothing after it is trusted.
2. **Coverage.** The gaps an element's graphics reach at detail level 3, of the census, split
   into the census's cohorts: both edges beside the gap placed alike by their two faces
   (`agree`: `sides_a` and `sides_b` both under 0.00001 ft), one of them placed apart
   (`apart`), or either unread (`unread`).
3. **Bodies.** Only the Body representations of the Tag's element and its aggregated parts.
   Revit's vertices are compared only where every Body item is polygonal: an
   `IfcPolygonalFaceSet`, `IfcTriangulatedFaceSet`, `IfcFacetedBrep` or
   `IfcShellBasedSurfaceModel` of polygon loops, alone or mapped. The mesh is taken twice from
   IfcOpenShell, as it meshes by default and with `weld-vertices` off where the setting exists;
   outcomes are read from the unwelded one.
4. **Two floors, for every faceted element.** The ring ends that close (`at_gap=false`) to
   Revit's nearest vertex, in the mesh and in Revit's raw coordinates placed into the probe's
   frame (through the element's placement, each mapped item's origin and target, the file's
   unit and the site frame, with no meshing between). The median distance is the floor; an
   element whose raw floor exceeds twice its mesh floor and 0.00001 ft is counted, since then the
   placement is read wrong. Where every element's median vector is one nonzero vector it is a
   frame offset, and the outcomes are read again with it taken off. An element whose mesh floor
   is 0.01 ft or more is a placement mismatch, counted apart.
5. **Outcomes, per occurrence and by cohort.** With `tol` the larger of twice the element's
   floor and 0.00001 ft, a gap under twice `tol` is `unresolved`. Otherwise Revit has a vertex
   within `tol` at P only (`p_only`), at Q only (`q_only`), at both (`both`), one strictly between
   them (within `tol` of the segment and farther than `tol` from both ends, `between`), or none
   of these (`none`). Per gap, the outcomes over its occurrences and whether they agree. The
   registered verdict per cohort: one end kept on every scored gap is the office's result;
   anything else is counted by outcome.
6. **Corners and sizes.** The scored gaps grouped by their two ends in world coordinates,
   either way round, so one corner seen from two faces counts once; then one line per cohort
   and gap size (one per corner with `--per-gap`).
7. **Raw outcomes.** The outcome of every scored occurrence read again from the placed raw
   points, against the mesh's; where Revit has a point between the ends, how far along the gap
   it lies, from P (0) to Q (1), by cohort: 0.5 is the gap's middle.

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
SCORED = ("p_only", "q_only", "both", "between", "none")
COHORTS = ("agree", "apart", "unread")


def point(text):
    return np.array([float(x) for x in text.split(",")])


def fields(line):
    return dict(token.split("=", 1) for token in line.split()[1:] if "=" in token)


def side(value):
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


def read_probe(path):
    pieces, gaps, joints = {}, [], []
    with open(path, errors="replace") as handle:
        for line in handle:
            if line.startswith("piece "):
                f = fields(line)
                key = (f["graph"], f["face"], f["loop"], f["at"].split("/")[0])
                a, b = side(f.get("sides_a")), side(f.get("sides_b"))
                split = "unread" if a is None or b is None else "agree" if max(a, b) < TOUCHING else "apart"
                pieces[key] = (float(f["gap"]), split)
            elif line.startswith("gapxyz "):
                gaps.append(fields(line))
            elif line.startswith("jointxyz "):
                joints.append(fields(line))
    return pieces, gaps, joints


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
                inner = m @ operator(item.MappingTarget) @ np.linalg.inv(axis2(source.MappingOrigin))
                stack.extend((x, inner) for x in source.MappedRepresentation.Items)
            else:
                out.append((item, m))
    return out


def local_points(item):
    """Revit's own coordinates of a polygonal item in its local frame, or None where the item is
    not made of polygons (a face of a polygon solid bounded by anything but an IfcPolyLoop)."""
    kind = item.is_a()
    if kind in FACESETS:
        return np.array(item.Coordinates.CoordList, dtype=float)
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
        return "between"
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


def score(gaps, bodies, faceted, floors, offset=None):
    rows = []
    for g in gaps:
        tag = g["owner"]
        if tag not in bodies:
            o = "no_revit_body"
        elif tag not in faceted:
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
    element's placement, each mapped item's origin and target, the file's unit and the site frame,
    into the frame of the probe's points, with no meshing between; None where an item is not
    polygonal."""
    import ifcopenshell.util.unit

    unit = ifcopenshell.util.unit.calculate_unit_scale(model)
    frame = site_frame(model)
    out = {}
    for tag in tags:
        pts = []
        for item, m in body_items(elements[tag]):
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

    def cohort(g):
        return pieces.get(key_of(g), (None, "not in the census"))[1]

    # 1. Row 0.
    equal = differ = unmatched = 0
    for g in gaps:
        key = key_of(g)
        if key not in pieces:
            unmatched += 1
        elif abs(pieces[key][0] - float(g["gap"])) <= 1e-9:
            equal += 1
        else:
            differ += 1
    verdict = "holds" if differ == 0 and unmatched == 0 else "FAILS"
    print(f"row 0 {verdict}: gap occurrences {len(gaps)}, equal to their census gap {equal}, different {differ}, not in the census {unmatched}")
    # 2. Coverage.
    reached = {key_of(g) for g in gaps}
    census = collections.Counter(s for _, s in pieces.values())
    hit = collections.Counter(pieces[k][1] for k in reached if k in pieces)
    print(f"coverage: census gaps {len(pieces)} ({counts(census)}); reached by an element at level 3 {len(reached)} ({counts(hit)})")
    tags = {g["owner"] for g in gaps}
    print(f"elements reaching a gap {len(tags)}; ring ends {len(joints)}")
    if not gaps:
        print("not exercised: no gap reached by an element's graphics")
        return 0
    # 3. Bodies.
    model = ifcopenshell.open(args[1])
    kinds = collections.defaultdict(collections.Counter)
    elements = {}
    for element in model.by_type("IfcElement"):
        if element.Tag in tags:
            elements[element.Tag] = element
            for item, _ in body_items(element):
                kinds[element.Tag][item.is_a()] += 1
    faceted = {t for t, c in kinds.items() if c and set(c) <= FACETED}
    print("Body item types: " + counts(collections.Counter(k for t in tags for k in kinds.get(t, {}))))

    def vertices(settings):
        got = meshes(args[1], tags=tags, model=model, settings=settings)
        return {t: np.unique(v[2].reshape(-1, 3), axis=0) for t, v in got.items()}

    welded_settings = ifcopenshell.geom.settings()
    welded_settings.set("use-world-coords", True)
    try:
        weld_default = welded_settings.get("weld-vertices")
    except Exception as e:  # the setting's name differs between IfcOpenShell releases
        weld_default = f"unread ({type(e).__name__})"
    welded = vertices(welded_settings)
    unwelded_settings = ifcopenshell.geom.settings()
    unwelded_settings.set("use-world-coords", True)
    try:
        unwelded_settings.set("weld-vertices", False)
        unwelded = vertices(unwelded_settings)
        weld_note = f"weld-vertices off (default {weld_default})"
    except Exception as e:
        unwelded = welded
        weld_note = f"weld-vertices could not be set ({type(e).__name__}); outcomes read from the default meshes"
    print(f"bodies: {weld_note}; elements with a Revit body by Tag {len(set(unwelded) & tags)} of {len(tags)}, faceted {len(faceted & set(unwelded))}")
    # 4. The floor, in the mesh and in the placed raw points, for every faceted element.
    scored_tags = sorted(set(unwelded) & faceted)
    floors = floors_of(unwelded, joints, scored_tags)
    raw = raw_points(model, elements, scored_tags)
    raw_floors = floors_of(raw, joints, sorted(set(raw) & set(floors)))
    placement = [t for t, f in floors.items() if f["floor"] >= PLACEMENT]
    usable = {t: f for t, f in floors.items() if f["floor"] < PLACEMENT}
    if floors:
        fl = sorted(f["floor"] for f in floors.values())
        within = sum(f["within"] * f["n"] for f in floors.values()) / sum(f["n"] for f in floors.values())
        print(f"mesh floor over {len(floors)} elements (0.01 ft or more on {len(placement)}): min {fl[0]:.9f} median {fl[len(fl) // 2]:.9f} max {fl[-1]:.9f} ft; closed ends within 0.00001 ft of a Revit vertex {within:.1%}")
    if raw_floors:
        rl = sorted(f["floor"] for f in raw_floors.values())
        apart = [t for t in raw_floors if raw_floors[t]["floor"] > max(2 * floors[t]["floor"], TOUCHING)]
        print(f"raw floor over {len(raw_floors)} elements: min {rl[0]:.9f} median {rl[len(rl) // 2]:.9f} max {rl[-1]:.9f} ft; elements whose raw floor exceeds twice the mesh floor and 0.00001 ft: {len(apart)}")
    print(f"raw points read on {len(raw)} of {len(scored_tags)} faceted elements")
    offset = None
    if usable:
        vecs = np.array([f["vector"] for f in usable.values()])
        common = np.median(vecs, axis=0)
        spread = float(np.abs(vecs - common).max())
        print(f"median offset vectors: common {common[0]:+.9f},{common[1]:+.9f},{common[2]:+.9f} ft, largest departure from it {spread:.9f} ft")
        offset = common if spread <= SAME_OFFSET and float(np.linalg.norm(common)) > SAME_OFFSET else None
        print("offset: one vector, taken off for a second reading" if offset is not None else "offset: no single nonzero vector; no second reading")
    # 5. Outcomes, by cohort.
    rows = score(gaps, unwelded, faceted, floors)
    per_gap = summary("unwelded", rows, cohort)
    if unwelded is not welded:
        summary("default meshes", score(gaps, welded, faceted, floors_of(welded, joints, sorted(set(welded) & faceted))), cohort)
    if offset is not None:
        shifted = floors_of(unwelded, joints, scored_tags, offset)
        rows = score(gaps, unwelded, faceted, shifted, offset)
        summary("offset taken off", rows, cohort)
        print(f"floor with the offset taken off: median {np.median([f['floor'] for f in shifted.values()]):.9f} ft")
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
    for ends, members in corners.items():
        size = (cohort(members[0][0]), round(float(members[0][0]["gap"]), 6))
        corners_by_size[size] += 1
        for _, o in members:
            by_size[size][o] += 1
        if per_gap_lines:
            print(f"corner {size[0]} gap {size[1]:.6f} ft: occurrences {len(members)}, " + counts(collections.Counter(o for _, o in members)))
    for c, size in sorted(by_size):
        print(f"gap size {c} {size:.6f} ft ({size * 304.8:.3f} mm): corners {corners_by_size[(c, size)]}, occurrences {counts(by_size[(c, size)])}")
    # 7. The same outcomes read from the placed raw points.
    pairs = collections.Counter()
    along = collections.defaultdict(list)
    for g, o in scored:
        tag = g["owner"]
        if tag not in raw:
            pairs[(o, "no_raw_points")] += 1
            continue
        tol = max(2 * floors[tag]["floor"], TOUCHING)
        verts = raw[tag] if offset is None else raw[tag] - offset
        p, q = point(g["p"]), point(g["q"])
        r = outcome(verts, float(g["gap"]), p, q, tol)
        pairs[(o, r)] += 1
        if r == "between":
            along[cohort(g)].append(between_at(verts, p, q, tol))
    print("raw points: mesh outcome -> raw outcome: " + (", ".join(f"{a}->{b} {n}" for (a, b), n in sorted(pairs.items())) or "none"))
    # Where the point between the ends lies, from P (0) to Q (1): 0.5 is the gap's middle.
    for c in COHORTS:
        if along.get(c):
            ts = sorted(along[c])
            print(f"raw points between, {c}: {len(ts)} drawings, along the gap from P (0) to Q (1): min {ts[0]:.3f} median {ts[len(ts) // 2]:.3f} max {ts[-1]:.3f}")
    if per_gap_lines:
        for key, c in sorted(per_gap.items()):
            print(f"gap graph={key[0]} face={key[1]} loop={key[2]} at={key[3]} cohort={pieces.get(key, (None, '-'))[1]} "
                  + " ".join(f"{k}={v}" for k, v in sorted(c.items())))
    return 0


if __name__ == "__main__":
    sys.exit(main())
