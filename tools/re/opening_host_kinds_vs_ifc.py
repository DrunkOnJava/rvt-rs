#!/usr/bin/env python3
"""The hosts of the unfilled openings rvt-rs writes, by what Revit's export
makes of them (#1987).

For each unfilled opening rvt-rs writes, whether Revit's export writes it too,
the host's entity in Revit's export, its body's representation type, and the
host's entity, Name and `ObjectType` in rvt-rs's export (an in-place wall is a
family instance there). It tests a reading of revit-ifc's `WallExporter`
(CLEANROOM.md source 5): a wall drawn as neither an extrusion along its axis nor
a vertical extrusion of its body is given no opening by that exporter, except
an in-place wall.

Usage:

    python3 tools/re/opening_host_kinds_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import collections
import sys

import ifcopenshell


def unfilled(model):
    out = {}
    for rel in model.by_type("IfcRelVoidsElement"):
        opening = rel.RelatedOpeningElement
        host = rel.RelatingBuildingElement
        if opening.HasFillings or not opening.Tag or not host.Tag:
            continue
        out[(opening.Tag, host.Tag)] = host
    return out


def body_type(element):
    shape = element.Representation
    if shape is None:
        return "none"
    for rep in shape.Representations:
        if rep.RepresentationIdentifier == "Body":
            kind = rep.RepresentationType or "unset"
            if kind == "MappedRepresentation" and rep.Items:
                kind = "mapped " + (rep.Items[0].MappingSource.MappedRepresentation.RepresentationType or "unset")
            return kind
    return "no body"


def main():
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    ours = ifcopenshell.open(sys.argv[1])
    revit = ifcopenshell.open(sys.argv[2])
    revit_open = unfilled(revit)
    revit_by_tag = {}
    for element in revit.by_type("IfcElement"):
        if element.Tag and not element.is_a("IfcOpeningElement"):
            revit_by_tag.setdefault(element.Tag, element)
    counts = collections.Counter()
    for (tag, host_tag), host in sorted(unfilled(ours).items()):
        kind = "both" if (tag, host_tag) in revit_open else "extra"
        theirs = revit_by_tag.get(host_tag)
        revit_host = f"{theirs.is_a()} [{body_type(theirs)}]" if theirs else "not in Revit's export"
        ours_host = f"{host.is_a()} {host.ObjectType or '-'}"
        counts[(kind, revit_host, host.is_a())] += 1
        if kind == "extra":
            print(f"  extra {tag} in {host_tag}: Revit's host {revit_host}; ours {ours_host} named {host.Name!r}")
    print("unfilled openings rvt-rs writes, by Revit's host and body and rvt-rs's host entity:")
    for (kind, revit_host, ours_class), count in sorted(counts.items()):
        print(f"  {count:>4}  {kind:<5} | {revit_host} | ours {ours_class}")
    # The hosts Revit draws tessellated that hold an opening both write.
    seen = set()
    for (tag, host_tag), host in sorted(unfilled(ours).items()):
        theirs = revit_by_tag.get(host_tag)
        if (tag, host_tag) in revit_open and theirs is not None and body_type(theirs) == "Tessellation" and host_tag not in seen:
            seen.add(host_tag)
            print(f"  both-host {host_tag}: Revit {theirs.is_a()} ObjectType {theirs.ObjectType!r}; ours {host.is_a()} {host.Name!r}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
