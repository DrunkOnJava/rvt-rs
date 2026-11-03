#!/usr/bin/env python3
"""Which family instances named `IfcSpace` Revit's export wrote, and what tells them apart (#1262, RE-312).

Reads the output of `probes/examples/probe_re312_parking_spaces.rs` and Revit's
IFC export of the same file. Revit names an `IfcSpace` it writes for a family
instance `Family:Type:ElementId`, so an element is written when some
`IFCSPACE` Name ends in `:<its ElementId>`. For every element the probe lists
(each carries "Export to IFC As" `IfcSpace`), it counts how many Revit wrote,
then, for every field the probe printed (the record's, the native instance's
and the instance's type symbol's), how the values split between the elements
Revit wrote and the ones it left out. A field `SEPARATES` when more than one
value occurs and no value occurs in both groups; that is a candidate rule, not
a proof, and it is only worth anything when both groups are large. Fields with
more than 12 distinct values (ids, positions) are summarised.

The IFC is read as text, so nothing here needs IfcOpenShell.

Usage:

    python3 tools/re/parking_spaces_vs_ifc.py <probe output> <revit-export.ifc>
"""

import collections
import re
import sys

SPACE = re.compile(r"=IFCSPACE\('[^']*',#\d+,'([^']*)'")
MANY = 12


def revit_written(ifc_text):
    ids = set()
    for name in SPACE.findall(ifc_text):
        match = re.search(r":(\d+)$", name)
        if match:
            ids.add(match.group(1))
    return ids


def elsewhere(ifc_text, wanted):
    """For ids Revit wrote no space for: the classes whose lines carry the id as a Tag or a Name suffix."""
    found = collections.defaultdict(collections.Counter)
    pattern = re.compile(r"=(IFC[A-Z0-9]+)\(")
    for line in ifc_text.splitlines():
        if not line.startswith("#"):
            continue
        for token in re.findall(r"'(?:[^']*:)?(\d{5,})'", line):
            if token in wanted:
                match = pattern.search(line)
                found[token][match.group(1) if match else "?"] += 1
    return found


def parse(path):
    rows = {"instance": {}, "record": {}, "export": {}, "symbol": {}}
    release = None
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            parts = line.split()
            if not parts:
                continue
            if parts[0] == "release":
                release = parts[1]
                continue
            if parts[0] in rows and len(parts) >= 2:
                fields = {}
                for token in parts[2:]:
                    key, sep, value = token.partition("=")
                    if sep:
                        fields[key] = value
                rows[parts[0]][parts[1]] = fields
    return release, rows


def split_table(title, fields_by_id, written):
    """Print each field's value split between written and left-out elements."""
    keys = sorted({key for fields in fields_by_id.values() for key in fields})
    separating = []
    print(f"-- {title}")
    for key in keys:
        histogram = collections.defaultdict(lambda: [0, 0])
        for element, fields in fields_by_id.items():
            histogram[fields.get(key, "(absent)")][0 if element in written else 1] += 1
        if len(histogram) == 1:
            continue
        both = any(w and n for w, n in histogram.values())
        if len(histogram) > MANY:
            print(f"   {key}: {len(histogram)} distinct values")
            continue
        flag = ""
        if not both and any(w for w, _ in histogram.values()) and any(n for _, n in histogram.values()):
            flag = "  SEPARATES"
            separating.append(key)
        cells = ", ".join(f"{value}: {w} written / {n} left out" for value, (w, n) in sorted(histogram.items()))
        print(f"   {key}{flag}: {cells}")
    return separating


def main():
    probe_path, ifc_path = sys.argv[1:3]
    release, rows = parse(probe_path)
    with open(ifc_path, encoding="utf-8", errors="replace") as handle:
        ifc_text = handle.read()
    written_all = revit_written(ifc_text)
    ids = sorted(rows["export"], key=int)
    written = {i for i in ids if i in written_all}
    print(f"release {release}: {len(ids)} elements carry Export to IFC As IfcSpace; Revit wrote a space for {len(written)}")
    left = [i for i in ids if i not in written]
    where = elsewhere(ifc_text, set(left))
    classes = collections.Counter()
    for element in left:
        classes[",".join(sorted(where[element])) if element in where else "no entity"] += 1
    print(f"   left out: {len(left)}; found elsewhere in Revit's export as: {dict(classes)}")

    named = collections.Counter()
    for name in SPACE.findall(ifc_text):
        match = re.fullmatch(r"(.*):(\d+)", name)
        if match and match.group(2) in rows["export"]:
            named[match.group(1)] += 1
    print(f"   written names (family:type): {dict(named)}")

    symbols = set(rows["symbol"])
    typed = {}
    for element, fields in rows["instance"].items():
        for key, value in fields.items():
            if value in symbols:
                typed[element] = (key, value)
                break
    kinds = collections.Counter()
    for element in ids:
        symbol = typed.get(element, ("-", "-"))[1]
        kinds[(symbol, "written" if element in written else "left out")] += 1
    for (symbol, status), count in sorted(kinds.items()):
        print(f"   type symbol {symbol}: {count} {status}")

    separating = []
    separating += split_table("record (partition frame)", {i: rows["record"].get(i, {}) for i in ids}, written)
    separating += split_table("native instance fields", {i: rows["instance"].get(i, {}) for i in ids}, written)
    separating += split_table("export parameters", {i: rows["export"].get(i, {}) for i in ids}, written)
    symbol_fields = {}
    for element in ids:
        link = typed.get(element)
        symbol_fields[element] = rows["symbol"].get(link[1], {}) if link else {}
    separating += split_table("type symbol fields", symbol_fields, written)
    print(f"fields that separate the written from the left out: {separating if separating else 'none'}")


if __name__ == "__main__":
    main()
