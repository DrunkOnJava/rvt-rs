#!/usr/bin/env python3
"""The lowest vertex of the mesh Revit saved for each wall that embeds a curtain
wall, against Revit's own lowest point of that wall (#1385, RE-340 section 3, #2148).

Reads `probe_1385_wall_saved_mesh`'s output (`saved tag:<id> volume <ft3> box
<z low> ...`, in feet, in the document frame) and Revit's IFC export of the same
model. For each `IfcWall` whose `Tag` the probe names, it meshes the Body in
IfcOpenShell's world coordinates, takes the vertices back into Revit's internal
frame (`ifc_common.site_frame`) and prints, with `--list` (as Measure runs it),
a line for each host:

    wall tag:<id> saved <z low> revit <z low> diff <ft>
    wall tag:<id> saved <z low> revit none
    wall tag:<id> saved <z low> revit unmeshed <reason>

and always the count:

    hosts <n> meshed <m> in_export <k> unmeshed <u> equal <e> (to 0.001 ft) worst <ft>

`hosts` is the probe's count of walls that name a `WallEmbedWallGStep`,
`meshed` those whose saved mesh it decoded, `in_export` those Revit's export
also has as an `IfcWall` with that Tag and IfcOpenShell meshes, `unmeshed`
those it has and IfcOpenShell does not mesh, and `equal` those whose two lowest
points agree to 0.001 ft. A host Revit's export does not have is in no count.

Usage:

    python3 tools/re/wall_saved_low_vs_ifc.py <probe output> <revit-export.ifc> [--list]

Measure runs it as `-f probe=probe_1385_wall_saved_mesh -f
probe_scorer=wall_saved_low_vs_ifc`.

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import os
import re
import sys

import ifcopenshell
import ifcopenshell.geom
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ifc_common import site_frame  # noqa: E402

TOLERANCE = 0.001
SAVED = re.compile(r"saved tag:(\d+) volume \S+ box (\S+)")
HOSTS = re.compile(r"hosts (\d+)")


def read_probe(path):
    hosts, saved = None, {}
    with open(path, encoding="utf-8") as probe:
        for line in probe:
            if hosts is None and (m := HOSTS.fullmatch(line.strip())):
                hosts = int(m.group(1))
            elif m := SAVED.match(line):
                saved[int(m.group(1))] = float(m.group(2))
    return hosts, saved


def main(argv):
    listing = "--list" in argv[1:]
    args = [arg for arg in argv[1:] if arg != "--list"]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    hosts, saved = read_probe(args[0])
    if hosts is None:
        print("the probe output has no `hosts` line", file=sys.stderr)
        return 1
    model = ifcopenshell.open(args[1])
    frame = site_frame(model)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    walls = {}
    for wall in model.by_type("IfcWall"):
        try:
            tag = int(wall.Tag)
        except (TypeError, ValueError):
            continue
        if tag in saved:
            walls.setdefault(tag, []).append(wall)
    in_export = unmeshed = equal = 0
    worst = 0.0
    for tag, low in sorted(saved.items()):
        if tag not in walls:
            if listing:
                print(f"wall tag:{tag} saved {low:.4f} revit none")
            continue
        try:
            # A wall Revit exports in parts gives each part its Tag, so the
            # lowest point is taken over every IfcWall that carries it.
            points = []
            for wall in walls[tag]:
                shape = ifcopenshell.geom.create_shape(settings, wall)
                geometry = shape.geometry
                points.append(np.array(geometry.verts, dtype=float).reshape(-1, 3))
            vertices = np.concatenate(points)
            if not len(vertices):
                raise RuntimeError("no vertices")
        except Exception as error:  # IfcOpenShell raises its own and RuntimeError
            unmeshed += 1
            if listing:
                print(f"wall tag:{tag} saved {low:.4f} revit unmeshed {str(error).splitlines()[0]}")
            continue
        revit = float(frame(vertices)[:, 2].min())
        diff = low - revit
        in_export += 1
        equal += abs(diff) <= TOLERANCE
        worst = max(worst, abs(diff))
        if listing:
            print(f"wall tag:{tag} saved {low:.4f} revit {revit:.4f} diff {diff:+.5f}")
    print(f"hosts {hosts} meshed {len(saved)} in_export {in_export} unmeshed {unmeshed} equal {equal} (to {TOLERANCE} ft) worst {worst:.5f}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
