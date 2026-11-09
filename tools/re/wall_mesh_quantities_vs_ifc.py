#!/usr/bin/env python3
"""Whether the saved mesh of a wall gives Revit's wall quantities (#1046, RE-388).

`probes/examples/probe_1046_wall_mesh_quantities.rs` meshes the saved graphics
of every wall and prints, per wall, the volume of the mesh, the span of its
heights and its largest vertical faces (the area inside each face's outer
loop, `gross`, and its area less its holes, `net`). Revit's export writes a
wall's `Qto_WallBaseQuantities` from the faces and the volume of its solid
(revit-ifc's `PropertyUtil.CreateWallBaseQuantities`, CLEANROOM.md source 5,
RE-270): `NetVolume` the volume, `Height` the span of the solids, `GrossSideArea`
the largest outer-loop area of a vertical face, `NetSideArea` the largest
vertical face's area, `GrossVolume` the wall type's width times the
`GrossSideArea`. This reads the probe's output and Revit's IFC and counts, for
each quantity Revit writes on a wall the probe meshed, how many of Revit's
values the candidate equals (a relative 1e-5, or 1e-6 of a metre, square or
cubic metre where that is larger, as `qtos_vs_ifc.py` scores), by whether the
wall's saved graphics decoded whole (no diagnostic and no unbounded face). It
scores nothing; it prints up to 8 differences of each quantity.

Usage:

    python3 tools/re/wall_mesh_quantities_vs_ifc.py <probe output> <revit-export.ifc> [--list]

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import collections
import math
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import qtos_vs_ifc  # noqa: E402

FOOT = 0.3048
SET = "Qto_WallBaseQuantities"
DIMENSION = {"NetVolume": 3, "GrossVolume": 3, "Height": 1, "GrossSideArea": 2, "NetSideArea": 2}


def parse(path):
    walls = {}
    for line in open(path, encoding="utf-8", errors="replace"):
        m = re.match(r"^wall tag:(\d+) status (\S+) primitives (\d+) triangles (\d+) unbounded (\d+) diagnostics (\d+)", line)
        if m:
            walls.setdefault(m.group(1), {}).update(
                status=m.group(2),
                primitives=int(m.group(3)),
                triangles=int(m.group(4)),
                unbounded=int(m.group(5)),
                diagnostics=int(m.group(6)),
            )
            continue
        m = re.match(r"^wall tag:(\d+) volume (\S+) z (\S+) (\S+) vfaces (\d+)", line)
        if m:
            walls.setdefault(m.group(1), {}).update(
                volume=float(m.group(2)), low=float(m.group(3)), high=float(m.group(4)), vfaces=int(m.group(5))
            )
            continue
        m = re.match(r"^wall tag:(\d+) vface net (\S+) gross (\S+) normal (\S+) (\S+)", line)
        if m:
            walls.setdefault(m.group(1), {}).setdefault("faces", []).append((float(m.group(2)), float(m.group(3))))
    return walls


def equal(a, b, dimension):
    return abs(a - b) <= max(1e-6 / FOOT**dimension, 1e-5 * max(abs(a), abs(b)))


def candidates(wall, width):
    """Each quantity's candidate from the mesh, feet, or None."""
    out = {}
    if "volume" in wall:
        out["NetVolume"] = wall["volume"]
        out["Height"] = wall["high"] - wall["low"]
    faces = wall.get("faces") or []
    if faces:
        out["NetSideArea"] = max(net for net, _ in faces)
        grosses = [gross for _, gross in faces if not math.isnan(gross)]
        if grosses:
            out["GrossSideArea"] = max(grosses)
            if width is not None:
                out["GrossVolume"] = width * out["GrossSideArea"]
    return out


def main(argv):
    args = [a for a in argv[1:] if not a.startswith("--")]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    walls = parse(args[0])
    revit, _, entities, _, _, _ = qtos_vs_ifc.quantities(args[1])
    meshed = {k: v for k, v in walls.items() if "volume" in v}
    whole = {k for k, v in meshed.items() if v.get("diagnostics", 1) == 0 and v.get("unbounded", 1) == 0}
    print(f"walls the probe read: {len(walls)}; meshed: {len(meshed)}; whole (no diagnostic, no unbounded face): {len(whole)}")
    statuses = collections.Counter(v.get("status", "?") for v in walls.values())
    print("  status " + ", ".join(f"{s} {n}" for s, n in sorted(statuses.items())))
    rows = collections.defaultdict(collections.Counter)
    examples = collections.defaultdict(list)
    for tag, wall in sorted(meshed.items()):
        key = f"tag:{tag}"
        if entities.get(key) not in ("IfcWall", "IfcWallStandardCase"):
            continue
        width_si = revit.get((key, SET, "Width"))
        width = None if width_si is None or width_si[1] is None else width_si[1] / FOOT
        mine = candidates(wall, width)
        group = "whole" if tag in whole else "partial"
        for name, dimension in DIMENSION.items():
            value = revit.get((key, SET, name))
            if value is None or value[1] is None:
                continue
            row = rows[(name, group)]
            row["revit"] += 1
            candidate = mine.get(name)
            if candidate is None:
                row["no candidate"] += 1
                continue
            theirs = value[1] / FOOT**dimension
            if equal(theirs, candidate, dimension):
                row["equal"] += 1
            else:
                row["different"] += 1
                if len(examples[name]) < 8:
                    examples[name].append((tag, group, theirs, candidate))
    for (name, group), row in sorted(rows.items()):
        print(
            f"{name} ({group} meshes): Revit writes {row['revit']}; candidate equal {row['equal']}, "
            f"different {row['different']}, no candidate {row['no candidate']}"
        )
    for name, items in sorted(examples.items()):
        for tag, group, theirs, candidate in items:
            print(f"  different {name} tag:{tag} ({group}): Revit {theirs:.4f}, mesh {candidate:.4f}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
