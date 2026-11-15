#!/usr/bin/env python3
"""Which saved parameter of a ceiling, if any, is Revit's `GrossArea` (#1209).

Research tool. Reads `probe_1209_ceiling_params`'s output (each ceiling, whether
its record holds a `CutPairGStep`, and its saved parameters) and Revit's own
IFC export, and for every parameter id counts, among the ceilings its export
writes a `Qto_CoveringBaseQuantities.GrossArea` for, those whose value equals
it to 0.1%, apart for cut and uncut ceilings. A value is read as Revit's
internal square feet.

Usage: ceiling_area_params_vs_ifc.py <probe output> <revit-export.ifc> [--list]
"""

import collections
import json
import os
import sys

import ifcopenshell
import ifcopenshell.util.element
import ifcopenshell.util.unit

SQUARE_FOOT = 0.09290304
TOLERANCE = 1e-3


def number(text):
    try:
        value = json.loads(text)
    except ValueError:
        return None
    if isinstance(value, dict):
        for key in ("double", "f64", "value", "m_value"):
            if key in value:
                return number(json.dumps(value[key]))
        return None
    return float(value) if isinstance(value, (int, float)) and not isinstance(value, bool) else None


def main(argv):
    positional = [a for a in argv[1:] if not a.startswith("--")]
    if len(positional) != 2:
        sys.exit(__doc__)
    probe, theirs = positional
    cut = {}
    params = collections.defaultdict(list)
    names = {}
    with open(probe) as handle:
        for line in handle:
            parts = line.rstrip("\n").split(" ", 5)
            if parts[0] == "ceiling" and len(parts) >= 4:
                cut[parts[1]] = parts[3] == "true"
            elif parts[0] == "param" and len(parts) >= 6:
                own, pid, value, source, name = parts[1], parts[2], parts[3], parts[4], parts[5]
                v = number(value)
                if v is not None:
                    params[own].append((pid, v, source == own))
                    names[pid] = name
    f = ifcopenshell.open(theirs)
    scale = ifcopenshell.util.unit.calculate_unit_scale(f) ** 2
    revit = {}
    for covering in f.by_type("IfcCovering"):
        q = ifcopenshell.util.element.get_psets(covering, qtos_only=True).get("Qto_CoveringBaseQuantities", {})
        if covering.Tag and "GrossArea" in q:
            revit[str(covering.Tag)] = float(q["GrossArea"]) * scale
    both = [tag for tag in revit if tag in cut]
    totals = collections.Counter("cut" if cut[t] else "uncut" for t in both)
    print(
        f"ceiling areas: ceilings in the probe {len(cut)} (cut {sum(cut.values())}); "
        f"with Revit's GrossArea {len(both)} (cut {totals['cut']}, uncut {totals['uncut']})"
    )
    hits = collections.defaultdict(collections.Counter)
    for tag in both:
        want = revit[tag]
        for pid, v, own in params.get(tag, []):
            got = v * SQUARE_FOOT
            if want > 0 and abs(got - want) <= TOLERANCE * want:
                hits[pid]["cut" if cut[tag] else "uncut"] += 1
                hits[pid]["own" if own else "inherited"] += 1
    for pid, c in sorted(hits.items(), key=lambda kv: -(kv[1]["cut"] + kv[1]["uncut"])):
        print(
            f"ceiling areas: parameter {pid} {names.get(pid, '-')}: equal on cut {c['cut']} of {totals['cut']}, "
            f"uncut {c['uncut']} of {totals['uncut']} (on the ceiling itself {c['own']}, from another element {c['inherited']})"
        )
    if not hits:
        print("ceiling areas: no parameter equals Revit's GrossArea on any ceiling")


if __name__ == "__main__":
    main(sys.argv)
