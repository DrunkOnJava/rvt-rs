#!/usr/bin/env python3
"""Bit 12 of the element record's flag word against Revit's export (#223, RE-261).

Reads `examples/probe_re261_flag_word_bits.rs`'s `rec`, `room-frame` and
`bit7` lines and pairs each record with Revit's element of the same Tag:

- walls (`SWall`): bit 12 against whether Revit's export joins the wall to
  another (`IfcRelConnectsPathElements`, either side);
- rooms (`RoomElem`): bit 12 against whether Revit exported the room as an
  `IfcSpace`.

Each prints a 2x2 table (bit set or clear, by the export's answer) and, with
`--list`, the ids off the diagonal. It also repeats the probe's bit 7 census by
class and its room-frame summary.

Usage:

    python3 tools/re/flag_word_vs_ifc.py <probe output> <revit.ifc> [--list]
"""

import collections
import sys

import ifcopenshell

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


def main():
    listing = "--list" in sys.argv[3:]
    records = {}
    bit7 = []
    frames = []
    for line in open(sys.argv[1], encoding="utf-8", errors="replace"):
        parts = line.split()
        if len(parts) >= 5 and parts[0] == "rec":
            records[parts[1]] = (parts[3], int(parts[4], 16))
        elif len(parts) == 4 and parts[0] == "bit7":
            bit7.append((parts[1], int(parts[2]), int(parts[3])))
        elif len(parts) == 6 and parts[0] == "room-frame":
            frames.append((parts[1], parts[3], parts[4], int(parts[5], 16)))
    model = ifcopenshell.open(sys.argv[2])
    by_tag = {str(e.Tag): e for e in model.by_type("IfcElement") if e.Tag}
    spaces = {str(s.Tag) for s in model.by_type("IfcSpace") if getattr(s, "Tag", None)}
    if not spaces:
        # IFC2x3 spaces have no Tag; Revit names them by number, so fall back
        # to the ElementId Revit writes in its IfcGUID-independent property.
        for space in model.by_type("IfcSpace"):
            for rel in getattr(space, "IsDefinedBy", []) or []:
                props = getattr(getattr(rel, "RelatingPropertyDefinition", None), "HasProperties", []) or []
                for prop in props:
                    if prop.Name == "ElementId" and prop.NominalValue is not None:
                        spaces.add(str(prop.NominalValue.wrappedValue))
    joined = set()
    for rel in model.by_type("IfcRelConnectsPathElements"):
        for element in (rel.RelatingElement, rel.RelatedElement):
            if element is not None and element.Tag:
                joined.add(str(element.Tag))

    walls = [
        (tag, bool(word & BIT12), tag in joined)
        for tag, (cls, word) in records.items()
        if cls == "SWall" and tag in by_tag
    ]
    rooms = [(tag, bool(word & BIT12), tag in spaces) for tag, (cls, word) in records.items() if cls == "RoomElem"]
    print(f"records {len(records)}; IfcSpace {len(spaces)}; walls in IfcRelConnectsPathElements {len(joined)}")
    table("walls (joined in Revit's export)", walls, listing)
    table("rooms (IfcSpace in Revit's export)", rooms, listing)
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
