#!/usr/bin/env python3
"""Do the type map's total geometry tags, or the maps of the types a family's
nested instances draw, say which Revit 2015 to 2023 families write `<Unnamed>`
(#1603, RE-487)?

Reads `probes/examples/probe_1603_unnamed_nested_types_before_2024.rs`'s `wnest`
lines and Revit's IFC export. For each door and window with a Tag in both,
whose drawn symbol's map holds geometry mapped to no material (-1) and to which
Revit gives materials, takes whether Revit's material set for the same Tag (the
occurrence's, or its type's, through IfcOpenShell) holds `<Unnamed>` (Revit
2018's IFC2X3 export spells it " <Unnamed>"; the white space around a name is
dropped), and counts it by the symbol's total number of geometry tags and by
what the family's nested instances draw.

Counts only, in fixed wording, for held-out projects
(`tools/ci/scrub-holdout-results.sh` keeps these lines), per IFC class:

    unnamed by total tags before 2024: class IfcWindow total tags N: written
        a, not written b; total tags M: written c, not written d; ...
    unnamed by nested types before 2024: class IfcWindow no nested instance:
        written a, not written b; nested types read, none holds an unset tag:
        written c, not written d; a nested type holds an unset tag: written e,
        not written f; a nested type not read: written g, not written h

A nested type *holds an unset tag* where its own `m_geomTag2MaterialId` maps
some geometry to no material; a nested type is *not read* where the family's
document gave no type record for the one a nested instance draws.

With `--list`, a `differs` line per element, with the symbol and family
ElementIds (the held-out scrub keeps only the count lines).

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).

Usage:

    python3 tools/re/window_door_unnamed_nested_types_vs_ifc.py <probe.txt> <revit.ifc>
"""

import collections
import re
import sys

from element_materials_vs_ifc import by_tag
from window_door_type_maps_vs_ifc import UNNAMED

LINE = re.compile(
    r"^wnest (\d+) (IFC[A-Z]+) map_unset=(0|1|na) found=([01]) symbol=(\d+) "
    r"family=(\d+) tags=(\d+) unset_tags=(\d+) nested=(\d+) nested_read=(\d+) "
    r"nested_unset=(\d+)$"
)


def nested_kind(nested, read, unset):
    """The category of an element's family by what its nested instances draw."""
    if nested == 0:
        return "none"
    if unset > 0:
        return "unset"
    if read < nested:
        return "unread"
    return "clean"


def main():
    listing = "--list" in sys.argv[1:]
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    revit = by_tag(args[1])
    elements = collections.defaultdict(list)
    for line in open(args[0], encoding="utf-8", errors="replace"):
        match = LINE.match(line.strip())
        if not match:
            continue
        (tag, ifc_class, map_unset, found, symbol, family, tags, unset_tags,
         nested, read, unset) = match.groups()
        if map_unset != "1" or tag not in revit:
            continue
        entity, theirs = revit[tag]
        if not theirs:
            continue
        written = any(n.strip() == UNNAMED for n in theirs)
        kind = nested_kind(int(nested), int(read), int(unset))
        elements[entity].append((written, int(tags), kind))
        if listing:
            print(
                f"differs {tag} {entity} {'written' if written else 'not_written'} "
                f"symbol={symbol} family={family} tags={tags} unset_tags={unset_tags} "
                f"found={found} nested={nested} nested_read={read} nested_unset={unset} "
                f"kind={kind}"
            )
    for entity, items in sorted(elements.items()):
        by_tags = collections.defaultdict(lambda: [0, 0])
        by_nested = collections.defaultdict(lambda: [0, 0])
        for written, tags, kind in items:
            slot = 0 if written else 1
            by_tags[tags][slot] += 1
            by_nested[kind][slot] += 1
        print(
            f"unnamed by total tags before 2024: class {entity} "
            + "; ".join(
                f"total tags {n}: written {w}, not written {x}"
                for n, (w, x) in sorted(by_tags.items())
            )
        )
        names = [
            ("none", "no nested instance"),
            ("clean", "nested types read, none holds an unset tag"),
            ("unset", "a nested type holds an unset tag"),
            ("unread", "a nested type not read"),
        ]
        print(
            f"unnamed by nested types before 2024: class {entity} "
            + "; ".join(
                f"{label}: written {by_nested[key][0]}, not written {by_nested[key][1]}"
                for key, label in names
            )
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
