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

Reads the output of `probes/examples/probe_re459_history_terminators.rs` (its `base`,
`episode`, `stored` and `level` lines) and Revit's export, and prints counts only:

    history base: <n> of 4 copies equal the first
    history base: equal to the oldest episode's GUID: <0|1> of 1
    history base: equal to the newest episode's GUID: <0|1> of 1
    history base: IfcProject equal to the base XOR -16: <k> of <n>
    history base: IfcBuilding equal to the base XOR -15: <k> of <n>
    history base: IfcSite equal to the base XOR -14: <k> of <n>
    history base: IfcBuildingStorey equal to the base XOR a Level's ElementId: <k> of <n>
    history base: IfcBuildingStorey paired with the Level of its name, equal: <k> of <n>
    history base: IfcBuildingStorey equal to the oldest episode's GUID XOR a Level's ElementId: <k> of <n>
    history base: <IfcProject|IfcBuilding|IfcSite|IfcBuildingStorey> equal to a GlobalId the file stores: <k> of <n>
    history base: elements equal to a GlobalId the file stores: <k> of <n>
    history base: elements equal to one the file stores but for the first character: <k> of <n>

With `--list` (Measure passes it) it also prints a `list` line for each project, building,
site and storey that is not equal, with its name and the Level of that name: never kept
for a model whose export has no licence (the scrub keeps only the `history base:` lines).

Usage:

    python3 tools/re/history_base_vs_ifc.py <probe output> <revit.ifc> [--list]

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
    flags = [a for a in argv[1:] if a.startswith("--")]
    args = [a for a in argv[1:] if not a.startswith("--")]
    if len(args) != 2 or any(f != "--list" for f in flags):
        sys.exit(__doc__)
    listing = "--list" in flags
    bases, levels, episodes, stored = [], {}, {}, set()
    for line in open(args[0], encoding="utf-8", errors="replace"):
        parts = line.rstrip("\n").split(" ", 3)
        if parts[0] == "base" and len(parts) == 3 and len(parts[2]) == 32:
            bases.append(canonical(bytes.fromhex(parts[2])))
        elif parts[0] == "episode" and len(parts) == 3 and len(parts[2]) == 32:
            episodes[int(parts[1])] = canonical(bytes.fromhex(parts[2]))
        elif parts[0] == "stored" and len(parts) == 3 and len(parts[2]) == 22:
            stored.add(parts[2])
        elif parts[0] == "level" and len(parts) == 4:
            levels.setdefault(parts[3], int(parts[1]))
    if not bases:
        print("history base: no base GUID in the probe output")
        return 0
    base = bases[0]
    print(f"history base: {sum(1 for b in bases if b == base)} of {len(bases)} copies equal the first")
    oldest = episodes.get(0)
    newest = episodes.get(max(episodes)) if episodes else None
    print(f"history base: equal to the oldest episode's GUID: {int(base == oldest)} of {int(oldest is not None)}")
    print(f"history base: equal to the newest episode's GUID: {int(base == newest)} of {int(newest is not None)}")
    model = ifcopenshell.open(args[1])
    for kind, fake in FAKE_IDS.items():
        found = model.by_type(kind)
        equal = {e.id() for e in found if xor_id(e.GlobalId, fake) == base}
        print(f"history base: {kind} equal to the base XOR {fake}: {len(equal)} of {len(found)}")
        if listing:
            for e in found:
                if e.id() not in equal:
                    print(f"list {kind} {e.GlobalId} {e.Name!r}")
    storeys = model.by_type("IfcBuildingStorey")
    ids = set(levels.values())
    anyone = {s.id() for s in storeys if any(xor_id(s.GlobalId, i) == base for i in ids)}
    print(f"history base: IfcBuildingStorey equal to the base XOR a Level's ElementId: {len(anyone)} of {len(storeys)}")
    named = [(s, levels[s.Name]) for s in storeys if s.Name in levels]
    paired = [s for s, ident in named if xor_id(s.GlobalId, ident) == base]
    print(f"history base: IfcBuildingStorey paired with the Level of its name, equal: {len(paired)} of {len(named)}")
    if oldest is not None:
        from_oldest = [s for s in storeys if any(xor_id(s.GlobalId, i) == oldest for i in ids)]
        print(
            "history base: IfcBuildingStorey equal to the oldest episode's GUID XOR a Level's ElementId: "
            f"{len(from_oldest)} of {len(storeys)}"
        )
    for kind in [*FAKE_IDS, "IfcBuildingStorey"]:
        found = model.by_type(kind)
        equal = sum(1 for e in found if e.GlobalId in stored)
        print(f"history base: {kind} equal to a GlobalId the file stores: {equal} of {len(found)}")
    special = {*FAKE_IDS, "IfcBuildingStorey"}
    elements = [e for e in model.by_type("IfcProduct") if e.is_a() not in special]
    same = sum(1 for e in elements if e.GlobalId in stored)
    but_first = sum(1 for e in elements if e.GlobalId not in stored and e.GlobalId[1:] in {g[1:] for g in stored})
    print(f"history base: elements equal to a GlobalId the file stores: {same} of {len(elements)}")
    print(f"history base: elements equal to one the file stores but for the first character: {but_first} of {len(elements)}")
    if listing:
        for s in storeys:
            if s.id() not in anyone:
                print(f"list IfcBuildingStorey {s.GlobalId} {s.Name!r} level of that name: {levels.get(s.Name)}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
