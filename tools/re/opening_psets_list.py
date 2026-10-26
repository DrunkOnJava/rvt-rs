#!/usr/bin/env python3
"""Which of Revit's openings carry Pset_OpeningElementCommon (#35, W7).

Reads Revit's IFC export (the probe's output is ignored, so any probe will
do). For each IfcOpeningElement it takes the element filling it
(HasFillings), the element it voids (VoidsElements) and its property sets,
and counts the openings by whether they carry Pset_OpeningElementCommon,
the entity of the filling element and of the host, and the opening's
PredefinedType and representation type, then lists up to 15 of each group.

Usage:

    python3 tools/re/opening_psets_list.py <probe.txt> <revit.ifc> [--list]

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.util.element


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    f = ifcopenshell.open(args[1])
    groups = collections.defaultdict(list)
    for opening in f.by_type("IfcOpeningElement"):
        psets = ifcopenshell.util.element.get_psets(opening, psets_only=True)
        common = psets.get("Pset_OpeningElementCommon", {})
        fillers = [rel.RelatedBuildingElement for rel in opening.HasFillings or ()]
        hosts = [rel.RelatingBuildingElement for rel in opening.VoidsElements or ()]
        filler = fillers[0].is_a() if fillers else "-"
        host = hosts[0].is_a() if hosts else "-"
        reps = []
        if opening.Representation:
            for rep in opening.Representation.Representations:
                reps.append(rep.RepresentationType or "-")
        key = (
            "set" if common else "no set",
            filler,
            host,
            getattr(opening, "PredefinedType", None) or "-",
            "+".join(reps) or "-",
        )
        fields = ",".join(k for k in sorted(common) if k != "id")
        groups[key].append(
            f"{opening.Tag} {opening.Name!r} fills={fillers[0].Name if fillers else '-'!r} {fields} "
            f"Reference={common.get('Reference', '-')!r} other_sets={sorted(k for k in psets if k != 'Pset_OpeningElementCommon')}"
        )
    print("openings: has Pset_OpeningElementCommon, filling entity, host entity, PredefinedType, representation: count")
    for key, rows in sorted(groups.items(), key=lambda kv: -len(kv[1])):
        print(f"  {key}: {len(rows)}")
    for key, rows in sorted(groups.items(), key=lambda kv: -len(kv[1])):
        print(f"== {key}")
        for row in rows[:15]:
            print(f"  {row}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
