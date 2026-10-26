#!/usr/bin/env python3
"""Which number of a wall's native record or saved parameters is each of
Revit's wall base quantities (#1016, RE-251).

Reads `examples/probe_re251_wall_numbers.rs`'s `rec <tag> <class>
<field>=<number> ...` lines and Revit's IFC export. For each of the wall's
`Qto_WallBaseQuantities` Revit writes, in feet, square feet or cubic feet as
its kind is a length, area or volume, it counts the walls on which each field
holds that number (a relative 1e-6), and prints, per quantity, how many walls
carry it, on how many any field holds it (and, for the face planes'
envelopes, `Plane#k.m_Envelope`, on how many any of them does), and the 8
fields equal on the most; then, for each quantity whose
best field holds on some walls but not all, ranks every field's value against
whether that best field is the quantity (`record_scalars_vs_ifc.py`'s
ranking), to tell the walls it holds on.

Usage:

    python3 tools/re/wall_numbers_vs_qtos.py <probe.txt> <revit.ifc> [--list]
"""

import collections
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from qtos_vs_ifc import quantities  # noqa: E402
import record_scalars_vs_ifc  # noqa: E402

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
    equal = collections.defaultdict(dict)
    for (key, qset, name), (kind, value) in quantities(args[1])[0].items():
        tag = key.removeprefix("tag:")
        if qset != "Qto_WallBaseQuantities" or value is None or tag not in rows or kind not in TO_FEET:
            continue
        target = value * TO_FEET[kind]
        totals[name] += 1
        for field, numbers in rows[tag].items():
            ok = any(abs(n - target) <= 1e-6 * max(1.0, abs(target)) for n in numbers)
            if ok:
                hits[name][field] += 1
            equal[(name, field)][tag] = ok
    print(f"probe: {len(rows)} walls")
    for name, n in sorted(totals.items()):
        best = ", ".join(f"{f} {k}" for f, k in hits[name].most_common(8)) or "none"
        tags = {t for (q, _), by in equal.items() if q == name for t, ok in by.items() if ok}
        planes = {t for (q, f), by in equal.items() if q == name and f.split(".")[0].split("#")[0] == "Plane" and ".m_Envelope." in f for t, ok in by.items() if ok}
        print(f"{name}: {n} walls, {len(tags)} with some field equal, {len(planes)} with a face plane's envelope; {best}")
    truths = {}
    for name, n in totals.items():
        if not hits[name]:
            continue
        field, k = hits[name].most_common(1)[0]
        if 0 < k < n:
            truths[f"{name}={field}"] = equal[(name, field)]
    if truths:
        record_scalars_vs_ifc.report(record_scalars_vs_ifc.read_probe(args[0]), truths)
    return 0


if __name__ == "__main__":
    sys.exit(main())
