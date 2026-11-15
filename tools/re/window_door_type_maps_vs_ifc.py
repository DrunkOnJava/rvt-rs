#!/usr/bin/env python3
"""Which symbol's geometry-material map names Revit's materials for a door or
window of a Revit 2015 to 2023 file (#1603, RE-478).

Reads `probes/examples/probe_1603_window_door_type_maps_pre2024.rs`'s `wmap`
lines and Revit's IFC export. For each door and window with a Tag in both, the
material names IfcOpenShell resolves for Revit's element (the occurrence's, or
its type's) are compared with the names the map of the symbol the instance's
record draws (`m_symbolId`) and of its master symbol (`m_masterSymbolId`) holds,
geometry mapped to no material (-1) counting as the one more material
`<Unnamed>` Revit exports for it (RE-149). Counts only, in fixed wording, for
held-out projects (`tools/ci/scrub-holdout-results.sh` keeps these lines):

    window door type maps: class IfcWindow elements N revit materials R \
        drawn read A drawn equal B master read C master equal D \
        either equal E both equal F

and the same for the grouped doors and windows (a member of a model group),
whose drawn symbol is a copy of the master, with `grouped` after the class.

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).

Usage:

    python3 tools/re/window_door_type_maps_vs_ifc.py <probe.txt> <revit.ifc>
"""

import collections
import re
import sys

from element_materials_vs_ifc import by_tag

UNNAMED = "<Unnamed>"
LINE = re.compile(
    r"^wmap (\d+) (IFC[A-Z]+) grouped=([01]) drawn=\S+ master=\S+ "
    r"drawn_map=(read|absent) (\S+) master_map=(read|absent) (\S+)$"
)


def names(state, text):
    """The set of material names a map holds, or None where it was not read or
    names none."""
    if state != "read":
        return None
    unset = text.endswith("+unset")
    text = text.removesuffix("+unset")
    found = set() if text == "-" else {n.replace("_", " ") for n in text.split("|")}
    if unset:
        found.add(UNNAMED)
    return found or None


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    revit = by_tag(args[1])
    counts = collections.defaultdict(collections.Counter)
    for line in open(args[0], encoding="utf-8", errors="replace"):
        match = LINE.match(line.strip())
        if not match:
            continue
        tag, ifc_class, grouped, d_state, d_text, m_state, m_text = match.groups()
        if tag not in revit:
            continue
        entity, theirs = revit[tag]
        if not theirs:
            continue
        drawn, master = names(d_state, d_text), names(m_state, m_text)
        for key in ((entity, False), (entity, True)) if grouped == "1" else ((entity, False),):
            row = counts[key]
            row["elements"] += 1
            row["revit materials"] += 1
            row["drawn read"] += drawn is not None
            row["drawn equal"] += drawn == theirs
            row["master read"] += master is not None
            row["master equal"] += master == theirs
            row["either equal"] += theirs in (drawn, master)
            row["both equal"] += drawn == theirs and master == theirs
    for (entity, grouped), row in sorted(counts.items()):
        print(
            f"window door type maps: class {entity}{' grouped' if grouped else ''} "
            f"elements {row['elements']} revit materials {row['revit materials']} "
            f"drawn read {row['drawn read']} drawn equal {row['drawn equal']} "
            f"master read {row['master read']} master equal {row['master equal']} "
            f"either equal {row['either equal']} both equal {row['both equal']}"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
