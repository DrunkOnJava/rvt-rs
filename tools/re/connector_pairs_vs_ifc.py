#!/usr/bin/env python3
"""Score the connector pairs read for ducts and pipes against the connections
in Revit's own IFC export (#528).

Research tool for `reports/element-framing/RE-138-connector-pairs.md`. The
input is the output of `examples/probe_re138_connector_pairs.rs` on a model:
the ElementIds of its ducts and pipes, and the pairs (a duct's or pipe's
connector and the element and connector joined to it) that
`partition_connector_pairs` reads. Revit's export writes each connector as an
`IfcDistributionPort` named `<In or Out>Port_<ElementId>_<index>` and each join
as an `IfcRelConnectsPorts`; the name is the same pair of numbers.

Compared as unordered pairs of (ElementId, connector index):

- the connections of Revit's export that involve a duct or pipe, and how many
  of them the file's lists give;
- the pairs read that Revit's export does not hold, split by whether Revit's
  export has any port for either element.

Needs IfcOpenShell (`pip install ifcopenshell`).

Usage:

    python3 tools/re/connector_pairs_vs_ifc.py <probe output> <revit.ifc> [--list]
"""

import argparse
import json
import re

import ifcopenshell

PORT_NAME = re.compile(r"^(?:In|Out)Port_(\d+)_(\d+)$")


def read_probe(path):
    """The ducts' and pipes' ElementIds and the pairs read."""
    curves, pairs = set(), []
    for line in open(path, encoding="utf-8", errors="replace"):
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            row = json.loads(line)
        except ValueError:
            continue
        if "curve" in row:
            curves.add(row["curve"])
        elif "pair" in row:
            pairs.append(tuple(row["pair"]))
    return curves, pairs


def read_ifc(path):
    """Every port's (ElementId, index) and the connections, as unordered pairs."""
    model = ifcopenshell.open(path)
    ports = {}
    for port in model.by_type("IfcDistributionPort"):
        match = PORT_NAME.match(port.Name or "")
        if match:
            ports[port.id()] = (int(match.group(1)), int(match.group(2)))
    connections = set()
    for rel in model.by_type("IfcRelConnectsPorts"):
        a, b = ports.get(rel.RelatingPort.id()), ports.get(rel.RelatedPort.id())
        if a and b:
            connections.add(frozenset((a, b)))
    return ports, connections


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("probe")
    parser.add_argument("ifc")
    parser.add_argument("--list", action="store_true", help="list the pairs not matched")
    args = parser.parse_args()

    curves, pairs = read_probe(args.probe)
    ports, connections = read_ifc(args.ifc)
    ported = {element for element, _ in ports.values()}
    decoded = {frozenset(((e, i), (o, j))) for e, i, o, j in pairs}
    involving = {c for c in connections if any(element in curves for element, _ in c)}
    reproduced = involving & decoded
    missed = involving - decoded
    extra = decoded - connections
    extra_ported = {c for c in extra if any(element in ported for element, _ in c)}

    print(f"{len(curves)} ducts and pipes read; Revit's export has {len(ports)} ports and {len(connections)} port-to-port connections")
    print(f"{len(pairs)} pairs read ({len(decoded)} distinct)")
    rate = 100.0 * len(reproduced) / len(involving) if involving else 0.0
    print(f"{len(involving)} connections of Revit's export involve a duct or pipe: {len(reproduced)} read ({rate:.1f}%), {len(missed)} not")
    print(f"{len(extra)} pairs read are not in Revit's export: {len(extra_ported)} where Revit has ports for an element of the pair, {len(extra) - len(extra_ported)} where it has none")
    if args.list:
        for pair in sorted(missed, key=sorted):
            print("  not read:", sorted(pair))
        for pair in sorted(extra, key=sorted):
            print("  not in Revit's export:", sorted(pair), "(ports for an element of it)" if pair in extra_ported else "(no ports)")


if __name__ == "__main__":
    main()
