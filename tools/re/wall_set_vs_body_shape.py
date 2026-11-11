#!/usr/bin/env python3
"""Which walls Revit's export gives the full Qto_WallBaseQuantities set and which a NetVolume alone, against the SHAPE of Revit's body.

The reading tested (revit-ifc WallExporter at IFC_v24.1.0, read and cited, no code copied): a wall not exported by Revit's internal
extrusion routine still counts as a wall with an axis, and so gets the full set, only when its geometry is exactly one solid and the
generic body exporter found a vertical extrusion for it. So the number of solids (items of the Body representation) and whether the
solid is a vertical prism (every face vertical or horizontal) should separate the two kinds.

Usage: wall_set_vs_body_shape.py <revit.ifc>
"""
import collections
import math
import re
import sys

LINE = re.compile(r"^#(\d+)\s*=\s*([A-Z0-9_]+)\s*\((.*)\);\s*$")


def split_top(s):
    out, depth, cur, quote = [], 0, [], False
    i = 0
    while i < len(s):
        c = s[i]
        if quote:
            cur.append(c)
            if c == "'":
                if i + 1 < len(s) and s[i + 1] == "'":
                    cur.append("'"); i += 1
                else:
                    quote = False
        elif c == "'":
            quote = True; cur.append(c)
        elif c == "(":
            depth += 1; cur.append(c)
        elif c == ")":
            depth -= 1; cur.append(c)
        elif c == "," and depth == 0:
            out.append("".join(cur).strip()); cur = []
        else:
            cur.append(c)
        i += 1
    out.append("".join(cur).strip())
    return out


def refs(s):
    return [int(x) for x in re.findall(r"#(\d+)", s)]


WANT = {"IFCWALL", "IFCWALLSTANDARDCASE", "IFCRELDEFINESBYPROPERTIES", "IFCELEMENTQUANTITY", "IFCQUANTITYLENGTH", "IFCQUANTITYAREA",
        "IFCQUANTITYVOLUME", "IFCPRODUCTDEFINITIONSHAPE", "IFCSHAPEREPRESENTATION", "IFCPOLYGONALFACESET", "IFCTRIANGULATEDFACESET",
        "IFCCARTESIANPOINTLIST3D", "IFCINDEXEDPOLYGONALFACE", "IFCINDEXEDPOLYGONALFACEWITHVOIDS", "IFCEXTRUDEDAREASOLID",
        "IFCMAPPEDITEM", "IFCREPRESENTATIONMAP", "IFCRELVOIDSELEMENT", "IFCBOOLEANCLIPPINGRESULT", "IFCFACETEDBREP"}
ent = {}
for raw in open(sys.argv[1], errors="replace"):
    m = LINE.match(raw)
    if m and m.group(2) in WANT:
        ent[int(m.group(1))] = (m.group(2), m.group(3))

qsets = {}  # wall id -> {name: value}
for i, (t, a) in ent.items():
    if t != "IFCRELDEFINESBYPROPERTIES":
        continue
    args = split_top(a)
    target = refs(args[5])
    if not target or target[0] not in ent or ent[target[0]][0] != "IFCELEMENTQUANTITY":
        continue
    qa = split_top(ent[target[0]][1])
    if "Qto_WallBaseQuantities" not in qa[2]:
        continue
    vals = {}
    for q in refs(qa[5]):
        if q in ent:
            qq = split_top(ent[q][1])
            vals[qq[0].strip("'")] = float(qq[3])
    for w in refs(args[4]):
        qsets[w] = vals
voided = collections.Counter()
for i, (t, a) in ent.items():
    if t == "IFCRELVOIDSELEMENT":
        voided[refs(split_top(a)[4])[0]] += 1


def points(pid):
    a = ent[pid][1]
    return [tuple(float(x) for x in p.split(",")) for p in re.findall(r"\(([-0-9.eE+,]+)\)", a)]


def body_items(rep_id):
    """The Body items of a product definition shape, following one level of mapped items."""
    items = []
    for r in refs(split_top(ent[rep_id][1])[2]):
        if r not in ent or ent[r][0] != "IFCSHAPEREPRESENTATION":
            continue
        ra = split_top(ent[r][1])
        if ra[1].strip("'") != "Body":
            continue
        for it in refs(ra[3]):
            if it in ent and ent[it][0] == "IFCMAPPEDITEM":
                src = refs(split_top(ent[it][1])[0])[0]
                mapped = refs(split_top(ent[src][1])[1])[0] if src in ent else None
                if mapped in ent:
                    items += [x for x in refs(split_top(ent[mapped][1])[3])]
            else:
                items.append(it)
    return items


def faces_of(item):
    """(polygons as vertex lists, has-voids count) of a polygonal or triangulated face set."""
    t, a = ent[item]
    args = split_top(a)
    pts = points(refs(args[0])[0])
    polys, voids = [], 0
    if t == "IFCPOLYGONALFACESET":
        for f in refs(args[2]):
            ft, fa = ent[f]
            fargs = split_top(fa)
            polys.append([pts[int(x) - 1] for x in re.findall(r"\d+", fargs[0])])
            if ft == "IFCINDEXEDPOLYGONALFACEWITHVOIDS":
                voids += 1
    elif t == "IFCTRIANGULATEDFACESET":
        for tri in re.findall(r"\((\d+),(\d+),(\d+)\)", args[3]):
            polys.append([pts[int(x) - 1] for x in tri])
    return polys, voids


def normal(poly):
    nx = ny = nz = 0.0
    for k in range(len(poly)):
        x1, y1, z1 = poly[k]; x2, y2, z2 = poly[(k + 1) % len(poly)]
        nx += (y1 - y2) * (z1 + z2); ny += (z1 - z2) * (x1 + x2); nz += (x1 - x2) * (y1 + y2)
    n = math.sqrt(nx * nx + ny * ny + nz * nz)
    return (nx / n, ny / n, nz / n, n / 2) if n > 0 else (0, 0, 0, 0)


rows = []
for i, (t, a) in ent.items():
    if t not in ("IFCWALL", "IFCWALLSTANDARDCASE") or i not in qsets:
        continue
    args = split_top(a)
    q = qsets[i]
    kind = "full" if "GrossSideArea" in q else ("volume-only" if set(q) <= {"NetVolume", "Width", "Length", "GrossFootprintArea"} and "NetVolume" in q else "other:" + ",".join(sorted(q)))
    rep = refs(args[6])
    items = body_items(rep[0]) if rep and rep[0] in ent else []
    classes = collections.Counter(ent[x][0] for x in items if x in ent)
    feat = {"tag": args[7].strip("'"), "kind": kind, "items": len(items), "classes": "+".join(sorted(classes)), "voided": voided[i] > 0}
    meshes = [x for x in items if x in ent and ent[x][0] in ("IFCPOLYGONALFACESET", "IFCTRIANGULATEDFACESET")]
    if meshes:
        sloped = horiz = vert = 0
        levels, voids, vdirs = set(), 0, set()
        zmin, zmax = 1e99, -1e99
        for msh in meshes:
            polys, v = faces_of(msh)
            voids += v
            for p in polys:
                nx, ny, nz, area = normal(p)
                if area <= 0:
                    continue
                for (_, _, z) in p:
                    zmin, zmax = min(zmin, z), max(zmax, z)
                if abs(nz) > 0.999999:
                    horiz += 1; levels.add(round(p[0][2], 3))
                elif abs(nz) < 1e-6:
                    vert += 1; vdirs.add((round(abs(nx), 4), round(abs(ny), 4)))
                else:
                    sloped += 1
        mid = [z for z in levels if zmin + 1e-3 < z < zmax - 1e-3]
        feat.update(sloped=sloped, horiz=horiz, vert=vert, levels=len(levels), mid_levels=len(mid), face_voids=voids, vdirs=len(vdirs))
    rows.append(feat)

print("walls with a Qto_WallBaseQuantities:", len(rows))
print("\nby set kind and body classes (number of Body items)")
c = collections.Counter((r["kind"], r["classes"], r["items"]) for r in rows)
for k, v in sorted(c.items()):
    print(f"  {v:4d}  {k[0]:12s} {k[1]:40s} items={k[2]}")
mesh_rows = [r for r in rows if "sloped" in r]
print("\nface-set walls:", len(mesh_rows), collections.Counter(r["kind"] for r in mesh_rows))


def table(name, key):
    print(f"\n{name}")
    c = collections.Counter((key(r), r["kind"]) for r in mesh_rows)
    for k in sorted({k for k, _ in c}, key=str):
        print(f"  {str(k):28s} full {c[(k, 'full')]:4d}   volume-only {c[(k, 'volume-only')]:4d}")


table("one solid?", lambda r: "1 item" if r["items"] == 1 else f"{min(r['items'], 3)}{'+' if r['items'] >= 3 else ''} items")
table("any sloped face?", lambda r: "sloped" if r["sloped"] else "none sloped")
table("horizontal faces strictly between the lowest and highest point (levels)", lambda r: min(r["mid_levels"], 4))
table("vertical prism (one item, no sloped face, no horizontal face between bottom and top)", lambda r: r["items"] == 1 and not r["sloped"] and r["mid_levels"] == 0)
table("faces with voids", lambda r: min(r["face_voids"], 3))
table("distinct vertical face directions", lambda r: min(r["vdirs"], 4))
table("voided by an IfcOpeningElement", lambda r: r["voided"])

table("vertical faces (polygons) of a vertical prism", lambda r: ("prism", "4" if r["vert"] <= 4 else "5-6" if r["vert"] <= 6 else "7+") if (r["items"] == 1 and not r["sloped"] and r["mid_levels"] == 0) else ("not a prism", "4" if r["vert"] <= 4 else "5+"))
table("why not a prism", lambda r: "prism" if (r["items"] == 1 and not r["sloped"] and r["mid_levels"] == 0) else "+".join(x for x, on in (("several items", r["items"] > 1), ("sloped face", r["sloped"] > 0), ("mid-level horizontal face", r["mid_levels"] > 0)) if on))
table("one item, not sloped, mid-level faces, by voided", lambda r: ("n/a" if not (r["items"] == 1 and not r["sloped"] and r["mid_levels"] > 0) else ("voided" if r["voided"] else "not voided", "face voids" if r["face_voids"] else "no face voids")))
print("\nvolume-only walls, one per line")
for r in rows:
    if r["kind"] != "full":
        print("  ", {k: v for k, v in r.items()})
