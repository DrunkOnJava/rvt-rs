#!/usr/bin/env python3
"""Compare Revit's `OverallWidth` and `OverallHeight` of each door and window with
the extents of the body rvt-rs draws it with, along the element's own X and Z
(RE-409).

For each `IfcDoor` and `IfcWindow` both files hold under one `Tag` (the Revit
ElementId) where Revit's export gives both sizes, it reads

- the `OverallWidth` and `OverallHeight` rvt-rs writes: equal to Revit's within
  0.001 ft, different, or `$`;
- the extent of rvt-rs's Body representation, meshed by IfcOpenShell in the
  element's own frame (the placement left out), along X (the width) and along Z
  (the height): equal to Revit's sizes within 0.001 ft, or different.

and counts the elements by those two readings per class, then by the class of the
element each fills an opening in, in each export. A cell whose first
reading differs and whose second is equal is a size the body would have
given; a cell whose first is equal and second differs is one it would have
broken. It then lists up to 40 elements of each cell that is not equal on
both, with the sizes.

Usage:

    python3 tools/re/door_window_mesh_extents_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>

Needs IfcOpenShell and numpy (the versions tools/ci/requirements-ifc.txt pins).
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.unit
import numpy

FEET = 0.3048
TOLERANCE_FEET = 1e-3
LIST_LIMIT = 40


def sizes(model):
    """(class, Tag) -> (OverallWidth ft, OverallHeight ft or None) of every door and window."""
    metres = ifcopenshell.util.unit.calculate_unit_scale(model)
    out = {}
    for entity in ("IfcDoor", "IfcWindow"):
        for element in model.by_type(entity):
            if not element.Tag:
                continue
            width, height = element.OverallWidth, element.OverallHeight
            out[(entity, str(element.Tag))] = (
                None if width is None else width * metres / FEET,
                None if height is None else height * metres / FEET,
            )
    return out


def body_extents(element, settings, metres):
    """The extents of `element`'s Body along its own X and Z, feet, or None."""
    if element.Representation is None:
        return None
    try:
        shape = ifcopenshell.geom.create_shape(settings, element)
    except Exception:  # noqa: BLE001 - IfcOpenShell raises its own types
        return None
    verts = numpy.array(shape.geometry.verts).reshape(-1, 3)
    if len(verts) == 0:
        return None
    span = (verts.max(0) - verts.min(0)) * metres / FEET
    return float(span[0]), float(span[2])


def host_class(element):
    """The class of the element `element` fills an opening in, or `-`."""
    for fills in getattr(element, "FillsVoids", None) or ():
        opening = fills.RelatingOpeningElement
        for voids in getattr(opening, "VoidsElements", None) or ():
            return voids.RelatingBuildingElement.is_a()
    return "-"


def fmt(pair):
    """`width x height` of a pair of feet, `none` where there is none."""
    return "none" if pair is None else " x ".join("-" if v is None else f"{v:.4f}" for v in pair)


def equal(a, b):
    return a is not None and b is not None and abs(a - b) <= TOLERANCE_FEET


def main(argv):
    args = argv[1:]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    ours = ifcopenshell.open(args[0])
    revit_model = ifcopenshell.open(args[1])
    revit = sizes(revit_model)
    revit_hosts = {
        (entity, str(element.Tag)): host_class(element)
        for entity in ("IfcDoor", "IfcWindow")
        for element in revit_model.by_type(entity)
        if element.Tag
    }
    written = sizes(ours)
    metres = ifcopenshell.util.unit.calculate_unit_scale(ours)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", False)
    tally = collections.Counter()
    hosts = collections.Counter()
    rows = collections.defaultdict(list)
    for entity in ("IfcDoor", "IfcWindow"):
        for element in ours.by_type(entity):
            key = (entity, str(element.Tag))
            if not element.Tag or key not in revit:
                continue
            width, height = revit[key]
            if width is None or height is None:
                continue
            own = written.get(key, (None, None))
            if own[0] is None or own[1] is None:
                current = "$"
            else:
                current = "equal" if equal(own[0], width) and equal(own[1], height) else "differs"
            extents = body_extents(element, settings, metres)
            if extents is None:
                body = "no body"
            else:
                body = "equal" if equal(extents[0], width) and equal(extents[1], height) else "differs"
            tally[(entity, current, body)] += 1
            hosts[(entity, current, body, host_class(element), revit_hosts.get(key, "-"))] += 1
            if (current, body) != ("equal", "equal"):
                rows[(entity, current, body)].append((element.Tag, (width, height), own, extents))
    for (entity, current, body), count in sorted(tally.items()):
        print(f"{entity}: size written {current}, body extents {body}: {count}")
    for (entity, current, body, ours_host, revit_host), count in sorted(hosts.items()):
        if (current, body) != ("equal", "equal"):
            print(
                f"{entity}: size written {current}, body extents {body}, hosted in rvt-rs's {ours_host}"
                f" and Revit's {revit_host}: {count}"
            )
    for (entity, current, body), items in sorted(rows.items()):
        print(f"{entity} {current} / {body}:")
        for tag, revit_size, own, extents in items[:LIST_LIMIT]:
            print(f"  {tag}: Revit {fmt(revit_size)}, written {fmt(own)}, body {fmt(extents)} ft")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
