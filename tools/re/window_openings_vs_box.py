#!/usr/bin/env python3
"""How a window's opening in Revit's export sits against rvt-rs's, which is
the window's record box where its symbol cuts no loop (#810).

Both files' openings are moved into Revit's internal feet through each
export's own site placement (as `opening_boxes_vs_ifc.py` does). For every
IfcWindow both hold, this prints, grouped, the difference of each face of
Revit's opening box from rvt-rs's, along X, Y and Z (Revit's less rvt-rs's,
min face then max face), rounded to 0.001 ft.

Usage:

    python3 tools/re/window_openings_vs_box.py <rvt-rs.ifc> <revit.ifc>
"""

import collections
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from opening_boxes_vs_ifc import openings  # noqa: E402


def main():
    (ours, _), (revit, _) = openings(sys.argv[1]), openings(sys.argv[2])
    groups = collections.Counter()
    example = {}
    for tag, (rmin, rmax, cls) in revit.items():
        if cls != "IfcWindow" or tag not in ours:
            continue
        omin, omax, _ = ours[tag]
        dmin, dmax = np.round(rmin - omin, 3), np.round(rmax - omax, 3)
        key = (tuple(float(v) for v in dmin), tuple(float(v) for v in dmax))
        groups[key] += 1
        example.setdefault(key, (tag, omax - omin, rmax - rmin))
    print(f"windows in both: {sum(groups.values())}, distinct: {len(groups)}")
    for (dmin, dmax), count in groups.most_common(20):
        tag, ours_size, revit_size = example[(dmin, dmax)]
        print(
            f"{count:4d} min {dmin} max {dmax}; e.g. {tag}: rvt-rs size {np.round(ours_size, 4).tolist()}, "
            f"Revit {np.round(revit_size, 4).tolist()}"
        )


if __name__ == "__main__":
    main()
