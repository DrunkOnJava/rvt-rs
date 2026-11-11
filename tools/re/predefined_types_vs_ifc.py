#!/usr/bin/env python3
"""Score the PredefinedType rvt-rs writes on stairs and their parts, and on the elements an
"Export to IFC As" value redirects, against Revit's own IFC export (#323, RE-417).

Pairs each IfcStair, IfcStairFlight, IfcSlab, IfcMember, IfcFurniture, IfcSolarDevice and
IfcTransportElement of the two files by Tag, and each IfcSpace that is a family instance by
GlobalId (Revit names it `<family>:<type>:<ElementId>`), and counts, per entity, how often the
PredefinedType is the same, differs, is unset in rvt-rs's, or is NOTDEFINED where Revit writes none
(RE-419: the default a furniture element takes), with Revit's values tallied, and names a few that
differ. It also tallies the PredefinedType of the IfcStairFlightType and IfcFurnitureType objects of
both files (RE-419: a type is written once for each predefined type its instances carry). A Revit
export in IFC2X3, which has no PredefinedType on these classes, is not scored and says so.

Needs IfcOpenShell.

Usage:

    python3 tools/re/predefined_types_vs_ifc.py <rvt-rs.ifc> <revit.ifc>
"""

import collections
import re
import sys

import ifcopenshell

CLASSES = (
    "IfcStair",
    "IfcStairFlight",
    "IfcSlab",
    "IfcMember",
    "IfcFurniture",
    "IfcSolarDevice",
    "IfcTransportElement",
    "IfcSpace",
)
TYPE_CLASSES = ("IfcStairFlightType", "IfcFurnitureType")
FAMILY_SPACE_NAME = re.compile(r":\d+$")


def predefined(path):
    model = ifcopenshell.open(path)
    out = {}
    for cls in CLASSES:
        for element in model.by_type(cls):
            if element.is_a() != cls:
                continue
            if cls == "IfcSpace":
                if element.Name and FAMILY_SPACE_NAME.search(element.Name):
                    out[(cls, element.GlobalId)] = getattr(element, "PredefinedType", None) or "-"
            elif element.Tag:
                out[(cls, str(element.Tag))] = getattr(element, "PredefinedType", None) or "-"
    return out


def type_values(path, cls):
    model = ifcopenshell.open(path)
    return collections.Counter(
        getattr(t, "PredefinedType", None) or "-" for t in model.by_type(cls) if t.is_a() == cls
    )


def main():
    if ifcopenshell.open(sys.argv[2]).schema == "IFC2X3":
        print("Revit's export is IFC2X3, which has no PredefinedType on these classes: not scored")
        return
    ours, revit = predefined(sys.argv[1]), predefined(sys.argv[2])
    for cls in TYPE_CLASSES:
        mine, theirs = type_values(sys.argv[1], cls), type_values(sys.argv[2], cls)
        if mine or theirs:
            print(
                f"{cls} PredefinedType: rvt-rs {dict(sorted(mine.items()))}, "
                f"Revit {dict(sorted(theirs.items()))}"
            )
    for cls in CLASSES:
        keys = [k for k in revit if k[0] == cls and k in ours]
        if not keys:
            continue
        tally = collections.Counter()
        values = collections.Counter(revit[k] for k in keys)
        misses = []
        for key in keys:
            mine, theirs = ours[key], revit[key]
            if mine == theirs:
                tally["same"] += 1
            elif mine == "NOTDEFINED" and theirs == "-":
                tally["NOTDEFINED where Revit writes none"] += 1
                misses.append((key[1], mine, theirs))
            elif mine in ("-", "NOTDEFINED"):
                tally["unset in rvt-rs"] += 1
                misses.append((key[1], mine, theirs))
            else:
                tally["differs"] += 1
                misses.append((key[1], mine, theirs))
        print(
            f"{cls}: {len(keys)} in both; same {tally['same']}, differs {tally['differs']}, "
            f"unset in rvt-rs {tally['unset in rvt-rs']}, "
            f"NOTDEFINED where Revit writes none {tally['NOTDEFINED where Revit writes none']}; "
            f"Revit's: {dict(sorted(values.items()))}"
        )
        for tag, mine, theirs in misses[:6]:
            print(f"  {tag}: rvt-rs {mine}, Revit {theirs}")


if __name__ == "__main__":
    main()
