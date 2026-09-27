#!/usr/bin/env python3
"""Score rvt-rs's type text parameters (RE-77: Type Mark, Description, Fire
Rating) per instance against the same parameter of the instance's family type
in a VIM export of the model.

Usage: python3 tools/re/type_parameters_vs_vim.py <rvt-rs.ifc> <model.vim>

A VIM is a BFAST container; the reader below is the minimum needed. Needs
IfcOpenShell (tested with 0.8.5)."""
import collections, struct, sys


def bfast(buf, base=0):
    _magic, _start, _end, n = struct.unpack_from("<QQQQ", buf, base)
    ranges = [struct.unpack_from("<QQ", buf, base + 32 + 16 * i) for i in range(n)]
    names = buf[base + ranges[0][0]:base + ranges[0][1]].split(b"\0")
    return {names[i - 1].decode(): (base + ranges[i][0], base + ranges[i][1]) for i in range(1, n)}


class Vim:
    def __init__(self, path):
        self.data = open(path, "rb").read()
        self.top = bfast(self.data)
        self.ent = bfast(self.data, self.top["entities"][0])
        a, b = self.top["strings"]
        self.strings = self.data[a:b].split(b"\0")

    def column(self, table, col):
        a, b = bfast(self.data, self.ent[table][0])[col]
        raw, kind = self.data[a:b], col.split(":")[0]
        if kind == "long":
            return list(struct.unpack("<%dq" % (len(raw) // 8), raw))
        vals = list(struct.unpack("<%di" % (len(raw) // 4), raw))
        if kind == "string":
            return [self.strings[i].decode("utf-8", "replace") if 0 <= i < len(self.strings) else None for i in vals]
        return vals


import ifcopenshell, ifcopenshell.util.element as ue
ours = ifcopenshell.open(sys.argv[1]); v = Vim(sys.argv[2])
el = v.column("Vim.Element", "long:Id")
fi_el = v.column("Vim.FamilyInstance", "index:Vim.Element:Element")
fi_ft = v.column("Vim.FamilyInstance", "index:Vim.FamilyType:FamilyType")
ft_el = v.column("Vim.FamilyType", "index:Vim.Element:Element")
type_of = {el[e]: el[ft_el[t]] for e, t in zip(fi_el, fi_ft) if e >= 0 and t >= 0 and ft_el[t] >= 0}
pd = v.column("Vim.Parameter", "index:Vim.ParameterDescriptor:ParameterDescriptor")
pe = v.column("Vim.Parameter", "index:Vim.Element:Element"); pv = v.column("Vim.Parameter", "string:Value")
dn = v.column("Vim.ParameterDescriptor", "string:Name")
params = collections.defaultdict(dict)
for a, b, c in zip(pd, pe, pv):
    if a >= 0 and b >= 0 and c is not None:
        params[el[b]][dn[a]] = c.split("|", 1)[1].strip() if "|" in c else c
for name in ("Type Mark", "Description", "Fire Rating"):
    compared = equal = no_vim = 0; bad = []
    for e in ours.by_type("IfcElement"):
        if not e.Tag or e.is_a("IfcOpeningElement"): continue
        val = next((p[name] for p in ue.get_psets(e).values() if name in p), None)
        if val is None: continue
        tid = type_of.get(int(e.Tag))
        vim = params.get(tid, {}).get(name) if tid is not None else None
        if vim is None: no_vim += 1; continue
        compared += 1
        if vim == val: equal += 1
        else: bad.append((e.Tag, val, vim))
    print(f"{name:12} ours {compared + no_vim:5}  comparable {compared:5}  equal {equal:5}  no VIM type value {no_vim:5}  differ {len(bad)}, of which VIM empty {sum(1 for b in bad if b[2] == "")}")
