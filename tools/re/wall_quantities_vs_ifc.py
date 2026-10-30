#!/usr/bin/env python3
"""What Revit's wall areas and volumes are, and whether a wall body gives them
(#1046, RE-270).

Revit's export writes a wall's `Qto_WallBaseQuantities` `GrossSideArea`,
`NetSideArea`, `GrossVolume` and `NetVolume` from the faces and the volume of
the wall's joined solid (revit-ifc's `PropertyUtil.CreateWallBaseQuantities`,
CLEANROOM.md source 5). This scorer meshes the Body of every `IfcWall` both
files hold under one `Tag` with IfcOpenShell (openings applied as it does by
default) and measures, in feet, square feet and cubic feet:

- the volume of the mesh;
- its vertical faces: connected sets of coplanar vertical triangles, each with
  its area (holes taken out) and the area inside its outer boundary (holes
  filled);
- variant A (revit-ifc 24.1 and 24.2): the largest single face's area is the
  net side area, the largest outer boundary the gross side area;
- variant B (revit-ifc 23.4, 25 and later): the faces are grouped by normal,
  the group with the most area gives the net side area, and the outer
  boundaries of its faces added give the gross side area;
- the area of the bottom faces seen from above.

For each quantity Revit writes on a wall it counts, for Revit's own body and
for rvt-rs's, how many walls each candidate value equals within a relative
1e-5 (or 1e-6 SI), and how many are within 1e-3. Revit's own `Width` is used
where a candidate needs one. It prints the counts by what rvt-rs writes on the
wall (a `Height`, a `Length`) and by its voids, then one line per wall:

  wall tag:<Tag> Q <Revit's quantities> | R <Revit's body> | U <rvt-rs's body>
       | same <bodies equal> kinds <Revit's body items> / <rvt-rs's>

Usage:

    python3 tools/re/wall_quantities_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>

Needs IfcOpenShell, numpy and shapely (the versions tools/ci/requirements-ifc.txt pins).
"""

import collections
import os
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element
import numpy as np
import shapely

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import qtos_vs_ifc  # noqa: E402
import wall_heights_vs_ifc as base  # noqa: E402

SET = "Qto_WallBaseQuantities"
FOOT = 0.3048
DIMENSION = {
    "Height": 1,
    "Length": 1,
    "Width": 1,
    "GrossFootprintArea": 2,
    "GrossSideArea": 2,
    "NetSideArea": 2,
    "GrossVolume": 3,
    "NetVolume": 3,
}
SHOWN = ("Height", "Length", "Width", "GrossFootprintArea", "GrossSideArea", "NetSideArea", "GrossVolume", "NetVolume")
SHORT = {
    "Height": "H",
    "Length": "L",
    "Width": "W",
    "GrossFootprintArea": "GFA",
    "GrossSideArea": "GSA",
    "NetSideArea": "NSA",
    "GrossVolume": "GV",
    "NetVolume": "NV",
}
COSINE = 1 - 1e-7


def equal(a, b, dimension, relative=1e-5):
    if a is None or b is None:
        return False
    return abs(a - b) <= max(1e-6 / FOOT**dimension, relative * max(abs(a), abs(b)))


def triangles(path, product, settings):
    geometry = base.body_geometry(settings, product, path)
    if geometry is None or not len(geometry.verts):
        return None
    points = np.asarray(geometry.verts, dtype=float).reshape(-1, 3) / FOOT
    faces = np.asarray(geometry.faces, dtype=int).reshape(-1, 3)
    return points[faces]


def analyse(tris):
    """Volume, extent and vertical faces of a closed triangle mesh, in feet."""
    centre = tris.reshape(-1, 3).mean(axis=0)
    shifted = tris - centre
    a, b, c = shifted[:, 0], shifted[:, 1], shifted[:, 2]
    normal = np.cross(b - a, c - a)
    twice = np.linalg.norm(normal, axis=1)
    live = twice > 1e-12
    unit = np.zeros_like(normal)
    unit[live] = normal[live] / twice[live, None]
    volume = abs(np.einsum("ij,ij->i", a, np.cross(b, c)).sum()) / 6
    down = live & (unit[:, 2] < -0.999)
    plan = float((twice[down] / 2 * -unit[down, 2]).sum())
    vertical = [int(i) for i in np.where(live & (np.abs(unit[:, 2]) < 1e-3))[0]]
    parent = {i: i for i in vertical}

    def find(i):
        while parent[i] != i:
            parent[i] = parent[parent[i]]
            i = parent[i]
        return i

    edges = {}
    rounded = np.round(tris * 1e4).astype(np.int64)
    for i in vertical:
        keys = [tuple(rounded[i, k]) for k in range(3)]
        for p, q in ((0, 1), (1, 2), (2, 0)):
            edge = (min(keys[p], keys[q]), max(keys[p], keys[q]))
            edges.setdefault(edge, []).append(i)
    for members in edges.values():
        for j in members[1:]:
            if float(unit[members[0]] @ unit[j]) > COSINE:
                parent[find(j)] = find(members[0])
    groups = collections.defaultdict(list)
    for i in vertical:
        groups[find(i)].append(i)
    faces = []
    for members in groups.values():
        nx, ny = float(unit[members[0], 0]), float(unit[members[0], 1])
        area = float(twice[members].sum() / 2)
        u = -ny * shifted[members, :, 0] + nx * shifted[members, :, 1]
        v = tris[members, :, 2]
        polygons = shapely.polygons(shapely.linearrings(np.stack([u, v], axis=-1)))
        polygons = polygons[shapely.area(polygons) > 1e-12]
        outer, holes = area, 0
        if len(polygons):
            merged = shapely.union_all(polygons)
            parts = [merged] if merged.geom_type == "Polygon" else list(getattr(merged, "geoms", []))
            outer = sum(shapely.Polygon(p.exterior).area for p in parts if p.geom_type == "Polygon")
            holes = sum(1 for p in parts if p.geom_type == "Polygon" for r in p.interiors if shapely.Polygon(r).area > 1e-6)
        faces.append({"n": (nx, ny), "area": area, "outer": float(outer), "holes": holes})
    out = {
        "volume": float(volume),
        "low": float(tris[:, :, 2].min()),
        "high": float(tris[:, :, 2].max()),
        "plan": plan,
        "faces": len(faces),
        "holes": sum(f["holes"] for f in faces),
        "A_net": max((f["area"] for f in faces), default=None),
        "A_gross": max((f["outer"] for f in faces), default=None),
        "B_net": None,
        "B_gross": None,
    }
    by_normal = collections.defaultdict(list)
    for f in faces:
        by_normal[(round(f["n"][0], 3), round(f["n"][1], 3))].append(f)
    if by_normal:
        best = max(by_normal.values(), key=lambda fs: sum(f["area"] for f in fs))
        out["B_net"] = sum(f["area"] for f in best)
        out["B_gross"] = sum(f["outer"] for f in best)
    out["tops"] = sorted((round(f["area"], 3) for f in faces), reverse=True)[:3]
    return out


def body_kinds(wall):
    if wall.Representation is None:
        return "-"
    kinds = set()
    for rep in wall.Representation.Representations:
        if rep.RepresentationIdentifier == "Body":
            kinds.add((rep.RepresentationType or "-") + ":" + "+".join(sorted({i.is_a() for i in rep.Items})))
    return "/".join(sorted(kinds)) or "-"


def profile_shape(wall):
    """The profile class of the wall's extrusion, with its corner count."""
    if wall.Representation is None:
        return "-"
    shapes = []
    for rep in wall.Representation.Representations:
        if rep.RepresentationIdentifier != "Body":
            continue
        for item in rep.Items:
            if not item.is_a("IfcExtrudedAreaSolid"):
                continue
            area = item.SweptArea
            if area.is_a("IfcArbitraryClosedProfileDef"):
                curve = area.OuterCurve
                if curve.is_a("IfcPolyline"):
                    shapes.append(f"poly{len(curve.Points) - 1}")
                else:
                    shapes.append("curve")
            else:
                shapes.append(area.is_a().replace("Ifc", "").replace("ProfileDef", ""))
    return "+".join(shapes) or "-"


def collect(path):
    f = qtos_vs_ifc.open_ifc(path)
    quantities = qtos_vs_ifc.quantities(path)[0]
    per = collections.defaultdict(dict)
    for (key, qset, name), (_, value) in quantities.items():
        if qset == SET and name in DIMENSION and value is not None:
            per[key][name] = value / FOOT ** DIMENSION[name]
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out = {}
    for wall in f.by_type("IfcWall"):
        if not wall.Tag or wall.Representation is None:
            continue
        tag = str(wall.Tag)
        if tag in out:
            continue
        tris = triangles(path, wall, settings)
        source = "-"
        top = "-"
        for props in ifcopenshell.util.element.get_psets(wall).values():
            if "BodySource" in props:
                source = str(props["BodySource"])
            if "BodyHeightSource" in props:
                top = str(props["BodyHeightSource"]) + ":" + str(props.get("TopCoverElementId", "-"))
        out[tag] = {
            "q": per.get(f"tag:{tag}", {}),
            "kinds": body_kinds(wall),
            "voids": len(wall.HasOpenings or ()),
            "source": source,
            "top": top,
            "shape": profile_shape(wall),
            "mesh": analyse(tris) if tris is not None else None,
        }
    return out


def fmt(value, digits=6):
    return "-" if value is None else f"{value:.{digits}f}"


def candidates(mesh, q):
    width = q.get("Width")
    gross_a, gross_b = mesh["A_gross"], mesh["B_gross"]
    return {
        "NetVolume": {"volume": mesh["volume"]},
        "GrossVolume": {
            "W x A gross": None if width is None or gross_a is None else width * gross_a,
            "W x B gross": None if width is None or gross_b is None else width * gross_b,
            "volume": mesh["volume"],
        },
        "GrossSideArea": {"A gross": gross_a, "B gross": gross_b, "A net": mesh["A_net"]},
        "NetSideArea": {"A net": mesh["A_net"], "B net": mesh["B_net"], "A gross": gross_a},
        "GrossFootprintArea": {
            "plan": mesh["plan"],
            "Length x Width": None if "Length" not in q or width is None else q["Length"] * width,
        },
        "Height": {"span": mesh["high"] - mesh["low"]},
    }


def main():
    args = sys.argv[1:]
    if len(args) != 2:
        print(__doc__)
        return 2
    ours, revit = collect(args[0]), collect(args[1])
    both = sorted(t for t in set(ours) & set(revit) if revit[t]["q"])
    print(f"walls with {SET} in Revit's export that rvt-rs also writes: {len(both)} (rvt-rs {len(ours)}, Revit {len(revit)})")
    unmeshed = collections.Counter(("rvt-rs" if t in ours and ours[t]["mesh"] is None else "") + ("Revit" if revit.get(t, {}).get("mesh") is None else "") for t in both)
    print(f"walls without a mesh: {dict(unmeshed)}")
    for quantity in ("NetVolume", "GrossVolume", "GrossSideArea", "NetSideArea", "GrossFootprintArea", "Height"):
        for side, files in (("Revit", revit), ("rvt-rs", ours)):
            tally = collections.defaultdict(lambda: [0, 0, 0])
            for tag in both:
                q = revit[tag]["q"]
                mesh = files[tag]["mesh"] if tag in files else None
                if quantity not in q or mesh is None:
                    continue
                for name, value in candidates(mesh, q)[quantity].items():
                    row = tally[name]
                    row[2] += 1
                    row[0] += equal(value, q[quantity], DIMENSION[quantity])
                    row[1] += equal(value, q[quantity], DIMENSION[quantity], 1e-3)
            for name, (exact, near, total) in tally.items():
                print(f"{quantity} = {side}'s {name}: equal {exact} of {total}, within 1e-3 {near}")
    sections = collections.Counter()
    for tag in both:
        q, u, r = revit[tag]["q"], ours[tag], revit[tag]
        wrote = "Height" if "Height" in u["q"] else "-"
        wrote += "+Length" if "Length" in u["q"] else ""
        for quantity, best in (("NetVolume", "volume"), ("GrossSideArea", "A gross"), ("NetSideArea", "A net"), ("GrossVolume", "W x A gross")):
            if quantity not in q or u["mesh"] is None:
                continue
            value = candidates(u["mesh"], q)[quantity][best]
            sections[(quantity, f"rvt-rs writes {wrote}", f"voids {'some' if u['voids'] else 'none'}", equal(value, q[quantity], DIMENSION[quantity]))] += 1
    for key, count in sorted(sections.items(), key=str):
        print(f"by rvt-rs's wall: {key[0]} | {key[1]} | {key[2]} | rvt-rs's best candidate equal {key[3]}: {count}")
    for tag in both:
        q, u, r = revit[tag]["q"], ours[tag], revit[tag]["mesh"]
        mine = u["mesh"]
        row = " ".join(f"{SHORT[name]}={fmt(q.get(name))}" for name in SHOWN)
        revit_body = "R none"
        if r is not None:
            revit_body = (
                f"R V={fmt(r['volume'])} lo={fmt(r['low'])} hi={fmt(r['high'])} H={fmt(r['high'] - r['low'])} plan={fmt(r['plan'])} faces={r['faces']} holes={r['holes']} "
                f"A={fmt(r['A_net'])}/{fmt(r['A_gross'])} B={fmt(r['B_net'])}/{fmt(r['B_gross'])} top={r['tops']}"
            )
        our_body = "U none"
        if mine is not None:
            our_body = (
                f"U V={fmt(mine['volume'])} lo={fmt(mine['low'])} hi={fmt(mine['high'])} H={fmt(mine['high'] - mine['low'])} plan={fmt(mine['plan'])} faces={mine['faces']} holes={mine['holes']} "
                f"A={fmt(mine['A_net'])}/{fmt(mine['A_gross'])} B={fmt(mine['B_net'])}/{fmt(mine['B_gross'])} top={mine['tops']}"
            )
        same = "-"
        if r is not None and mine is not None:
            same = "".join(
                "1" if equal(x, y, d, 1e-4) else "0"
                for x, y, d in (
                    (r["volume"], mine["volume"], 3),
                    (r["high"] - r["low"], mine["high"] - mine["low"], 1),
                    (r["plan"], mine["plan"], 2),
                )
            )
        ours_q = " ".join(f"{SHORT[name]}={fmt(u['q'].get(name))}" for name in SHOWN if name in u["q"])
        print(
            f"wall tag:{tag} Q {row} | {revit_body} | {our_body} | same(V,H,plan) {same} | rvt-rs wrote [{ours_q}] "
            f"voids R{revit[tag]['voids']} U{u['voids']} source {u['source']} top {u['top']} shape {u['shape']} "
            f"kinds {revit[tag]['kinds']} / {u['kinds']}"
        )
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    f = qtos_vs_ifc.open_ifc(args[0])
    for kind in base.CUTTERS:
        for product in f.by_type(kind):
            if not product.Tag or product.Representation is None:
                continue
            tris = triangles(args[0], product, settings)
            if tris is not None:
                print(f"cutter tag:{product.Tag} {kind} lo={fmt(tris[:, :, 2].min())} hi={fmt(tris[:, :, 2].max())}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
