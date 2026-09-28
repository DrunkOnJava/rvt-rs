#!/usr/bin/env python3
"""Compare the transforms `probe_re87_instance_transforms` reads with Revit's
own IFC placements.

Research tool for `reports/element-framing/RE-87-instance-transforms.md`.
For every element of Revit's export whose Tag the probe printed, the
placement is moved into Revit's internal frame through the export's site
placement and compared with the transform the element's data holds:

- how the plan X axes relate: the same, reversed, a right angle apart, or
  none of these (within 1e-6), for upright transforms (Z the model's);
- how far apart the origins are in plan.

Usage:

    python3 tools/re/instance_transforms_vs_ifc.py <transforms.jsonl> <revit-export.ifc>

Needs IfcOpenShell (tested with 0.8.5) and NumPy.
"""

import collections
import json
import sys

import ifcopenshell
import ifcopenshell.util.placement
import numpy as np

FEET = 0.3048
SKIP = ("IfcOpeningElement", "IfcSpatialElement")


def main(argv):
    if len(argv) != 3:
        raise SystemExit(__doc__)
    transforms = {}
    for line in open(argv[1]):
        row = json.loads(line)
        transforms[str(row["id"])] = row
    f = ifcopenshell.open(argv[2])
    site = f.by_type("IfcSite")[0]
    to_internal = np.linalg.inv(ifcopenshell.util.placement.get_local_placement(site.ObjectPlacement))
    axes = collections.Counter()
    origins = collections.Counter()
    seen = set()
    for element in f.by_type("IfcElement"):
        tag = element.Tag
        if any(element.is_a(k) for k in SKIP) or not element.ObjectPlacement or tag not in transforms or tag in seen:
            continue
        seen.add(tag)
        t = transforms[tag]
        if abs(t["z"][2] - 1) > 1e-9:
            axes[("not upright", element.is_a())] += 1
            continue
        placement = to_internal @ ifcopenshell.util.placement.get_local_placement(element.ObjectPlacement)
        revit_x = placement[:2, 0] / np.linalg.norm(placement[:2, 0])
        ours = np.array(t["x"][:2])
        dot = float(ours @ revit_x)
        if abs(dot - 1) < 1e-6:
            kind = "same"
        elif abs(dot + 1) < 1e-6:
            kind = "reversed"
        elif abs(dot) < 1e-6:
            kind = "right angle"
        else:
            kind = "other"
        axes[(kind, element.is_a())] += 1
        gap = float(np.linalg.norm(np.array(t["origin"][:2]) - placement[:2, 3] / FEET))
        origins[("within 0.001 ft" if gap < 1e-3 else "within 2 ft" if gap < 2 else "further", element.is_a())] += 1
    upright = sum(v for (kind, _), v in axes.items() if kind != "not upright")
    agree = sum(v for (kind, _), v in axes.items() if kind in ("same", "reversed", "right angle"))
    print(f"elements of Revit's export with a transform: {len(seen)}; upright {upright}; X axis the same, reversed or a right angle: {agree}")
    by_kind = collections.Counter()
    for (kind, _), v in axes.items():
        by_kind[kind] += v
    print(f"  X axis against Revit's placement X: {dict(by_kind)}")
    by_gap = collections.Counter()
    for (gap, _), v in origins.items():
        by_gap[gap] += v
    print(f"  origin against Revit's placement origin, upright: {dict(by_gap)}")
    for (kind, cls), v in sorted(axes.items(), key=lambda x: -x[1]):
        print(f"    {v:5d} {kind:12s} {cls}")


if __name__ == "__main__":
    main(sys.argv)
