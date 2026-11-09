#!/usr/bin/env python3
"""Each distribution port in two rvt-rs exports of the same project saved by
two Revit releases, port for port (RE-346).

A port is paired by its host's `Tag` (ElementId), its `Name` and its
`FlowDirection`. A pair is the same where the world origins agree to 3 mm
and the port axes (placement Z) to 1e-3.

As with `save_pair_bodies.py`, a 2026 or 2027 save is scored against the same
Autodesk sample saved by Revit 2024 or 2025.

Usage:

    python3 tools/re/save_pair_ports.py <candidate.ifc> <reference.ifc>
"""

import argparse
import sys

import ifcopenshell
import ifcopenshell.util.placement
import numpy as np

ORIGIN_TOLERANCE_M = 0.003
AXIS_TOLERANCE = 1e-3


def ports(path):
    """(host Tag, Name, FlowDirection) -> [(origin, axis)]."""
    model = ifcopenshell.open(path)
    out = {}
    for rel in model.by_type("IfcRelNests"):
        tag = getattr(rel.RelatingObject, "Tag", None)
        if not tag:
            continue
        for port in rel.RelatedObjects:
            if not port.is_a("IfcDistributionPort") or not port.ObjectPlacement:
                continue
            matrix = ifcopenshell.util.placement.get_local_placement(port.ObjectPlacement)
            key = (tag, port.Name, port.FlowDirection)
            out.setdefault(key, []).append((matrix[:3, 3], matrix[:3, 2]))
    return out


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("candidate")
    parser.add_argument("reference")
    args = parser.parse_args()
    candidate = ports(args.candidate)
    reference = ports(args.reference)
    same = differ = mine = theirs = 0
    for key, placed in candidate.items():
        if key not in reference:
            mine += len(placed)
            continue
        order = lambda p: tuple(np.round(p[0], 6))
        placed, others = sorted(placed, key=order), sorted(reference[key], key=order)
        for (origin, axis), (origin2, axis2) in zip(placed, others):
            if np.abs(origin - origin2).max() <= ORIGIN_TOLERANCE_M and np.abs(axis - axis2).max() <= AXIS_TOLERANCE:
                same += 1
            else:
                differ += 1
        # A key with more ports on one side than the other: the extra ones are on that side only.
        mine += max(len(placed) - len(others), 0)
        theirs += max(len(others) - len(placed), 0)
    theirs += sum(len(placed) for key, placed in reference.items() if key not in candidate)
    print(f"same {same}, differ {differ}, only candidate {mine}, only reference {theirs}")
    if differ or mine or theirs:
        sys.exit(1)


if __name__ == "__main__":
    main()
