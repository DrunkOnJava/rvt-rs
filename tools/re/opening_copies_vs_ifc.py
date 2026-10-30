#!/usr/bin/env python3
"""Which copy of a door's or window's instance data places Revit's opening
(#227, #548).

Research tool for `probes/examples/probe_227_missing_transforms.rs`, whose `copy`
lines give, for each door or window whose data copies disagree, each copy's
transform (RE-87) with the cut loop (RE-215). For each such element this
moves the vertices of the `IfcOpeningElement` it fills in Revit's export into
Revit's internal feet through the export's site placement, then into each
copy's frame, as `opening_frames_vs_ifc.py` does, and compares their X and Z
ranges with the loop's, and its Y range with the loop's plane and its mirror
(the host's band). A copy whose frame puts Revit's opening there is the one
Revit's export used.

Usage:

    python3 tools/re/opening_copies_vs_ifc.py <probe output> <revit-export.ifc> [--list]

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins) and NumPy.
"""

import collections
import json
import sys

import ifcopenshell
import ifcopenshell.geom
import numpy as np

from ifc_common import site_frame

BANDS = (0.001, 0.01, 0.1, 0.5)


def band(diff):
    return next((f"within {b} ft" for b in BANDS if diff <= b), f"over {BANDS[-1]} ft")


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    listing = "--list" in sys.argv
    copies = collections.defaultdict(list)
    for line in open(args[0]):
        if line.startswith("copy "):
            _, tag, stream, lo, hi, axes, origin = line.split(" ", 6)
            copies[tag].append((stream, np.array(json.loads(lo)), np.array(json.loads(hi)),
                                np.array(json.loads(axes)), np.array(json.loads(origin))))
    f = ifcopenshell.open(args[1])
    to_internal = site_frame(f)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    tally = collections.Counter()
    rows = []
    for rel in f.by_type("IfcRelFillsElement"):
        filler = rel.RelatedBuildingElement
        tag = str(filler.Tag or "")
        if tag not in copies:
            continue
        try:
            shape = ifcopenshell.geom.create_shape(settings, rel.RelatingOpeningElement)
        except Exception:
            tally["opening not meshed"] += 1
            continue
        verts = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
        if not len(verts):
            continue
        tally["elements"] += 1
        diffs = []
        for stream, lo, hi, axes, origin in copies[tag]:
            local = (to_internal(verts) - origin) @ axes.T
            omin, omax = local.min(axis=0), local.max(axis=0)
            xz = max(abs(omin[0] - lo[0]), abs(omax[0] - hi[0]), abs(omin[2] - lo[2]), abs(omax[2] - hi[2]))
            y0 = lo[1]
            y = max(abs(omin[1] - min(y0, -y0)), abs(omax[1] - max(y0, -y0)))
            diff = max(xz, y)
            diffs.append((diff, stream))
            tally[f"copy in {stream}: X and Z ranges {band(xz)}"] += 1
            tally[f"copy in {stream}: Y range (loop plane to its mirror) {band(y)}"] += 1
        diffs.sort()
        if len(diffs) > 1 and diffs[0][0] <= 0.001 < diffs[1][0]:
            tally[f"only the copy in {diffs[0][1]} within 0.001 ft"] += 1
        rows.append((tag, filler.is_a(), diffs))
    print(f"probe: {len(copies)} doors and windows whose copies disagree")
    for label, n in sorted(tally.items()):
        print(f"  {label}: {n}")
    if listing:
        print("\neach element, its copies' largest X, Y or Z difference:")
        for tag, cls, diffs in rows:
            print(f"  {cls} {tag}: " + ", ".join(f"{stream} {d:.4f} ft" for d, stream in diffs))


main()
