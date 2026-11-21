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
   as the census splits them: both edges beside the gap placed alike by their two faces
   (`sides_a` and `sides_b` both under 0.00001 ft), or one of them placed apart.
3. **Bodies.** Revit's vertices are compared only where the Tag's body is faceted (an
   `IfcPolygonalFaceSet` or `IfcTriangulatedFaceSet`, alone or mapped). They are taken twice
   from IfcOpenShell: as it meshes by default, and with `weld-vertices` off where the setting
   exists. The outcomes are read from the unwelded vertices, so a point IfcOpenShell merged
   cannot pass for one Revit merged; both are printed.
4. **The floor and the offset.** For each element, every ring end that closes (`at_gap=false`)
   to Revit's nearest vertex: the median distance (its floor), the median vector, and the share
   of ends within 0.00001 ft. Where every element's median vector is one vector (within
   0.000001 ft) and is not zero, it is a frame offset, and the outcomes are read a second time
   with it taken off Revit's vertices. An element whose floor is 0.01 ft or more is a placement
   mismatch, counted apart.
5. **Outcomes, per occurrence.** With `tol` the larger of twice the element's floor and
   0.00001 ft, a gap under twice `tol` is `unresolved`. Otherwise Revit has a vertex within
   `tol` at P only (`p_only`), at Q only (`q_only`), at both (`both`), one strictly between them
   (within `tol` of the segment and farther than `tol` from both ends, `between`), or none of
   these (`none`). Per gap (graph, face, loop and position), the outcomes over its occurrences
   and whether they agree.
6. **Corners.** The scored gaps grouped by their two ends in world coordinates, either way
   round, so one corner seen from the two faces that meet there counts once; with the elements
   and graphs they are on.
7. **Raw points, placed.** Revit's own coordinate lists of each face set, before any meshing,
   placed through the element's placement, each mapped item's origin and target, the file's
   unit and the site frame into the frame P and Q are in, and read by the same outcome rules:
   the mesh's outcome against the raw points' for every scored occurrence. The ring ends that
   close give the placed points' floor, which says whether the placement was read right.

Needs IfcOpenShell and numpy.

    python3 tools/re/boundary_gaps_vs_ifc.py <probe output> <revit.ifc> [--per-gap]
"""

import collections
import math
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ifc_common import site_frame  # noqa: E402
from mesh_bodies_vs_ifc import meshes  # noqa: E402

FACESETS = {"IfcPolygonalFaceSet", "IfcTriangulatedFaceSet"}
TOUCHING = 1e-5
SAME_OFFSET = 1e-6
PLACEMENT = 0.01
SCORED = ("p_only", "q_only", "both", "between", "none")


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


def items_of(element):
    out = []
    if element.Representation is None:
        return out
    for rep in element.Representation.Representations:
        stack = list(rep.Items)
        while stack:
            item = stack.pop()
            if item.is_a("IfcMappedItem"):
                stack.extend(item.MappingSource.MappedRepresentation.Items)
            else:
                out.append(item)
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


def placed_items(element, top):
    """Each geometric item of an element's representations with the matrix that places it."""
    out = []
    if element.Representation is None:
        return out
    stack = [(item, top) for rep in element.Representation.Representations for item in rep.Items]
    while stack:
        item, m = stack.pop()
        if item.is_a("IfcMappedItem"):
            source = item.MappingSource
            inner = m @ operator(item.MappingTarget) @ np.linalg.inv(axis2(source.MappingOrigin))
            stack.extend((x, inner) for x in source.MappedRepresentation.Items)
        else:
            out.append((item, m))
    return out


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


def summary(label, rows):
    counts = collections.Counter(o for _, o in rows)
    per_gap = collections.defaultdict(collections.Counter)
    for g, o in rows:
        per_gap[(g["graph"], g["face"], g["loop"], g["at"])][o] += 1
    agree = sum(1 for c in per_gap.values() if len(c) == 1)
    by_gap = collections.Counter(c.most_common(1)[0][0] for c in per_gap.values())
    print(f"{label}: occurrences " + ", ".join(f"{k} {v}" for k, v in sorted(counts.items())))
    print(f"{label}: gaps {len(per_gap)} (occurrences all agree on {agree}); by outcome " + ", ".join(f"{k} {v}" for k, v in sorted(by_gap.items())))
    return per_gap


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    per_gap_lines = "--per-gap" in sys.argv
    if len(args) < 2:
        print(__doc__)
        return 2
    import ifcopenshell
    import ifcopenshell.geom
    import ifcopenshell.util.unit

    pieces, gaps, joints = read_probe(args[0])
    # 1. Row 0.
    equal = differ = unmatched = 0
    for g in gaps:
        key = (g["graph"], g["face"], g["loop"], g["at"])
        if key not in pieces:
            unmatched += 1
        elif abs(pieces[key][0] - float(g["gap"])) <= 1e-9:
            equal += 1
        else:
            differ += 1
    verdict = "holds" if differ == 0 and unmatched == 0 else "FAILS"
    print(f"row 0 {verdict}: gap occurrences {len(gaps)}, equal to their census gap {equal}, different {differ}, not in the census {unmatched}")
    # 2. Coverage.
    reached = {(g["graph"], g["face"], g["loop"], g["at"]) for g in gaps}
    census = collections.Counter(s for _, s in pieces.values())
    hit = collections.Counter(pieces[k][1] for k in reached if k in pieces)
    print(f"coverage: census gaps {len(pieces)} (" + ", ".join(f"{k} {v}" for k, v in sorted(census.items()))
          + f"); reached by an element at level 3 {len(reached)} (" + ", ".join(f"{k} {v}" for k, v in sorted(hit.items())) + ")")
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
            for item in items_of(element):
                kinds[element.Tag][item.is_a()] += 1
    faceted = {t for t, c in kinds.items() if c and set(c) <= FACESETS}
    census_items = collections.Counter(k for t in tags for k in kinds.get(t, {}))
    print("item types: " + ", ".join(f"{k} {v}" for k, v in sorted(census_items.items())))

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
    # 4. The floor and the offset.
    scored_tags = sorted(set(unwelded) & faceted)
    floors = floors_of(unwelded, joints, scored_tags)
    placement = [t for t, f in floors.items() if f["floor"] >= PLACEMENT]
    usable = {t: f for t, f in floors.items() if f["floor"] < PLACEMENT}
    if usable:
        fl = sorted(f["floor"] for f in usable.values())
        within = sum(f["within"] * f["n"] for f in usable.values()) / sum(f["n"] for f in usable.values())
        print(f"floor over {len(usable)} elements (placement mismatches apart: {len(placement)}): min {fl[0]:.9f} median {fl[len(fl) // 2]:.9f} max {fl[-1]:.9f} ft; closed ends within 0.00001 ft of a Revit vertex {within:.1%}")
        vecs = np.array([f["vector"] for f in usable.values()])
        common = np.median(vecs, axis=0)
        spread = float(np.abs(vecs - common).max())
        print(f"median offset vectors: common {common[0]:+.9f},{common[1]:+.9f},{common[2]:+.9f} ft, largest departure from it {spread:.9f} ft")
        offset = common if spread <= SAME_OFFSET and float(np.linalg.norm(common)) > SAME_OFFSET else None
        print("offset: one vector, taken off for a second reading" if offset is not None else "offset: no single nonzero vector; no second reading")
    else:
        offset = None
    # 5. Outcomes.
    rows = score(gaps, unwelded, faceted, floors)
    per_gap = summary("unwelded", rows)
    if unwelded is not welded:
        summary("default meshes", score(gaps, welded, faceted, floors_of(welded, joints, sorted(set(welded) & faceted))))
    if offset is not None:
        shifted = floors_of(unwelded, joints, scored_tags, offset)
        rows_shifted = score(gaps, unwelded, faceted, shifted, offset)
        summary("offset taken off", rows_shifted)
        print(f"floor with the offset taken off: median {np.median([f['floor'] for f in shifted.values()]):.9f} ft")
        # With a frame offset the reading without it is the floor's artefact, so the corners
        # and raw points below follow the reading with it taken off.
        rows = rows_shifted
    # 6. Corners.
    scored = [(g, o) for g, o in rows if o in SCORED]
    corners = collections.defaultdict(list)
    for g, o in scored:
        ends = sorted([tuple(np.round(point(g["p"]), 6)), tuple(np.round(point(g["q"]), 6))])
        corners[tuple(ends)].append((g, o))
    print(f"scored occurrences {len(scored)}, distinct gaps {len({(g['graph'], g['face'], g['loop'], g['at']) for g, _ in scored})}, "
          f"corners {len(corners)}, elements {len({g['owner'] for g, _ in scored})}, graphs {len({g['graph'] for g, _ in scored})}")
    for ends, members in sorted(corners.items(), key=lambda kv: float(kv[1][0][0]["gap"])):
        size = float(members[0][0]["gap"])
        outs = collections.Counter(o for _, o in members)
        print(f"corner gap {size:.6f} ft ({size * 304.8:.3f} mm): occurrences {len(members)}, " + ", ".join(f"{k} {v}" for k, v in sorted(outs.items())))
    # 7. Raw points, placed: Revit's own coordinate lists, through the element's placement,
    # each mapped item's origin and target, the file's unit and the site frame, into the frame
    # P and Q are in, with no meshing between. The ring ends that close give their floor; a
    # transform this reads wrong shows there, and then nothing below is trusted.
    import ifcopenshell.util.placement

    unit = ifcopenshell.util.unit.calculate_unit_scale(model)
    frame = site_frame(model)
    raw_rows = []
    raw_floors = []
    for tag in sorted({g["owner"] for g, _ in scored}):
        element = elements[tag]
        top = np.array(ifcopenshell.util.placement.get_local_placement(element.ObjectPlacement), dtype=float)
        pts = []
        for item, m in placed_items(element, top):
            if item.is_a() in FACESETS:
                local = np.array(item.Coordinates.CoordList, dtype=float)
                world = (m[:3, :3] @ local.T).T + m[:3, 3]
                pts.append(world * unit)
        if not pts:
            continue
        raw = frame(np.unique(np.concatenate(pts), axis=0))
        closed = [point(j["x"]) for j in joints if j["owner"] == tag and j["at_gap"] == "false"]
        raw_floors.extend(float(np.linalg.norm(raw - c, axis=1).min()) for c in closed)
        tol = max(2 * floors[tag]["floor"], TOUCHING) if tag in floors else TOUCHING
        for g, o in scored:
            if g["owner"] == tag:
                raw_rows.append((o, outcome(raw, float(g["gap"]), point(g["p"]), point(g["q"]), tol)))
    if raw_floors:
        rf = sorted(raw_floors)
        print(f"raw points placed: closed ring ends to the nearest raw point, median {rf[len(rf) // 2]:.9f} max {rf[-1]:.9f} ft over {len(rf)} ends")
    pairs = collections.Counter(raw_rows)
    print("raw points: mesh outcome -> raw outcome: " + ", ".join(f"{a}->{b} {n}" for (a, b), n in sorted(pairs.items())))
    if per_gap_lines:
        for key, c in sorted(per_gap.items()):
            print(f"gap graph={key[0]} face={key[1]} loop={key[2]} at={key[3]} " + " ".join(f"{k}={v}" for k, v in sorted(c.items())))
    return 0


if __name__ == "__main__":
    sys.exit(main())
