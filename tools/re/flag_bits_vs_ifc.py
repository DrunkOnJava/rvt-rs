#!/usr/bin/env python3
"""Census the element record's `+0x46` flag bits against Revit's export (#223).

Reads `examples/probe_223_flag_census.rs`'s `rec` lines, pairs each with
Revit's element of the same Tag, and reads that element's
`Pset_*Common.LoadBearing` and `IsExternal`. Per IFC entity and property,
it prints, for each of the 32 bits, how many paired elements have the bit
equal to the property, and names every bit that equals it on all of them
(or is its exact negation). It also tallies the distinct flag words per
entity.

Usage:

    python3 tools/re/flag_bits_vs_ifc.py <probe output> <revit.ifc> [--list]
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.util.element

PROPERTIES = ("LoadBearing", "IsExternal")


def main():
    flags = {}
    for line in open(sys.argv[1], encoding="utf-8", errors="replace"):
        parts = line.split()
        if len(parts) >= 4 and parts[0] == "rec":
            flags[parts[1]] = (int(parts[3], 16), parts[2])
    model = ifcopenshell.open(sys.argv[2])
    paired = collections.defaultdict(list)
    for element in model.by_type("IfcElement"):
        tag = str(element.Tag or "")
        if tag not in flags:
            continue
        psets = ifcopenshell.util.element.get_psets(element, psets_only=True)
        common = {}
        for name, values in psets.items():
            if name.startswith("Pset_") and name.endswith("Common"):
                common.update(values)
        paired[element.is_a()].append((tag, flags[tag][0], {p: common.get(p) for p in PROPERTIES}))
    print(f"records {len(flags)}, paired {sum(len(v) for v in paired.values())}")
    for entity in sorted(paired):
        rows = paired[entity]
        words = collections.Counter(f"{word:#010x}" for _, word, _ in rows)
        print(f"{entity}: {len(rows)} paired; flag words {dict(words.most_common(8))}")
        for prop in PROPERTIES:
            known = [(word, value) for _, word, value in ((t, w, v[prop]) for t, w, v in rows) if isinstance(value, bool)]
            if not known:
                continue
            truth = collections.Counter(value for _, value in known)
            exact, partial = [], []
            for bit in range(32):
                mask = 1 << bit
                same = sum(bool(word & mask) == value for word, value in known)
                if same == len(known):
                    exact.append(f"0x{mask:x}")
                elif same == 0:
                    exact.append(f"not 0x{mask:x}")
                elif max(same, len(known) - same) >= 0.9 * len(known):
                    partial.append(f"0x{mask:x} {same}/{len(known)}")
            print(f"  {prop}: {len(known)} with a value ({dict(truth)}); exact bits {exact or 'none'}; near {partial or 'none'}")


if __name__ == "__main__":
    main()
