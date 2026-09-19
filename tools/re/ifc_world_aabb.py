#!/usr/bin/env python3
"""World axis-aligned bounding boxes out of an IFC4 STEP file, no dependencies.

Research tool for `reports/element-framing/` (RE-28). IfcOpenShell is
not always installable in a checkout, and the RE reports need exactly
one number per element: the world AABB of its `Body` (or `Axis`)
representation, keyed by the `Tag` attribute that carries the source
Revit ElementId.

It handles the entity set Revit's own IFC4 exporter and rvt-rs's STEP
writer actually emit on the recorded edge — `IfcLocalPlacement` /
`IfcAxis2Placement3D` chains, `IfcMappedItem` over
`IfcRepresentationMap`, `IfcExtrudedAreaSolid` over
`IfcArbitraryClosedProfileDef` (+`WithVoids`) / `IfcRectangleProfileDef`
/ `IfcCircleProfileDef` on `IfcIndexedPolyCurve` or `IfcPolyline`, and
`IfcPolygonalFaceSet` / `IfcTriangulatedFaceSet` — and raises on
anything else rather than silently returning a smaller box.

Cross-check: on `IFC Exports/2024_Core_Interior_slim.ifc` it reproduces
RE-26 §2.2's column plan-extent histogram (176 / 38 / 20 / 18 / 4) and
§4.3's wall exactness ladder (39 untrimmed, 336 trimmed) exactly, which
is what says it reads the same geometry IfcOpenShell 0.8.5 did.

Usage:

    python3 tools/re/ifc_world_aabb.py <file.ifc> [--type IFCWALL] \
        [--representation Body] [--scale 1.0]

prints one `Tag min_x min_y min_z max_x max_y max_z` row per element.
`--scale` multiplies every coordinate, which is how a metre-unit
rvt-rs export is compared against a foot-unit Revit export
(`--scale 3.280839895013123`).
"""

import re
import math

NUM = re.compile(r"^[-+0-9.]")


def tokenize_args(s):
    """Split a STEP argument string at top-level commas."""
    out, depth, cur, instr = [], 0, [], False
    i = 0
    while i < len(s):
        c = s[i]
        if instr:
            cur.append(c)
            if c == "'":
                if i + 1 < len(s) and s[i + 1] == "'":
                    cur.append("'")
                    i += 1
                else:
                    instr = False
        elif c == "'":
            instr = True
            cur.append(c)
        elif c == "(":
            depth += 1
            cur.append(c)
        elif c == ")":
            depth -= 1
            cur.append(c)
        elif c == "," and depth == 0:
            out.append("".join(cur).strip())
            cur = []
        else:
            cur.append(c)
        i += 1
    out.append("".join(cur).strip())
    return out


class Model:
    def __init__(self, path):
        self.ent = {}
        text = open(path, "r", encoding="utf-8", errors="replace").read()
        data = text[text.index("DATA;") + 5 : text.rindex("ENDSEC;")]
        # entities are '#id=TYPE(args);' possibly spanning lines
        for m in re.finditer(r"#(\d+)\s*=\s*([A-Z0-9_]+)\s*\((.*?)\);\s*(?=#|$)", data, re.S):
            self.ent[int(m.group(1))] = (m.group(2), m.group(3))
        self._cache = {}

    def type_of(self, ref):
        e = self.ent.get(ref)
        return e[0] if e else None

    def args(self, ref):
        e = self.ent[ref]
        return tokenize_args(e[1])

    def by_type(self, name):
        return [r for r, (t, _) in self.ent.items() if t == name]


def ref(tok):
    tok = tok.strip()
    if tok.startswith("#"):
        return int(tok[1:])
    return None


def reflist(tok):
    tok = tok.strip()
    if not tok.startswith("("):
        return []
    return [ref(x) for x in tokenize_args(tok[1:-1]) if x.strip().startswith("#")]


def num(tok):
    tok = tok.strip()
    if tok in ("$", "*"):
        return None
    return float(tok)


def numlist(tok):
    tok = tok.strip()
    if not tok.startswith("("):
        return []
    return [num(x) for x in tokenize_args(tok[1:-1])]


def nested_numlist(tok):
    tok = tok.strip()
    return [numlist(x) for x in tokenize_args(tok[1:-1])]


def nested_intlist(tok):
    tok = tok.strip()
    return [[int(v) for v in numlist(x)] for x in tokenize_args(tok[1:-1])]


IDENT = ((1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (0.0, 0.0, 1.0), (0.0, 0.0, 0.0))


def mat_mul(a, b):
    """a ∘ b : apply b first, then a."""
    ax, ay, az, ao = a
    bx, by, bz, bo = b
    def tx(v):
        return tuple(ax[i] * v[0] + ay[i] * v[1] + az[i] * v[2] for i in range(3))
    nx, ny, nz = tx(bx), tx(by), tx(bz)
    bt = tx(bo)
    no = tuple(ao[i] + bt[i] for i in range(3))
    return (nx, ny, nz, no)


def apply(m, p):
    x, y, z, o = m
    return tuple(o[i] + x[i] * p[0] + y[i] * p[1] + z[i] * p[2] for i in range(3))


def normalize(v):
    n = math.sqrt(sum(c * c for c in v))
    return tuple(c / n for c in v)


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


class Geom:
    def __init__(self, model):
        self.m = model

    def point(self, r):
        return tuple(numlist(self.m.args(r)[0]))

    def direction(self, r):
        return tuple(numlist(self.m.args(r)[0]))

    def placement3d(self, r):
        """IfcAxis2Placement3D -> 4-tuple matrix."""
        a = self.m.args(r)
        o = self.point(ref(a[0]))
        if len(o) == 2:
            o = (o[0], o[1], 0.0)
        z = (0.0, 0.0, 1.0)
        x = (1.0, 0.0, 0.0)
        if a[1].strip() != "$":
            z = normalize(self.direction(ref(a[1])))
        if a[2].strip() != "$":
            x = self.direction(ref(a[2]))
        # Gram-Schmidt x against z
        d = sum(x[i] * z[i] for i in range(3))
        x = normalize(tuple(x[i] - d * z[i] for i in range(3)))
        y = cross(z, x)
        return (x, y, z, o)

    def placement2d(self, r):
        a = self.m.args(r)
        o = self.point(ref(a[0]))
        x = (1.0, 0.0)
        if len(a) > 1 and a[1].strip() != "$":
            x = normalize(self.direction(ref(a[1]))[:2])
        y = (-x[1], x[0])
        return ((x[0], x[1], 0.0), (y[0], y[1], 0.0), (0.0, 0.0, 1.0), (o[0], o[1], 0.0))

    def local_placement(self, r):
        if r is None:
            return IDENT
        t = self.m.type_of(r)
        if t != "IFCLOCALPLACEMENT":
            raise ValueError(t)
        a = self.m.args(r)
        rel = ref(a[0])
        own = self.placement3d(ref(a[1]))
        if rel is None:
            return own
        return mat_mul(self.local_placement(rel), own)

    # ---- curves / profiles -------------------------------------------------
    def curve_points(self, r):
        t = self.m.type_of(r)
        a = self.m.args(r)
        if t == "IFCPOLYLINE":
            return [self.point(p) for p in reflist(a[0])]
        if t == "IFCINDEXEDPOLYCURVE":
            base = ref(a[0])
            bt = self.m.type_of(base)
            pts = nested_numlist(self.m.args(base)[0])
            if a[1].strip() == "$":
                return pts
            # segments: IFCLINEINDEX((i,j)) / IFCARCINDEX((i,j,k))
            out = []
            for seg in tokenize_args(a[1].strip()[1:-1]):
                sm = re.match(r"IFC(LINE|ARC)INDEX\((.*)\)$", seg.strip())
                if not sm:
                    continue
                idx = [int(float(v)) for v in tokenize_args(sm.group(2))]
                for i in idx:
                    out.append(pts[i - 1])
            return out
        if t == "IFCCOMPOSITECURVE":
            out = []
            for seg in reflist(a[0]):
                out += self.curve_points(ref(self.m.args(seg)[0]))
            return out
        if t == "IFCTRIMMEDCURVE":
            return []
        if t == "IFCCIRCLE":
            centre = self.placement2d(ref(a[0]))[3]
            rad = num(a[1])
            return [
                (centre[0] + rad * math.cos(k * math.pi / 8), centre[1] + rad * math.sin(k * math.pi / 8))
                for k in range(16)
            ]
        raise ValueError("curve " + str(t))

    def profile_points(self, r):
        t = self.m.type_of(r)
        a = self.m.args(r)
        if t in ("IFCARBITRARYCLOSEDPROFILEDEF", "IFCARBITRARYPROFILEDEFWITHVOIDS"):
            return self.curve_points(ref(a[2]))
        if t == "IFCRECTANGLEPROFILEDEF":
            xd, yd = num(a[3]), num(a[4])
            pts = [(-xd / 2, -yd / 2), (xd / 2, -yd / 2), (xd / 2, yd / 2), (-xd / 2, yd / 2)]
            if a[2].strip() != "$":
                m = self.placement2d(ref(a[2]))
                return [apply(m, (p[0], p[1], 0.0))[:2] for p in pts]
            return pts
        if t == "IFCCIRCLEPROFILEDEF":
            rad = num(a[3])
            pts = [(rad * math.cos(k * math.pi / 8), rad * math.sin(k * math.pi / 8)) for k in range(16)]
            if a[2].strip() != "$":
                m = self.placement2d(ref(a[2]))
                return [apply(m, (p[0], p[1], 0.0))[:2] for p in pts]
            return pts
        raise ValueError("profile " + str(t))

    # ---- items -------------------------------------------------------------
    def item_points(self, r, m):
        """World points of representation item `r` under transform `m`."""
        t = self.m.type_of(r)
        a = self.m.args(r)
        if t == "IFCEXTRUDEDAREASOLID":
            prof = self.profile_points(ref(a[0]))
            pm = self.placement3d(ref(a[1])) if a[1].strip() != "$" else IDENT
            d = self.direction(ref(a[2]))
            depth = num(a[3])
            local = mat_mul(m, pm)
            out = []
            for p in prof:
                base = (p[0], p[1], 0.0)
                top = (p[0] + d[0] * depth, p[1] + d[1] * depth, d[2] * depth)
                out.append(apply(local, base))
                out.append(apply(local, top))
            return out
        if t in ("IFCPOLYGONALFACESET", "IFCTRIANGULATEDFACESET"):
            pts = nested_numlist(self.m.args(ref(a[0]))[0])
            return [apply(m, p) for p in pts]
        if t == "IFCMAPPEDITEM":
            rmap = ref(a[0])
            ra = self.m.args(rmap)
            origin = self.placement3d(ref(ra[0]))
            op = ref(a[1])
            oa = self.m.args(op)
            scale = num(oa[3]) if len(oa) > 3 and oa[3].strip() != "$" else 1.0
            opm = IDENT
            if oa[2].strip() != "$":
                o = self.point(ref(oa[2]))
                opm = ((1.0, 0, 0), (0, 1.0, 0), (0, 0, 1.0), (o[0], o[1], o[2]))
            assert abs(scale - 1.0) < 1e-12, "non-unit scale"
            inner = mat_mul(mat_mul(m, opm), origin)
            out = []
            for it in reflist(self.m.args(ref(ra[1]))[3]):
                out += self.item_points(it, inner)
            return out
        if t in ("IFCPOLYLINE", "IFCINDEXEDPOLYCURVE"):
            return [apply(m, (p[0], p[1], p[2] if len(p) > 2 else 0.0)) for p in self.curve_points(r)]
        if t == "IFCGEOMETRICCURVESET":
            out = []
            for it in reflist(a[0]):
                out += self.item_points(it, m)
            return out
        if t == "IFCBOOLEANCLIPPINGRESULT":
            return self.item_points(ref(a[1]), m)
        raise ValueError("item " + str(t))

    def product_points(self, prod, rep_id="Body"):
        a = self.m.args(prod)
        place = self.local_placement(ref(a[5]))
        shape = ref(a[6])
        if shape is None:
            return []
        out = []
        for rep in reflist(self.m.args(shape)[2]):
            ra = self.m.args(rep)
            if ra[1].strip().strip("'") != rep_id:
                continue
            for it in reflist(ra[3]):
                out += self.item_points(it, place)
        return out


def aabb(points):
    if not points:
        return None
    lo = [min(p[i] for p in points) for i in range(3)]
    hi = [max(p[i] for p in points) for i in range(3)]
    return lo + hi


def main(argv):
    import argparse

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("ifc")
    parser.add_argument("--type", default="IFCWALL")
    parser.add_argument("--representation", default="Body")
    parser.add_argument("--scale", type=float, default=1.0)
    args = parser.parse_args(argv)

    model = Model(args.ifc)
    geom = Geom(model)
    for entity in model.by_type(args.type.upper()):
        entity_args = model.args(entity)
        tag = entity_args[7].strip().strip("'")
        box = aabb(geom.product_points(entity, args.representation))
        if box is None:
            continue
        values = " ".join(f"{v * args.scale:.9f}" for v in box)
        print(f"{tag} {values}")
    return 0


if __name__ == "__main__":
    import sys

    raise SystemExit(main(sys.argv[1:]))
