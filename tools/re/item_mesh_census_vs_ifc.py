#!/usr/bin/env python3
"""How many elements of a Revit export lose a Body item IfcOpenShell cannot process, and whether
`mesh_parts` names each of them (#2133).

IfcOpenShell can mesh an element whole while leaving out a Body item it cannot process, and say
nothing: on the BIM4LCA housing, 0.9.0.post1 meshes the basin `Allas LE:PA-2.LE` without its
largest face set. This counts such items twice, and the two counts must agree:

1. **The census.** Each Body item of each `IfcElement` but openings (which carry their host's
   Tag), mapped items replaced by the items they map, is meshed on its own; the items that raise
   are counted by class and error, and the elements that hold one by class and type. The openings'
   and the spaces' Body items, which scorers of openings and rooms mesh, are counted the same way on
   a line of their own.
2. **The helper.** `mesh_bodies_vs_ifc.meshes()` meshes every element with a Tag, and
   `ifc_common.mesh_parts` names in `ITEM_FAILURES` the Body items each mesh leaves out that raise
   on their own (`lost_items`). An item the element's own opening cuts away is left out of the
   mesh too, but meshes alone, so neither count holds it.

The last line says whether the helper named every item the census found on the parts it meshed,
and no other. Counts only; the first argument, a probe's output, is ignored, so Measure can run it
as a `probe_scorer`.

    python3 tools/re/item_mesh_census_vs_ifc.py <ignored> <revit.ifc>
"""

import collections
import os
import sys

import ifcopenshell
import ifcopenshell.geom

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ifc_common import ITEM_FAILURES, body_leaves  # noqa: E402
from mesh_bodies_vs_ifc import meshes  # noqa: E402
from roof_bodies_vs_ifc import shapes_of  # noqa: E402


def census(model, settings, products=None):
    """(product count, products with a Body, Body items, {(product id, item id): (class, error)}),
    over every `IfcElement` but openings, or over `products`."""
    failed = {}
    elements = bodied = items = 0
    chosen = products if products is not None else [e for e in model.by_type("IfcElement") if not e.is_a("IfcOpeningElement")]
    for element in chosen:
        elements += 1
        leaves = body_leaves(element)
        if not leaves:
            continue
        bodied += 1
        for item in leaves:
            items += 1
            try:
                ifcopenshell.geom.create_shape(settings, item)
            except RuntimeError as error:
                failed[(element.id(), item.id())] = (item.is_a(), type(error).__name__)
    return elements, bodied, items, failed


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        return 2
    path = sys.argv[2]
    model = ifcopenshell.open(path)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    elements, bodied, items, failed = census(model, settings)
    print(f"ifcopenshell {ifcopenshell.version}")
    losing = collections.Counter((model.by_id(p).is_a(), model.by_id(p).ObjectType or "") for p in {p for p, _ in failed})
    print(f"item census: IfcElement but openings {elements}, with a Body {bodied}, Body items {items}; "
          f"items IfcOpenShell cannot process {len(failed)} on {len({p for p, _ in failed})} elements")
    if failed:
        kinds = collections.Counter(failed.values())
        print("items that fail: " + ", ".join(f"{k} {e} {n}" for (k, e), n in sorted(kinds.items())))
        print("elements that lose one: " + ", ".join(f"{k} {t!r} {n}" for (k, t), n in sorted(losing.items())))
    others = model.by_type("IfcOpeningElement") + model.by_type("IfcSpace")
    o_all, o_bodied, o_items, o_failed = census(model, settings, others)
    o_losing = collections.Counter(model.by_id(p).is_a() for p in {p for p, _ in o_failed})
    print(f"openings and spaces: {o_all}, with a Body {o_bodied}, Body items {o_items}; "
          f"items IfcOpenShell cannot process {len(o_failed)} on {sum(o_losing.values())} of them"
          + (": " + ", ".join(f"{k} {n}" for k, n in sorted(o_losing.items())) if o_losing else ""))
    # The helper meshes the elements with a Tag and their aggregated parts: the census is held
    # to the same parts.
    meshed = {part.id() for e in model.by_type("IfcElement")
              if e.Tag and not e.is_a("IfcOpeningElement") for part in shapes_of(e)}
    meshes(path, model=model, settings=settings)
    named = {(part, item) for part, lost in ITEM_FAILURES.items() for item, _, _ in lost}
    expected = {key for key in failed if key[0] in meshed}
    print(f"helper: parts meshed {len(meshed)}; Body items named as lost {len(named)} on {len({p for p, _ in named})} parts; "
          f"census items on those parts {len(expected)}")
    if named == expected:
        print("helper agrees with the census: every item that fails alone on a meshed part is named, and no other")
        return 0
    print(f"helper DISAGREES with the census: {len(expected - named)} census items not named, {len(named - expected)} named but not in the census")
    return 1


if __name__ == "__main__":
    sys.exit(main())
