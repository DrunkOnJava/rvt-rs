#!/usr/bin/env python3
"""The local axes rvt-rs writes for each door and window against Revit's, by
Tag, with no probe (RE-347, #1447).

`door_placements_vs_ifc.py` explains the same comparison by flip state and by
how an element is bound to its wall, which needs the probe
`probe_re343_door_flips`. This one needs only the two exports, so Measure runs
it on every model with a Revit export, and the base-against-head diff of the
run catches a regression of the placement rule.

For every `IfcDoor` and `IfcWindow` whose Tag both exports hold, it composes the
chain of `IfcLocalPlacement`s, moves the X, Y and Z axes into Revit's internal
frame through each file's own `IfcSite` placement (rotation only), and says how
rvt-rs's axes compare with Revit's: `same`, `half turn` (X and Y reversed, Z the
same), `quarter turn` (X and Y a right angle from Revit's, Z the same) or
`other`. Elements rvt-rs placed by the rule carry
`RvtElementRecordGeometry.PlacementAxesSource`; they are counted apart, and one
that differs from Revit's is a bug: the output then ends `exit 1`.

It prints count lines beginning `door axes:`, in a restricted alphabet and with
no id or name, so a held-out model's lines can leave the runner
(tools/ci/scrub-holdout-results.sh).

Usage:

    python3 tools/re/door_axes_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins) and NumPy.
"""

import collections
import sys

import ifcopenshell
import numpy as np

import door_flips_vs_ifc as flips
from door_placements_vs_ifc import KINDS, VERDICTS, placement, placement_source, tagged, verdict


def main(argv):
    if len(argv) != 3:
        raise SystemExit(__doc__)
    ours, theirs = ifcopenshell.open(argv[1]), ifcopenshell.open(argv[2])
    ours_by_tag, theirs_by_tag = tagged(ours), tagged(theirs)
    to_ours, to_theirs = flips.site_rotation(ours).T, flips.site_rotation(theirs).T
    print(
        f"door axes: the rvt-rs export has {len(ours_by_tag)} doors and windows, the Revit export {len(theirs_by_tag)}; "
        f"in both {len(set(ours_by_tag) & set(theirs_by_tag))}; only in the rvt-rs export {len(set(ours_by_tag) - set(theirs_by_tag))}; "
        f"only in the Revit export {len(set(theirs_by_tag) - set(ours_by_tag))}"
    )
    counts = {kind: collections.Counter() for kind in KINDS}
    ruled = {kind: collections.Counter() for kind in KINDS}
    for tag in sorted(set(ours_by_tag) & set(theirs_by_tag), key=lambda t: (len(t), t)):
        mine, other = ours_by_tag[tag], theirs_by_tag[tag]
        kind = "door" if mine.is_a("IfcDoor") else "window"
        axes_mine, _ = placement(mine, to_ours, ours)
        axes_other, _ = placement(other, to_theirs, theirs)
        found = verdict(axes_mine, axes_other)
        counts[kind][found] += 1
        if placement_source(mine) is not None:
            ruled[kind][found] += 1
    wrong = 0
    for kind in KINDS:
        total = sum(counts[kind].values())
        print(f"door axes: {kind}s paired {total}; " + "; ".join(f"{v} {counts[kind][v]}" for v in VERDICTS))
        written = sum(ruled[kind].values())
        differ = written - ruled[kind]["same"]
        wrong += differ
        print(f"door axes: {kind}s placed by the rule {written}; differ from Revit {differ}")
    print(f"door axes: placed by the rule that differ from Revit {wrong}")
    sys.exit(1 if wrong else 0)


if __name__ == "__main__":
    main(sys.argv)
