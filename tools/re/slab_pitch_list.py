#!/usr/bin/env python3
"""Revit's slabs with their PitchAngle, beside rvt-rs's for the same Tag
(RE-238, #35, W7).

Reads Revit's IFC export and the rvt-rs export next to the probe's output
(`model.ifc` in the probe output's directory; the probe's own output is
ignored). For each IfcSlab Revit writes it prints its Tag, Name,
PredefinedType and Pset_SlabCommon.PitchAngle in degrees, and rvt-rs's
PitchAngle, body representation types and BodySource for that Tag.

Usage:

    python3 tools/re/slab_pitch_list.py <probe.txt> <revit.ifc>
"""

import math
import os
import sys

import ifcopenshell
import ifcopenshell.util.element
import ifcopenshell.util.unit


def pitch(element, radians_per_unit):
    value = ifcopenshell.util.element.get_psets(element, psets_only=True).get("Pset_SlabCommon", {}).get("PitchAngle")
    return None if value is None else math.degrees(value * radians_per_unit)


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    ours_path = os.path.join(os.path.dirname(os.path.abspath(args[0])), "model.ifc")
    revit = ifcopenshell.open(args[1])
    revit_rad = ifcopenshell.util.unit.calculate_unit_scale(revit, "PLANEANGLEUNIT")
    ours = ifcopenshell.open(ours_path) if os.path.exists(ours_path) else None
    ours_rad = ifcopenshell.util.unit.calculate_unit_scale(ours, "PLANEANGLEUNIT") if ours else 1.0
    by_tag = {}
    if ours:
        for element in ours.by_type("IfcElement"):
            by_tag.setdefault(getattr(element, "Tag", None), element)
    for slab in sorted(revit.by_type("IfcSlab"), key=lambda s: str(s.Tag)):
        theirs = pitch(slab, revit_rad)
        mine = by_tag.get(slab.Tag)
        reps = "-"
        source = "-"
        ours_pitch = None
        if mine is not None:
            ours_pitch = pitch(mine, ours_rad)
            if mine.Representation:
                reps = "+".join(r.RepresentationType or "-" for r in mine.Representation.Representations)
            psets = ifcopenshell.util.element.get_psets(mine)
            for props in psets.values():
                if "BodySource" in props:
                    source = props["BodySource"]
        fmt = lambda v: "-" if v is None else f"{v:.4f}"
        print(
            f"slab {slab.Tag} {slab.Name!r} {slab.PredefinedType} revit={fmt(theirs)} "
            f"ours={fmt(ours_pitch)} {mine.is_a() if mine is not None else '-'} {reps} {source}"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
