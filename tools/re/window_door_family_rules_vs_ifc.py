#!/usr/bin/env python3
"""Do the rules RE-204 to RE-206 bring the drawn symbol's geometry-material map
to Revit's material set for a door or window of a Revit 2015 to 2023 file
(#1603, RE-481)?

Reads `probes/examples/probe_1603_family_rules_pre2024.rs`'s `wrule` lines and
Revit's IFC export. For each door and window with a Tag in both, the material
names IfcOpenShell resolves for Revit's element (the occurrence's, or its
type's) are compared with the names the drawn symbol's map holds (geometry
mapped to no material counting as `<Unnamed>`, RE-149) before the rules and
after them (the family's hidden-only materials left out, `<Unnamed>` written
only where the family shows it). Counts only, in fixed wording, for held-out
projects (`tools/ci/scrub-holdout-results.sh` keeps these lines).

Each element is in the first of these kinds that holds, as in RE-478
(`window_door_type_maps_vs_ifc.py`):

- *equal*: the same names.
- *equal with the unset material spelled apart*: the same once the white space
  around each of Revit's names is dropped (Revit 2018's IFC2X3 export names the
  unset material " <Unnamed>").
- *unset material only*: the same but for whether `<Unnamed>` is in the set.
- *Revit more*: Revit's set holds names the set does not.
- *drawn more*: the set holds names Revit's set does not.
- *apart*: anything else.

Lines, per IFC class (and again for the grouped doors and windows, with
`grouped` after the class):

    family rules: class IfcWindow elements N map read A forms found B shows
        unset true T false F unknown K
    family rules: class IfcWindow before the rules: equal ..., equal with the
        unset material spelled apart ..., unset material only ..., Revit more
        ..., drawn more ..., apart ...
    family rules: class IfcWindow after the rules: <the same kinds>
    family rules: class IfcWindow moves: <kind> to <kind> N, ...  (or: none)

With `--list`, a `differs` line per element that is not equal after the rules,
with Tags and names (the held-out scrub keeps only the count lines).

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).

Usage:

    python3 tools/re/window_door_family_rules_vs_ifc.py <probe.txt> <revit.ifc>
"""

import collections
import re
import sys
import urllib.parse

from element_materials_vs_ifc import by_tag
from window_door_type_maps_vs_ifc import UNNAMED

LINE = re.compile(
    r"^wrule (\d+) (IFC[A-Z]+) grouped=([01]) map=(read|absent) (\S+) "
    r"rule=(read|absent) (\S+) forms=(found|none) "
    r"shows_unset=(true|false|unknown) hidden=(\d+)$"
)
KINDS = [
    "equal",
    "equal with the unset material spelled apart",
    "unset material only",
    "Revit more",
    "drawn more",
    "apart",
]


def names(state, text):
    """The set of material names, `<Unnamed>` for `+unset`, or None where the
    map was not read."""
    if state != "read":
        return None
    unset = text.endswith("+unset")
    text = text.removesuffix("+unset")
    found = set() if text == "-" else {urllib.parse.unquote(n) for n in text.split("|")}
    if unset:
        found.add(UNNAMED)
    return found


def kind(ours, revit):
    """How `ours` stands against Revit's set, in the order of the docstring."""
    if ours == revit:
        return "equal"
    if {n.strip() for n in ours} == {n.strip() for n in revit}:
        return "equal with the unset material spelled apart"
    plain_ours = {n.strip() for n in ours} - {UNNAMED}
    plain_revit = {n.strip() for n in revit} - {UNNAMED}
    if plain_ours == plain_revit:
        return "unset material only"
    if plain_ours < plain_revit:
        return "Revit more"
    if plain_ours > plain_revit:
        return "drawn more"
    return "apart"


def shown(found):
    return "|".join(urllib.parse.quote(n, safe="") for n in sorted(found)) if found else "-"


def main():
    listing = "--list" in sys.argv[1:]
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    revit = by_tag(args[1])
    counts = collections.defaultdict(collections.Counter)
    before = collections.defaultdict(collections.Counter)
    after = collections.defaultdict(collections.Counter)
    moves = collections.defaultdict(collections.Counter)
    for line in open(args[0], encoding="utf-8", errors="replace"):
        match = LINE.match(line.strip())
        if not match:
            continue
        tag, ifc_class, grouped, m_state, m_text, r_state, r_text, forms, shows, _hidden = match.groups()
        if tag not in revit:
            continue
        entity, theirs = revit[tag]
        if not theirs:
            continue
        ours_before = names(m_state, m_text) or set()
        ours_after = names(r_state, r_text) or set()
        was, now = kind(ours_before, theirs), kind(ours_after, theirs)
        if listing and now != "equal":
            print(
                f"differs {tag} {entity} {now.replace(' ', '_')} map={shown(ours_before)} "
                f"rule={shown(ours_after)} revit={shown(theirs)} forms={forms} shows_unset={shows}"
            )
        for key in ((entity, False), (entity, True)) if grouped == "1" else ((entity, False),):
            row = counts[key]
            row["elements"] += 1
            row["map read"] += m_state == "read"
            row["forms found"] += forms == "found"
            row["shows " + shows] += 1
            before[key][was] += 1
            after[key][now] += 1
            if was != now:
                moves[key][(was, now)] += 1
    for (entity, grouped), row in sorted(counts.items()):
        key = (entity, grouped)
        label = f"{entity}{' grouped' if grouped else ''}"
        print(
            f"family rules: class {label} elements {row['elements']} map read {row['map read']} "
            f"forms found {row['forms found']} shows unset true {row['shows true']} "
            f"false {row['shows false']} unknown {row['shows unknown']}"
        )
        for stage, table in (("before", before), ("after", after)):
            print(
                f"family rules: class {label} {stage} the rules: "
                + ", ".join(f"{k} {table[key][k]}" for k in KINDS)
            )
        changed = ", ".join(
            f"{a} to {b} {n}" for (a, b), n in sorted(moves[key].items())
        )
        print(f"family rules: class {label} moves: {changed or 'none'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
