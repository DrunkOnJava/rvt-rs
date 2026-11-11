#!/usr/bin/env python3
"""Is a wall that Revit's export gives the full Qto_WallBaseQuantities set a vertical prism once its openings are filled?

Follows wall_set_vs_body_shape.py. For every wall Revit draws as one face set with no sloped face, each horizontal face strictly
between the body's lowest and highest point is tested against the wall's IfcOpeningElements: it is "explained" when an opening's box
(in the wall's own frame) spans the face along the wall and has its top or bottom at the face's height. A wall all of whose such faces
are explained is a prism less its openings.

Usage: wall_prism_after_openings.py <revit.ifc> [tolerance in file length units, default 2.0]
   or, as a Measure scorer: wall_prism_after_openings.py <rvt-rs.ifc> <revit.ifc> (the first is not read)

Ends with one JSON line per wall that has a quantity set (`rows:`): its Tag, the kind of its set and the class of its body.
"""
import collections
import math
import re
import sys

LINE = re.compile(r"^#(\d+)\s*=\s*([A-Z0-9_]+)\s*\((.*)\);\s*$")
AS_SCORER = len(sys.argv) > 2 and sys.argv[2].lower().endswith(".ifc")
REVIT = sys.argv[2] if AS_SCORER else sys.argv[1]
TOL = float(sys.argv[2]) if len(sys.argv) > 2 and not AS_SCORER else 2.0


def split_top(s):
    out, depth, cur, quote, i = [], 0, [], False, 0
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


def nums(s):
    return [float(x) for x in re.findall(r"-?\d+\.?\d*(?:[eE][-+]?\d+)?", s)]


SKIP = {"IFCPROPERTYSINGLEVALUE", "IFCPROPERTYSET", "IFCSTYLEDITEM", "IFCMEMBER", "IFCPLATE"}
ent = {}
for raw in open(REVIT, errors="replace"):
    m = LINE.match(raw)
    if m and m.group(2) not in SKIP:
        ent[int(m.group(1))] = (m.group(2), m.group(3))


def A(i):
    return split_top(ent[i][1])


def vec(i, default):
    return tuple(nums(ent[i][1])[:3]) + (0.0,) * (3 - len(nums(ent[i][1])[:3])) if i in ent else default


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def unit(a):
    n = math.sqrt(sum(x * x for x in a)) or 1.0
    return tuple(x / n for x in a)


def axis2(i):
    """(origin, x, y, z) of an IfcAxis2Placement3D."""
    a = A(i)
    o = vec(refs(a[0])[0], (0, 0, 0))
    z = unit(vec(refs(a[1])[0], (0, 0, 1))) if len(a) > 1 and refs(a[1]) else (0.0, 0.0, 1.0)
    xr = unit(vec(refs(a[2])[0], (1, 0, 0))) if len(a) > 2 and refs(a[2]) else (1.0, 0.0, 0.0)
    y = unit(cross(z, xr)); x = unit(cross(y, z))
    return o, x, y, z


def apply(f, p):
    o, x, y, z = f
    return tuple(o[k] + x[k] * p[0] + y[k] * p[1] + z[k] * p[2] for k in range(3))


def invert_apply(f, p):
    o, x, y, z = f
    d = tuple(p[k] - o[k] for k in range(3))
    return (sum(d[k] * x[k] for k in range(3)), sum(d[k] * y[k] for k in range(3)), sum(d[k] * z[k] for k in range(3)))


def world(pl, p):
    """A point of local placement pl in world coordinates."""
    while pl is not None and pl in ent and ent[pl][0] == "IFCLOCALPLACEMENT":
        a = A(pl)
        p = apply(axis2(refs(a[1])[0]), p)
        r = refs(a[0])
        pl = r[0] if r else None
    return p


def profile_points(pid):
    t, a = ent[pid]
    args = split_top(a)
    if t == "IFCRECTANGLEPROFILEDEF":
        pos, xd, yd = refs(args[2]), float(args[3]), float(args[4])
        o, dx = (0.0, 0.0), (1.0, 0.0)
        if pos:
            pa = A(pos[0])
            o = tuple(nums(ent[refs(pa[0])[0]][1])[:2])
            if len(pa) > 1 and refs(pa[1]):
                dx = tuple(nums(ent[refs(pa[1])[0]][1])[:2])
        n = math.hypot(*dx) or 1.0
        dx = (dx[0] / n, dx[1] / n); dy = (-dx[1], dx[0])
        return [(o[0] + sx * xd / 2 * dx[0] + sy * yd / 2 * dy[0], o[1] + sx * xd / 2 * dx[1] + sy * yd / 2 * dy[1]) for sx, sy in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
    if t in ("IFCARBITRARYCLOSEDPROFILEDEF", "IFCARBITRARYPROFILEDEFWITHVOIDS"):
        c = refs(args[2])[0]
        ct, ca = ent[c]
        if ct == "IFCPOLYLINE":
            return [tuple(nums(ent[p][1])[:2]) for p in refs(ca)]
        if ct == "IFCINDEXEDPOLYCURVE":
            pl = refs(split_top(ca)[0])[0]
            return [tuple(float(v) for v in p.split(",")) for p in re.findall(r"\(([-0-9.eE+,]+)\)", ent[pl][1])]
    return []


def body_items(pds):
    items = []
    for r in refs(A(pds)[2]):
        if r in ent and ent[r][0] == "IFCSHAPEREPRESENTATION" and A(r)[1].strip("'") == "Body":
            items += refs(A(r)[3])
    return items


def opening_box(op, wall_pl):
    """The opening's box in the wall's frame, or None."""
    a = A(op)
    pl, pds = refs(a[5]), refs(a[6])
    if not pl or not pds:
        return None
    pts = []
    for it in body_items(pds[0]):
        if it not in ent or ent[it][0] != "IFCEXTRUDEDAREASOLID":
            return None
        ia = A(it)
        prof = profile_points(refs(ia[0])[0])
        if not prof:
            return None
        frame = axis2(refs(ia[1])[0]) if refs(ia[1]) else ((0, 0, 0), (1, 0, 0), (0, 1, 0), (0, 0, 1))
        d = unit(vec(refs(ia[2])[0], (0, 0, 1))); depth = float(ia[3])
        for (x, y) in prof:
            for k in (0.0, depth):
                pts.append(world(pl[0], apply(frame, (x + d[0] * k, y + d[1] * k, d[2] * k))))
    if not pts:
        return None
    w0, wx, wy, wz = world(wall_pl, (0, 0, 0)), world(wall_pl, (1, 0, 0)), world(wall_pl, (0, 1, 0)), world(wall_pl, (0, 0, 1))
    wf = (w0, tuple(wx[k] - w0[k] for k in range(3)), tuple(wy[k] - w0[k] for k in range(3)), tuple(wz[k] - w0[k] for k in range(3)))
    loc = [invert_apply(wf, p) for p in pts]
    return tuple((min(p[k] for p in loc), max(p[k] for p in loc)) for k in range(3))


qsets, voids = {}, collections.defaultdict(list)
for i, (t, a) in ent.items():
    if t == "IFCRELDEFINESBYPROPERTIES":
        args = split_top(a)
        target = refs(args[5])
        if target and target[0] in ent and ent[target[0]][0] == "IFCELEMENTQUANTITY" and "Qto_WallBaseQuantities" in A(target[0])[2]:
            names = {A(q)[0].strip("'") for q in refs(A(target[0])[5]) if q in ent}
            for w in refs(args[4]):
                qsets[w] = names
    elif t == "IFCRELVOIDSELEMENT":
        args = split_top(a)
        voids[refs(args[4])[0]].append(refs(args[5])[0])


def normal(poly):
    nx = ny = nz = 0.0
    for k in range(len(poly)):
        x1, y1, z1 = poly[k]; x2, y2, z2 = poly[(k + 1) % len(poly)]
        nx += (y1 - y2) * (z1 + z2); ny += (z1 - z2) * (x1 + x2); nz += (x1 - x2) * (y1 + y2)
    n = math.sqrt(nx * nx + ny * ny + nz * nz)
    return (nx / n, ny / n, nz / n) if n > 0 else (0.0, 0.0, 0.0)


rows = []
others = []
for i, (t, a) in ent.items():
    if t not in ("IFCWALL", "IFCWALLSTANDARDCASE") or i not in qsets:
        continue
    args = split_top(a)
    kind = "full" if "GrossSideArea" in qsets[i] else "volume-only"
    items = body_items(refs(args[6])[0]) if refs(args[6]) else []
    if len(items) != 1 or ent[items[0]][0] != "IFCPOLYGONALFACESET":
        others.append({"tag": args[7].strip("'"), "kind": kind, "cls": "several-items" if len(items) != 1 else "other-body"})
        continue
    fa = A(items[0])
    pts = [tuple(float(v) for v in p.split(",")) for p in re.findall(r"\(([-0-9.eE+,]+)\)", ent[refs(fa[0])[0]][1])]
    polys = [[pts[int(x) - 1] for x in re.findall(r"\d+", split_top(ent[f][1])[0])] for f in refs(fa[2])]
    zs = [p[2] for poly in polys for p in poly]
    zmin, zmax = min(zs), max(zs)
    sloped, mids = 0, []
    for poly in polys:
        n = normal(poly)
        if abs(n[2]) > 0.999999:
            z = poly[0][2]
            if zmin + TOL < z < zmax - TOL:
                mids.append((z, min(p[0] for p in poly), max(p[0] for p in poly), min(p[1] for p in poly), max(p[1] for p in poly), n[2] > 0))
        elif abs(n[2]) > 1e-6:
            sloped += 1
    if sloped:
        others.append({"tag": args[7].strip("'"), "kind": kind, "cls": "sloped"})
        continue
    boxes = [b for b in (opening_box(o, refs(args[5])[0]) for o in voids.get(i, [])) if b]
    unexplained = 0
    for (z, x0, x1, y0, y1, up) in mids:
        ok = any(b[0][0] - TOL <= x0 and x1 <= b[0][1] + TOL and b[1][0] - TOL <= y0 and y1 <= b[1][1] + TOL
                 and (abs(z - b[2][0]) <= TOL or abs(z - b[2][1]) <= TOL) for b in boxes)
        unexplained += 0 if ok else 1
    xs = [p[0] for poly in polys for p in poly]
    top_reach = sum(1 for b in boxes if b[2][1] >= zmax - TOL)
    end_reach = sum(1 for b in boxes if b[0][0] <= min(xs) + TOL or b[0][1] >= max(xs) - TOL)
    rows.append(dict(tag=args[7].strip("'"), kind=kind, mids=len(mids), unexplained=unexplained, openings=len(voids.get(i, [])), boxes=len(boxes),
                     top_reach=top_reach, end_reach=end_reach, z=(round(zmin), round(zmax)), x=(round(min(xs)), round(max(xs))),
                     box=[tuple((round(lo), round(hi)) for lo, hi in b) for b in boxes][:2], mid=[(round(m[0]), round(m[1]), round(m[2]), "up" if m[5] else "down") for m in mids][:3]))

print("walls drawn as one face set with no sloped face:", len(rows), collections.Counter(r["kind"] for r in rows))
c = collections.Counter((("prism" if r["mids"] == 0 else "prism less its openings" if r["unexplained"] == 0 else "faces no opening explains"), r["kind"]) for r in rows)
for k in ("prism", "prism less its openings", "faces no opening explains"):
    print(f"  {k:28s} full {c[(k, 'full')]:4d}   volume-only {c[(k, 'volume-only')]:4d}")
c2 = collections.Counter((("opening reaches the top" if r["top_reach"] else "opening reaches an end" if r["end_reach"] else "openings enclosed"), r["kind"]) for r in rows if r["mids"] and not r["unexplained"])
print("prism less its openings, split:")
for k in ("openings enclosed", "opening reaches the top", "opening reaches an end"):
    print(f"  {k:28s} full {c2[(k, 'full')]:4d}   volume-only {c2[(k, 'volume-only')]:4d}")
print("openings whose box could not be read:", sum(r["openings"] - r["boxes"] for r in rows))
print("\nwalls with faces no opening explains, by kind (first 40)")
for r in [r for r in rows if r["unexplained"]][:40]:
    print("  ", r)
print("\nvolume-only walls among the first two classes")
for r in rows:
    if r["kind"] != "full" and r["unexplained"] == 0:
        print("  ", r)

import json, os

def row_class(r):
    return "prism" if r["mids"] == 0 else "less-openings" if r["unexplained"] == 0 else "unexplained"

print("\nrows:")
for r in rows:
    print(json.dumps({"tag": r["tag"], "kind": r["kind"], "cls": row_class(r), "openings": r["openings"]}))
for r in others:
    print(json.dumps(r))
if os.environ.get("ROWS_OUT"):
    with open(os.environ["ROWS_OUT"], "w") as handle:
        for r in rows:
            handle.write(json.dumps({"tag": r["tag"], "kind": r["kind"], "cls": "prism" if r["mids"] == 0 else "less-openings" if r["unexplained"] == 0 else "unexplained"}) + "\n")
