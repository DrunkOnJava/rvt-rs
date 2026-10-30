#!/usr/bin/env python3
"""Test every GUID a file holds as text as the base of Revit's project, site
and building GlobalIds (#878).

Reads `probes/examples/probe_878_text_guids.rs`'s output. revit-ifc makes the three
from `ExportUtils.GetExportId` of the fake ElementIds -16, -15 and -14,
which XORs them into the base GUID's last 32 bits as an element's id is
(RE-48). For each GUID, read in text order and in .NET byte order, this
prints whether the base XOR 0xFFFFFFF0 / F1 / F2 is Revit's IfcProject /
IfcBuilding / IfcSite GlobalId.

Usage:

    python3 tools/re/text_guids_vs_ifc.py <probe output> <revit.ifc> [--list]
"""

import sys
import uuid

import ifcopenshell

ALPHABET = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$"


def compress(value):
    out = []
    for _ in range(22):
        out.append(ALPHABET[value % 64])
        value //= 64
    return "".join(reversed(out))


def main():
    found = {}
    for line in open(sys.argv[1], encoding="utf-8", errors="replace"):
        parts = line.split()
        if len(parts) == 3 and parts[0] == "guid":
            found.setdefault(parts[2], set()).add(parts[1])
    model = ifcopenshell.open(sys.argv[2])
    revit = {e: {x.GlobalId for x in model.by_type(e)} for e in ("IfcProject", "IfcBuilding", "IfcSite")}
    print(f"text GUIDs: {len(found)}; Revit {revit}")
    hits = 0
    for text, streams in sorted(found.items()):
        u = uuid.UUID(text)
        for order, value in (("text", int.from_bytes(u.bytes, "big")), ("dotnet", int.from_bytes(u.bytes_le, "big"))):
            for entity, fake in (("IfcProject", 0xFFFFFFF0), ("IfcBuilding", 0xFFFFFFF1), ("IfcSite", 0xFFFFFFF2)):
                if compress(value ^ fake) in revit[entity]:
                    hits += 1
                    print(f"  {text} ({order} order, in {sorted(streams)}) XOR {fake:#x} = Revit's {entity}")
    print(f"matches: {hits}")


if __name__ == "__main__":
    main()
