#!/usr/bin/env python3
"""Where an rvt-rs stair flight's body differs from Revit's (#357).

For a few flights of the BIM4LCA office that `stair_bodies_vs_ifc.py` finds
short of Revit's volume, and two that match, this prints, in Revit's
internal feet and each file's own site frame:

- every vertex of Revit's body more than 0.001 ft from rvt-rs's surface,
  with its distance;
- every vertex of rvt-rs's body more than 0.001 ft from Revit's surface;
- both bodies' boxes.

Needs IfcOpenShell and NumPy.

Usage:

    python3 tools/re/flight_junctions_vs_ifc.py <rvt-rs.ifc> <revit.ifc>
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from roof_bodies_vs_ifc import surface_distances  # noqa: E402
from stair_bodies_vs_ifc import bodies  # noqa: E402

# One stair's three flights (2859203: first, middle, last), and two flights
# that match, for contrast.
SHOWN = ("2859204", "2859206", "2859210", "2861134", "2861136")


def main():
    ours = bodies(sys.argv[1], "IfcStairFlight")
    ref = bodies(sys.argv[2], "IfcStairFlight")
    for tag in SHOWN:
        if tag not in ours or tag not in ref:
            print(f"{tag}: not in both")
            continue
        (our_v, our_vol, our_t), (ref_v, ref_vol, ref_t) = ours[tag], ref[tag]
        print(f"{tag}: volume rvt-rs {our_vol:.6f}, Revit {ref_vol:.6f}")
        for name, verts in (("rvt-rs", our_v), ("Revit", ref_v)):
            lo = [min(v[k] for v in verts) for k in range(3)]
            hi = [max(v[k] for v in verts) for k in range(3)]
            print(f"  {name} box {' '.join(f'{c:.4f}' for c in lo)} to {' '.join(f'{c:.4f}' for c in hi)}")
        for v, d in zip(ref_v, surface_distances(ref_v, our_t)):
            if d > 0.001:
                print(f"  Revit vertex off rvt-rs's surface: {v[0]:.4f} {v[1]:.4f} {v[2]:.4f} by {d:.4f}")
        for v, d in zip(our_v, surface_distances(our_v, ref_t)):
            if d > 0.001:
                print(f"  rvt-rs vertex off Revit's surface: {v[0]:.4f} {v[1]:.4f} {v[2]:.4f} by {d:.4f}")


if __name__ == "__main__":
    main()
