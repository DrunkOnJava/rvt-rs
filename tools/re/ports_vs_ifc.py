#!/usr/bin/env python3
"""Score rvt-rs's ports and port-to-port connections against Revit's own IFC
export (#528).

Research tool for `reports/element-framing/RE-138-connector-pairs.md`. Revit
writes each connector as an `IfcDistributionPort` named
`<In or Out>Port_<ElementId>_<index>`, ties it to its element with an
`IfcRelConnectsPortToElement`, and each join with an `IfcRelConnectsPorts`.
rvt-rs writes the same entities, named `Port_<ElementId>_<index>`, and nests a
proxy's ports in it with an `IfcRelNests`, as IFC4 ties a port to a
distribution element only (B83). Both are read as unordered pairs of
(ElementId, connector index), and the connections that involve a duct or pipe
(an `IfcFlowSegment`) are compared:

- how many of Revit's reproduce;
- how many rvt-rs writes that Revit's export does not hold;
- whether each of rvt-rs's ports is tied to the element its name says;
- the `FlowDirection`, `PredefinedType` and `SystemType` of the ports both
  exports hold, those the schema of Revit's export gives a port (#828): an
  IFC2X3 port, as in Revit's exports of the RE1 models, has `FlowDirection`
  only.

Needs IfcOpenShell (`pip install ifcopenshell`).

Usage:

    python3 tools/re/ports_vs_ifc.py <rvt-rs.ifc> <revit.ifc> [--list]
"""

import argparse
import collections
import json
import re

import ifcopenshell
import ifcopenshell.validate

PORT_NAME = re.compile(r"^(?:In|Out)?Port_(\d+)_(\d+)$")
PORT_ENTITY = re.compile(r"IfcDistributionPort|IfcRelConnectsPort|IfcRelNests")


def port_schema_findings(path):
    """IfcOpenShell's schema validation of a file: every finding, and those about the port entities."""
    logger = ifcopenshell.validate.json_logger()
    ifcopenshell.validate.validate(ifcopenshell.open(path), logger)
    findings = list(logger.statements)
    return findings, [f for f in findings if PORT_ENTITY.search(json.dumps(f, default=str))]


# The port attributes compared (#828); an IFC2X3 port has FlowDirection only.
PORT_ATTRIBUTES = ("FlowDirection", "PredefinedType", "SystemType")


def read(path):
    """Ports, connections as unordered pairs, the ElementIds that are ducts or
    pipes, which element each port is tied to, each port's PORT_ATTRIBUTES by
    (ElementId, connector index), and those of PORT_ATTRIBUTES the file's
    schema gives a port."""
    model = ifcopenshell.open(path)
    declaration = ifcopenshell.ifcopenshell_wrapper.schema_by_name(model.schema).declaration_by_name(
        "IfcDistributionPort"
    )
    names = {attribute.name() for attribute in declaration.all_attributes()}
    ports = {}
    attributes = {}
    for port in model.by_type("IfcDistributionPort"):
        match = PORT_NAME.match(port.Name or "")
        if match:
            ports[port.id()] = (int(match.group(1)), int(match.group(2)))
            attributes[ports[port.id()]] = tuple(getattr(port, name, None) for name in PORT_ATTRIBUTES)
    connections = set()
    for rel in model.by_type("IfcRelConnectsPorts"):
        a, b = ports.get(rel.RelatingPort.id()), ports.get(rel.RelatedPort.id())
        if a and b:
            connections.add(frozenset((a, b)))
    segments = {int(e.Tag) for e in model.by_type("IfcFlowSegment") if e.Tag and str(e.Tag).isdigit()}
    def tag(element):
        return int(element.Tag) if getattr(element, "Tag", None) and str(element.Tag).isdigit() else None

    tied = {}
    for rel in model.by_type("IfcRelConnectsPortToElement"):
        tied[rel.RelatingPort.id()] = tag(rel.RelatedElement)
    for rel in model.by_type("IfcRelNests"):
        for part in rel.RelatedObjects:
            if part.is_a("IfcDistributionPort"):
                tied[part.id()] = tag(rel.RelatingObject)
    return ports, connections, segments, tied, attributes, {name for name in PORT_ATTRIBUTES if name in names}


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("rvt_rs_ifc")
    parser.add_argument("revit_ifc")
    parser.add_argument("--list", action="store_true", help="list the connections not matched")
    args = parser.parse_args()

    ports, connections, segments, tied, attributes, _ = read(args.rvt_rs_ifc)
    revit_ports, revit_connections, revit_segments, _, revit_attributes, compared = read(args.revit_ifc)

    def involves(connection, segment_ids):
        return any(element in segment_ids for element, _ in connection)

    revit_curve = {c for c in revit_connections if involves(c, revit_segments)}
    ours_curve = {c for c in connections if involves(c, segments | revit_segments)}
    reproduced = revit_curve & connections
    missed = revit_curve - connections
    extra = connections - revit_connections
    wrongly_tied = sum(1 for port, (element, _) in ports.items() if tied.get(port) != element)

    print(f"Revit's export: {len(revit_ports)} ports, {len(revit_connections)} port-to-port connections, {len(revit_curve)} involving a duct or pipe")
    print(f"rvt-rs: {len(ports)} ports, {len(connections)} port-to-port connections, {len(ours_curve)} involving a duct or pipe")
    rate = 100.0 * len(reproduced) / len(revit_curve) if revit_curve else 0.0
    print(f"{len(reproduced)} of Revit's {len(revit_curve)} connections involving a duct or pipe are written ({rate:.1f}%), {len(missed)} are not")
    # RE-141: a fitting's own joins are read too, so every connection is scored.
    reproduced_all = revit_connections & connections
    missed_all = revit_connections - connections
    rate_all = 100.0 * len(reproduced_all) / len(revit_connections) if revit_connections else 0.0
    print(f"{len(reproduced_all)} of all {len(revit_connections)} of Revit's connections are written ({rate_all:.1f}%), {len(missed_all)} are not")
    print(f"{len(extra)} connections written are not in Revit's export")
    print(f"{wrongly_tied} of rvt-rs's {len(ports)} ports are not tied to the element their name gives")
    try:
        findings, about_ports = port_schema_findings(args.rvt_rs_ifc)
        print(f"schema validation of rvt-rs's file: {len(findings)} findings, {len(about_ports)} about the port entities")
        for finding in findings[:10]:
            print(f"  {json.dumps(finding, default=str)[:300]}")
        for finding in about_ports[:5]:
            print("  schema:", json.dumps(finding, default=str)[:400])
    except Exception as error:  # the validator is a measurement, not the score
        print(f"schema validation unavailable: {type(error).__name__}: {error}")
    both = sorted(set(attributes) & set(revit_attributes))
    print(f"ports in both exports: {len(both)}")
    for i, name in enumerate(PORT_ATTRIBUTES):
        if name not in compared:
            print(f"  {name}: not compared, the schema of Revit's export gives a port none")
            continue
        pairs = collections.Counter((revit_attributes[key][i], attributes[key][i]) for key in both)
        equal = sum(n for (theirs, ours), n in pairs.items() if theirs == ours)
        print(f"  {name}: {equal} equal of {len(both)}")
        for (theirs, ours), n in sorted(pairs.items(), key=lambda kv: -kv[1]):
            print(f"    Revit {theirs}, rvt-rs {ours}: {n}")
    if args.list:
        for pair in sorted(missed_all, key=sorted):
            print("  not written:", sorted(pair))
        for pair in sorted(extra, key=sorted):
            print("  not in Revit's export:", sorted(pair))


if __name__ == "__main__":
    main()
