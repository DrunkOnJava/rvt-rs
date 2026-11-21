#!/usr/bin/env python3
"""How many elements of a Revit export lose a Body item IfcOpenShell cannot process (#2133).

`mesh_bodies_vs_ifc.meshes()` meshes an element as a whole, and IfcOpenShell can succeed on the
element while leaving out one of its items it cannot process, with nothing in `MESH_FAILURES`.
This meshes each Body item of each element (openings, which carry their host's Tag, left out)
on its own and counts the items that fail, by item class and error, and the elements they are
on, by class. It reads only the export; the first argument, a probe's output, is ignored, so
Measure can run it as a `probe_scorer`. Counts only.

    python3 tools/re/item_mesh_census_vs_ifc.py <ignored> <revit.ifc>
"""

import collections
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from boundary_gaps_vs_ifc import body_items  # noqa: E402


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        return 2
    import ifcopenshell
    import ifcopenshell.geom

    settings = ifcopenshell.geom.settings()
    settings.set("weld-vertices", False)
    model = ifcopenshell.open(sys.argv[2])
    failed = collections.Counter()
    losing = collections.Counter()
    elements = items = 0
    for element in model.by_type("IfcElement"):
        if element.is_a("IfcOpeningElement") or not element.Representation:
            continue
        found = body_items(element)
        if not found:
            continue
        elements += 1
        lost = False
        for item, _ in found:
            items += 1
            try:
                ifcopenshell.geom.create_shape(settings, item)
            except Exception as e:  # IfcOpenShell raises RuntimeError on a shape it cannot build
                failed[(item.is_a(), type(e).__name__)] += 1
                lost = True
        if lost:
            losing[element.is_a()] += 1
    print(f"ifcopenshell {ifcopenshell.version}")
    print(f"item mesh census: elements with a Body {elements}, Body items {items}; "
          f"items IfcOpenShell cannot process {sum(failed.values())} on {sum(losing.values())} elements")
    if failed:
        print("items that fail: " + ", ".join(f"{k} {e} {n}" for (k, e), n in sorted(failed.items())))
        print("elements that lose one: " + ", ".join(f"{k} {n}" for k, n in sorted(losing.items())))
    return 0


if __name__ == "__main__":
    sys.exit(main())
