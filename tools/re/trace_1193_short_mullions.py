#!/usr/bin/env python3
"""Research trace for #1193 (not to land): each curtain mullion whose
`Pset_MemberCommon.Span` Revit's export writes and rvt-rs's does not, with both
files' Span, `Qto_MemberBaseQuantities` Length and NetVolume, the placement's
axes in Revit's internal frame, how far the mullion's X axis is turned about
its Z off the model's axes, and rvt-rs's element-record properties.

    python3 tools/re/trace_1193_short_mullions.py <rvt-rs.ifc> <revit-export.ifc>
"""

import math
import os
import sys

import ifcopenshell
import ifcopenshell.util.element
import ifcopenshell.util.placement
import ifcopenshell.util.unit
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ifc_common import FEET, site_frame  # noqa: E402


def members(path):
    f = ifcopenshell.open(path)
    frame = site_frame(f)
    unit = ifcopenshell.util.unit.calculate_unit_scale(f)
    out = {}
    for m in f.by_type("IfcMember"):
        if not m.Tag:
            continue
        psets = ifcopenshell.util.element.get_psets(m)
        def feet(value, power=1):
            return None if value is None else value * unit**power / FEET**power
        mat = np.array(ifcopenshell.util.placement.get_local_placement(m.ObjectPlacement), dtype=float)
        o = mat[:3, 3] * unit
        pts = frame(np.array([o, o + mat[:3, 0], o + mat[:3, 1], o + mat[:3, 2]]))
        axes = [(pts[i] - pts[0]) / np.linalg.norm(pts[i] - pts[0]) for i in (1, 2, 3)]
        out[m.Tag] = {
            "span": feet(psets.get("Pset_MemberCommon", {}).get("Span")),
            "length": feet(psets.get("Qto_MemberBaseQuantities", {}).get("Length")),
            "volume": feet(psets.get("Qto_MemberBaseQuantities", {}).get("NetVolume"), 3),
            "origin": pts[0],
            "axes": axes,
            "record": psets.get("RvtElementRecordGeometry", {}),
            "type": m.ObjectType,
        }
    return out


def turn_about_z(axes):
    """Degrees the X axis lies off the nearest model axis, about the mullion's Z."""
    x, _, z = axes
    if abs(z[2]) > 0.999:  # vertical mullion: X's plan angle
        a = math.degrees(math.atan2(x[1], x[0]))
    elif abs(z[2]) < 1e-3:  # horizontal mullion: X against the vertical
        a = math.degrees(math.asin(max(-1.0, min(1.0, x[2]))))
    else:
        return None
    return (a + 45.0) % 90.0 - 45.0


def main(argv):
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    ours, revit = members(argv[1]), members(argv[2])
    missing = sorted(tag for tag, m in revit.items() if m["span"] is not None and tag in ours and ours[tag]["span"] is None)
    print(f"trace 1193: mullions with Revit's Span and none in rvt-rs's: {len(missing)}")
    fmt = lambda v: "-" if v is None else f"{v:.6f}"
    vec = lambda v: " ".join(f"{c:+.4f}" for c in v)
    for tag in missing:
        r, o = revit[tag], ours[tag]
        print(f"trace 1193 {tag} type {r['type']!r}: Revit span {fmt(r['span'])} length {fmt(r['length'])} volume {fmt(r['volume'])}; rvt-rs length {fmt(o['length'])} volume {fmt(o['volume'])}")
        for side, m in (("Revit", r), ("rvt-rs", o)):
            t = turn_about_z(m["axes"])
            print(f"  {side} origin {vec(m['origin'])} x {vec(m['axes'][0])} z {vec(m['axes'][2])} turn about z {'-' if t is None else f'{t:+.4f}'} deg")
        print("  rvt-rs record " + "; ".join(f"{k}={v}" for k, v in sorted(o["record"].items()) if k != "id"))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
