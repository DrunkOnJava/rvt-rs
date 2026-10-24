#!/usr/bin/env python3
"""What sets apart an element Revit's export writes in another class, or
leaves out, from the others rvt-rs writes in its class (#792).

Takes the output of `examples/probe_792_class_peers.rs` (each element's
native class and scalar fields, by the class rvt-rs writes it as) and
Revit's IFC export. An element is odd when Revit's export writes its Tag in
another class or not at all. For each class with odd elements, this lists
every field whose values on the odd elements never occur on the others: a
field that holds the reason puts the odd ones apart.

Pure standard library.

Usage:

    python3 tools/re/class_peers_vs_ifc.py <probe.txt> <revit.ifc> [--list]
"""

import collections
import json
import re
import sys


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    listing = "--list" in sys.argv
    elements = collections.defaultdict(dict)
    for line in open(args[0], encoding="utf-8", errors="replace"):
        if not line.startswith("element "):
            continue
        _, cls, eid, native, rest = line.rstrip("\n").split(" ", 4)
        decoder = json.JSONDecoder()
        root, end = decoder.raw_decode(rest)
        info, _ = decoder.raw_decode(rest[end:].lstrip())
        fields = {f"root.{k}": v for k, v in root.items()}
        fields["native class"] = native
        for k, v in (info or {}).items():
            fields[f"InstanceInfo.{k}"] = v
        elements[cls][eid] = fields
    revit = {}
    for line in open(args[1], encoding="utf-8", errors="replace"):
        m = re.match(r"#\d+=\s*(IFC[A-Z0-9]+)\('[0-9A-Za-z_$]{22}',(.*)\);\s*$", line)
        if not m:
            continue
        tags = re.findall(r"'([0-9]+)'", m.group(2))
        if tags:
            revit[tags[-1]] = m.group(1)
    for cls, members in sorted(elements.items()):
        verdict = {eid: revit.get(eid, "(not in Revit's export)") for eid in members}
        same = [e for e, c in verdict.items() if c == cls]
        odd = [e for e, c in verdict.items() if c != cls]
        print(f"{cls}: {len(members)} written by rvt-rs, {len(same)} in the same class in Revit's export, {len(odd)} odd")
        for eid in odd:
            print(f"  odd {eid}: Revit {verdict[eid]}")
        if not odd or not same:
            continue
        names = sorted({k for e in members for k in members[e]})
        apart = []
        for name in names:
            odd_values = {json.dumps(members[e].get(name)) for e in odd}
            peer_values = collections.Counter(json.dumps(members[e].get(name)) for e in same)
            if odd_values.isdisjoint(peer_values):
                apart.append((name, odd_values, peer_values))
        print(f"  fields whose odd values no peer has: {len(apart)} of {len(names)}")
        for name, odd_values, peer_values in apart:
            common = ", ".join(f"{v} x{n}" for v, n in peer_values.most_common(4))
            print(f"    {name}: odd {sorted(odd_values)}; peers {common}{' ...' if len(peer_values) > 4 else ''}")
        if listing:
            for eid in odd:
                print(f"  odd {eid} fields: {json.dumps(members[eid])[:1500]}")


if __name__ == "__main__":
    main()
