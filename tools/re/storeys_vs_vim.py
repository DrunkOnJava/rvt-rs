#!/usr/bin/env python3
"""Score rvt-rs's storey containment against the Levels of a VIM export
(RE-118).

For each element rvt-rs writes with a `Tag` (its ElementId), the VIM gives
the element's Level in the named document and whether that Level is a
building story (its "Building Story" parameter). An element counts as
"same" when rvt-rs contains it in the storey of that Level's name. Elements
whose VIM Level is not a building story are counted apart: Revit's IFC
export makes no storey of such a Level, and the VIM does not say where it
puts them.

Usage:
    python3 tools/re/storeys_vs_vim.py OURS.ifc MODEL.vim "Document Title"
"""

import collections
import struct
import sys

import ifcopenshell


def bfast(buf, base=0):
    _magic, _start, _end, n = struct.unpack_from("<QQQQ", buf, base)
    ranges = [struct.unpack_from("<QQ", buf, base + 32 + 16 * i) for i in range(n)]
    names = buf[base + ranges[0][0] : base + ranges[0][1]].split(b"\0")
    return {names[i - 1].decode(): (base + ranges[i][0], base + ranges[i][1]) for i in range(1, n)}


def vim_levels(path, title):
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
    levels = column("Vim.Element", "index:Vim.Level:Level", "i")
    documents = column("Vim.Element", "index:Vim.BimDocument:BimDocument", "i")
    titles = column("Vim.BimDocument", "string:Title", "i")
    level_elements = column("Vim.Level", "index:Vim.Element:Element", "i")
    descriptors = column("Vim.ParameterDescriptor", "string:Name", "i")
    story_descriptor = next(i for i, n in enumerate(descriptors) if n >= 0 and strings[n] == b"Building Story")
    story = {
        element: strings[value].decode().startswith("1")
        for descriptor, element, value in zip(
            column("Vim.Parameter", "index:Vim.ParameterDescriptor:ParameterDescriptor", "i"),
            column("Vim.Parameter", "index:Vim.Element:Element", "i"),
            column("Vim.Parameter", "string:Value", "i"),
        )
        if descriptor == story_descriptor
    }
    document = next(i for i, t in enumerate(titles) if t >= 0 and strings[t].decode() == title)
    out = {}
    for element_id, level, doc in zip(ids, levels, documents):
        if doc == document and level >= 0:
            level_element = level_elements[level]
            out[element_id] = (strings[names[level_element]].decode(), story.get(level_element))
    return out


def main():
    levels = vim_levels(sys.argv[2], sys.argv[3])
    model = ifcopenshell.open(sys.argv[1])
    storey_of = {}
    for rel in model.by_type("IfcRelContainedInSpatialStructure"):
        structure = rel.RelatingStructure
        for element in rel.RelatedElements:
            if getattr(element, "Tag", None):
                storey_of[element.Tag] = structure.Name if structure.is_a("IfcBuildingStorey") else None
    counts = collections.Counter()
    for element in model.by_type("IfcElement"):
        tag = getattr(element, "Tag", None)
        if not tag or not tag.isdigit():
            continue
        if int(tag) not in levels:
            counts["no Level in the VIM"] += 1
            continue
        name, is_story = levels[int(tag)]
        mine = storey_of.get(tag)
        kind = "VIM Level is a building story" if is_story else "VIM Level is not a building story"
        verdict = "same" if mine == name else ("in the building" if mine is None else "another storey")
        counts[(kind, verdict)] += 1
    storeys = sorted(s.Name for s in model.by_type("IfcBuildingStorey"))
    print(f"storeys: {len(storeys)} {storeys}")
    for key, value in sorted(counts.items(), key=str):
        print(value, key)


if __name__ == "__main__":
    main()
