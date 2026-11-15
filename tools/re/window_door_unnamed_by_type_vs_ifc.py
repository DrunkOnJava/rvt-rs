#!/usr/bin/env python3
"""Is `<Unnamed>` in a Revit 2015 to 2023 door or window's material set constant
within a drawn symbol (Revit's type) and within a family (#1603, RE-485)?

Reads `probes/examples/probe_1603_unnamed_by_type_before_2024.rs`'s `wtype`
lines and Revit's IFC export. For each door and window with a Tag in both,
whose drawn symbol's map holds geometry mapped to no material (-1) and to which
Revit gives materials, takes whether Revit's material set for the same Tag (the
occurrence's, or its type's, through IfcOpenShell) holds `<Unnamed>` (Revit
2018's IFC2X3 export spells it " <Unnamed>"; the white space around a name is
dropped), and counts it by the drawn symbol, the family, the number of unset
tags of the symbol's map and the visibility of the family's nested instances.

Counts only, in fixed wording, for held-out projects
(`tools/ci/scrub-holdout-results.sh` keeps these lines), per IFC class:

    unnamed by type before 2024: class IfcWindow elements N symbols S: all
        written A, none written B, mixed C; elements of mixed symbols: written
        W, not written X; families F: all written D, none written E, mixed G;
        elements of mixed families: written Y, not written Z
    unnamed by facts before 2024: class IfcWindow fact groups G with both
        outcomes N: constant within a symbol P, within a family Q, within an
        unset tag count R
    unnamed by unset tags before 2024: class IfcWindow unset tags 1: written a,
        not written b; unset tags 2: written c, not written d; unset tags 3 or
        more: written e, not written f
    unnamed by nested before 2024: class IfcWindow no nested instance: written
        a, not written b; nested instances all shown, none bound: written c,
        not written d; a nested instance hidden: written e, not written f; a
        nested instance bound to a parameter: written g, not written h

A *fact group* is the doors or windows of one project that share every fact of
RE-483's probe (the family's forms with no material effectively shown, hidden
or unresolved, bound and ambiguous, voids, nested instances); one *with both outcomes*
has `<Unnamed>` written on some of its elements and not on others, and is
*constant within a symbol* where each symbol's elements in it are all written
or all not written.

With `--list`, a `differs` line per element, with the symbol and family
ElementIds (the held-out scrub keeps only the count lines).

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).

Usage:

    python3 tools/re/window_door_unnamed_by_type_vs_ifc.py <probe.txt> <revit.ifc>
"""

import collections
import re
import sys

from element_materials_vs_ifc import by_tag
from window_door_type_maps_vs_ifc import UNNAMED

LINE = re.compile(
    r"^wtype (\d+) (IFC[A-Z]+) grouped=([01]) map_unset=(0|1|na) found=([01]) "
    r"eff_shown=(\d+) eff_hidden=(\d+) unresolved=(\d+) bound=(\d+) resolved=(\d+) "
    r"ambiguous=(\d+) voids=(\d+) nested=(\d+) nested_hidden=(\d+) nested_bound=(\d+) "
    r"symbol=(\d+) family=(\d+) tags=(\d+) unset_tags=(\d+)$"
)


def split(outcomes):
    """(all written, none written, mixed) over a mapping key -> [written, not]."""
    everyone = sum(1 for w, n in outcomes.values() if w and not n)
    nobody = sum(1 for w, n in outcomes.values() if n and not w)
    return everyone, nobody, len(outcomes) - everyone - nobody


def mixed_elements(outcomes):
    mixed = [(w, n) for w, n in outcomes.values() if w and n]
    return sum(w for w, _ in mixed), sum(n for _, n in mixed)


def constant(elements, key):
    """Whether, grouping `elements` by `key`, each group is all-or-none."""
    groups = collections.defaultdict(set)
    for element in elements:
        groups[key(element)].add(element["written"])
    return all(len(v) == 1 for v in groups.values())


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
        (tag, ifc_class, grouped, map_unset, found, shown, hidden, unresolved, bound,
         resolved, ambiguous, voids, nested, nested_hidden, nested_bound, symbol,
         family, tags, unset_tags) = match.groups()
        if map_unset != "1" or tag not in revit:
            continue
        entity, theirs = revit[tag]
        if not theirs:
            continue
        written = any(n.strip() == UNNAMED for n in theirs)
        element = {
            "written": written,
            "symbol": symbol,
            "family": family,
            "unset_tags": int(unset_tags),
            "facts": (shown, hidden, unresolved, bound, voids, nested),
            "nested": "none" if int(nested) == 0 else (
                "bound" if int(nested_bound) else ("hidden" if int(nested_hidden) else "shown")
            ),
        }
        elements[entity].append(element)
        if listing:
            print(
                f"differs {tag} {entity} {'written' if written else 'not_written'} "
                f"symbol={symbol} family={family} tags={tags} unset_tags={unset_tags} "
                f"eff_shown={shown} eff_hidden={hidden} unresolved={unresolved} bound={bound} "
                f"ambiguous={ambiguous} voids={voids} nested={nested} "
                f"nested_hidden={nested_hidden} nested_bound={nested_bound}"
            )
    for entity, items in sorted(elements.items()):
        by_symbol = collections.defaultdict(lambda: [0, 0])
        by_family = collections.defaultdict(lambda: [0, 0])
        by_tags = collections.defaultdict(lambda: [0, 0])
        for e in items:
            slot = 0 if e["written"] else 1
            by_symbol[e["symbol"]][slot] += 1
            by_family[e["family"]][slot] += 1
            by_tags[min(e["unset_tags"], 3)][slot] += 1
        sa, sn, sm = split(by_symbol)
        fa, fn, fm = split(by_family)
        sw, sx = mixed_elements(by_symbol)
        fw, fx = mixed_elements(by_family)
        print(
            f"unnamed by type before 2024: class {entity} elements {len(items)} "
            f"symbols {len(by_symbol)}: all written {sa}, none written {sn}, mixed {sm}; "
            f"elements of mixed symbols: written {sw}, not written {sx}; "
            f"families {len(by_family)}: all written {fa}, none written {fn}, mixed {fm}; "
            f"elements of mixed families: written {fw}, not written {fx}"
        )
        groups = collections.defaultdict(list)
        for e in items:
            groups[e["facts"]].append(e)
        both = [g for g in groups.values() if len({e["written"] for e in g}) == 2]
        print(
            f"unnamed by facts before 2024: class {entity} fact groups {len(groups)} "
            f"with both outcomes {len(both)}: "
            f"constant within a symbol {sum(constant(g, lambda e: e['symbol']) for g in both)}, "
            f"within a family {sum(constant(g, lambda e: e['family']) for g in both)}, "
            f"within an unset tag count {sum(constant(g, lambda e: min(e['unset_tags'], 3)) for g in both)}"
        )
        by_nested = collections.defaultdict(lambda: [0, 0])
        for e in items:
            by_nested[e["nested"]][0 if e["written"] else 1] += 1
        names = [
            ("none", "no nested instance"),
            ("shown", "nested instances all shown, none bound"),
            ("hidden", "a nested instance hidden"),
            ("bound", "a nested instance bound to a parameter"),
        ]
        print(
            f"unnamed by nested before 2024: class {entity} "
            + "; ".join(
                f"{label_}: written {by_nested[key][0]}, not written {by_nested[key][1]}"
                for key, label_ in names
            )
        )
        label = {1: "unset tags 1", 2: "unset tags 2", 3: "unset tags 3 or more"}
        print(
            f"unnamed by unset tags before 2024: class {entity} "
            + "; ".join(
                f"{label[k]}: written {by_tags[k][0]}, not written {by_tags[k][1]}"
                for k in (1, 2, 3)
            )
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
