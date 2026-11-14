#!/usr/bin/env python3
"""For each opening nothing fills that rvt-rs writes and Revit's export does not
(#1987), how Revit's export draws the host: the representation type of its
body, whether it voids the host with any opening, and whether the host's body
is smaller than its box would be by the opening's volume (the cut taken into
the body).

Usage:

    python3 tools/re/extra_opening_hosts_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>

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
        if opening.HasFillings or not opening.Tag:
            continue
        out[(opening.Tag, host.Tag)] = opening
    return out


def body_types(element):
    shape = element.Representation
    if shape is None:
        return ["none"]
    found = []
    for rep in shape.Representations:
        if rep.RepresentationIdentifier == "Body":
            found.append(rep.RepresentationType or "unset")
            if rep.RepresentationType == "MappedRepresentation":
                for item in rep.Items:
                    inner = item.MappingSource.MappedRepresentation
                    found.append("mapped " + (inner.RepresentationType or "unset"))
    return found or ["no body"]


def main():
    ours = ifcopenshell.open(sys.argv[1])
    revit = ifcopenshell.open(sys.argv[2])
    ours_open = unfilled(ours)
    revit_open = unfilled(revit)
    revit_by_tag = {}
    for element in revit.by_type("IfcElement"):
        if element.Tag and not element.is_a("IfcOpeningElement"):
            revit_by_tag.setdefault(element.Tag, element)
    counts = collections.Counter()
    for (tag, host_tag), opening in sorted(ours_open.items()):
        if (tag, host_tag) in revit_open:
            continue
        host = revit_by_tag.get(host_tag)
        if host is None:
            counts["host not in Revit's export"] += 1
            print(f"  extra {tag} in {host_tag}: host not in Revit's export")
            continue
        voids = len(host.HasOpenings or [])
        reps = ", ".join(body_types(host))
        cutter = revit_by_tag.get(tag)
        counts[f"host body {reps}"] += 1
        print(
            f"  extra {tag} in {host_tag}: Revit's host {host.is_a()} body [{reps}], "
            f"its openings {voids}, the cutter in Revit's export: {cutter.is_a() if cutter else 'none'}"
        )
    print(f"extra opening hosts: {dict(sorted(counts.items()))}")
    # The same for the openings both write, to compare.
    both = collections.Counter()
    for (tag, host_tag) in ours_open:
        if (tag, host_tag) in revit_open and host_tag in revit_by_tag:
            cutter = revit_by_tag.get(tag)
            kind = cutter.is_a() if cutter else "none"
            both[f"{', '.join(body_types(revit_by_tag[host_tag]))} | {kind}"] += 1
    print("hosts of openings both write, by Revit's body and cutter:")
    for key, count in sorted(both.items()):
        print(f"  {count:>4}  {key}")


if __name__ == "__main__":
    main()
