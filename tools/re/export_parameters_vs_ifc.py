#!/usr/bin/env python3
"""Pair the IFC export parameters rvt-rs reads (RE-45) with what Revit's own export wrote (#792).

Reads the output of `examples/probe_re45_ifc_export_parameters.rs` and Revit's
IFC export of the same file. For each element the probe lists, it finds the
element of the same `Tag` in Revit's export, or the type object of that `Tag`
and the classes of the elements it types, or a space whose Name ends in
`:<id>` (Revit names a family instance it exports as a space that way). It
counts, per "Export to IFC As" value rvt-rs reads, the classes Revit wrote,
with the category Revit's export gives the element (`Other.Category`). Then
it counts the probe's entries by key, value and the category of the element
the owner rule gave them, and lists the entries whose owner Revit wrote as
another class than the value names. `--list` lists every element too.

Usage:

    python3 tools/re/export_parameters_vs_ifc.py <probe output> <revit-export.ifc> [--list]

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import collections
import re
import sys

import ifcopenshell
import ifcopenshell.util.element

ELEMENT = re.compile(
    r"^\s+(\d+)\s+export as (\S+)\s+predefined (\S+)\s+type export as (\S+)\s+type predefined (\S+)$"
)
ENTRY = re.compile(r'^\s+entry (\S+) (0x[0-9a-f]+) (as|type-as) "(.*)" \| owner (.*) \| next (.*)$')
OWNER = re.compile(r"id (\d+) (declared|undeclared) (.*)$")
LEFT_OUT = ("IfcOpeningElement", "IfcVirtualElement")


def category(product):
    """The category Revit's export gives a product, or "-"."""
    for name, properties in ifcopenshell.util.element.get_psets(product).items():
        if name == "Other" and properties.get("Category"):
            return str(properties["Category"])
    return "-"


def revit_view(model):
    """Tag -> what Revit wrote for that id: ("element", class, product), ("type", class, instance classes, product) or ("space", "IfcSpace", product)."""
    out = {}
    for element in model.by_type("IfcElement"):
        if element.Tag and not any(element.is_a(c) for c in LEFT_OUT):
            out.setdefault(str(element.Tag), ("element", element.is_a(), element))
    for type_object in model.by_type("IfcTypeObject"):
        tag = getattr(type_object, "Tag", None)
        if tag and str(tag) not in out:
            typed = collections.Counter(e.is_a() for e in ifcopenshell.util.element.get_types(type_object))
            out[str(tag)] = ("type", type_object.is_a(), typed, type_object)
    for space in model.by_type("IfcSpace"):
        match = re.search(r":(\d+)$", space.Name or "")
        if match:
            out.setdefault(match.group(1), ("space", "IfcSpace", space))
    return out


def describe(seen):
    if seen is None:
        return "not in Revit's export"
    if seen[0] == "type":
        typed = ", ".join(f"{c} {n}" for c, n in sorted(seen[2].items())) or "no elements"
        return f"type {seen[1]} ({typed})"
    return f"{seen[0]} {seen[1]}"


def entity_of(value):
    """The instance entity an export value names: `IfcCoveringType` is `IfcCovering`."""
    value = value.split(".")[0]
    return value[:-4] if value.lower().endswith("type") else value


def main(argv):
    listing = "--list" in argv
    argv = [a for a in argv if a != "--list"]
    if len(argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    elements, entries = [], []
    for line in open(argv[0], errors="replace"):
        line = line.rstrip("\n")
        if m := ELEMENT.match(line):
            elements.append(m.groups())
        elif m := ENTRY.match(line):
            entries.append(m.groups())
    model = ifcopenshell.open(argv[1])
    revit = revit_view(model)
    categories = {}

    def category_of(tag):
        seen = revit.get(tag)
        if seen is None:
            return "-"
        if tag not in categories:
            categories[tag] = category(seen[-1])
        return categories[tag]

    print(f"probe: {len(elements)} elements with export parameters, {len(entries)} entries")
    own = collections.Counter()
    by_type = collections.Counter()
    for tag, export_as, predefined, type_export_as, type_predefined in elements:
        seen = revit.get(tag)
        if export_as != "-":
            own[(export_as, describe(seen), category_of(tag))] += 1
        if type_export_as != "-":
            by_type[(type_export_as, describe(seen), category_of(tag))] += 1
    print("export as (the element's own) | what Revit wrote for its Tag | Revit's category:")
    for (value, seen, cat), n in sorted(own.items(), key=lambda kv: -kv[1]):
        print(f"  {n:6}  {value!r} | {seen} | {cat!r}")
    print("type export as (on a type) | what Revit wrote for its Tag | Revit's category:")
    for (value, seen, cat), n in sorted(by_type.items(), key=lambda kv: -kv[1]):
        print(f"  {n:6}  {value!r} | {seen} | {cat!r}")

    print("entries by key, value and the owner's record category:")
    counted = collections.Counter()
    disagree = []
    for stream, offset, key, value, owner, after in entries:
        parsed = OWNER.search(owner)
        state, record = (parsed.group(2), parsed.group(3)) if parsed else ("no owner", "-")
        counted[(key, value, state, record)] += 1
        if parsed and state == "declared":
            seen = revit.get(parsed.group(1))
            if seen is not None and seen[0] != "type" and seen[1].lower() != entity_of(value).lower():
                disagree.append((stream, offset, key, value, owner, after, describe(seen)))
    for (key, value, state, record), n in sorted(counted.items(), key=lambda kv: -kv[1]):
        print(f"  {n:6}  {key} {value!r} owner {state} {record!r}")
    print(f"entries whose owner Revit wrote as another class: {len(disagree)}")
    for stream, offset, key, value, owner, after, seen in disagree[:200]:
        print(f"  {stream} {offset} {key} {value!r} | owner {owner} | next {after} | Revit: {seen}")

    if listing:
        print("elements:")
        for tag, export_as, predefined, type_export_as, type_predefined in elements:
            print(
                f"  {tag:>9}  as {export_as} {predefined}  type-as {type_export_as} {type_predefined}"
                f"  | {describe(revit.get(tag))} | {category_of(tag)!r}"
            )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
