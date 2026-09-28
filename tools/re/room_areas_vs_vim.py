#!/usr/bin/env python3
"""Score rvt-rs's room plan areas against the room areas of a VIM export.

Research tool for `reports/element-framing/RE-101-room-solids.md` (#90).
For each IfcSpace rvt-rs writes (its ElementId is the `ElementId` property), it
measures the space's top surface as `plan_profiles_vs_ifc.py` does and
compares it with `Vim.Room`'s `Area` for the element of that id.

Usage:

    python3 tools/re/room_areas_vs_vim.py <rvt-rs.ifc> <model.vim> [--list N]

A VIM is a BFAST container; the reader below is the minimum needed. Needs
IfcOpenShell (tested with 0.8.5), NumPy and Shapely.
"""

import importlib.util
import os
import struct
import sys


def bfast(buf, base=0):
    _magic, _start, _end, n = struct.unpack_from("<QQQQ", buf, base)
    ranges = [struct.unpack_from("<QQ", buf, base + 32 + 16 * i) for i in range(n)]
    names = buf[base + ranges[0][0] : base + ranges[0][1]].split(b"\0")
    return {names[i - 1].decode(): (base + ranges[i][0], base + ranges[i][1]) for i in range(1, n)}


def vim_room_areas(path):
    data = open(path, "rb").read()
    entities = bfast(data, bfast(data)["entities"][0])

    def column(table, col, fmt):
        a, b = bfast(data, entities[table][0])[col]
        size = struct.calcsize(fmt)
        return list(struct.unpack("<%d%s" % ((b - a) // size, fmt), data[a:b]))

    ids = column("Vim.Element", "long:Id", "q")
    rooms = column("Vim.Room", "index:Vim.Element:Element", "i")
    areas = column("Vim.Room", "double:Area", "d")
    return {ids[e]: a for e, a in zip(rooms, areas) if e >= 0}


def main(argv):
    args = argv[1:]
    listing = 0
    if "--list" in args:
        i = args.index("--list")
        listing = int(args[i + 1])
        del args[i : i + 2]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    here = os.path.join(os.path.dirname(os.path.abspath(__file__)), "plan_profiles_vs_ifc.py")
    spec = importlib.util.spec_from_file_location("plan_profiles_vs_ifc", here)
    profiles = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(profiles)
    import ifcopenshell

    import ifcopenshell.util.element

    element_ids = {}
    for space in ifcopenshell.open(args[0]).by_type("IfcSpace"):
        for properties in ifcopenshell.util.element.get_psets(space).values():
            if isinstance(properties.get("ElementId"), int):
                element_ids[space.GlobalId] = properties["ElementId"]
    ours = profiles.tops(args[0], ["IfcSpace"])
    vim = vim_room_areas(args[1])
    rows = []
    for gid, (area, _points, _cls, _surface) in ours.items():
        element = element_ids.get(gid)
        if element is None:
            continue
        if element in vim and vim[element] > 0:
            rows.append((abs(area - vim[element]) / vim[element], element, area, vim[element]))
    within = sum(1 for rel, *_ in rows if rel <= 1e-3)
    print(f"rooms in both: {len(rows)}")
    print(f"  area within 0.1% of the VIM's: {within}")
    print(f"  area off: {len(rows) - within}")
    for rel, element, area, want in sorted(rows, reverse=True)[:listing]:
        print(f"  {element}: {area:.3f} vs {want:.3f} ft2 ({rel:.2%})")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
