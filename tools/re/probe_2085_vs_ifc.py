#!/usr/bin/env python3
"""Score the bodies `probe_2085_boxed_family_instances` rebuilds from a family's extrusions
against the body Revit's own IFC export writes for the same element (#2085).

    python3 tools/re/probe_2085_vs_ifc.py <probe output> <revit-export.ifc>

Needs IfcOpenShell and numpy.

The probe takes the family instances an export leaves as a box because the file holds no saved
mesh for them, rebuilds each from its family's extrusions where it can, places it by the
instance's transform and prints the rebuilt solids' volume and box in rvt-rs's frame (the
model's internal coordinates, feet). This meshes Revit's body of the same Tag in the same frame
(the IfcSite's placement taken out, `ifc_common.site_frame`) and prints, per IFC class and in
all, for the bodies both hold:

- how many have Revit's volume within 1 per cent, 10 per cent, and further;
- how many have Revit's box within 0.01 ft, 0.1 ft, and further, at the farthest of its six faces;
- how many the probe rebuilt only in part (`others` above 0: forms the rebuild does not draw).

Counts only; no position, area or volume of a model is printed. The probe's tallies are repeated
first, prefixed `boxed:`.
"""

import collections
import math
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from mesh_bodies_vs_ifc import meshes, volume  # noqa: E402

VOLUME_BINS = (("within 1 per cent", 0.01), ("within 10 per cent", 0.1), ("further", math.inf))
BOX_BINS = (("within 0.01 ft", 0.01), ("within 0.1 ft", 0.1), ("further", math.inf))


def read_probe(path):
    """(tallies, Tag -> fields of the instances the probe rebuilt and placed)."""
    tallies, rebuilt = [], {}
    for line in open(path, encoding="utf-8", errors="replace"):
        if line.startswith(("tally ", "file ")):
            tallies.append(line.rstrip())
        if not line.startswith("instance "):
            continue
        fields = dict(part.split("=", 1) for part in line.split()[1:] if "=" in part)
        if fields.get("volume", "-") != "-":
            rebuilt[fields["tag"]] = fields
    return tallies, rebuilt


def point(text):
    return np.array([float(v) for v in text.split(",")])


def binned(value, bins):
    return next(name for name, bound in bins if value <= bound)


def main():
    if len(sys.argv) != 3:
        print(__doc__)
        return 2
    tallies, rebuilt = read_probe(sys.argv[1])
    for line in tallies:
        print(f"boxed: {line}")
    print(f"boxed: {len(rebuilt)} bodies rebuilt from their family's extrusions and placed")
    if not rebuilt:
        return 0
    revit = meshes(sys.argv[2], tags=set(rebuilt))
    rows = collections.Counter()
    seen = collections.Counter()
    for tag, fields in rebuilt.items():
        cls = fields.get("class", "?")
        if tag not in revit:
            seen[(cls, "not in Revit's export, or no body there")] += 1
            continue
        triangles = revit[tag][2]
        theirs = volume(triangles)
        ours = float(fields["volume"])
        points = triangles.reshape(-1, 3)
        low, high = points.min(axis=0), points.max(axis=0)
        box = max(
            float(np.abs(point(fields["low"]) - low).max()),
            float(np.abs(point(fields["high"]) - high).max()),
        )
        part = "in part" if fields.get("others", "0") != "0" else "whole"
        seen[(cls, f"scored, rebuilt {part}")] += 1
        ratio = abs(ours - theirs) / theirs if theirs > 0 else math.inf
        for key in (cls, "all"):
            rows[(key, part, "volume", binned(ratio, VOLUME_BINS))] += 1
            rows[(key, part, "box", binned(box, BOX_BINS))] += 1
    for (cls, what), n in sorted(seen.items()):
        print(f"boxed: {cls}: {n} {what}")
    keys = sorted({(cls, part) for cls, part, *_ in rows}, key=lambda k: (k[0] == "all", k))
    for cls, part in keys:
        for what, bins in (("volume", VOLUME_BINS), ("box", BOX_BINS)):
            cells = "; ".join(
                f"{name} {rows[(cls, part, what, name)]}" for name, _ in bins if rows[(cls, part, what, name)]
            )
            print(f"boxed: {cls}, rebuilt {part}, Revit's {what}: {cells}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
