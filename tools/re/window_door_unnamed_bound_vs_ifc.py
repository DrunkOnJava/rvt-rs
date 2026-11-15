#!/usr/bin/env python3
"""Is `<Unnamed>` in a Revit 2015 to 2023 door or window's material set decided
by whether its family's form with no material is effectively shown (#1603,
RE-483)?

Reads `probes/examples/probe_1603_unnamed_bound_before_2024.rs`'s `wbound`
lines and Revit's IFC export. For each door and window with a Tag in both,
whose drawn symbol's map holds geometry mapped to no material (-1) and to which
Revit gives materials, counts whether Revit's material set for the same Tag
(the occurrence's, or its type's, through IfcOpenShell) holds `<Unnamed>`
(Revit 2018's IFC2X3 export spells it " <Unnamed>"; the white space around a
name is dropped), by what the family's forms with no material show, a `Visible`
bound to a family parameter being read from that parameter's value on the
instance, else on its type:

- *unset form effectively shown*: one is shown.
- *unset form unresolved, none shown*: none is shown and one is bound to a
  parameter whose caption or value was not read.
- *unset forms effectively hidden*: all are hidden.
- *unset voids only*: only voids have no material.
- *no unset form, nested instance*: no form has none, and the family nests an
  instance.
- *no unset form*: no form has none, and it nests no instance.
- *family not read*: the family's document gave no decoded record.

Counts only, in fixed wording, for held-out projects
(`tools/ci/scrub-holdout-results.sh` keeps these lines), per IFC class and
again for the grouped doors and windows with `grouped` after the class:

    unnamed bound before 2024: class IfcWindow map unset N; unset form
        effectively shown: written A, not written B; unset form unresolved,
        none shown: written C, not written D; unset forms effectively hidden:
        written E, not written F; unset voids only: written G, not written H;
        no unset form, nested instance: written I, not written J; no unset
        form: written K, not written L; family not read: written M, not
        written N

and the reference export's schema and Revit release, read from its header
(`unknown` where it names none; no author or organisation is read):

    reference export: schema IFC2X3, Revit 2018

With `--list`, a `differs` line per element, with Tags and the family's counts
(the held-out scrub keeps only the count lines).

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).

Usage:

    python3 tools/re/window_door_unnamed_bound_vs_ifc.py <probe.txt> <revit.ifc>
"""

import collections
import re
import sys

from element_materials_vs_ifc import by_tag
from window_door_type_maps_vs_ifc import UNNAMED

LINE = re.compile(
    r"^wbound (\d+) (IFC[A-Z]+) grouped=([01]) map_unset=(0|1|na) found=([01]) "
    r"eff_shown=(\d+) eff_hidden=(\d+) unresolved=(\d+) bound=(\d+) resolved=(\d+) "
    r"voids=(\d+) nested=(\d+)$"
)
CATEGORIES = [
    "unset form effectively shown",
    "unset form unresolved, none shown",
    "unset forms effectively hidden",
    "unset voids only",
    "no unset form, nested instance",
    "no unset form",
    "family not read",
]
HEADER_BYTES = 1 << 20
SCHEMA = re.compile(rb"FILE_SCHEMA\s*\(\s*\(\s*'([A-Z0-9_]+)'", re.IGNORECASE)
RELEASE = re.compile(rb"IFCAPPLICATION\([^;]*?Revit\s+(20\d\d)", re.IGNORECASE)


def category(found, shown, hidden, unresolved, voids, nested):
    if not found:
        return "family not read"
    if shown:
        return "unset form effectively shown"
    if unresolved:
        return "unset form unresolved, none shown"
    if hidden:
        return "unset forms effectively hidden"
    if voids:
        return "unset voids only"
    return "no unset form, nested instance" if nested else "no unset form"


def reference_export(path):
    """The reference export's schema and Revit release from the head of the
    file, `unknown` where it names none. Only the two fixed fields are read."""
    try:
        with open(path, "rb") as handle:
            head = handle.read(HEADER_BYTES)
    except OSError:
        return "unknown", "unknown"
    schema = SCHEMA.search(head)
    release = RELEASE.search(head)
    return (
        schema.group(1).decode() if schema else "unknown",
        release.group(1).decode() if release else "unknown",
    )


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
        tag, ifc_class, grouped, map_unset, found, shown, hidden, unresolved, bound, resolved, voids, nested = match.groups()
        if map_unset != "1" or tag not in revit:
            continue
        entity, theirs = revit[tag]
        if not theirs:
            continue
        kind = category(found == "1", int(shown), int(hidden), int(unresolved), int(voids), int(nested))
        written = "written" if any(n.strip() == UNNAMED for n in theirs) else "not written"
        if listing:
            print(
                f"differs {tag} {entity} {kind.replace(' ', '_').replace(',', '')} {written.replace(' ', '_')} "
                f"eff_shown={shown} eff_hidden={hidden} unresolved={unresolved} bound={bound} "
                f"resolved={resolved} voids={voids} nested={nested}"
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
        print(f"unnamed bound before 2024: class {label} map unset {total}; {cells}")
    schema, release = reference_export(args[1])
    print(f"reference export: schema {schema}, Revit {release}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
