#!/usr/bin/env python3
"""What a curtain wall's mullion joins say about the ends of its mullions
(#1046, RE-268, W7).

`probes/examples/probe_1046_mullion_joins.rs` lists, per curtain wall, the
entries of `WallCGDriver.m_mullionJoins`: the mullion ends that meet at a grid
vertex, each as the `MullionFamRef.m_edgeTag` of a mullion and which end of
it (`m_end`, 0 or 1). This takes Revit's export of the same file and, for each
IfcMember drawn as one extruded solid, the ends its NetSurfaceArea counts (0,
1 or 2: (NetSurfaceArea - perimeter x length) / section area), and prints:

- how many joins there are, of how many members, on how many walls, and
  `CurtainGrid.m_joinTypes` per wall;
- which end of the exported extrusion each `m_end` is (the one a join's
  members agree on, placed within a spread of the least);
- which member of a join of three runs across the other two;
- a predictor from the joins alone, "an end is hidden when it is one of the
  two members of a join that run along one another", against the ends Revit's
  NetSurfaceArea counts.

Usage:

    python3 tools/re/mullion_joins_vs_ifc.py <probe_1046_mullion_joins output> <revit-export.ifc>
"""

import collections
import itertools
import os
import sys

import ifcopenshell
import ifcopenshell.util.element
import ifcopenshell.util.placement
import ifcopenshell.util.unit
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from mullion_ends_vs_records import mapping_matrix  # noqa: E402
from qto_candidates_list import profile_measures  # noqa: E402
from wall_qtos_list import solids  # noqa: E402


def read_probe(path):
    mul, joins, drivers = {}, collections.defaultdict(list), {}
    for line in open(path):
        parts = line.rstrip("\n").split(" ")
        if parts[0] == "mul" and len(parts) >= 5:
            mul[parts[1]] = (parts[2], parts[4])
        elif parts[0] == "drv":
            drivers[parts[1]] = " ".join(parts[5:]) if len(parts) > 5 else "-"
        elif parts[0] == "join" and len(parts) >= 5:
            members = [(e, int(end)) for e, end in (m.split(":") for m in parts[4].split(",") if m)]
            joins[parts[1]].append(members)
    return mul, joins, drivers


def profile_centre(profile, scale):
    """The centre of the profile's box in its own plane, metres."""
    if profile.is_a("IfcRectangleProfileDef"):
        if profile.Position is not None:
            location = profile.Position.Location.Coordinates
            return location[0] * scale, location[1] * scale
    elif profile.is_a("IfcArbitraryClosedProfileDef") and profile.OuterCurve.is_a("IfcPolyline"):
        points = [(p.Coordinates[0] * scale, p.Coordinates[1] * scale) for p in profile.OuterCurve.Points]
        xs, ys = [x for x, _ in points], [y for _, y in points]
        return (max(xs) + min(xs)) / 2, (max(ys) + min(ys)) / 2
    return 0.0, 0.0


def read_mullions(f):
    """Tag -> (start centre, end centre, mullion's ends counted) of each
    IfcMember drawn as one extruded solid, metres."""
    scale = ifcopenshell.util.unit.calculate_unit_scale(f)
    out = {}
    for member in f.by_type("IfcMember"):
        if not member.Tag or member.Representation is None:
            continue
        qtos = ifcopenshell.util.element.get_psets(member, qtos_only=True).get("Qto_MemberBaseQuantities", {})
        nsa = qtos.get("NetSurfaceArea")
        if nsa is None:
            continue
        items = []
        for rep in member.Representation.Representations:
            if rep.RepresentationIdentifier == "Body":
                pending = [(item, np.eye(4)) for item in rep.Items]
                while pending:
                    item, mapped = pending.pop()
                    if item.is_a("IfcMappedItem"):
                        inner = mapped @ mapping_matrix(item)
                        pending.extend((i, inner) for i in item.MappingSource.MappedRepresentation.Items)
                        continue
                    items.append((item, mapped))
        found = [(s, mapped) for item, mapped in items for s in solids(item)]
        if len(found) != 1 or len(items) != 1:
            continue
        solid, mapped = found[0]
        measures = profile_measures(solid.SweptArea, scale)
        if measures is None or measures[0] <= 0:
            continue
        depth = solid.Depth * scale
        ends = (nsa - measures[1] * depth) / measures[0]
        if abs(ends - round(ends)) > 1e-3 or round(ends) not in (0, 1, 2):
            continue
        place = np.array(ifcopenshell.util.placement.get_local_placement(member.ObjectPlacement), dtype=float) @ mapped
        if solid.Position is not None:
            place = place @ np.array(ifcopenshell.util.placement.get_axis2placement(solid.Position), dtype=float)
        place[:3, 3] *= scale
        direction = place[:3, :3] @ np.array(solid.ExtrudedDirection.DirectionRatios, dtype=float)
        direction /= np.linalg.norm(direction)
        cx, cy = profile_centre(solid.SweptArea, scale)
        start = (place @ np.array([cx, cy, 0.0, 1.0]))[:3]
        out[str(member.Tag)] = (start, start + direction * depth, round(ends), direction)
    return out


def main(argv):
    args = [a for a in argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    mul, joins, drivers = read_probe(args[0])
    mullions = read_mullions(ifcopenshell.open(args[1]))
    by_edge = {(host, edge): tag for tag, (host, edge) in mul.items()}
    print(f"mullions and panels listed: {len(mul)}, mullions drawn as one extruded solid with a whole count of ends: {len(mullions)}")
    print(f"curtain walls: {len(drivers)}, with joins: {len(joins)}, joins: {sum(len(v) for v in joins.values())}")
    print("members per join: " + ", ".join(f"{k} x{v}" for k, v in sorted(collections.Counter(len(m) for js in joins.values() for m in js).items())))
    print("CurtainGrid.m_joinTypes per wall: " + ", ".join(f"{k} x{v}" for k, v in sorted(collections.Counter(drivers.values()).items())))
    unnamed = sum(1 for host, js in joins.items() for members in js for e, _ in members if (host, e) not in by_edge)
    print(f"join members naming an edge with no mullion in the export: {unnamed}")

    end_of = collections.Counter()
    across_up = collections.Counter()
    role = {}
    located = 0
    for host, js in joins.items():
        for members in js:
            tags = [by_edge.get((host, e)) for e, _ in members]
            if any(t is None or t not in mullions for t in tags):
                continue
            located += 1
            best = None
            for combo in itertools.product((0, 1), repeat=len(tags)):
                points = [mullions[t][combo[i]] for i, t in enumerate(tags)]
                spread = max(np.linalg.norm(a - b) for a in points for b in points)
                if best is None or spread < best[0]:
                    best = (spread, combo)
            for (e, end), t, c in zip(members, tags, best[1]):
                end_of[(end, c)] += 1
            if len(tags) == 3:
                d = [mullions[t][3] for t in tags]
                for i in range(3):
                    others = [j for j in range(3) if j != i]
                    if abs(d[others[0]] @ d[others[1]]) > 0.99 and abs(d[i] @ d[others[0]]) < 0.1:
                        across_up["upright" if abs(d[i][2]) > 0.999 else "not upright"] += 1
                        for j in range(3):
                            role[(tags[j], best[1][j])] = "across" if j == i else "along"
    print(f"joins whose members all are such mullions: {located}")
    print("m_end, the end of the exported extrusion (0 start, 1 end): members")
    for (m, c), n in sorted(end_of.items()):
        print(f"  {m} {c}: {n}")
    print("the member of a join of three that runs across the other two: " + ", ".join(f"{k} x{v}" for k, v in sorted(across_up.items())))
    confusion = collections.Counter()
    for tag, (_, _, ends, _) in mullions.items():
        hidden = sum(1 for c in (0, 1) if role.get((tag, c)) == "along")
        confusion[(2 - hidden, ends)] += 1
    right = sum(n for (p, k), n in confusion.items() if p == k)
    print(f"ends predicted from the joins alone, ends Revit's NetSurfaceArea counts: mullions (agree {right} of {sum(confusion.values())})")
    for (p, k), n in sorted(confusion.items()):
        print(f"  {p} {k}: {n}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
