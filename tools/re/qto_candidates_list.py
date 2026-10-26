#!/usr/bin/env python3
"""Which value of rvt-rs's body each of Revit's base quantities is (#1016, W7).

For every object Revit's export gives a `Qto_` set and rvt-rs also writes
(by Tag, a space or storey by Name), it derives candidates from rvt-rs's
`Body`: the extruded solids' depth (largest), profile areas, profile
perimeters, profile extents along X and Y and their largest, volumes, side
surfaces (perimeter times depth) and closed surfaces (side plus both ends),
each summed over the solids where that is meaningful. For each class,
set and quantity it prints how many objects carry it and the three
candidates equal (a relative 1e-5, or 1e-6 absolute in SI units) on the most
of them, so a quantity rvt-rs's own body already gives can be written from
it. With `--by-source` each count is split by the `BodySource` rvt-rs gives
the object, so a rule can be narrowed to the bodies drawn exactly. With
`--misses` (which implies `--by-source`), each group whose best candidate
holds on 90% or more of its objects but not all lists the objects it misses
(at most 20): Tag, Revit's value and rvt-rs's.

Usage:

    python3 tools/re/qto_candidates_list.py <rvt-rs.ifc> <revit-export.ifc> [--by-source] [--misses]
"""

import collections
import math
import os
import sys

import ifcopenshell
import ifcopenshell.util.element
import ifcopenshell.util.unit

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from psets_vs_ifc import key_of  # noqa: E402
from qtos_vs_ifc import quantities  # noqa: E402
from wall_qtos_list import points_of, solids  # noqa: E402


def profile_measures(p, scale):
    """(area, perimeter, extent x, extent y) of a profile, SI units."""
    if p.is_a("IfcRectangleProfileDef"):
        x, y = p.XDim * scale, p.YDim * scale
        return x * y, 2 * (x + y), x, y
    if p.is_a("IfcCircleProfileDef"):
        r = p.Radius * scale
        return math.pi * r * r, 2 * math.pi * r, 2 * r, 2 * r
    if p.is_a("IfcArbitraryClosedProfileDef"):
        outer = points_of(p.OuterCurve, scale)
        if not outer:
            return None
        def ring(points):
            area = sum(x0 * y1 - x1 * y0 for (x0, y0), (x1, y1) in zip(points, points[1:] + points[:1])) / 2
            length = sum(math.dist(a, b) for a, b in zip(points, points[1:] + points[:1]))
            return abs(area), length
        area, perimeter = ring(outer)
        for inner in getattr(p, "InnerCurves", None) or ():
            pts = points_of(inner, scale)
            if not pts:
                return None
            a, l = ring(pts)
            area -= a
            perimeter += l
        xs, ys = [x for x, _ in outer], [y for _, y in outer]
        return area, perimeter, max(xs) - min(xs), max(ys) - min(ys)
    return None


def body_source(product):
    for props in ifcopenshell.util.element.get_psets(product).values():
        if "BodySource" in props:
            return str(props["BodySource"])
    return "-"


def candidates(f, sources=None):
    scale = ifcopenshell.util.unit.calculate_unit_scale(f)
    out = {}
    for product in f.by_type("IfcProduct"):
        key = key_of(product)
        if not key or product.Representation is None:
            continue
        found = []
        for rep in product.Representation.Representations:
            if rep.RepresentationIdentifier == "Body":
                for item in rep.Items:
                    found.extend(solids(item))
        if not found:
            continue
        if sources is not None:
            sources.setdefault(key, body_source(product))
        measures = []
        for solid in found:
            m = profile_measures(solid.SweptArea, scale)
            if m is None:
                break
            measures.append((m, solid.Depth * scale))
        else:
            area = sum(m[0] for m, _ in measures)
            perimeter = sum(m[1] for m, _ in measures)
            depth = max(d for _, d in measures)
            x = max(m[2] for m, _ in measures)
            y = max(m[3] for m, _ in measures)
            volume = sum(m[0] * d for m, d in measures)
            side = sum(m[1] * d for m, d in measures)
            out.setdefault(key, {
                "depth": depth,
                "profile area": area,
                "profile perimeter": perimeter,
                "profile x": x,
                "profile y": y,
                "profile larger side": max(x, y),
                "profile smaller side": min(x, y),
                "volume": volume,
                "side surface": side,
                "closed surface": side + 2 * area,
                "side surface / 2": side / 2,
                "depth x larger side": depth * max(x, y),
            })
    return out


def same(a, b):
    return a is not None and b is not None and abs(a - b) <= max(1e-6, 1e-5 * max(abs(a), abs(b)))


def main(argv):
    misses = "--misses" in argv
    by_source = misses or "--by-source" in argv
    args = [a for a in argv[1:] if a not in ("--list", "--by-source", "--misses")]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    sources = {} if by_source else None
    ours_file = ifcopenshell.open(args[0])
    ours = candidates(ours_file, sources)
    theirs, _, entities, *_ = quantities(args[1])
    rows = collections.defaultdict(lambda: collections.Counter())
    totals = collections.Counter()
    members = collections.defaultdict(list)
    for (key, qset, name), (_, value) in theirs.items():
        if value is None or key not in ours:
            continue
        group = (entities.get(key, "-"), qset, name) + ((sources.get(key, "-"),) if sources is not None else ())
        totals[group] += 1
        members[group].append((key, value))
        for candidate, v in ours[key].items():
            if same(value, v):
                rows[group][candidate] += 1
    print(f"objects with a body in both: {len(ours)}")
    for group in sorted(totals, key=lambda g: -totals[g]):
        best = ", ".join(f"{c} {n}" for c, n in rows[group].most_common(3)) or "none"
        source = f" [{group[3]}]" if len(group) > 3 else ""
        print(f"  {group[0]} {group[1]}.{group[2]}{source}: {totals[group]}; {best}")
        if not misses or not rows[group]:
            continue
        candidate, hits = rows[group].most_common(1)[0]
        if hits == totals[group] or hits < 0.9 * totals[group]:
            continue
        missed = [(k, v) for k, v in members[group] if not same(v, ours[k][candidate])]
        for k, v in sorted(missed, key=str)[:20]:
            print(f"    miss {k}: Revit {v:.6g}, {candidate} {ours[k][candidate]:.6g}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
