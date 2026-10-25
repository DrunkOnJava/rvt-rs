#!/usr/bin/env python3
"""List the walls whose ends differ from Revit's, with Revit's own joins at
each end (W6).

`wall_bodies_vs_ifc.py` scores a wall's ends as the larger of two
differences. This splits them: for every straight `IfcWall` in both files
(same `Tag`), it measures both bodies in plan along Revit's wall direction
(the X axis of the wall's placement, from its start to its end) and gives
the signed difference at each end, rvt-rs's less Revit's, in feet: at the
start, positive means rvt-rs's wall starts later (shorter there); at the
end, positive means it runs on further (longer there).

Each end is described by Revit's `IfcRelConnectsPathElements` that name it:
the partner wall's Tag and how the partner meets it (`ATSTART`, `ATEND`,
`ATPATH`). Walls are grouped by rvt-rs's `BodySource`, by whether each end
is right, short or long, and by the joins at that end, so a rule shows as
one row.

Needs IfcOpenShell.

Usage:

    python3 tools/re/wall_ends_list.py <rvt-rs.ifc> <revit.ifc> [--list N]
"""

import collections
import math
import os
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element
import ifcopenshell.util.placement

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from saved_meshes_vs_ifc import body_sources  # noqa: E402
from wall_layers_vs_ifc import site_to_internal  # noqa: E402

TOLERANCE_FT = 0.001


def plan_points(f):
    """Tag -> (wall, plan points in internal feet) for each meshed IfcWall."""
    to_internal, _ = site_to_internal(f)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out = {}
    for wall in f.by_type("IfcWall"):
        if not wall.Tag or not wall.Representation:
            continue
        body = next(
            (r for r in wall.Representation.Representations if r.RepresentationIdentifier == "Body"),
            None,
        )
        try:
            shape = ifcopenshell.geom.create_shape(settings, wall, body)
        except Exception:
            continue
        verts = shape.geometry.verts
        points = [to_internal(verts[i], verts[i + 1]) for i in range(0, len(verts), 3)]
        if points:
            out[wall.Tag] = (wall, points)
    return out


def joins(f):
    """Tag -> {'ATSTART': [(partner Tag, how the partner meets it)], 'ATEND': [...], 'ATPATH': [...]}."""
    out = collections.defaultdict(lambda: collections.defaultdict(list))
    for rel in f.by_type("IfcRelConnectsPathElements"):
        a, b = rel.RelatingElement, rel.RelatedElement
        if not (a and b and a.Tag and b.Tag):
            continue
        out[a.Tag][rel.RelatingConnectionType].append((b.Tag, rel.RelatedConnectionType))
        out[b.Tag][rel.RelatedConnectionType].append((a.Tag, rel.RelatingConnectionType))
    return out


def describe(partners):
    if not partners:
        return "free"
    return "+".join(sorted(how for _, how in partners))


def state(error):
    if abs(error) <= TOLERANCE_FT:
        return "ok"
    return "short" if error > 0 else "long"


def main():
    args = sys.argv[1:]
    if len(args) < 2:
        raise SystemExit(__doc__)
    listing = int(args[args.index("--list") + 1]) if "--list" in args else 40
    ours_file, ref_file = ifcopenshell.open(args[0]), ifcopenshell.open(args[1])
    ours, ref = plan_points(ours_file), plan_points(ref_file)
    _, to_internal_direction = site_to_internal(ref_file)
    revit_joins = joins(ref_file)
    groups = collections.Counter()
    rows = []
    for tag in sorted(set(ours) & set(ref), key=lambda t: (len(t), t)):
        our_wall, our_points = ours[tag]
        wall, points = ref[tag]
        placement = ifcopenshell.util.placement.get_local_placement(wall.ObjectPlacement)
        d = to_internal_direction(placement[0][0], placement[1][0])
        along = lambda p: p[0] * d[0] + p[1] * d[1]  # noqa: E731
        start = min(map(along, our_points)) - min(map(along, points))
        end = max(map(along, our_points)) - max(map(along, points))
        # The start end shortened is a positive start error; flip the end's
        # sign so that both read "positive = rvt-rs's wall is shorter there".
        start_state, end_state = state(start), state(-end)
        at = revit_joins.get(tag, {})
        source = body_sources(our_wall) or "-"
        key = (source, f"start {start_state} [{describe(at.get('ATSTART'))}]", f"end {end_state} [{describe(at.get('ATEND'))}]")
        groups[key] += 1
        wall_type = ifcopenshell.util.element.get_type(wall)
        rows.append(
            (
                max(abs(start), abs(end)),
                tag,
                source,
                wall_type.Name if wall_type else wall.ObjectType,
                start,
                -end,
                at.get("ATSTART", []),
                at.get("ATEND", []),
                len(at.get("ATPATH", [])),
            )
        )
    right = sum(1 for r in rows if r[0] <= TOLERANCE_FT)
    print(f"straight and angled walls in both: {len(rows)}; both ends within {TOLERANCE_FT} ft: {right}")
    print("BodySource | start (Revit's joins there) | end (Revit's joins there) | walls; short/long = rvt-rs's wall shorter/longer there")
    for (source, start, end), count in sorted(groups.items(), key=lambda kv: -kv[1]):
        print(f"  {count:>5}  {source} | {start} | {end}")
    print(f"the {listing} largest (tag, BodySource, type, start and end shortfall ft, Revit's joins at start, at end, walls meeting its path):")
    for row in sorted(rows, reverse=True)[:listing]:
        far, tag, source, name, s, e, at_start, at_end, on_path = row
        print(f"  {tag} {source} '{name}' start {s:+.4f} end {e:+.4f} start{at_start} end{at_end} path {on_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
