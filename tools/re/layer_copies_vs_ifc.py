#!/usr/bin/env python3
"""Which copy of a host type's layers Revit's export holds (#1805, RE-446).

Reads the output of `probes/examples/probe_1805_host_type_layer_copies.rs`
(each wall, floor, roof or ceiling type whose decoded copies of its layers
disagree, with every copy and the type's elements) and Revit's own IFC export
of the same file. For every element of such a type that Revit's export gives a
material layer set, it says whether the first, the latest and the current copy
hold Revit's widths (each layer within 0.5 mm, in order, either way round) and
Revit's widths and materials (each layer's material name equal, a layer the
copy gives no material read as matching any). The current copy is the one
RE-446 takes on Revit 2024 and later; on Revit 2008 to 2023 the export drops
such a type, and this says whether taking the current copy there would be
right.

Prints counts only, as `layer copies:` lines (the held-out scrub keeps them),
never an ElementId or a name:

    layer copies: release R; types whose copies disagree N; with a current copy C; elements E; in Revit's export with a layer set L
    layer copies: CLASS elements with Revit's layer set L; widths equal first a latest b current c; widths and materials equal first d latest e current f; no copy holds Revit's widths g

Usage:

    python3 tools/re/layer_copies_vs_ifc.py <probe output> <revit-export.ifc>
"""

import collections
import json
import sys

import ifcopenshell
import ifcopenshell.util.element
import ifcopenshell.util.unit

# Two layer widths are one where they differ by less than this, mm (Revit's export writes them to its own rounding).
WIDTH_MM = 0.5
ROLES = ("first", "latest", "current")


def revit_layers(element, unit_mm):
    """[(width mm, material name or None)] of the element's material layer set, or None. Revit 2024's
    export gives an occurrence an `IfcMaterialConstituentSet` and its type object the layer set
    (the BIM4LCA files), so the type's is read where the occurrence has none."""
    material = ifcopenshell.util.element.get_material(element, should_skip_usage=True)
    if material is None or not material.is_a("IfcMaterialLayerSet"):
        kind = ifcopenshell.util.element.get_type(element)
        material = (
            ifcopenshell.util.element.get_material(kind, should_skip_usage=True) if kind is not None else None
        )
    if material is None or not material.is_a("IfcMaterialLayerSet"):
        return None
    return [
        (layer.LayerThickness * unit_mm, layer.Material.Name if layer.Material else None)
        for layer in material.MaterialLayers
    ]


def same_widths(copy, revit):
    if len(copy) != len(revit):
        return False
    return all(abs(a["mm"] - b[0]) <= WIDTH_MM for a, b in zip(copy, revit))


def same_materials(copy, revit):
    return all(a["material"] is None or a["material"] == b[1] for a, b in zip(copy, revit))


def holds(copy, revit, materials):
    """Whether the copy's layers are Revit's, top first or bottom first."""
    for order in (copy, list(reversed(copy))):
        if same_widths(order, revit) and (not materials or same_materials(order, revit)):
            return True
    return False


def main(argv):
    # Measure passes `--list` to every probe scorer; this one prints counts only, so a flag is ignored.
    argv = [arg for arg in argv if not arg.startswith("--")]
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    release = "?"
    types = []
    with open(argv[1], encoding="utf-8") as probe:
        for line in probe:
            if line.startswith("file "):
                release = line.split()[-1]
            elif line.startswith("json "):
                types.append(json.loads(line[5:]))
    revit = ifcopenshell.open(argv[2])
    unit_mm = ifcopenshell.util.unit.calculate_unit_scale(revit) * 1000.0
    by_tag = {}
    for element in revit.by_type("IfcElement"):
        if element.Tag:
            by_tag.setdefault(element.Tag, element)
    elements = sum(len(t["elements"]) for t in types)
    tallies = collections.defaultdict(collections.Counter)
    kinds = collections.Counter()
    with_set = 0
    for kind in types:
        copies = {role: None for role in ROLES}
        for copy in kind["copies"]:
            for role in copy["role"]:
                copies[role] = copy["layers"]
        for element_id in kind["elements"]:
            element = by_tag.get(str(element_id))
            if element is None:
                kinds["not found by Tag"] += 1
            else:
                material = ifcopenshell.util.element.get_material(element, should_skip_usage=True)
                kind = ifcopenshell.util.element.get_type(element)
                on_type = (
                    ifcopenshell.util.element.get_material(kind, should_skip_usage=True) if kind is not None else None
                )
                kinds[material.is_a() if material is not None else "no material"] += 1
                kinds["on its type " + (on_type.is_a() if on_type is not None else "none")] += 1
            layers = revit_layers(element, unit_mm) if element is not None else None
            if layers is None:
                continue
            with_set += 1
            tally = tallies[kind["class"]]
            tally["set"] += 1
            for role in ROLES:
                if copies[role] is not None and holds(copies[role], layers, False):
                    tally[f"widths {role}"] += 1
                if copies[role] is not None and holds(copies[role], layers, True):
                    tally[f"both {role}"] += 1
            if not any(holds(copy["layers"], layers, False) for copy in kind["copies"]):
                tally["none"] += 1
            # A layer the copy gives no material of its own takes its category's in the export
            # (RE-91, RE-115), which need not be Revit's: count the copies that name every layer.
            if copies["current"] is not None:
                unnamed = sum(1 for layer in copies["current"] if layer["material"] is None)
                tally["current layers with no material of their own"] += unnamed
                tally["current copy names every layer"] += int(unnamed == 0)
    print(
        f"layer copies: release {release}; types whose copies disagree {len(types)}; "
        f"with a current copy {sum(1 for t in types if t['current'] is not None)}; "
        f"elements {elements}; in Revit's export with a layer set {with_set}"
    )
    # What Revit's export gives the elements, so a count of no layer set can be told apart from a
    # Tag that is not found: entity names and counts only.
    print("layer copies: Revit's material on those elements: " + ", ".join(f"{k} {n}" for k, n in sorted(kinds.items())))
    for kind, tally in sorted(tallies.items()):
        widths = " ".join(f"{role} {tally[f'widths {role}']}" for role in ROLES)
        both = " ".join(f"{role} {tally[f'both {role}']}" for role in ROLES)
        print(
            f"layer copies: {kind} elements with Revit's layer set {tally['set']}; widths equal {widths}; "
            f"widths and materials equal {both}; no copy holds Revit's widths {tally['none']}; "
            f"current copy names every layer {tally['current copy names every layer']}, "
            f"layers with no material of their own {tally['current layers with no material of their own']}"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
