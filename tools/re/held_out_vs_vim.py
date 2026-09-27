#!/usr/bin/env python3
"""Held-out validation of an rvt-rs IFC export against a VIM export (#408).

Compares an rvt-rs IFC of one Revit model with the same model's elements
in a VIM file (vimaec's BIM format, which holds each element's ElementId,
BuiltInCategory, Level and UniqueId per source document):

- which exported elements (by `Tag`) the VIM document holds, and their
  VIM BuiltInCategory per exported IFC entity;
- how many of the VIM's instances with a Level in the categories rvt-rs
  exports are exported;
- each exported element's storey against its VIM Level name;
- each exported element's GlobalId against the one Revit's exporter derives
  from its UniqueId (the episode GUID with its last four bytes XORed with
  the element suffix, compressed to 22 characters).

Needs IfcOpenShell 0.8.5.

Usage:

    python3 tools/re/held_out_vs_vim.py <rvt-rs.ifc> <file.vim> "<VIM document title>"
"""

import collections
import re
import struct
import sys
import uuid

import ifcopenshell
import ifcopenshell.guid
import ifcopenshell.util.element as element_util

SUPPORTED = {
    "OST_PipeCurves", "OST_PipeFitting", "OST_DuctCurves", "OST_DuctFitting",
    "OST_PlumbingFixtures", "OST_DuctTerminal", "OST_LightingFixtures",
    "OST_ElectricalFixtures", "OST_LightingDevices", "OST_MechanicalEquipment",
    "OST_ElectricalEquipment", "OST_FireAlarmDevices", "OST_DataDevices",
    "OST_SpecialityEquipment", "OST_GenericModel", "OST_Walls", "OST_Floors",
}
UNIQUE_ID = re.compile(r"^[0-9a-f]{8}(-[0-9a-f]{4}){3}-[0-9a-f]{12}-[0-9a-f]{8}$")


def bfast(buf, base=0):
    _, _, _, n = struct.unpack_from("<QQQQ", buf, base)
    ranges = [struct.unpack_from("<QQ", buf, base + 32 + 16 * i) for i in range(n)]
    names = buf[base + ranges[0][0]:base + ranges[0][1]].split(b"\0")
    return {names[i - 1].decode(): (base + ranges[i][0], base + ranges[i][1]) for i in range(1, n)}


class Vim:
    def __init__(self, path):
        self.data = open(path, "rb").read()
        self.top = bfast(self.data)
        self.entities = bfast(self.data, self.top["entities"][0])
        a, b = self.top["strings"]
        self.strings = self.data[a:b].split(b"\0")

    def column(self, table, col):
        start, end = bfast(self.data, self.entities[table][0])[col]
        raw = self.data[start:end]
        kind = col.split(":")[0]
        if kind == "long":
            return list(struct.unpack("<%dq" % (len(raw) // 8), raw))
        values = list(struct.unpack("<%di" % (len(raw) // 4), raw))
        if kind == "string":
            return [self.strings[v].decode("utf-8", "replace") if 0 <= v < len(self.strings) else None for v in values]
        return values


def export_guid(unique_id):
    episode, suffix = unique_id[:36], int(unique_id[37:], 16)
    raw = bytearray(uuid.UUID(episode).bytes)
    raw[12:16] = (int.from_bytes(raw[12:16], "big") ^ suffix).to_bytes(4, "big")
    return ifcopenshell.guid.compress(uuid.UUID(bytes=bytes(raw)).hex)


def main():
    ifc_path, vim_path, title = sys.argv[1:4]
    vim = Vim(vim_path)
    col = lambda t, c: vim.column(t, c)
    ids = col("Vim.Element", "long:Id")
    category = col("Vim.Element", "index:Vim.Category:Category")
    document = col("Vim.Element", "index:Vim.BimDocument:BimDocument")
    level = col("Vim.Element", "index:Vim.Level:Level")
    unique = col("Vim.Element", "string:UniqueId")
    names = col("Vim.Element", "string:Name")
    builtin = col("Vim.Category", "string:BuiltInCategory")
    level_element = col("Vim.Level", "index:Vim.Element:Element")
    titles = col("Vim.BimDocument", "string:Title")
    doc = titles.index(title)
    level_name = [names[e] if e >= 0 else None for e in level_element]
    in_doc = [i for i in range(len(ids)) if document[i] == doc]
    vim_cat = {ids[i]: builtin[category[i]] if category[i] >= 0 else None for i in in_doc}
    vim_level = {ids[i]: level_name[level[i]] for i in in_doc if level[i] >= 0}
    vim_guid = {ids[i]: export_guid(unique[i]) for i in in_doc if unique[i] and UNIQUE_ID.match(unique[i])}

    model = ifcopenshell.open(ifc_path)
    ours = {}
    for element in model.by_type("IfcElement"):
        if element.is_a("IfcOpeningElement") or not (element.Tag or "").isdigit():
            continue
        ours.setdefault(int(element.Tag), element)

    by_entity = collections.defaultdict(lambda: [0, 0, collections.Counter()])
    storeys = collections.Counter()
    guids = collections.Counter()
    for tag, element in ours.items():
        row = by_entity[element.is_a()]
        row[0] += 1
        if tag in vim_cat:
            row[1] += 1
            row[2][vim_cat[tag]] += 1
        if tag in vim_level:
            container = element_util.get_container(element)
            name = container.Name if container and container.is_a("IfcBuildingStorey") else None
            storeys["none" if name is None else "same" if name == vim_level[tag] else "other"] += 1
        if tag in vim_guid:
            guids["same" if element.GlobalId == vim_guid[tag] else "different"] += 1

    print(f"{title}: {len(ours)} exported, {sum(r[1] for r in by_entity.values())} in the VIM document")
    for entity, (n, found, cats) in sorted(by_entity.items(), key=lambda kv: -kv[1][0]):
        print(f"  {entity:24} {n:6}  in the VIM {found:6}  VIM categories {dict(cats.most_common(4))}")
    instances = [i for i in vim_cat if vim_cat[i] in SUPPORTED and i in vim_level]
    missing = collections.Counter(vim_cat[i] for i in instances if i not in ours)
    print(f"  VIM instances with a Level in exported categories: {len(instances)}, exported "
          f"{sum(1 for i in instances if i in ours)}; not exported {dict(missing.most_common())}")
    print(f"  storey against VIM Level: {dict(storeys)}")
    print(f"  GlobalId against the UniqueId-derived one: {dict(guids)}")


if __name__ == "__main__":
    main()
