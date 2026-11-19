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
held-out projects (`tools/ci/scrub-holdout-results.sh` keeps these lines).

Line 1, the names as written (a name must match exactly):

    window door type maps: class IfcWindow elements N revit materials R \
        drawn read A drawn equal B master read C master equal D \
        either equal E both equal F

Line 2, the drawn symbol's map against Revit's set, each element once, in the
first of these that holds:

    window door type maps: class IfcWindow drawn symbol: equal G, equal with
        the unset material spelled apart H, unset material only I, Revit more
        found in the drawn symbol's record J, in the master symbol's K, in the
        family's L, in the instance's M, Revit more not found N, drawn more O,
        apart P; Revit spells the unset material apart on Q

- *equal*: the same names.
- *spelled apart*: the same once the white space around each of Revit's names is
  dropped (Revit 2018's IFC2X3 export names the unset material " <Unnamed>").
- *unset material only*: the same but for whether `<Unnamed>` is in the set
  (RE-204 and RE-206 decide where Revit writes it).
- *Revit more*: Revit's set holds names the map does not; *found in* the first
  of the drawn symbol's record (outside its map), the master symbol's, the
  family's and the instance's that names every such name.
- *drawn more*: the map holds names Revit's set does not.
- *apart*: anything else.

And the same lines for the grouped doors and windows (a member of a model group,
whose drawn symbol is a copy of the master), with `grouped` after the class.

With `--list`, a `differs` line per element whose drawn map is not equal as
written, with Tags and names (the held-out scrub keeps only the count lines).

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).

Usage:

    python3 tools/re/window_door_type_maps_vs_ifc.py <probe.txt> <revit.ifc>
"""

import collections
import re
import sys
import urllib.parse

from element_materials_vs_ifc import by_tag

UNNAMED = "<Unnamed>"
LINE = re.compile(
    r"^wmap (\d+) (IFC[A-Z]+) grouped=([01]) drawn=\S+ master=\S+ "
    r"drawn_map=(read|absent) (\S+) master_map=(read|absent) (\S+) "
    r"drawn_other=(\S+) master_other=(\S+) family_other=(\S+) instance_other=(\S+)$"
)


def names(state, text):
    """The set of material names a map holds, or None where it was not read or
    names none."""
    if state != "read":
        return None
    unset = text.endswith("+unset")
    text = text.removesuffix("+unset")
    found = set() if text == "-" else {urllib.parse.unquote(n) for n in text.split("|")}
    if unset:
        found.add(UNNAMED)
    return found or None


def other_names(text):
    return set() if text == "-" else {urllib.parse.unquote(n) for n in text.split("|")}


def kind(drawn, revit, elsewhere):
    """How the drawn symbol's set stands against Revit's, in the order of the
    docstring; `drawn` may be None, `elsewhere` the sets the drawn symbol's
    record, the master symbol's, the family's and the instance's name outside
    the map, in that order."""
    if drawn == revit:
        return "equal"
    drawn = drawn or set()
    if {n.strip() for n in drawn} == {n.strip() for n in revit}:
        return "equal with the unset material spelled apart"
    plain_drawn = {n.strip() for n in drawn} - {UNNAMED}
    plain_revit = {n.strip() for n in revit} - {UNNAMED}
    if plain_drawn == plain_revit:
        return "unset material only"
    if plain_drawn < plain_revit:
        missing = plain_revit - plain_drawn
        for label, found in zip(WHERE, elsewhere):
            if missing <= {n.strip() for n in found}:
                return f"Revit more found in the {label} record"
        return "Revit more not found"
    if plain_drawn > plain_revit:
        return "drawn more"
    return "apart"


WHERE = ["drawn symbol's", "master symbol's", "family's", "instance's"]
KINDS = [
    "equal",
    "equal with the unset material spelled apart",
    "unset material only",
    *[f"Revit more found in the {label} record" for label in WHERE],
    "Revit more not found",
    "drawn more",
    "apart",
]


def main():
    listing = "--list" in sys.argv[1:]
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    revit = by_tag(args[1])
    counts = collections.defaultdict(collections.Counter)
    kinds = collections.defaultdict(collections.Counter)
    spelled = collections.defaultdict(collections.Counter)
    for line in open(args[0], encoding="utf-8", errors="replace"):
        match = LINE.match(line.strip())
        if not match:
            continue
        tag, ifc_class, grouped, d_state, d_text, m_state, m_text, *others = match.groups()
        if tag not in revit:
            continue
        entity, theirs = revit[tag]
        if not theirs:
            continue
        drawn, master = names(d_state, d_text), names(m_state, m_text)
        how = kind(drawn, theirs, [other_names(text) for text in others])
        if listing and how != "equal":
            # Tags and names: the held-out scrub keeps only the count lines.
            shown = lambda found: "|".join(urllib.parse.quote(n, safe="") for n in sorted(found)) if found else "-"
            print(
                f"differs {tag} {entity} {how.replace(' ', '_')} drawn={shown(drawn)} "
                f"master={shown(master)} revit={shown(theirs)} other={' '.join(others)}"
            )
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
            kinds[key][how] += 1
            spelled[key]["spelled"] += any(n != n.strip() and n.strip() == UNNAMED for n in theirs)
    for (entity, grouped), row in sorted(counts.items()):
        label = f"{entity}{' grouped' if grouped else ''}"
        print(
            f"window door type maps: class {label} "
            f"elements {row['elements']} revit materials {row['revit materials']} "
            f"drawn read {row['drawn read']} drawn equal {row['drawn equal']} "
            f"master read {row['master read']} master equal {row['master equal']} "
            f"either equal {row['either equal']} both equal {row['both equal']}"
        )
        print(
            f"window door type maps: class {label} drawn symbol: "
            + ", ".join(f"{k} {kinds[(entity, grouped)][k]}" for k in KINDS)
            + f"; Revit spells the unset material apart on {spelled[(entity, grouped)]['spelled']}"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
