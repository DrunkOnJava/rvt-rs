#!/usr/bin/env python3
"""Whether the project's, building's, site's and storeys' GlobalIds in Revit's export are
the GUID in the header of `Global/History` with an id XORed in (RE-459, #878, RE-220).

Revit's exporter gives the project, the building and the site the GlobalIds of the fake
ElementIds -16, -15 and -14 (revit-ifc `GUIDUtil.ProjectLevelGUIDType`), and RE-220
found that in Revit's export each is one GUID with the fake id XORed into its last 32
bits, a GUID that no element, episode or text string of the file gave. On the 2018 files
read first, `Global/History` holds it in its header, three times in a row from `0x16`
and a fourth time at `0x56`, and a storey's GlobalId is the same GUID with the
storey's Level ElementId XORed in, as an element's is its episode's.

Reads the output of `probes/examples/probe_re459_history_terminators.rs` (its `base` and
`level` lines) and Revit's export, and prints counts only:

    history base: <n> of 4 copies equal the first
    history base: IfcProject equal to the base XOR -16: <k> of <n>
    history base: IfcBuilding equal to the base XOR -15: <k> of <n>
    history base: IfcSite equal to the base XOR -14: <k> of <n>
    history base: IfcBuildingStorey equal to the base XOR a Level's ElementId: <k> of <n>
    history base: IfcBuildingStorey paired with the Level of its name, equal: <k> of <n>

Usage:

    python3 tools/re/history_base_vs_ifc.py <probe output> <revit.ifc>

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import struct
import sys

import ifcopenshell

ALPHABET = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$"
FAKE_IDS = {"IfcProject": -16, "IfcBuilding": -15, "IfcSite": -14}


def expand(global_id):
    value = 0
    for char in global_id:
        value = value * 64 + ALPHABET.index(char)
    return value.to_bytes(16, "big")


def canonical(stored):
    """A GUID stored in Windows byte order, in the order of its text form."""
    out = bytearray(stored)
    out[0:4] = out[0:4][::-1]
    out[4:6] = out[4:6][::-1]
    out[6:8] = out[6:8][::-1]
    return bytes(out)


def xor_id(global_id, ident):
    """The GUID of a GlobalId with `ident` XORed out of its last 32 bits."""
    out = bytearray(expand(global_id))
    low = struct.unpack(">I", out[12:16])[0] ^ (ident & 0xFFFFFFFF)
    out[12:16] = struct.pack(">I", low)
    return bytes(out)


def main(argv):
    if len(argv) != 3:
        sys.exit(__doc__)
    bases, levels = [], {}
    for line in open(argv[1], encoding="utf-8", errors="replace"):
        parts = line.rstrip("\n").split(" ", 3)
        if parts[0] == "base" and len(parts) == 3 and len(parts[2]) == 32:
            bases.append(canonical(bytes.fromhex(parts[2])))
        elif parts[0] == "level" and len(parts) == 4:
            levels.setdefault(parts[3], int(parts[1]))
    if not bases:
        print("history base: no base GUID in the probe output")
        return 0
    base = bases[0]
    print(f"history base: {sum(1 for b in bases if b == base)} of {len(bases)} copies equal the first")
    model = ifcopenshell.open(argv[2])
    for kind, fake in FAKE_IDS.items():
        found = model.by_type(kind)
        equal = sum(1 for e in found if xor_id(e.GlobalId, fake) == base)
        print(f"history base: {kind} equal to the base XOR {fake}: {equal} of {len(found)}")
    storeys = model.by_type("IfcBuildingStorey")
    ids = set(levels.values())
    anyone = sum(1 for s in storeys if any(xor_id(s.GlobalId, i) == base for i in ids))
    print(f"history base: IfcBuildingStorey equal to the base XOR a Level's ElementId: {anyone} of {len(storeys)}")
    named = [(s, levels[s.Name]) for s in storeys if s.Name in levels]
    paired = sum(1 for s, ident in named if xor_id(s.GlobalId, ident) == base)
    print(f"history base: IfcBuildingStorey paired with the Level of its name, equal: {paired} of {len(named)}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
