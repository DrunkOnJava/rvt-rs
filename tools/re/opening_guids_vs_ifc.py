#!/usr/bin/env python3
"""Which key Revit's export hashes an unfilled opening's GlobalId from (#824).

revit-ifc's `OpeningUtil.CreateOpeningGUID` (CLEANROOM.md source 5) hashes
`Opening:<opening index>Solid:<solid index>`, then the host's and the
cutting element's export GUIDs in IFC form, as a .NET GUID of the MD5;
`OpeningUtil` hashes `IfcOpeningElement:<opening number>:<host GlobalId>` for
the openings its host's own extrusion data carries. This tries the second
with opening numbers 1 to 256, then the first. For
every opening Revit's export cuts in a host that is not a wall, this tries
every door and window of the export as the cutting element, opening indices
1 to 8 and solid indices 1 to 4, and prints the key that gives the opening's
GlobalId, if one does. Takes any probe output (ignored), so Measure can run
it as a probe scorer.

Usage:

    python3 tools/re/opening_guids_vs_ifc.py <probe.txt> <revit.ifc> [--list]
"""

import hashlib
import sys
import uuid

import ifcopenshell
import ifcopenshell.guid


def hashed(key):
    digest = hashlib.md5(key.encode("utf-8")).digest()
    return ifcopenshell.guid.compress(uuid.UUID(bytes_le=digest).hex)


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    model = ifcopenshell.open(args[1])
    cutters = [e for t in ("IfcDoor", "IfcWindow") for e in model.by_type(t)]
    found = 0
    total = 0
    for rel in model.by_type("IfcRelVoidsElement"):
        host = rel.RelatingBuildingElement
        if host.is_a("IfcWall") or host.is_a("IfcCurtainWall"):
            continue
        opening = rel.RelatedOpeningElement
        total += 1
        match = None
        # revit-ifc's `OpeningUtil` for openings its host's own extrusion data
        # carries: `IfcOpeningElement:<opening number>:<host GlobalId>`.
        for i in range(1, 257):
            if hashed(f"IfcOpeningElement:{i}:{host.GlobalId}") == opening.GlobalId:
                found += 1
                print(f"opening {opening.GlobalId} '{opening.Name}' in {host.Tag}: host key, opening number {i}")
                match = "host"
                break
        if match:
            continue
        for cutter in cutters:
            for i in range(1, 9):
                for s in range(1, 5):
                    if hashed(f"Opening:{i}Solid:{s}{host.GlobalId}{cutter.GlobalId}") == opening.GlobalId:
                        match = (cutter, i, s)
                        break
                if match:
                    break
            if match:
                break
        if match:
            found += 1
            cutter, i, s = match
            print(f"opening {opening.GlobalId} '{opening.Name}' in {host.Tag}: cut by {cutter.is_a()} {cutter.Tag}, opening {i}, solid {s}")
        else:
            print(f"opening {opening.GlobalId} '{opening.Name}' in {host.Tag}: no key found")
    print(f"{found} of {total} openings in hosts other than walls have a key")
    return 0


if __name__ == "__main__":
    sys.exit(main())
