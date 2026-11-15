#!/usr/bin/env python3
"""Whether the id a Revit 2015 to 2023 instance holds that only its category
holds is the element's type in Revit's own export (#1603, RE-474).

`probe_1603_unjoined_lists_pre2024` prints, for each instance whose reference
list holds no symbol record of its category, the named ids with no record of
their own that only its category's records hold:

    unjoined instance: <ElementId> category <id> candidates <id>=<name> ...

This scorer reads those lines and Revit's export of the same file. Revit's
element of the same Tag (the ElementId) gives its type through
IfcRelDefinesByType, whose Tag is the type's ElementId and whose Name is
`Family:Type`. For each instance with exactly one candidate it counts whether
the candidate's id is the type's Tag and whether the candidate's name is the
type's, the family's or the whole `Family:Type`, with underscores and spaces
read alike.

A candidate id that is the type's Tag would give the element its type without
a symbol record; a name that is the family's says the id is a family. The
export is IFC2X3 for most samples, and a type there is an IfcWindowStyle or
IfcDoorStyle: only the Tag and Name are read.

Usage:
    python3 tools/re/unjoined_types_vs_ifc.py <probe.txt> <revit.ifc>
"""

import collections
import re
import sys

import ifcopenshell

LINE = re.compile(r"^unjoined instance: (\d+) category (-?\d+) candidates(.*)$")
SHOWN = 12


def norm(text: str) -> str:
    return re.sub(r"[\s_]+", "_", text.strip())


def main() -> int:
    # Measure passes `--list` to every probe scorer; the detail lines print anyway.
    args = [arg for arg in sys.argv[1:] if arg != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    instances = {}
    with open(args[0], errors="replace") as probe:
        for line in probe:
            found = LINE.match(line.rstrip("\n"))
            if found:
                tag, category, rest = found.groups()
                candidates = [
                    tuple(part.split("=", 1)) for part in rest.split() if "=" in part
                ]
                instances[tag] = (category, candidates)
    revit = ifcopenshell.open(args[1])
    typed = {}
    for rel in revit.by_type("IfcRelDefinesByType"):
        for element in rel.RelatedObjects:
            typed[element.id()] = rel.RelatingType
    ours = {}
    for element in revit.by_type("IfcElement"):
        if element.Tag in instances:
            ours.setdefault(element.Tag, element)
    summary = collections.defaultdict(collections.Counter)
    shown = collections.Counter()
    for tag, (category, candidates) in sorted(
        instances.items(), key=lambda item: int(item[0])
    ):
        counts = summary[category]
        counts["instances"] += 1
        element = ours.get(tag)
        if element is None:
            counts["not in Revit's export"] += 1
            continue
        kind = typed.get(element.id())
        if kind is None:
            counts["Revit writes no type"] += 1
            continue
        counts["typed by Revit"] += 1
        counts[f"candidates {min(len(candidates), 3)}"] += 1
        if len(candidates) != 1:
            continue
        counts["exactly one candidate"] += 1
        candidate_id, candidate_name = candidates[0]
        full = kind.Name or ""
        family, _, kind_name = full.partition(":")
        is_tag = str(getattr(kind, "Tag", None)) == candidate_id
        counts["the candidate is the type's Tag"] += is_tag
        name = norm(candidate_name)
        verdict = "neither"
        if name == norm(kind_name):
            verdict = "type name"
        elif name == norm(family):
            verdict = "family name"
        elif name == norm(full):
            verdict = "Family:Type"
        counts[
            f"candidate name is the {verdict}"
            if verdict != "neither"
            else "candidate name is neither"
        ] += 1
        if not is_tag and shown[category] < SHOWN:
            shown[category] += 1
            print(
                f"  not the type: {tag} {category} candidate {candidate_id}={candidate_name!r} "
                f"Revit type {getattr(kind, 'Tag', None)} {full!r}"
            )
    for category in sorted(summary):
        print(
            f"category {category}: "
            + ", ".join(
                f"{key} {value}" for key, value in sorted(summary[category].items())
            )
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
