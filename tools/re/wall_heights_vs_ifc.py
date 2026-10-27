#!/usr/bin/env python3
"""Compare how high each wall of an rvt-rs IFC stands with Revit's own IFC
export of the same file (#1067, RE-257).

For each `IfcWall` both files hold under one `Tag` (the Revit ElementId), it
meshes both bodies with IfcOpenShell in world coordinates and takes their
lowest and highest points. Revit's export is in shared coordinates, so the
two files' heights differ by one offset: the median of the walls' base
differences. It then counts the walls whose base and top are Revit's
within 0.001 ft, those where only one of them is, and those where neither
is, and gives each file's storey for the walls that miss. Plan positions
are `wall_bodies_vs_ifc.py`'s.

Usage:

    python3 tools/re/wall_heights_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>

It lists at most 150 of the walls that miss.

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import collections
import statistics
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element

FOOT = 0.3048
TOLERANCE = 0.001


def ranges(path):
    """Tag -> (lowest z, highest z in feet, storey name, representation
    types) of every wall with a body."""
    f = ifcopenshell.open(path)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    out = {}
    for wall in f.by_type("IfcWall"):
        if not wall.Tag or wall.Representation is None or str(wall.Tag) in out:
            continue
        try:
            shape = ifcopenshell.geom.create_shape(settings, wall)
            zs = shape.geometry.verts[2::3]
        except Exception:
            continue
        if not zs:
            continue
        container = ifcopenshell.util.element.get_container(wall)
        kinds = sorted(
            {r.RepresentationType or "-" for r in wall.Representation.Representations if r.RepresentationIdentifier == "Body"}
        )
        # IfcOpenShell's geometry is in metres whatever the file's unit.
        out[str(wall.Tag)] = (
            min(zs) / FOOT,
            max(zs) / FOOT,
            container.Name if container is not None and container.is_a("IfcBuildingStorey") else "-",
            "/".join(kinds),
        )
    return out


def main():
    args = sys.argv[1:]
    if len(args) != 2:
        print(__doc__)
        return 2
    ours, revit = ranges(args[0]), ranges(args[1])
    both = sorted(set(ours) & set(revit))
    if not both:
        print("no wall in both files")
        return 0
    offset = statistics.median(revit[t][0] - ours[t][0] for t in both)
    counts = collections.Counter()
    storeys = collections.Counter()
    lines = []
    for tag in both:
        low = revit[tag][0] - offset - ours[tag][0]
        high = revit[tag][1] - offset - ours[tag][1]
        base_ok, top_ok = abs(low) <= TOLERANCE, abs(high) <= TOLERANCE
        group = {
            (True, True): "base and top Revit's",
            (True, False): "base Revit's, top not",
            (False, True): "top Revit's, base not",
            (False, False): "neither Revit's",
        }[(base_ok, top_ok)]
        counts[group] += 1
        if not (base_ok and top_ok):
            storeys[("same storey" if ours[tag][2] == revit[tag][2] else "other storey")] += 1
            lines.append(
                f"  tag:{tag} {group}: base {low:+.4f} ft, top {high:+.4f} ft, rvt-rs storey {ours[tag][2]!r},"
                f" Revit storey {revit[tag][2]!r}, Revit body {revit[tag][3]}"
            )
    print(f"walls in both files: {len(both)} (rvt-rs {len(ours)}, Revit {len(revit)}); offset {offset:.6f} ft")
    for group, n in counts.most_common():
        print(f"  {group}: {n}")
    for group, n in storeys.most_common():
        print(f"  misses in the {group}: {n}")
    print("\n".join(lines[:150]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
