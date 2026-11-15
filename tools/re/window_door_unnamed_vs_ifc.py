#!/usr/bin/env python3
"""What decides whether Revit writes `<Unnamed>` for a door or window of a
Revit 2015 to 2023 file (#1603, RE-482)?

Reads `probes/examples/probe_1603_unnamed_before_2024.rs`'s `wfam` lines and
Revit's IFC export. For each door and window with a Tag in both, whose drawn
symbol's map holds geometry mapped to no material (-1), counts whether Revit's
material set for the same Tag (the occurrence's, or its type's, through
IfcOpenShell) holds `<Unnamed>` (Revit 2018's IFC2X3 export spells it
" <Unnamed>"; the white space around a name is dropped), by what the family's
forms show:

- *unset form shown*: a form with no material that is not a void and whose
  stored `Visible` is not 0.
- *unset form hidden*: none shown, one whose stored `Visible` is 0.
- *unset voids only*: only voids have no material.
- *no unset form, nested instance*: no form has none, and the family nests an
  instance.
- *no unset form*: no form has none, and it nests no instance.
- *family not read*: the family's document gave no decoded record.

Counts only, in fixed wording, for held-out projects
(`tools/ci/scrub-holdout-results.sh` keeps these lines), per IFC class and
again for the grouped doors and windows with `grouped` after the class:

    unnamed before 2024: class IfcWindow map unset N; unset form shown: written
        A, not written B; unset form hidden: written C, not written D; unset
        voids only: written E, not written F; no unset form, nested instance:
        written G, not written H; no unset form: written I, not written J;
        family not read: written K, not written L

With `--list`, a `differs` line per element, with Tags and the family's counts
(the held-out scrub keeps only the count lines).

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).

Usage:

    python3 tools/re/window_door_unnamed_vs_ifc.py <probe.txt> <revit.ifc>
"""

import collections
import re
import sys

from element_materials_vs_ifc import by_tag
from window_door_type_maps_vs_ifc import UNNAMED

LINE = re.compile(
    r"^wfam (\d+) (IFC[A-Z]+) grouped=([01]) map_unset=(0|1|na) found=([01]) "
    r"shown=(\d+) hidden=(\d+) voids=(\d+) nested=(\d+) zero=(\d+) unset_bound=(\d+)$"
)
CATEGORIES = [
    "unset form shown",
    "unset form hidden",
    "unset voids only",
    "no unset form, nested instance",
    "no unset form",
    "family not read",
]


def category(found, shown, hidden, voids, nested):
    if not found:
        return "family not read"
    if shown:
        return "unset form shown"
    if hidden:
        return "unset form hidden"
    if voids:
        return "unset voids only"
    return "no unset form, nested instance" if nested else "no unset form"


def main():
    listing = "--list" in sys.argv[1:]
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    revit = by_tag(args[1])
    mapped = collections.Counter()
    table = collections.defaultdict(collections.Counter)
    for line in open(args[0], encoding="utf-8", errors="replace"):
        match = LINE.match(line.strip())
        if not match:
            continue
        tag, ifc_class, grouped, map_unset, found, shown, hidden, voids, nested, zero, bound = match.groups()
        if map_unset != "1" or tag not in revit:
            continue
        entity, theirs = revit[tag]
        if not theirs:
            continue
        kind = category(found == "1", int(shown), int(hidden), int(voids), int(nested))
        written = "written" if any(n.strip() == UNNAMED for n in theirs) else "not written"
        if listing:
            print(
                f"differs {tag} {entity} {kind.replace(' ', '_')} {written.replace(' ', '_')} "
                f"shown={shown} hidden={hidden} voids={voids} nested={nested} zero={zero} unset_bound={bound}"
            )
        for key in ((entity, False), (entity, True)) if grouped == "1" else ((entity, False),):
            mapped[key] += 1
            table[key][(kind, written)] += 1
    for (entity, grouped), total in sorted(mapped.items()):
        label = f"{entity}{' grouped' if grouped else ''}"
        cells = "; ".join(
            f"{k}: written {table[(entity, grouped)][(k, 'written')]}, "
            f"not written {table[(entity, grouped)][(k, 'not written')]}"
            for k in CATEGORIES
        )
        print(f"unnamed before 2024: class {label} map unset {total}; {cells}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
