#!/usr/bin/env python3
"""Which number of a wall's native record or saved parameters is each of
Revit's wall base quantities (#1016, RE-251).

Reads `examples/probe_re251_wall_numbers.rs`'s `rec <tag> <class>
<field>=<number> ...` lines and Revit's IFC export. For each of the wall's
`Qto_WallBaseQuantities` Revit writes, in feet, square feet or cubic feet as
its kind is a length, area or volume, it counts the walls on which each field
holds that number (a relative 1e-6), and prints, per quantity, how many walls
carry it and the 8 fields equal on the most.

Usage:

    python3 tools/re/wall_numbers_vs_qtos.py <probe.txt> <revit.ifc> [--list]
"""

import collections
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from qtos_vs_ifc import quantities  # noqa: E402

FOOT = 0.3048
TO_FEET = {
    "IfcQuantityLength": 1 / FOOT,
    "IfcQuantityArea": 1 / FOOT**2,
    "IfcQuantityVolume": 1 / FOOT**3,
}


def read_probe(path):
    rows = {}
    for line in open(path, encoding="utf-8", errors="replace"):
        parts = line.split()
        if len(parts) < 3 or parts[0] != "rec" or parts[2] == "-":
            continue
        fields = collections.defaultdict(list)
        for part in parts[3:]:
            name, _, value = part.partition("=")
            try:
                fields[name].append(float(value))
            except ValueError:
                pass
        rows[parts[1]] = fields
    return rows


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    rows = read_probe(args[0])
    totals = collections.Counter()
    hits = collections.defaultdict(collections.Counter)
    for (key, qset, name), (kind, value) in quantities(args[1])[0].items():
        tag = key.removeprefix("tag:")
        if qset != "Qto_WallBaseQuantities" or value is None or tag not in rows or kind not in TO_FEET:
            continue
        target = value * TO_FEET[kind]
        totals[name] += 1
        for field, numbers in rows[tag].items():
            if any(abs(n - target) <= 1e-6 * max(1.0, abs(target)) for n in numbers):
                hits[name][field] += 1
    print(f"probe: {len(rows)} walls")
    for name, n in sorted(totals.items()):
        best = ", ".join(f"{f} {k}" for f, k in hits[name].most_common(8)) or "none"
        print(f"{name}: {n} walls; {best}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
