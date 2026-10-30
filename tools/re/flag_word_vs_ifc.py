#!/usr/bin/env python3
"""Bit 12 of the element record's flag word against Revit's export (#223, RE-261).

Reads `probes/examples/probe_re261_flag_word_bits.rs`'s `rec`, `room-frame` and
`bit7` lines and pairs each record with Revit's element of the same Tag:

- walls (`SWall`): bit 12 against whether Revit's export joins an end of the
  wall (`IfcRelConnectsPathElements`, `ATSTART` or `ATEND` on the wall's own
  side), and against any join (`ATPATH` included);
- rooms (`RoomElem`): bit 12 against whether Revit exported the room as an
  `IfcSpace`, paired by the ElementId in the space's GlobalId.

Each prints a 2x2 table (bit set or clear, by the export's answer) and, with
`--list`, the ids off the diagonal. It also repeats the probe's bit 7 census by
class and its room-frame summary.

Usage:

    python3 tools/re/flag_word_vs_ifc.py <probe output> <revit.ifc> [--list]
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.guid

BIT12 = 0x1000


def table(name, rows, listing):
    cells = collections.Counter((bit, truth) for _, bit, truth in rows)
    print(f"{name}: {len(rows)} paired")
    print(f"  bit 12 set,   export yes {cells[(True, True)]:6}  export no {cells[(True, False)]:6}")
    print(f"  bit 12 clear, export yes {cells[(False, True)]:6}  export no {cells[(False, False)]:6}")
    if listing:
        for label, key in (("set, export no", (True, False)), ("clear, export yes", (False, True))):
            ids = sorted(int(i) for i, bit, truth in rows if (bit, truth) == key)
            if ids:
                print(f"  {label}: {' '.join(map(str, ids))}")


def space_element_ids(model, by_tag, room_ids):
    """ElementIds of Revit's IfcSpaces, read back from their GlobalIds.

    Revit's GlobalId for element `id` of an episode is the episode's GUID with
    `id` XORed into its last four bytes (RE-167, `revit_global_ids.rs`). Every
    tagged element gives its episode's first 12 bytes and last word; a space
    pairs with the room `low ^ word` of an episode with the same 12 bytes.
    """
    episodes = collections.defaultdict(set)
    for tag, element in by_tag.items():
        if not tag.isdigit():
            continue
        value = bytes.fromhex(ifcopenshell.guid.expand(element.GlobalId))
        episodes[value[:12]].add(int.from_bytes(value[12:], "big") ^ int(tag))
    out = set()
    for space in model.by_type("IfcSpace"):
        value = bytes.fromhex(ifcopenshell.guid.expand(space.GlobalId))
        low = int.from_bytes(value[12:], "big")
        hits = {str(low ^ word) for word in episodes.get(value[:12], ())} & room_ids
        if len(hits) == 1:
            out |= hits
    return out


def main():
    listing = "--list" in sys.argv[3:]
    records = {}
    bit7 = []
    frames = []
    for line in open(sys.argv[1], encoding="utf-8", errors="replace"):
        parts = line.split()
        if len(parts) >= 5 and parts[0] == "rec":
            # The last field, p1 or p2, says which prologue the frame has.
            records[parts[1]] = (parts[3], int(parts[4], 16))
        elif len(parts) == 4 and parts[0] == "bit7":
            bit7.append((parts[1], int(parts[2]), int(parts[3])))
        elif len(parts) == 6 and parts[0] == "room-frame":
            frames.append((parts[1], parts[3], parts[4], int(parts[5], 16)))
    model = ifcopenshell.open(sys.argv[2])
    by_tag = {str(e.Tag): e for e in model.by_type("IfcElement") if e.Tag}
    spaces = space_element_ids(model, by_tag, {tag for tag, (cls, _) in records.items() if cls == "RoomElem"})
    sides = collections.defaultdict(set)
    for rel in model.by_type("IfcRelConnectsPathElements"):
        for element, kind in (
            (rel.RelatingElement, rel.RelatingConnectionType),
            (rel.RelatedElement, rel.RelatedConnectionType),
        ):
            if element is not None and element.Tag:
                sides[str(element.Tag)].add(kind)
    print(
        f"records {len(records)}; IfcSpace {len(model.by_type('IfcSpace'))}, paired by GlobalId {len(spaces)}; "
        f"walls in IfcRelConnectsPathElements {len(sides)}"
    )
    walls = [(tag, word) for tag, (cls, word) in records.items() if cls == "SWall" and tag in by_tag]
    if sides:
        table(
            "walls (an end joined in Revit's export: ATSTART or ATEND on its own side)",
            [(t, bool(w & BIT12), bool(sides[t] & {"ATSTART", "ATEND"})) for t, w in walls],
            listing,
        )
        table(
            "walls (any join in Revit's export, ATPATH included)",
            [(t, bool(w & BIT12), bool(sides[t])) for t, w in walls],
            listing,
        )
    else:
        print(f"walls: {len(walls)} paired; Revit's export writes no IfcRelConnectsPathElements, not tested")
    if model.by_type("IfcSpace"):
        table(
            "rooms (IfcSpace in Revit's export)",
            [(tag, bool(word & BIT12), tag in spaces) for tag, (cls, word) in records.items() if cls == "RoomElem"],
            listing,
        )
    else:
        print("rooms: Revit's export has no IfcSpace, not tested")
    print("bit 7 (0x80) by class, records with it of records:")
    for cls, set_, total in sorted(bit7, key=lambda r: (-r[1], r[0])):
        if set_:
            print(f"  {cls}: {set_} of {total}")
    undecoded = [f for f in frames if f[2] == "no"]
    print(f"RoomElem frames {len(frames)}; not decoded {len(undecoded)}; without the bbox marker {sum(f[1] == 'no' for f in frames)}")
    for rid, marker, _, word in undecoded:
        print(f"  room {rid}: marker {marker}, +0x46 {word:#010x}, IfcSpace {'yes' if rid in spaces else 'no'}")


if __name__ == "__main__":
    main()
