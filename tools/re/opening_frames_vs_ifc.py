#!/usr/bin/env python3
"""Revit's door and window openings in the instance's frame, against the
outline the symbol cuts (#227).

Research tool for `examples/probe_227_opening_frames.rs`. For each
`IfcOpeningElement` filling a door or window whose `frame` line the probe
prints, the opening's vertices are moved into Revit's internal feet through
the export's site placement, as `opening_boxes_vs_ifc.py` does, and then
into the instance's frame (RE-87): `x` along its X axis, `y` along Y, `z`
along Z, from its origin. Their ranges are compared with the cut loop's
(RE-215): the loop spans `x` and `z` and lies at one `y`.

Usage:

    python3 tools/re/opening_frames_vs_ifc.py <probe output> <revit-export.ifc> [--list]

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
    frames = {}
    for line in open(args[0]):
        if line.startswith("frame "):
            _, tag, lo, hi, axes, origin = line.split(" ", 5)
            frames[tag] = (np.array(json.loads(lo)), np.array(json.loads(hi)),
                           np.array(json.loads(axes)), np.array(json.loads(origin)))
    f = ifcopenshell.open(args[1])
    to_internal = site_frame(f)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    tally = collections.Counter()
    rows = []
    for rel in f.by_type("IfcRelFillsElement"):
        filler = rel.RelatedBuildingElement
        tag = str(filler.Tag or "")
        if tag not in frames or filler.is_a() not in ("IfcDoor", "IfcWindow"):
            continue
        try:
            shape = ifcopenshell.geom.create_shape(settings, rel.RelatingOpeningElement)
        except Exception:
            tally[(filler.is_a(), "opening not meshed")] += 1
            continue
        verts = np.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
        if not len(verts):
            continue
        lo, hi, axes, origin = frames[tag]
        local = (to_internal(verts) - origin) @ axes.T
        omin, omax = local.min(axis=0), local.max(axis=0)
        dx = max(abs(omin[0] - lo[0]), abs(omax[0] - hi[0]))
        dz = max(abs(omin[2] - lo[2]), abs(omax[2] - hi[2]))
        dx_mirrored = max(abs(omin[0] + hi[0]), abs(omax[0] + lo[0]))
        cls = filler.is_a()
        tally[(cls, "elements")] += 1
        tally[(cls, f"X range {band(dx)}")] += 1
        tally[(cls, f"X range mirrored {band(dx_mirrored)}")] += 1
        tally[(cls, f"Z range {band(dz)}")] += 1
        plane = lo[1]
        side = "loop plane at the opening's -Y face" if abs(omin[1] - plane) <= 0.01 else \
            "loop plane at its +Y face" if abs(omax[1] - plane) <= 0.01 else \
            "loop plane inside it" if omin[1] < plane < omax[1] else "loop plane outside it"
        tally[(cls, side)] += 1
        rows.append((max(min(dx, dx_mirrored), dz), tag, cls,
                     [round(float(v), 4) for v in (*omin, *omax)],
                     [round(float(v), 4) for v in (*lo, *hi)]))
    print(f"probe: {len(frames)} doors and windows with a cut loop and a transform")
    for cls in sorted({c for c, _ in tally}):
        print(f"\n{cls}: {tally[(cls, 'elements')]}")
        for (c, label), n in sorted(tally.items()):
            if c == cls and label != "elements":
                print(f"  {label}: {n}")
    if listing:
        print("\nworst first (opening min, max in the instance frame; loop min, max):")
        for diff, tag, cls, opening, loop in sorted(rows, reverse=True)[:40]:
            print(f"  {cls} {tag}: {diff:.4f} ft; opening {opening}; loop {loop}")


main()
