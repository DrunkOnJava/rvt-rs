#!/usr/bin/env python3
"""Score the placed instances of every BuiltInCategory against a Revit IFC export's Tags (RE-319).

Reads the `any`, `rule` and `category` lines that
`probes/examples/probe_re319_category_census.rs` prints, one `<kind> <id> <name> <count> <ids>`
each per category, and compares the ElementIds with the numeric `Tag` of every element of
the export, openings and virtual elements left out. For each category it prints the
ElementIds of any decoded frame, of the placed instances (the RE-21 rule) and of the
instances rvt-rs's exporter writes (a 3D body, in the export phase, in a primary design
option), how many of each the export holds, and the IFC classes they land on. Then the
export's elements that no written instance covers, by class and by the category whose
frames hold their id.

Usage:

    python3 tools/re/category_census_vs_ifc.py <probe-output.txt> <revit-export.ifc> [--list]

`--list` is what Measure passes to a probe's scorer; it also lists the first ElementIds of each
category that the export lacks.
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.util.element


def parse(path):
    """kind -> category id -> (name, set of ElementIds)."""
    found = {"any": {}, "rule": {}, "category": {}}
    for line in open(path):
        kind = line.split(" ", 1)[0]
        if kind not in found:
            continue
        _, category, name, _count, ids = line.rstrip("\n").split(" ", 4)
        found[kind][int(category)] = (name, {int(i) for i in ids.split(",") if i})
    return found


def category_name(element):
    """The category an export gives an element (`Other.Category`), or "-"."""
    value = ifcopenshell.util.element.get_psets(element).get("Other", {}).get("Category")
    return str(value) if value else "-"


def main(argv):
    listing = "--list" in argv
    argv = [arg for arg in argv if arg != "--list"]
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    found = parse(argv[1])
    tags = {}
    elements = {}
    # The file stays bound while its instances are in use: IfcOpenShell 0.9
    # frees a file its instances outlive (#773).
    model = ifcopenshell.open(argv[2])
    for element in model.by_type("IfcElement"):
        if element.is_a("IfcOpeningElement") or element.is_a("IfcVirtualElement"):
            continue
        tag = getattr(element, "Tag", None)
        if tag and tag.isdigit():
            tags[int(tag)] = element.is_a()
            elements[int(tag)] = element
    print(f"export schema {model.schema}; elements with a numeric Tag: {len(tags)}")
    written = set()
    for _, (_, ids) in found["category"].items():
        written |= ids
    print(
        f"written instances {len(written)}; in the export {len(written & set(tags))}, "
        f"not in it {len(written - set(tags))}, export elements not written {len(set(tags) - written)}"
    )
    print("category | name | any | rule (in export) | written (in export, not in export) | export classes of the written, categories of the instances")
    for category in sorted(found["any"], reverse=True):
        name, any_ids = found["any"][category]
        _, rule_ids = found["rule"].get(category, (name, set()))
        _, wrote = found["category"].get(category, (name, set()))
        classes = collections.Counter(tags[i] for i in wrote if i in tags)
        shown = ", ".join(f"{cls} {n}" for cls, n in sorted(classes.items()))
        names = collections.Counter(
            category_name(elements[i]) for i in rule_ids if i in tags
        )
        shown += "".join(f" | category {name!r} {n}" for name, n in sorted(names.items()))
        print(
            f"{category} | {name} | {len(any_ids)} | {len(rule_ids)} ({len(rule_ids & set(tags))}) | "
            f"{len(wrote)} ({len(wrote & set(tags))}, {len(wrote - set(tags))}) | {shown}"
        )
        if listing and wrote - set(tags):
            print("    not in the export:", ",".join(str(i) for i in sorted(wrote - set(tags))[:20]))
    missed = collections.defaultdict(collections.Counter)
    for tag, cls in tags.items():
        if tag in written:
            continue
        holders = [name for _, (name, ids) in found["any"].items() if tag in ids]
        rule_holders = [name for _, (name, ids) in found["rule"].items() if tag in ids]
        if rule_holders:
            where = "rule:" + "/".join(sorted(set(rule_holders)))
        elif holders:
            where = "frame:" + "/".join(sorted(set(holders)))
        else:
            where = "no frame"
        missed[cls][where] += 1
    print("export elements not written, by IFC class and where rvt-rs holds their id:")
    for cls in sorted(missed):
        total = sum(missed[cls].values())
        parts = ", ".join(f"{where} {n}" for where, n in sorted(missed[cls].items()))
        print(f"  {cls}: {total} ({parts})")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
