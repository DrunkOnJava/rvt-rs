#!/usr/bin/env python3
"""Whether a railing's record's curves give Revit's
`Qto_RailingBaseQuantities.Length` (#1016, RE-254).

Reads `examples/probe_re254_railing_paths.rs`'s `rail <tag> <lines> <arcs>
<classes>` lines and Revit's IFC export, and prints how many railings Revit
gives a `Length`, on how many the record's line lengths, its arcs', or both
together are it (within 1e-4 ft), and, with `--list`, each railing's
values and curve classes. With the probe's `stair <tag> <feet> <slope>`
lines, it also prints on how many of the railings a stair holds that
length (the lines along the stair's runs climbing at its slope) is
Revit's, and with `--list` each one's.

Usage:

    python3 tools/re/railing_length_vs_ifc.py <probe.txt> <revit.ifc> [--list]
"""

import collections
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from qtos_vs_ifc import quantities  # noqa: E402

FOOT = 0.3048
TOL = 1e-4


def main(argv):
    listing = "--list" in argv
    args = [a for a in argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    rows = {}
    stairs = {}
    stairs2 = {}
    for line in open(args[0], encoding="utf-8", errors="replace"):
        parts = line.split()
        if len(parts) >= 4 and parts[0] == "rail" and parts[2] != "-":
            rows[parts[1]] = (float(parts[2]), float(parts[3]), parts[4:])
        elif len(parts) == 4 and parts[0] == "stair" and parts[2] != "-":
            stairs[parts[1]] = (float(parts[2]), float(parts[3]))
        elif len(parts) == 3 and parts[0] == "stair2":
            stairs2[parts[1]] = float(parts[2])
    revit = {}
    for (key, qset, name), (_, value) in quantities(args[1])[0].items():
        if qset == "Qto_RailingBaseQuantities" and name == "Length" and value is not None:
            revit[key.removeprefix("tag:")] = value / FOOT
    tally = collections.Counter()
    for tag, length in sorted(revit.items()):
        if tag not in rows:
            tally["no record"] += 1
            continue
        lines, arcs, classes = rows[tag]
        if abs(lines - length) <= TOL and arcs == 0:
            tally["lines"] += 1
        elif abs(lines + arcs - length) <= TOL:
            tally["lines and arcs"] += 1
        else:
            tally["neither"] += 1
        if listing:
            print(f"railing {tag} {length!r} {lines!r} {arcs!r} {' '.join(classes)}")
    print(f"railings with Revit's Length: {len(revit)}; " + "; ".join(f"{k} {n}" for k, n in sorted(tally.items())))
    held = [tag for tag in sorted(revit) if tag in stairs]
    equal = sum(1 for tag in held if abs(stairs[tag][0] - revit[tag]) <= TOL)
    print(f"railings a stair holds: {len(held)}; the stair rule gives Revit's Length on {equal}")
    equal2 = sum(1 for tag in held if tag in stairs2 and abs(stairs2[tag] - revit[tag]) <= TOL)
    print(f"railings a stair holds: {len(held)}; the run count rule gives Revit's Length on {equal2}")
    if listing:
        for tag in held:
            print(f"stair railing {tag} {revit[tag]!r} {stairs[tag][0]!r} {stairs2.get(tag)!r} slope {stairs[tag][1]!r}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
