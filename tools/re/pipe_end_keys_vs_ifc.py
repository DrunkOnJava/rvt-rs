#!/usr/bin/env python3
"""Which connector of a pipe each of its ends is, from the connector entry
keys there (#847), against the ports Revit's export places at that end.

Reads `probe_847_pipe_end_keys`'s output: each end of each pipe rvt-rs draws
as a cylinder, with the `element:index` keys of the connector entries whose
point it is. For each end, the port of Revit's export of the pipe that lies
there (within 0.01 ft) gives the end's connector index `k`. Then each key is
one of:

- **self, equal**: it names the pipe and index `k`;
- **self, other index**: it names the pipe and another index;
- **joined, connected**: it names another element and connector `j`, and
  Revit's export connects the pipe's port `k` to that element's port `j`
  (`IfcRelConnectsPorts`);
- **joined, not connected**: it names another element whose port `j` Revit's
  export does not connect to the pipe's port `k`.

A rule reads `k` from the keys if every end has a self key equal to `k`, or a
joined key that a join list resolves to `k`.

Needs IfcOpenShell (`pip install ifcopenshell`).

Usage:

    python3 tools/re/pipe_end_keys_vs_ifc.py <probe.txt> <revit.ifc> [--list]
"""

import argparse
import collections
import math
import re

import ifcopenshell
import ifcopenshell.util.placement as placement
import ifcopenshell.util.unit as unit

FT = 0.3048
TOLERANCE_FT = 0.01
PORT_NAME = re.compile(r"^(?:In|Out)?Port_(\d+)_(\d+)$")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("probe")
    ap.add_argument("reference")
    ap.add_argument("--list", action="store_true")
    args = ap.parse_args()
    model = ifcopenshell.open(args.reference)
    scale = unit.calculate_unit_scale(model) / FT
    ports = {}
    by_entity = {}
    for port in model.by_type("IfcDistributionPort"):
        match = PORT_NAME.match(port.Name or "")
        if not match or not port.ObjectPlacement:
            continue
        key = (int(match.group(1)), int(match.group(2)))
        matrix = placement.get_local_placement(port.ObjectPlacement)
        ports[key] = [float(v) * scale for v in matrix[:3, 3]]
        by_entity[port.id()] = key
    connected = set()
    for rel in model.by_type("IfcRelConnectsPorts"):
        a, b = by_entity.get(rel.RelatingPort.id()), by_entity.get(rel.RelatedPort.id())
        if a and b:
            connected.add((a, b))
            connected.add((b, a))
    outcomes = collections.Counter()
    ends = collections.Counter()
    listed = []
    for line in open(args.probe, encoding="utf-8", errors="replace"):
        parts = line.split()
        if len(parts) < 7 or parts[0] != "end":
            continue
        pipe, which = int(parts[1]), parts[2]
        point = [float(v) for v in parts[3:6]]
        keys = [tuple(int(v) for v in key.split(":")) for key in parts[7:]]
        here = [index for (element, index), place in ports.items() if element == pipe and math.dist(place, point) <= TOLERANCE_FT]
        if len(here) != 1:
            ends["no single Revit port of the pipe at this end"] += 1
            continue
        k = here[0]
        kinds = set()
        for element, index in keys:
            if element == pipe:
                kind = "self, equal" if index == k else "self, other index"
            else:
                kind = "joined, connected" if ((pipe, k), (element, index)) in connected else "joined, not connected"
            outcomes[kind] += 1
            kinds.add(kind)
        if "self, equal" in kinds:
            ends["read from a self key"] += 1
        elif "joined, connected" in kinds:
            ends["read from a joined key"] += 1
        elif not keys:
            ends["no key at the end"] += 1
        else:
            ends["not read"] += 1
        if args.list or "self, other index" in kinds or "joined, not connected" in kinds:
            listed.append(f"  {pipe} {which} (Revit's index {k}): " + " ".join(f"{e}:{i}" for e, i in keys))
    # #834: the words after each entry's point, against the In/Out name of
    # the port Revit's export gives that entry's element and connector.
    names = {}
    for port in model.by_type("IfcDistributionPort"):
        match = PORT_NAME.match(port.Name or "")
        if match:
            side = (port.Name or "").split("Port_")[0] or "-"
            names[(int(match.group(1)), int(match.group(2)))] = side
    words = collections.Counter()
    for line in open(args.probe, encoding="utf-8", errors="replace"):
        parts = line.split()
        if len(parts) < 5 or parts[0] != "entry":
            continue
        element, index = (int(v) for v in parts[3].split(":"))
        raw = bytes.fromhex(parts[4])
        if len(raw) < 12:
            continue
        a, b, c = (int.from_bytes(raw[i : i + 4], "little", signed=True) for i in (0, 4, 8))
        own = "self" if element == int(parts[1]) else "joined"
        words[(own, names.get((element, index), "no port"), a, b, c)] += 1
    for (own, side, a, b, c), n in sorted(words.items()):
        print(f"  words after the point: {own} entry, Revit {side} port: {a} {b} {c} x{n}")
    print("ends: " + ", ".join(f"{k} {v}" for k, v in ends.most_common()))
    print("keys: " + ", ".join(f"{k} {v}" for k, v in outcomes.most_common()))
    for line in listed[:60]:
        print(line)


if __name__ == "__main__":
    main()
