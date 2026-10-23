#!/usr/bin/env python3
"""A family's connector data against the ports of Revit's export (RE-212,
#834, #869).

Takes the output of `examples/probe_re212_connector_data.rs` and Revit's IFC
export. Revit names each port `InPort_`, `OutPort_` or
`Port_<ElementId>_<index>`. For every port of a family instance the probe
lists, this finds the instance's symbol, that symbol's connector data entry
with the port's index and its family's `ConnectorDataCell` with it, and
reports:

- for every scalar field of the cell and of the entry with at most eight
  values, how its values split over Revit's In, Out and unprefixed ports
  (a field that is the flow direction puts each value on one side);
- for every field of the entry holding three numbers, how far that point is
  from the port's location in its element's frame (a connector's origin
  there is the port's position).

Needs IfcOpenShell (`pip install ifcopenshell`).

Usage:

    python3 tools/re/connector_data_vs_ifc.py <probe.txt> <revit.ifc> [--list]
"""

import argparse
import collections
import json
import math
import re

import ifcopenshell
import ifcopenshell.util.placement
import ifcopenshell.util.unit
import numpy

PORT_NAME = re.compile(r"^(In|Out)?Port_(\d+)_(\d+)$")
FT = 0.3048


def read_probe(path):
    cells, symbols, instances = collections.defaultdict(dict), {}, {}
    for line in open(path, encoding="utf-8", errors="replace"):
        kind, _, rest = line.rstrip("\n").partition(" ")
        if kind == "cell":
            family, data = rest.split(" ", 1)
            cell = json.loads(data)
            if isinstance(cell.get("m_index"), int):
                cells[int(family)][cell["m_index"]] = cell
        elif kind == "symbol":
            symbol, family, data = rest.split(" ", 2)
            entries = {e["m_index"]: e for e in json.loads(data) if isinstance(e, dict) and isinstance(e.get("m_index"), int)}
            symbols[int(symbol)] = (None if family == "-" else int(family), entries)
        elif kind == "instance":
            instance, symbol, _ = rest.split(" ", 2)
            instances[int(instance)] = None if symbol == "-" else int(symbol)
    return cells, symbols, instances


def scalars(prefix, value, out):
    if isinstance(value, dict):
        for key, item in value.items():
            scalars(f"{prefix}.{key}" if prefix else key, item, out)
    elif isinstance(value, (int, float, str, bool)) or value is None:
        out[prefix] = value
    return out


def points(prefix, value, out):
    if isinstance(value, dict):
        nums = [value.get(k) for k in ("m_x", "m_y", "m_z")]
        if all(isinstance(n, (int, float)) for n in nums):
            out[prefix] = nums
        for key, item in value.items():
            points(f"{prefix}.{key}" if prefix else key, item, out)
    elif isinstance(value, list) and len(value) == 3 and all(isinstance(n, (int, float)) for n in value):
        out[prefix] = list(value)
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("probe")
    ap.add_argument("reference")
    ap.add_argument("--list", action="store_true")
    args = ap.parse_args()
    cells, symbols, instances = read_probe(args.probe)
    model = ifcopenshell.open(args.reference)
    to_feet = ifcopenshell.util.unit.calculate_unit_scale(model) / FT
    host_of = {}
    for rel in model.by_type("IfcRelConnectsPortToElement"):
        host_of[rel.RelatingPort.id()] = rel.RelatedElement
    for rel in model.by_type("IfcRelNests"):
        for port in rel.RelatedObjects or []:
            if port.is_a("IfcDistributionPort"):
                host_of.setdefault(port.id(), rel.RelatingObject)
    joined = []
    missing = collections.Counter()
    for port in model.by_type("IfcDistributionPort"):
        m = PORT_NAME.match(port.Name or "")
        if not m:
            missing["port name not In/Out/Port_<id>_<index>"] += 1
            continue
        side, element, index = m.group(1) or "-", int(m.group(2)), int(m.group(3))
        if element not in instances:
            missing["port of no family instance the probe lists"] += 1
            continue
        symbol = instances[element]
        family, entries = symbols.get(symbol, (None, {}))
        entry = entries.get(index)
        cell = cells.get(family, {}).get(index)
        if entry is None or cell is None:
            missing["no connector data or cell with the port's index"] += 1
            continue
        local = None
        host = host_of.get(port.id())
        if host is not None and port.ObjectPlacement and host.ObjectPlacement:
            p = ifcopenshell.util.placement.get_local_placement(port.ObjectPlacement)
            h = ifcopenshell.util.placement.get_local_placement(host.ObjectPlacement)
            local = (numpy.linalg.inv(h) @ p)[:3, 3] * to_feet
        joined.append((side, port.FlowDirection, cell, entry, local, port.Name))
    print(f"Revit's ports joined to a connector cell and entry: {len(joined)}; not joined: {dict(missing)}")
    print("sides: " + ", ".join(f"{k} {n}" for k, n in sorted(collections.Counter(j[0] for j in joined).items())))
    print("FlowDirection by side: " + ", ".join(f"{k} {n}" for k, n in sorted(collections.Counter((j[0], j[1]) for j in joined).items())))
    for label, pick in (("cell", 2), ("entry", 3)):
        tables = collections.defaultdict(collections.Counter)
        for j in joined:
            for name, value in scalars("", j[pick], {}).items():
                tables[name][(json.dumps(value), j[0])] += 1
        for name, table in sorted(tables.items()):
            values = {v for v, _ in table}
            if len(values) > 8 or len(values) < 2:
                continue
            sides_per_value = collections.defaultdict(set)
            for (v, s), n in table.items():
                sides_per_value[v].add(s)
            pure = all(len(s) == 1 for s in sides_per_value.values())
            print(f"{label} {name}{' (each value on one side)' if pure else ''}: "
                  + ", ".join(f"{v}/{s} {n}" for (v, s), n in sorted(table.items())))
    distances = collections.defaultdict(list)
    for j in joined:
        if j[4] is None:
            continue
        for name, point in points("", j[3], {}).items():
            distances[name].append(math.dist(point, list(j[4])))
    for name, ds in sorted(distances.items(), key=lambda kv: sorted(kv[1])[len(kv[1]) // 2]):
        within = sum(1 for d in ds if d < 0.01)
        print(f"entry point {name}: within 0.01 ft of the port in its element's frame {within} of {len(ds)}, median {sorted(ds)[len(ds) // 2]:.4f} ft")
    if args.list:
        for side, flow, cell, entry, local, name in joined[:6]:
            print(f"  {name} {side} {flow} cell={json.dumps(cell)[:600]} entry={json.dumps(entry)[:600]} local={None if local is None else [round(v, 4) for v in local]}")


if __name__ == "__main__":
    main()
