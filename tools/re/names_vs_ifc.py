#!/usr/bin/env python3
"""Compare the Name and ObjectType of each element in an rvt-rs IFC with
Revit's own IFC export of the same file.

Research tool for `reports/element-framing/RE-63-system-family-element-names.md`
and `RE-64-panel-walls-slab-edges-ramps.md`.

Elements are matched by `Tag`, the Revit ElementId. An `IfcOpeningElement`
carries the Tag of the element that cuts it, so openings are left out. An
element counts as a system-family element when Revit's Name starts with a
system family (Basic Wall, Curtain Wall, Floor, Pad, Slab Edge, Ramp,
Compound Ceiling, Basic Roof, Railing, ...), and as a family instance otherwise.

Usage:

    python3 tools/re/names_vs_ifc.py [-v] <rvt-rs.ifc> <revit-export.ifc>

`-v` prints the system-family elements whose Name differs. Needs
IfcOpenShell (tested with 0.8.5).
"""

import collections
import sys

import ifcopenshell

SYSTEM_FAMILIES = {
    "Basic Wall",
    "Curtain Wall",
    "Stacked Wall",
    "Floor",
    "Foundation Slab",
    "Pad",
    "Slab Edge",
    "Ramp",
    "Compound Ceiling",
    "Basic Ceiling",
    "Basic Roof",
    "Sloped Glazing",
    "Railing",
}


def tag(entity):
    try:
        return entity.Tag
    except AttributeError:
        return None


def main():
    args = [a for a in sys.argv[1:] if a != "-v"]
    verbose = "-v" in sys.argv[1:]
    if len(args) != 2:
        raise SystemExit(__doc__)
    theirs = {
        tag(e): e
        for e in ifcopenshell.open(args[1]).by_type("IfcElement")
        if tag(e) and not e.is_a("IfcOpeningElement")
    }
    counts = collections.Counter()
    for element in ifcopenshell.open(args[0]).by_type("IfcElement"):
        if element.is_a("IfcOpeningElement"):
            continue
        revit = theirs.get(tag(element))
        if revit is None:
            continue
        family = (revit.Name or "").split(":")[0]
        kind = "system family" if family in SYSTEM_FAMILIES else "family instance"
        name = "Name equal" if element.Name == revit.Name else "Name differs"
        object_type = (
            "ObjectType equal"
            if getattr(element, "ObjectType", None) == getattr(revit, "ObjectType", None)
            else "ObjectType differs"
        )
        counts[(kind, element.is_a(), name, object_type)] += 1
        if verbose and kind == "system family" and element.Name != revit.Name:
            print(f"    {tag(element)} {element.is_a()}: {element.Name} vs Revit's {revit.Name}")
    for key, count in sorted(counts.items(), key=lambda kv: -kv[1]):
        print(f"  {count:>5}  {' | '.join(key)}")


if __name__ == "__main__":
    main()
