#!/usr/bin/env python3
"""Door and window types whose `FireRating` rvt-rs writes and Revit's export
leaves out of the type's own common set (#1987, RE-354).

For each door or window type object both exports hold by `Tag` where rvt-rs's
`Pset_DoorCommon` / `Pset_WindowCommon` has a `FireRating` and Revit's has
none, it prints the type's two values side by side, every property whose name
holds "Fire" in any set Revit writes on that type, and, for each of its
occurrences (Revit's `IfcRelDefinesByType`), the occurrence's own
`FireRating` in each export and any other "Fire" property Revit writes on it.
It also prints, for contrast, the types where both exports write the type's
`FireRating`, with their counts.

Usage:

    python3 tools/re/door_type_fire_rating_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>
"""

import sys

import ifcopenshell
import ifcopenshell.util.element

COMMON = {"IfcDoorType": "Pset_DoorCommon", "IfcDoorStyle": "Pset_DoorCommon",
          "IfcWindowType": "Pset_WindowCommon", "IfcWindowStyle": "Pset_WindowCommon"}


def types_by_tag(model):
    out = {}
    for cls in COMMON:
        try:
            found = model.by_type(cls)
        except RuntimeError:
            continue
        for element in found:
            if element.Tag:
                out.setdefault(element.Tag, element)
    return out


def fire(psets):
    return {f"{s}.{p}": v for s, props in psets.items() for p, v in props.items() if "fire" in p.lower()}


def occurrences(type_object):
    out = []
    for rel in getattr(type_object, "Types", None) or getattr(type_object, "ObjectTypeOf", None) or []:
        out.extend(rel.RelatedObjects)
    return out


def main(argv):
    argv = [arg for arg in argv if not arg.startswith("--")]
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    ours_model = ifcopenshell.open(argv[1])
    theirs_model = ifcopenshell.open(argv[2])
    ours = types_by_tag(ours_model)
    theirs = types_by_tag(theirs_model)
    ours_by_tag = {}
    for element in ours_model.by_type("IfcElement"):
        if element.Tag and not element.is_a("IfcOpeningElement"):
            ours_by_tag.setdefault(element.Tag, element)
    both_written = 0
    extra = 0
    for tag in sorted(set(ours) & set(theirs), key=lambda t: (len(t), t)):
        our_type, their_type = ours[tag], theirs[tag]
        common = COMMON[their_type.is_a()]
        our_psets = ifcopenshell.util.element.get_psets(our_type)
        their_psets = ifcopenshell.util.element.get_psets(their_type)
        ours_value = our_psets.get(common, {}).get("FireRating")
        theirs_value = their_psets.get(common, {}).get("FireRating")
        if ours_value is None:
            continue
        if theirs_value is not None:
            both_written += 1
            continue
        extra += 1
        print(
            f"type {tag} {their_type.is_a()} {their_type.Name!r}: rvt-rs {common}.FireRating {ours_value!r}; "
            f"Revit writes {common} {common in their_psets}, its fire properties {fire(their_psets)}"
        )
        for occurrence in occurrences(their_type):
            their_fire = fire(ifcopenshell.util.element.get_psets(occurrence))
            mine = ours_by_tag.get(occurrence.Tag)
            our_fire = fire(ifcopenshell.util.element.get_psets(mine)) if mine is not None else None
            print(f"  occurrence {occurrence.Tag} {occurrence.is_a()}: Revit {their_fire}; rvt-rs {our_fire}")
    print(f"types with rvt-rs's FireRating where Revit's type has none: {extra}; where both write it: {both_written}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
