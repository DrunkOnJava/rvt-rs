#!/usr/bin/env python3
"""Score rvt-rs's room numbers and names against Revit's (RE-117).

rvt-rs writes a room's number as `IfcSpace.Name` and its name as
`IfcSpace.LongName`, as Revit's own exporter does, and its ElementId as the
`ElementId` property. The reference is either Revit's own IFC export,
matched by GlobalId, or a VIM export (`.vim`), matched by that ElementId,
whose `Vim.Room.Number` is the number and whose element Name is Revit's
display name, `<name> <number>`.

Usage:
    python3 tools/re/room_names_vs_ifc.py OURS.ifc REVIT.ifc
    python3 tools/re/room_names_vs_ifc.py OURS.ifc MODEL.vim
"""

import struct
import sys

import ifcopenshell
import ifcopenshell.util.element


def ours(path):
    out = {}
    for space in ifcopenshell.open(path).by_type("IfcSpace"):
        element_id = None
        for properties in ifcopenshell.util.element.get_psets(space).values():
            element_id = properties.get("ElementId", element_id)
        out[space] = (space.Name, space.LongName, element_id)
    return out


def bfast(buf, base=0):
    _magic, _start, _end, n = struct.unpack_from("<QQQQ", buf, base)
    ranges = [struct.unpack_from("<QQ", buf, base + 32 + 16 * i) for i in range(n)]
    names = buf[base + ranges[0][0] : base + ranges[0][1]].split(b"\0")
    return {names[i - 1].decode(): (base + ranges[i][0], base + ranges[i][1]) for i in range(1, n)}


def vim_rooms(path):
    data = open(path, "rb").read()
    top = bfast(data)
    entities = bfast(data, top["entities"][0])
    start, end = top["strings"]
    strings = data[start:end].split(b"\0")

    def column(table, name, fmt):
        a, b = bfast(data, entities[table][0])[name]
        size = struct.calcsize(fmt)
        return list(struct.unpack("<%d%s" % ((b - a) // size, fmt), data[a:b]))

    ids = column("Vim.Element", "long:Id", "q")
    names = column("Vim.Element", "string:Name", "i")
    elements = column("Vim.Room", "index:Vim.Element:Element", "i")
    numbers = column("Vim.Room", "string:Number", "i")
    text = lambda index: strings[index].decode() if index >= 0 else None
    out = {}
    for element, number in zip(elements, numbers):
        if element >= 0:
            out.setdefault(ids[element], set()).add((text(number), text(names[element])))
    return out


def main():
    rooms = ours(sys.argv[1])
    same = differ = absent = 0
    if sys.argv[2].endswith(".vim"):
        reference = vim_rooms(sys.argv[2])
        for space, (number, name, element_id) in rooms.items():
            candidates = reference.get(element_id)
            if not candidates:
                absent += 1
            elif (number, f"{name} {number}") in candidates:
                same += 1
            else:
                differ += 1
                print(f"  {space.Name}: ours {(number, name)} VIM {sorted(candidates)}")
    else:
        reference = {s.GlobalId: (s.Name, s.LongName) for s in ifcopenshell.open(sys.argv[2]).by_type("IfcSpace")}
        for space, (number, name, _) in rooms.items():
            pair = (number, name)
            theirs = reference.get(space.GlobalId)
            if theirs is None:
                absent += 1
            elif theirs == pair:
                same += 1
            else:
                differ += 1
                print(f"  {space.Name}: ours {pair} Revit {theirs}")
    print(f"rooms: {len(rooms)}; Revit's number and name {same}, different {differ}, not in the reference {absent}")


if __name__ == "__main__":
    main()
